//! `rox-runner`: roda um jogo numa janela própria.
//!
//! Exemplo:
//! `rox-runner --core cores/bin/swanstation_libretro.dll --game "D:\Jogos\PS1\Jogo (USA).cue" --bios "%APPDATA%\ROXemu\bios"`
//!
//! Atalhos: Esc ou botão Guide = sair · Espaço (segurar) = acelerar · F2 = salvar estado · F4 = carregar estado

mod audio;
mod input;
mod screen;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Context;
use clap::Parser;
use minifb::{Window, WindowOptions};
use rox_common::ipc::RunnerEvent;
use rox_common::DataDirs;
use rox_core_host::{Core, Frontend, FrontendConfig};

#[derive(Parser, Debug)]
#[command(name = "rox-runner", version, about = "Roda um jogo no ROXemu")]
struct Args {
    /// Caminho da .dll do core libretro.
    #[arg(long)]
    core: PathBuf,
    /// Arquivo do jogo (.cue, .chd, .m3u, .pbp, .iso).
    #[arg(long)]
    game: Option<PathBuf>,
    /// Pasta com as BIOS (padrão: pasta bios do ROXemu).
    #[arg(long)]
    bios: Option<PathBuf>,
    /// Pasta de saves e save states (padrão: pastas do ROXemu).
    #[arg(long)]
    saves: Option<PathBuf>,
    /// ID do jogo no banco do launcher (usado nos eventos).
    #[arg(long, default_value_t = 0)]
    game_id: i64,
    /// Só mostra as informações do core e sai.
    #[arg(long)]
    info: bool,
    /// Abre em tela cheia sem bordas.
    #[arg(long)]
    fullscreen: bool,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Stderr)
        .init();

    if let Err(err) = run() {
        log::error!("{err:#}");
        emit(&RunnerEvent::Error {
            message: format!("{err:#}"),
        });
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    let core = Core::load(&args.core).context("falha ao carregar o core")?;

    if args.info {
        println!("core:       {} {}", core.info.name, core.info.version);
        println!("extensões:  {}", core.info.extensions.join(", "));
        println!("fullpath:   {}", core.info.need_fullpath);
        return Ok(());
    }

    let game = args.game.context("informe --game com o arquivo do jogo")?;
    anyhow::ensure!(
        game.exists(),
        "o arquivo do jogo não existe: {}",
        game.display()
    );

    let dirs = DataDirs::resolve()?;
    dirs.ensure()?;
    let bios_dir = args.bios.unwrap_or_else(|| dirs.bios.clone());
    let saves_dir = args.saves.clone().unwrap_or_else(|| dirs.saves.clone());
    let states_dir = args.saves.unwrap_or_else(|| dirs.states.clone());
    std::fs::create_dir_all(&saves_dir)?;
    std::fs::create_dir_all(&states_dir)?;

    let stem = game
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "jogo".into());
    let srm_path = saves_dir.join(format!("{stem}.srm"));
    let state_path = states_dir.join(format!("{stem}.state1"));

    let mut fe = Frontend::new(
        core,
        FrontendConfig {
            system_dir: bios_dir.clone(),
            save_dir: saves_dir.clone(),
            option_overrides: vec![],
        },
    )?;
    fe.load_game(&game).with_context(|| {
        format!(
            "o core não conseguiu abrir o jogo. Confira se a BIOS do PS1 (ex.: scph5501.bin) está em {}",
            bios_dir.display()
        )
    })?;
    load_save_ram(&mut fe, &srm_path);

    let (core_name, core_version) = (fe.core().info.name.clone(), fe.core().info.version.clone());
    emit(&RunnerEvent::Started {
        game_id: args.game_id,
        core: core_name,
        core_version,
    });

    let title = format!("{stem} · ROXemu");
    let mut window = Window::new(
        &title,
        960,
        720,
        WindowOptions {
            resize: true,
            borderless: args.fullscreen,
            scale_mode: minifb::ScaleMode::UpperLeft,
            ..WindowOptions::default()
        },
    )
    .context("falha ao abrir a janela do jogo")?;
    window.set_target_fps(0); // o ritmo é controlado pelo loop abaixo
    window.set_background_color(11, 6, 20);

    let mut audio = match audio::AudioOut::open() {
        Ok(a) => Some(a),
        Err(e) => {
            log::warn!("jogando sem som: {e:#}");
            None
        }
    };
    let mut input = input::Input::new();
    let mut screen = screen::Screen::new();
    let mut samples = Vec::with_capacity(4096);

    let started = Instant::now();
    let mut last_beat = Instant::now();
    let mut next_frame = Instant::now();
    let mut title_reset: Option<Instant> = None;

    while window.is_open() {
        let (buttons, hot) = input.poll(&window);
        if hot.quit {
            break;
        }
        fe.set_input(0, buttons);

        if hot.save_state {
            let msg = match fe.save_state().map(|s| std::fs::write(&state_path, s)) {
                Some(Ok(())) => "Estado salvo (F4 para carregar)".to_string(),
                _ => "Não foi possível salvar o estado".to_string(),
            };
            window.set_title(&format!("{msg} · {title}"));
            title_reset = Some(Instant::now());
        }
        if hot.load_state {
            let ok = std::fs::read(&state_path).is_ok_and(|s| fe.load_state(&s));
            let msg = if ok {
                "Estado carregado"
            } else {
                "Nenhum estado salvo ainda (F2 salva)"
            };
            window.set_title(&format!("{msg} · {title}"));
            title_reset = Some(Instant::now());
        }
        if title_reset.is_some_and(|t| t.elapsed() > Duration::from_secs(3)) {
            window.set_title(&title);
            title_reset = None;
        }

        let av = fe.av_info();
        let fps = if av.timing.fps > 1.0 {
            av.timing.fps
        } else {
            60.0
        };
        let frames = if hot.fast_forward { 4 } else { 1 };
        for _ in 0..frames {
            fe.run_frame();
            fe.take_audio(&mut samples);
            if let (Some(a), false) = (audio.as_mut(), hot.fast_forward) {
                a.push(&samples, av.timing.sample_rate);
            }
            samples.clear();
        }
        for msg in fe.take_messages() {
            window.set_title(&format!("{msg} · {title}"));
            title_reset = Some(Instant::now());
        }

        let (ww, wh) = window.get_size();
        let aspect = av.geometry.aspect_ratio;
        fe.with_frame(|buf, w, h| screen.draw(buf, w, h, ww, wh, aspect));
        if screen.width > 0 {
            window
                .update_with_buffer(&screen.buffer, screen.width, screen.height)
                .context("falha ao desenhar na janela")?;
        } else {
            window.update();
        }

        // Ritmo: o relógio do jogo manda; o áudio é ajustado para acompanhar.
        let frame_time = Duration::from_secs_f64(1.0 / fps);
        next_frame += frame_time;
        let now = Instant::now();
        if next_frame > now {
            std::thread::sleep(next_frame - now);
        } else if now - next_frame > frame_time * 4 {
            next_frame = now; // atrasou muito (ex.: janela arrastada): recomeça a contagem
        }

        if last_beat.elapsed() >= Duration::from_secs(60) {
            last_beat = Instant::now();
            write_save_ram(&mut fe, &srm_path);
            emit(&RunnerEvent::Heartbeat {
                game_id: args.game_id,
                played_secs: started.elapsed().as_secs(),
            });
        }
    }

    write_save_ram(&mut fe, &srm_path);
    emit(&RunnerEvent::Exited {
        game_id: args.game_id,
        played_secs: started.elapsed().as_secs(),
    });
    Ok(())
}

/// Carrega o memory card / save salvo, se existir e tiver o tamanho certo.
fn load_save_ram(fe: &mut Frontend, path: &Path) {
    let Ok(data) = std::fs::read(path) else {
        return;
    };
    match fe.save_ram() {
        Some(ram) if ram.len() == data.len() => {
            ram.copy_from_slice(&data);
            log::info!("save carregado: {}", path.display());
        }
        Some(ram) => log::warn!(
            "save ignorado: tamanho {} diferente do esperado {}",
            data.len(),
            ram.len()
        ),
        None => {}
    }
}

fn write_save_ram(fe: &mut Frontend, path: &Path) {
    if let Some(ram) = fe.save_ram() {
        if let Err(e) = std::fs::write(path, &*ram) {
            log::error!("falha ao gravar o save {}: {e}", path.display());
        }
    }
}

/// Escreve um evento para o launcher (uma linha JSON no stdout).
fn emit(event: &RunnerEvent) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", event.to_line());
    let _ = out.flush();
}
