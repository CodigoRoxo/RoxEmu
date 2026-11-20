//! Inspeciona um arquivo de jogo e descobre tudo que a estante precisa mostrar:
//! console, título, região, formato, tamanho e serial. O usuário só escolhe o arquivo.

use std::path::{Path, PathBuf};

use rox_common::SystemId;
use serde::Serialize;

use crate::{cue, formats::detect_disc_format, ps1, GameFormat};

#[derive(Debug, Clone, Serialize)]
pub struct GameFile {
    pub system: SystemId,
    pub title: String,
    pub region: Option<String>,
    pub format: String,
    pub serial: Option<String>,
    pub size_bytes: u64,
    pub discs: u32,
    /// Arquivo que o core recebe (o .cue, .m3u, .chd...).
    pub launch_path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum InspectError {
    #[error("formato não suportado: {0}")]
    Unsupported(String),
    #[error("não foi possível ler {0}")]
    Read(PathBuf),
}

/// Região pelo código de jogo do PS3 (3ª letra): BLUS -> EUA, BLES -> Europa, BLJM -> Japão.
fn ps3_region(title_id: &str) -> Option<String> {
    let r = match title_id.chars().nth(2)? {
        'U' => "NTSC-U",
        'E' => "PAL",
        'J' | 'A' | 'K' | 'H' => "NTSC-J",
        _ => return None,
    };
    Some(r.to_string())
}

fn dir_size(dir: &Path) -> u64 {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| match e.metadata() {
                    Ok(m) if m.is_dir() => dir_size(&e.path()),
                    Ok(m) => m.len(),
                    Err(_) => 0,
                })
                .sum()
        })
        .unwrap_or(0)
}

/// Jogo de PS3 (pasta, EBOOT.BIN, PARAM.SFO ou ISO com PS3_GAME dentro).
fn inspect_ps3(path: &Path) -> Option<GameFile> {
    let game = crate::ps3::identify(path)?;
    let is_iso = game.root.is_file();
    let size = if is_iso {
        std::fs::metadata(&game.root).map(|m| m.len()).unwrap_or(0)
    } else {
        dir_size(&game.root)
    };
    Some(GameFile {
        system: SystemId::Ps3,
        region: game.title_id.as_deref().and_then(ps3_region),
        format: if is_iso { "ISO".into() } else { "Pasta".into() },
        serial: game.title_id,
        title: game.title,
        size_bytes: size,
        discs: 1,
        launch_path: game.root,
    })
}

/// Descobre console, formato e serial do arquivo. Ordem de confiança para o console:
/// `PARAM.SFO` (PS3) / `SYSTEM.CNF` dentro do disco > extensão exclusiva > nome da pasta > tamanho.
pub fn inspect(path: &Path) -> Result<GameFile, InspectError> {
    // PS3: pasta do jogo, EBOOT.BIN, PARAM.SFO ou ISO de Blu-ray.
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_ascii_uppercase())
        .unwrap_or_default();
    let maybe_ps3 =
        path.is_dir() || name == "EBOOT.BIN" || name == "PARAM.SFO" || name.ends_with(".ISO");
    if maybe_ps3 {
        if let Some(game) = inspect_ps3(path) {
            return Ok(game);
        }
        if path.is_dir() || name == "EBOOT.BIN" || name == "PARAM.SFO" {
            return Err(InspectError::Unsupported(
                "não achei o PARAM.SFO do jogo de PS3 nessa pasta".into(),
            ));
        }
    }

    let format = detect_disc_format(path).ok_or_else(|| {
        InspectError::Unsupported(
            path.extension()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default(),
        )
    })?;
    let meta = std::fs::metadata(path).map_err(|_| InspectError::Read(path.to_path_buf()))?;
    let dir = path.parent().unwrap_or(Path::new("."));
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (title, region) = parse_name(&stem);

    let mut size = meta.len();
    let mut discs = 1;
    let mut disc_id = None;
    let mut inner_system = None;
    let label = match format {
        GameFormat::Cue => {
            let text = std::fs::read_to_string(path)
                .map_err(|_| InspectError::Read(path.to_path_buf()))?;
            let sheet = cue::parse_cue(&text);
            for f in &sheet.files {
                size += std::fs::metadata(dir.join(f)).map(|m| m.len()).unwrap_or(0);
            }
            disc_id = sheet
                .files
                .first()
                .and_then(|f| ps1::identify_disc(&dir.join(f)));
            match sheet.track_count {
                0 | 1 => "BIN/CUE".to_string(),
                n => format!("BIN/CUE · {n} faixas"),
            }
        }
        GameFormat::M3u => {
            let text = std::fs::read_to_string(path)
                .map_err(|_| InspectError::Read(path.to_path_buf()))?;
            let entries: Vec<&str> = text
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect();
            discs = entries.len().max(1) as u32;
            if let Some(first) = entries.first() {
                if let Ok(inner) = inspect(&dir.join(first)) {
                    inner_system = Some(inner.system);
                    disc_id = inner.serial.map(|serial| ps1::DiscId {
                        system: inner.system,
                        serial,
                    });
                }
            }
            format!("M3U · {discs} discos")
        }
        GameFormat::Iso | GameFormat::Bin => {
            disc_id = ps1::identify_disc(path);
            format.label().to_string()
        }
        other => other.label().to_string(),
    };

    let system = disc_id
        .as_ref()
        .map(|d| d.system)
        .or(inner_system)
        .unwrap_or_else(|| guess_system(path, format, size));

    Ok(GameFile {
        system,
        title,
        region,
        format: label,
        serial: disc_id.map(|d| d.serial),
        size_bytes: size,
        discs,
        launch_path: path.to_path_buf(),
    })
}

