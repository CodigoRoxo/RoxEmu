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
use crate::rpcs3::{self, PadKind};
use crate::{pcsx2, session, AppState};

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

/// É a BIOS principal do PS2? Não basta o tamanho: os extratores de BIOS geram vários arquivos
/// de 4 MB (ROM0 = BIOS, ROM1 = leitor de DVD, DIFF...). A BIOS principal tem a tabela interna
/// `RESET` + `ROMDIR` depois do código de boot (por volta de 0x2700); a ROM1 tem logo no início.
pub fn is_ps2_main_bios(path: &Path) -> bool {
    use std::io::Read;
    let mut buf = Vec::new();
    if std::fs::File::open(path)
        .and_then(|f| f.take(1024 * 1024).read_to_end(&mut buf))
        .is_err()
    {
        return false;
    }
    buf.windows(16).enumerate().any(|(i, w)| {
        i >= 0x1000 && w.starts_with(b"RESET\0") && buf[i + 16..].starts_with(b"ROMDIR")
    })
}

/// Acha a BIOS do console na pasta. PS1: pelo tamanho (512 KB). PS2: pelo conteúdo.
/// Prefere nomes no padrão "scph..." quando houver mais de uma.
pub fn find_bios(dir: &Path, system: SystemId) -> Option<PathBuf> {
    let size = system.bios_size()?;
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.metadata().is_ok_and(|m| m.is_file() && m.len() == size))
        .map(|e| e.path())
        .filter(|p| system != SystemId::Ps2 || is_ps2_main_bios(p))
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

