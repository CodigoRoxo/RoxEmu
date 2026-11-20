//! `rox-runner`: roda um jogo.
//!
//! Fase 0: carrega o core e informa o que ele é (prova de que o pipeline de cores funciona).
//! Fase 1: janela SDL3, áudio, controle e o loop do jogo.
//!
//! Exemplo:
//! `rox-runner --core cores/bin/swanstation_libretro.dll --game "D:\Jogos\PS1\Jogo (USA).cue"`

use std::io::Write;
use std::path::PathBuf;

use anyhow::Context;
use clap::Parser;
use rox_common::ipc::RunnerEvent;
use rox_core_host::Core;

#[derive(Parser, Debug)]
#[command(name = "rox-runner", version, about = "Roda um jogo no ROXemu")]
struct Args {
    /// Caminho da .dll do core libretro.
    #[arg(long)]
    core: PathBuf,
    /// Arquivo do jogo (.cue, .chd, .m3u, ...).
    #[arg(long)]
    game: Option<PathBuf>,
    /// Pasta com as BIOS.
    #[arg(long)]
    bios: Option<PathBuf>,
    /// ID do jogo no banco do launcher (usado nos eventos).
    #[arg(long, default_value_t = 0)]
    game_id: i64,
    /// Só mostra as informações do core e sai.
    #[arg(long)]
    info: bool,
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Stderr)
        .init();

    if let Err(err) = run() {
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

    emit(&RunnerEvent::Started {
        game_id: args.game_id,
        core: core.info.name.clone(),
        core_version: core.info.version.clone(),
    });

    // Fase 1: abrir a janela, carregar o jogo no core e rodar o loop.
    log::warn!("o loop do jogo chega na Fase 1; por enquanto o runner só valida o core");

    emit(&RunnerEvent::Exited {
        game_id: args.game_id,
        played_secs: 0,
    });
    Ok(())
}

/// Escreve um evento para o launcher (uma linha JSON no stdout).
fn emit(event: &RunnerEvent) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{}", event.to_line());
    let _ = out.flush();
}
