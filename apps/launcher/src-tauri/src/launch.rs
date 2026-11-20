//! Inicia o `rox-runner` para um jogo e repassa os eventos dele para a interface.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

use rox_common::ipc::RunnerEvent;
use rox_common::{DataDirs, Engine, SystemId};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::library::{now_secs, LibraryGame};
use crate::{pcsx2, AppState};

/// Evento enviado para a interface (`listen("runner")`).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiRunnerEvent {
    Started { game_id: i64 },
    Exited { game_id: i64, played_secs: u64 },
    Error { game_id: i64, message: String },
}

/// Qual jogo está rodando agora (só um por vez).
#[derive(Default)]
pub struct Running(pub Mutex<Option<i64>>);

/// Acha um arquivo com cara de BIOS do console na pasta (pelo tamanho exato).
/// Prefere nomes no padrão "scph..." quando houver mais de um.
pub fn find_bios(dir: &Path, system: SystemId) -> Option<PathBuf> {
    let size = system.bios_size()?;
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.metadata().is_ok_and(|m| m.is_file() && m.len() == size))
        .map(|e| e.path())
        .collect();
    found.sort_by_key(|p| {
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        (!name.starts_with("scph"), name)
    });
    found.into_iter().next()
}

pub fn has_bios(dir: &Path, system: SystemId) -> bool {
    !system.requires_bios() || find_bios(dir, system).is_some()
}

fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_default()
}

/// Onde está o `rox-runner.exe`. Instalado: ao lado do ROXemu. Em desenvolvimento: `target/`.
fn find_runner() -> Option<PathBuf> {
    let dir = exe_dir();
    let mut candidates = vec![dir.join("rox-runner.exe")];
    if dir.ends_with("debug") {
        // Em dev o runner em release roda bem melhor; usa ele se existir.
        candidates.insert(0, dir.with_file_name("release").join("rox-runner.exe"));
    }
    candidates.into_iter().find(|p| p.exists())
}

/// Onde está a .dll do core. Instalado: `cores/` ao lado do ROXemu. Em dev: `cores/bin` do repo.
fn find_core(data: &DataDirs, name: &str) -> Option<PathBuf> {
    let file = format!("{name}_libretro.dll");
    let mut candidates = vec![exe_dir().join("cores").join(&file), data.cores.join(&file)];
    if let Some(repo) = option_env!("CARGO_MANIFEST_DIR") {
        candidates.push(Path::new(repo).join("../../../cores/bin").join(&file));
    }
    candidates.into_iter().find(|p| p.exists())
}

pub fn launch(app: &AppHandle, game: LibraryGame) -> Result<(), String> {
    let state = app.state::<AppState>();
    let running = app.state::<Running>();
    {
        let mut slot = running.0.lock().map_err(|e| e.to_string())?;
        if slot.is_some() {
            return Err("Já tem um jogo aberto. Feche ele antes de abrir outro.".into());
        }
        *slot = Some(game.id);
    }
    let result = spawn(app, &state.data, &game);
    if result.is_err() {
        if let Ok(mut slot) = running.0.lock() {
            *slot = None;
        }
    }
    result
}