/// Quando não dá para ler o disco (CHD, PBP, CSO...): decide pela extensão, pasta e tamanho.
fn guess_system(path: &Path, format: GameFormat, size: u64) -> SystemId {
    match format {
        GameFormat::Cso | GameFormat::Gz | GameFormat::Elf => return SystemId::Ps2,
        GameFormat::Pbp => return SystemId::Ps1,
        _ => {}
    }
    let lower = path.to_string_lossy().to_ascii_lowercase();
    let in_folder = |names: &[&str]| {
        lower.split(['\\', '/']).any(|part| {
            names
                .iter()
                .any(|n| part == *n || part.starts_with(&format!("{n} ")))
        })
    };
    if in_folder(&["ps2", "playstation 2", "playstation2"]) {
        return SystemId::Ps2;
    }
    if in_folder(&["ps1", "psx", "psone", "playstation", "playstation 1"]) {
        return SystemId::Ps1;
    }
    // Um CD de PS1 tem no máximo ~800 MB; acima disso é DVD de PS2.
    if size > 900 * 1024 * 1024 {
        SystemId::Ps2
    } else {
        SystemId::Ps1
    }
}

/// "Jogo Legal (USA) (Disc 1)" -> ("Jogo Legal", Some("NTSC-U")).
pub fn parse_name(stem: &str) -> (String, Option<String>) {
    let mut title = String::new();
    let mut region = None;
    let mut depth = 0;
    let mut tag = String::new();
    for c in stem.chars() {
        match c {
            '(' | '[' => {
                depth += 1;
                tag.clear();
            }
            ')' | ']' if depth > 0 => {
                depth -= 1;
                region = region.or_else(|| region_from_tag(&tag));
            }
            _ if depth > 0 => tag.push(c),
            _ => title.push(c),
        }
    }
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    (
        if title.is_empty() {
            stem.to_string()
        } else {
            title
        },
        region,
    )
}

