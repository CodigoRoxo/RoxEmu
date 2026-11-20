//! Consoles suportados.
//!
//! Cada console declara os formatos que aceita, o emulador padrão e se precisa de BIOS. Para
//! quem joga, um jogo é sempre só "um jogo": quem resolve BIN/CUE, ISO, CHD, CSO ou multi-disco
//! é o módulo do console.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SystemId {
    Ps1,
    Ps2,
    Ps3,
    Snes,
    Gba,
}

/// Como o ROXemu roda o console.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Engine {
    /// Core libretro dentro do `rox-runner`.
    Libretro,
    /// Emulador completo embutido (ex.: PCSX2), controlado pelo ROXemu.
    Standalone,
}

impl SystemId {
    pub const ALL: [SystemId; 5] = [
        SystemId::Ps1,
        SystemId::Ps2,
        SystemId::Ps3,
        SystemId::Snes,
        SystemId::Gba,
    ];

    /// Nome exibido na interface.
    pub fn display_name(self) -> &'static str {
        match self {
            SystemId::Ps1 => "PlayStation",
            SystemId::Ps2 => "PlayStation 2",
            SystemId::Ps3 => "PlayStation 3",
            SystemId::Snes => "Super Nintendo",
            SystemId::Gba => "Game Boy Advance",
        }
    }

    /// Extensões aceitas (minúsculas, sem ponto). A ordem indica a preferência
    /// quando há mais de um arquivo para o mesmo jogo.
    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            SystemId::Ps1 => &["m3u", "cue", "chd", "pbp", "iso", "bin"],
            SystemId::Ps2 => &["m3u", "chd", "iso", "cso", "zso", "gz", "cue", "bin", "elf"],
            // Jogo em pasta: escolhe-se o EBOOT.BIN ou o PARAM.SFO de dentro dela.
            SystemId::Ps3 => &["iso", "bin", "sfo"],
            SystemId::Snes => &["sfc", "smc", "zip"],
            SystemId::Gba => &["gba", "zip"],
        }
    }

    pub fn engine(self) -> Engine {
        match self {
            SystemId::Ps2 | SystemId::Ps3 => Engine::Standalone,
            _ => Engine::Libretro,
        }
    }

    /// Core libretro (sem `_libretro.dll`) ou nome do emulador completo (pasta em `emulators/`).
    pub fn default_core(self) -> &'static str {
        match self {
            SystemId::Ps1 => "swanstation",
            SystemId::Ps2 => "pcsx2",
            SystemId::Ps3 => "rpcs3",
            SystemId::Snes => "snes9x",
            SystemId::Gba => "mgba",
        }
    }

    /// Precisa de BIOS fornecida pelo usuário?
    pub fn requires_bios(self) -> bool {
        matches!(self, SystemId::Ps1 | SystemId::Ps2)
    }

    /// O PS3 não usa BIOS, mas precisa do firmware oficial (PS3UPDAT.PUP, do site da Sony).
    pub fn requires_firmware(self) -> bool {
        matches!(self, SystemId::Ps3)
    }

    /// Tamanho exato do arquivo de BIOS, usado para reconhecer a BIOS na pasta.
    pub fn bios_size(self) -> Option<u64> {
        match self {
            SystemId::Ps1 => Some(512 * 1024),
            SystemId::Ps2 => Some(4 * 1024 * 1024),
            _ => None,
        }
    }

    /// Nome da pasta usada pelo libretro-thumbnails para buscar capas.
    pub fn thumbnails_repo(self) -> &'static str {
        match self {
            SystemId::Ps1 => "Sony_-_PlayStation",
            SystemId::Ps2 => "Sony_-_PlayStation_2",
            SystemId::Ps3 => "Sony_-_PlayStation_3",
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
        assert_eq!(serde_json::to_string(&SystemId::Ps2).unwrap(), "\"ps2\"");
    }

    #[test]
    fn bios_e_motor() {
        assert!(SystemId::Ps2.requires_bios());
        assert!(!SystemId::Gba.requires_bios());
        assert_eq!(SystemId::Ps2.engine(), Engine::Standalone);
        assert_eq!(SystemId::Ps1.engine(), Engine::Libretro);
    }
}
