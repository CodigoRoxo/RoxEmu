//! Baixa a capa de um jogo de PS1 pelo terminal (útil para testar o buscador).
//!
//! `cargo run -p rox-metadata --example fetch_cover -- "Jogo (USA)" "Jogo" NTSC-U pasta-de-saida`

use rox_common::SystemId;
use rox_metadata::CoverFetcher;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [stem, title, region, out] = args.as_slice() else {
        eprintln!("uso: fetch_cover <nome do arquivo> <título> <região> <pasta>");
        std::process::exit(2);
    };
    let fetcher = CoverFetcher::new(out);
    match fetcher.fetch_boxart(SystemId::Ps1, stem, title, Some(region)) {
        Ok(Some(path)) => println!("capa: {}", path.display()),
        Ok(None) => println!("nenhuma capa encontrada"),
        Err(e) => {
            eprintln!("erro: {e}");
            std::process::exit(1);
        }
    }
}