/// Pasta do executável do ROXemu (onde ficam os emuladores na instalação).
pub fn exe_dir_pub() -> PathBuf {
    exe_dir()
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

pub fn launch(app: &AppHandle, game: LibraryGame, pad: PadKind) -> Result<(), String> {
    let state = app.state::<AppState>();
    let running = app.state::<Running>();
    {
        let mut slot = running.0.lock().map_err(|e| e.to_string())?;
        if slot.is_some() {
            return Err("Já tem um jogo aberto. Feche ele antes de abrir outro.".into());
        }
        *slot = Some(game.id);
    }
    let result = spawn(app, &state.data, &game, pad);
    if result.is_err() {
        if let Ok(mut slot) = running.0.lock() {
            *slot = None;
        }
    }
    result
}

fn spawn(app: &AppHandle, data: &DataDirs, game: &LibraryGame, pad: PadKind) -> Result<(), String> {
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
        let prepared = prepare_standalone(data, game, pad)?;
        return watch_standalone(app, game, prepared);
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
    session::set_pid(app, Some(child.id()));
    session::watch_close_combo(app, child.id());

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
        let by_user = session::take_stop_requested();
        if !finished {
            let played = now_secs().saturating_sub(started_at);
            record(&app, game_id, played);
            let event = if by_user {
                UiRunnerEvent::Exited {
                    game_id,
                    played_secs: played,
                }
            } else {
                UiRunnerEvent::Error {
                    game_id,
                    message: format!(
                        "O jogo fechou inesperadamente ({}). Detalhes em {}",
                        status.map(|s| s.to_string()).unwrap_or_default(),
                        log_path.display()
                    ),
                }
            };
            let _ = app.emit("runner", event);
        }
        session::set_pid(&app, None);
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

/// Um emulador completo pronto para abrir o jogo.
struct Prepared {
    name: &'static str,
    command: Command,
    /// Dica de erro se o emulador fechar logo ao abrir.
    hint: String,
}

/// Configura o emulador completo do console (PS2 = PCSX2, PS3 = RPCS3) e monta o comando.
fn prepare_standalone(
    data: &DataDirs,
    game: &LibraryGame,
    pad: PadKind,
) -> Result<Prepared, String> {
    let exe_dir = exe_dir();
    match game.system {
        SystemId::Ps3 => {
            let dir = rpcs3::find_dir(&exe_dir).ok_or(
                "O RPCS3 não foi encontrado junto do ROXemu. Rode emulators/fetch-emulators.ps1.",
            )?;
            if rpcs3::firmware_version(&dir).is_none() {
                return Err("Falta o firmware do PS3. Baixe o PS3UPDAT.PUP no site oficial da Sony e instale em Configurações → PlayStation 3.".into());
            }
            rpcs3::configure(&dir, pad, game.serial.as_deref())?;
            let target = rox_library::ps3::boot_target(&game.path);
            log::info!("iniciando {} com RPCS3 ({})", game.title, target.display());
            Ok(Prepared {
                name: "RPCS3",
                command: rpcs3::command(&dir, &target),
                hint: format!(
                    "Se for ISO, ela precisa estar decifrada ou ter o arquivo .dkey ao lado. Detalhes em {}",
                    dir.join("log").display()
                ),
            })
        }
        _ => {
            let dir = pcsx2::find_dir(&exe_dir).ok_or(
                "O PCSX2 não foi encontrado junto do ROXemu. Rode emulators/fetch-emulators.ps1.",
            )?;
            let bios = find_bios(&data.bios, game.system).ok_or("BIOS do PS2 não encontrada.")?;
            pcsx2::configure(&dir, &exe_dir, data, &bios)?;
            log::info!("iniciando {} com PCSX2 ({})", game.title, bios.display());
            Ok(Prepared {
                name: "PCSX2",
                command: pcsx2::command(&dir, &game.path),
                hint: format!(
                    "Confira se a BIOS do PS2 e o arquivo do jogo estão certos. Detalhes em {}",
                    dir.join("logs").display()
                ),
            })
        }
    }
}

/// Abre o emulador completo e acompanha até ele fechar (conta o tempo de jogo).
fn watch_standalone(app: &AppHandle, game: &LibraryGame, prepared: Prepared) -> Result<(), String> {
    let Prepared {
        name,
        mut command,
        hint,
    } = prepared;
    let mut child = command
        .spawn()
        .map_err(|e| format!("Não foi possível abrir o {name}: {e}"))?;
    let _ = app.emit("runner", UiRunnerEvent::Started { game_id: game.id });
    session::set_pid(app, Some(child.id()));
    session::watch_close_combo(app, child.id());

    let app = app.clone();
    let game_id = game.id;
    let started = std::time::Instant::now();
    std::thread::spawn(move || {
        let status = child.wait();
        let by_user = session::take_stop_requested();
        session::set_pid(&app, None);
        let played = started.elapsed().as_secs();
        record(&app, game_id, played);
        let failed_fast = !by_user && played < 5 && status.as_ref().map_or(true, |s| !s.success());
        let event = if failed_fast {
            UiRunnerEvent::Error {
                game_id,
                message: format!(
                    "O {name} fechou logo ao abrir ({}). {hint}",
                    status.map(|s| s.to_string()).unwrap_or_default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rom(reset_at: usize) -> Vec<u8> {
        let mut b = vec![0u8; 4 * 1024 * 1024];
        b[reset_at..reset_at + 6].copy_from_slice(b"RESET\0");
        b[reset_at + 16..reset_at + 22].copy_from_slice(b"ROMDIR");
        b
    }

    #[test]
    fn escolhe_rom0_e_ignora_diff_e_rom1() {
        let dir = std::env::temp_dir().join(format!("rox-bios-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("SCPH-90001.DIFF"), vec![7u8; 4 * 1024 * 1024]).unwrap();
        std::fs::write(dir.join("SCPH-90001.ROM1"), rom(16)).unwrap();
        std::fs::write(dir.join("SCPH-90001.ROM0"), rom(0x2780)).unwrap();
        std::fs::write(dir.join("SCPH1001.BIN"), vec![0u8; 512 * 1024]).unwrap();

        let ps2 = find_bios(&dir, SystemId::Ps2).unwrap();
        assert!(ps2.to_string_lossy().ends_with("SCPH-90001.ROM0"));
        let ps1 = find_bios(&dir, SystemId::Ps1).unwrap();
        assert!(ps1.to_string_lossy().ends_with("SCPH1001.BIN"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
