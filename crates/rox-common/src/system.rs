//! Consoles suportados.
//!
//! Cada console declara os formatos que aceita e o core padrão. Para quem joga, um jogo é
//! sempre só "um jogo": quem resolve BIN/CUE, ISO, CHD, ZIP ou multi-disco é o módulo do console.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemId {
    Ps1,
    Snes,
    Gba,
}

impl SystemId {
    pub const ALL: [SystemId; 3] = [SystemId::Ps1, SystemId::Snes, SystemId::Gba];

    /// Nome exibido na interface.
    pub fn display_name(self) -> &'static str {
        match self {
            SystemId::Ps1 => "PlayStation",
            SystemId::Snes => "Super Nintendo",
            SystemId::Gba => "Game Boy Advance",
        }
    }

    /// Extensões aceitas (minúsculas, sem ponto). A ordem indica a preferência
    /// quando há mais de um arquivo para o mesmo jogo.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            SystemId::Ps1 => &["m3u", "cue", "chd", "pbp", "iso", "bin"],
            SystemId::Snes => &["sfc", "smc", "zip"],
            SystemId::Gba => &["gba", "zip"],
        }
    }

    /// Nome do core libretro padrão (sem sufixo `_libretro.dll`).
    pub fn default_core(self) -> &'static str {
        match self {
            SystemId::Ps1 => "swanstation",
            SystemId::Snes => "snes9x",
            SystemId::Gba => "mgba",
        }
    }

    /// Precisa de BIOS fornecida pelo usuário?
    pub fn requires_bios(self) -> bool {
        matches!(self, SystemId::Ps1)
    }

    /// Nome da pasta usada pelo libretro-thumbnails para buscar capas.
    pub fn thumbnails_repo(self) -> &'static str {
        match self {
            SystemId::Ps1 => "Sony_-_PlayStation",
            SystemId::Snes => "Nintendo_-_Super_Nintendo_Entertainment_System",
            SystemId::Gba => "Nintendo_-_Game_Boy_Advance",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializa_em_minusculas() {
        assert_eq!(serde_json::to_string(&SystemId::Ps1).unwrap(), "\"ps1\"");
    }

    #[test]
    fn ps1_exige_bios() {
        assert!(SystemId::Ps1.requires_bios());
        assert!(!SystemId::Gba.requires_bios());
    }
}
