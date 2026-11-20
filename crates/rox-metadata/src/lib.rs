//! Metadados e capas.
//!
//! Fase 0: monta as URLs do libretro-thumbnails (gratuito, sem chave de API).
//! Fase 2: download com cache local, casamento serial -> título pela base Redump,
//! e fontes opcionais (SteamGridDB para hero/logo).

pub mod covers;
pub mod steamgriddb;

pub use covers::CoverFetcher;
use rox_common::SystemId;
use serde::Serialize;
pub use steamgriddb::{HeroArt, SteamGridDb};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtKind {
    BoxArt,
    Snap,
    Title,
}

impl ArtKind {
    fn folder(self) -> &'static str {
        match self {
            ArtKind::BoxArt => "Named_Boxarts",
            ArtKind::Snap => "Named_Snaps",
            ArtKind::Title => "Named_Titles",
        }
    }
}

/// O libretro-thumbnails troca estes caracteres por `_` no nome do arquivo.
pub fn thumbnail_file_name(title: &str) -> String {
    title
        .chars()
        .map(|c| if "&*/:`<>?\\|\"".contains(c) { '_' } else { c })
        .collect()
}

/// URL da imagem no repositório oficial do libretro-thumbnails.
/// `title` deve ser o nome no padrão No-Intro/Redump, ex.: `Jogo (USA)`.
pub fn thumbnail_url(system: SystemId, kind: ArtKind, title: &str) -> String {
    let name = thumbnail_file_name(title);
    format!(
        "https://raw.githubusercontent.com/libretro-thumbnails/{}/master/{}/{}.png",
        system.thumbnails_repo(),
        kind.folder(),
        percent_encode_path(&name)
    )
}

fn percent_encode_path(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'-'
            | b'.'
            | b'_'
            | b'~'
            | b'('
            | b')'
            | b','
            | b'\''
            | b'!' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn troca_caracteres_proibidos() {
        assert_eq!(thumbnail_file_name("A & B: O Jogo?"), "A _ B_ O Jogo_");
    }

    #[test]
    fn monta_url_de_capa() {
        let url = thumbnail_url(SystemId::Ps1, ArtKind::BoxArt, "Jogo Teste (USA)");
        assert_eq!(
            url,
            "https://raw.githubusercontent.com/libretro-thumbnails/Sony_-_PlayStation/master/Named_Boxarts/Jogo%20Teste%20(USA).png"
        );
    }
}
