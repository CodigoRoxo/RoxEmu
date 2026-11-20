//! Baixa a capa de um jogo de PS1 pelo terminal (útil para testar o buscador).
//!
//! `cargo run -p rox-metadata --example fetch_cover -- "Jogo (USA)" "Jogo" NTSC-U pasta-de-saida [ps2]`

use rox_common::SystemId;
use rox_metadata::CoverFetcher;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (stem, title, region, out, system) = match args.as_slice() {
        [s, t, r, o] => (s, t, r, o, SystemId::Ps1),
        [s, t, r, o, sys] if sys == "ps2" => (s, t, r, o, SystemId::Ps2),
        _ => {
            eprintln!("uso: fetch_cover <nome do arquivo> <título> <região> <pasta> [ps2]");
            std::process::exit(2);
        }
    };
    let fetcher = CoverFetcher::new(out);
    match fetcher.fetch_boxart(system, stem, title, Some(region)) {
        Ok(Some(path)) => println!("capa: {}", path.display()),
        Ok(None) => println!("nenhuma capa encontrada"),
        Err(e) => {
            eprintln!("erro: {e}");
            std::process::exit(1);
        }
    }
}