fn region_from_tag(tag: &str) -> Option<String> {
    let t = tag.to_ascii_lowercase();
    let r = if t.contains("usa") || t == "us" || t.contains("ntsc-u") {
        "NTSC-U"
    } else if t.contains("japan") || t == "jp" || t.contains("ntsc-j") {
        "NTSC-J"
    } else if t.contains("europe")
        || t.contains("pal")
        || ["brazil", "france", "germany", "spain", "italy", "uk"]
            .iter()
            .any(|c| t.contains(c))
    {
        "PAL"
    } else {
        return None;
    };
    Some(r.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separa_titulo_e_regiao() {
        assert_eq!(
            parse_name("Jogo Legal (USA) (Disc 1)"),
            ("Jogo Legal".into(), Some("NTSC-U".into()))
        );
        assert_eq!(
            parse_name("Outro [Japan]"),
            ("Outro".into(), Some("NTSC-J".into()))
        );
        assert_eq!(parse_name("Sem Tag"), ("Sem Tag".into(), None));
    }

    #[test]
    fn inspeciona_bin_cue() {
        let dir = std::env::temp_dir().join(format!("rox-inspect-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut bin = vec![0u8; 40_000];
        let cnf = b"BOOT = cdrom:\\SLUS_005.94;1\r\n";
        bin[20_000..20_000 + cnf.len()].copy_from_slice(cnf);
        std::fs::write(dir.join("Teste (USA).bin"), &bin).unwrap();
        std::fs::write(
            dir.join("Teste (USA).cue"),
            "FILE \"Teste (USA).bin\" BINARY\n  TRACK 01 MODE2/2352\n    INDEX 01 00:00:00\n",
        )
        .unwrap();

        let g = inspect(&dir.join("Teste (USA).cue")).unwrap();
        assert_eq!(g.title, "Teste");
        assert_eq!(g.region.as_deref(), Some("NTSC-U"));
        assert_eq!(g.serial.as_deref(), Some("SLUS-00594"));
        assert_eq!(g.format, "BIN/CUE");
        assert_eq!(g.system, SystemId::Ps1);
        assert!(g.size_bytes >= 40_000);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn iso_de_ps2_vira_ps2() {
        let dir = std::env::temp_dir().join(format!("rox-ps2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let iso = crate::iso9660::tests::fake_iso("BOOT2 = cdrom0:\\SLUS_203.12;1\r\n");
        let path = dir.join("Jogo Dois (USA).iso");
        std::fs::write(&path, iso).unwrap();
        let g = inspect(&path).unwrap();
        assert_eq!(g.system, SystemId::Ps2);
        assert_eq!(g.serial.as_deref(), Some("SLUS-20312"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn iso_e_pasta_de_ps3() {
        let dir = std::env::temp_dir().join(format!("rox-ps3g-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sfo =
            crate::ps3::tests::fake_sfo(&[("TITLE", "Corrida Rox"), ("TITLE_ID", "BLES01234")]);
        let iso = crate::iso9660::tests::fake_iso_in_dir("PS3_GAME", "PARAM.SFO", &sfo);
        let path = dir.join("corrida.iso");
        std::fs::write(&path, iso).unwrap();
        let g = inspect(&path).unwrap();
        assert_eq!(g.system, SystemId::Ps3);
        assert_eq!(g.title, "Corrida Rox");
        assert_eq!(g.serial.as_deref(), Some("BLES-01234"));
        assert_eq!(g.region.as_deref(), Some("PAL"));
        assert_eq!(g.format, "ISO");

        let folder = dir.join("Jogo");
        std::fs::create_dir_all(folder.join("PS3_GAME").join("USRDIR")).unwrap();
        std::fs::write(folder.join("PS3_GAME").join("PARAM.SFO"), &sfo).unwrap();
        let g = inspect(&folder).unwrap();
        assert_eq!(g.system, SystemId::Ps3);
        assert_eq!(g.format, "Pasta");
        assert_eq!(g.launch_path, folder);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn chd_usa_pasta_para_decidir() {
        let p = Path::new("E:/RoxEmuGames/ps2/Jogo (USA).chd");
        assert_eq!(guess_system(p, GameFormat::Chd, 10), SystemId::Ps2);
        let p = Path::new("E:/RoxEmuGames/ps1/Jogo (USA).chd");
        assert_eq!(
            guess_system(p, GameFormat::Chd, 2_000_000_000),
            SystemId::Ps1
        );
        let p = Path::new("E:/Jogos/Jogo.chd");
        assert_eq!(
            guess_system(p, GameFormat::Chd, 2_000_000_000),
            SystemId::Ps2
        );
        assert_eq!(
            guess_system(Path::new("x.cso"), GameFormat::Cso, 1),
            SystemId::Ps2
        );
    }
}