fn spawn(app: &AppHandle, data: &DataDirs, game: &LibraryGame) -> Result<(), String> {
    if !game.path.exists() {
        return Err(format!(
            "O arquivo do jogo não foi encontrado: {}",
            game.path.display()
        ));
    }
    if !has_bios(&data.bios, game.system) {
        let (console, example) = match game.system {
            SystemId::Ps2 => ("PS2", "scph39001.bin, 4 MB"),
            _ => ("PS1", "scph5501.bin, 512 KB"),
        };
        return Err(format!(
            "Falta a BIOS do {console}. Coloque o arquivo da BIOS do seu console (ex.: {example}) na pasta {}",
            data.bios.display()
        ));
    }
    if game.system.engine() == Engine::Standalone {
        return spawn_standalone(app, data, game);
    }
    let runner = find_runner().ok_or("O rox-runner não foi encontrado junto do ROXemu.")?;
    let core_name = game.system.default_core();
    let core = find_core(data, core_name)
        .ok_or_else(|| format!("O emulador {core_name} não foi encontrado na pasta de cores."))?;

    let log_path = data.root.join("runner.log");
    let log = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;

    let mut cmd = Command::new(&runner);
    cmd.arg("--core")
        .arg(&core)
        .arg("--game")
        .arg(&game.path)
        .arg("--bios")
        .arg(&data.bios)
        .arg("--game-id")
        .arg(game.id.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(log));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    log::info!("iniciando {} com {}", game.title, core.display());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Não foi possível abrir o runner: {e}"))?;
    let stdout = child.stdout.take().ok_or("sem stdout do runner")?;

    let app = app.clone();
    let game_id = game.id;
    let started_at = now_secs();
    std::thread::spawn(move || {
        let mut finished = false;
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let Ok(ev) = RunnerEvent::from_line(&line) else {
                continue;
            };
            match ev {
                RunnerEvent::Started { .. } => {
                    let _ = app.emit("runner", UiRunnerEvent::Started { game_id });
                }
                RunnerEvent::Heartbeat { .. } => {}
                RunnerEvent::Exited { played_secs, .. } => {
                    finished = true;
                    record(&app, game_id, played_secs);
                    let _ = app.emit(
                        "runner",
                        UiRunnerEvent::Exited {
                            game_id,
                            played_secs,
                        },
                    );
                }
                RunnerEvent::Error { message } => {
                    finished = true;
                    let _ = app.emit("runner", UiRunnerEvent::Error { game_id, message });
                }
            }
        }
        let status = child.wait();
        if !finished {
            let played = now_secs().saturating_sub(started_at);
            record(&app, game_id, played);
            let message = format!(
                "O jogo fechou inesperadamente ({}). Detalhes em {}",
                status.map(|s| s.to_string()).unwrap_or_default(),
                log_path.display()
            );
            let _ = app.emit("runner", UiRunnerEvent::Error { game_id, message });
        }
        if let Ok(mut slot) = app.state::<Running>().0.lock() {
            *slot = None;
        }
        // Traz o ROXemu de volta para a frente quando o jogo fecha.
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.set_focus();
        }
    });
    Ok(())
}

/// Abre um emulador completo (PS2 = PCSX2) e acompanha até ele fechar.
fn spawn_standalone(app: &AppHandle, data: &DataDirs, game: &LibraryGame) -> Result<(), String> {
    let exe_dir = exe_dir();
    let dir = pcsx2::find_dir(&exe_dir)
        .ok_or("O PCSX2 não foi encontrado junto do ROXemu. Rode emulators/fetch-emulators.ps1.")?;
    let bios = find_bios(&data.bios, game.system).ok_or("BIOS do PS2 não encontrada.")?;
    pcsx2::configure(&dir, &exe_dir, data, &bios)?;

    log::info!("iniciando {} com PCSX2 ({})", game.title, bios.display());
    let mut child = pcsx2::command(&dir, &game.path)
        .spawn()
        .map_err(|e| format!("Não foi possível abrir o PCSX2: {e}"))?;
    let _ = app.emit("runner", UiRunnerEvent::Started { game_id: game.id });

    let app = app.clone();
    let game_id = game.id;
    let started = std::time::Instant::now();
    std::thread::spawn(move || {
        let status = child.wait();
        let played = started.elapsed().as_secs();
        record(&app, game_id, played);
        let failed_fast = played < 5 && status.as_ref().map_or(true, |s| !s.success());
        let event = if failed_fast {
            UiRunnerEvent::Error {
                game_id,
                message: format!(
                    "O PCSX2 fechou logo ao abrir ({}). Confira se a BIOS do PS2 e o arquivo do jogo estão certos. Detalhes em {}",
                    status.map(|s| s.to_string()).unwrap_or_default(),
                    dir.join("logs").display()
                ),
            }
        } else {
            UiRunnerEvent::Exited {
                game_id,
                played_secs: played,
            }
        };
        let _ = app.emit("runner", event);
        if let Ok(mut slot) = app.state::<Running>().0.lock() {
            *slot = None;
        }
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.set_focus();
        }
    });
    Ok(())
}

fn record(app: &AppHandle, game_id: i64, played_secs: u64) {
    if let Ok(mut lib) = app.state::<AppState>().library.lock() {
        if let Err(e) = lib.record_session(game_id, played_secs) {
            log::error!("falha ao salvar tempo de jogo: {e}");
        }
    }
}
