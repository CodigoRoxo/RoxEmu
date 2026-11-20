//! Reconhecimento do formato de um arquivo de jogo.

use std::path::Path;

use rox_common::SystemId;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GameFormat {
    /// Lista de discos (jogos de vários CDs).
    M3u,
    /// Folha de faixas que aponta para um ou mais `.bin`.
    Cue,
    /// Imagem comprimida sem perda (MAME CHD).
    Chd,
    /// Pacote do PSP/PS1 Classics.
    Pbp,
    /// Imagem ISO de 2048 bytes por setor (pode não ter faixas de áudio).
    Iso,
    /// Faixa crua solta, sem `.cue`.
    Bin,
    /// ROM de cartucho.
    Rom,
    /// ROM dentro de um `.zip`.
    Zip,
}

impl GameFormat {
    /// Rótulo curto exibido na interface.
    pub fn label(self) -> &'static str {
        match self {
            GameFormat::M3u => "M3U",
            GameFormat::Cue => "BIN/CUE",
            GameFormat::Chd => "CHD",
            GameFormat::Pbp => "PBP",
            GameFormat::Iso => "ISO",
            GameFormat::Bin => "BIN",
            GameFormat::Rom => "ROM",
            GameFormat::Zip => "ZIP",
        }
    }

    /// Um arquivo deste formato pode ser aberto sozinho, ou é parte de outro
    /// (um `.bin` costuma pertencer a um `.cue`)?
    pub fn is_entry_point(self) -> bool {
        !matches!(self, GameFormat::Bin)
    }
}

/// Descobre o formato pelo nome do arquivo, considerando as extensões aceitas pelo console.
pub fn detect_format(system: SystemId, path: &Path) -> Option<GameFormat> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if !system.extensions().contains(&ext.as_str()) {
        return None;
    }
    Some(match ext.as_str() {
        "m3u" => GameFormat::M3u,
        "cue" => GameFormat::Cue,
        "chd" => GameFormat::Chd,
        "pbp" => GameFormat::Pbp,
        "iso" => GameFormat::Iso,
        "bin" => GameFormat::Bin,
        "zip" => GameFormat::Zip,
        _ => GameFormat::Rom,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reconhece_formatos_do_ps1() {
        let f = |p: &str| detect_format(SystemId::Ps1, Path::new(p));
        assert_eq!(f("Jogo (USA).CUE"), Some(GameFormat::Cue));
        assert_eq!(f("jogo.chd"), Some(GameFormat::Chd));
        assert_eq!(f("jogo.iso"), Some(GameFormat::Iso));
        assert_eq!(f("jogo.gba"), None);
    }

    #[test]
    fn bin_solto_nao_e_ponto_de_entrada() {
        assert!(!GameFormat::Bin.is_entry_point());
        assert!(GameFormat::Cue.is_entry_point());
    }

    #[test]
    fn cartucho_vira_rom() {
        assert_eq!(
            detect_format(SystemId::Snes, Path::new("a.sfc")),
            Some(GameFormat::Rom)
        );
        assert_eq!(
            detect_format(SystemId::Gba, Path::new("a.zip")),
            Some(GameFormat::Zip)
        );
    }
}
