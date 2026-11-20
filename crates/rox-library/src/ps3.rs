//! Jogos de PS3. Todo jogo tem um `PARAM.SFO` (a "identidade" do jogo): título, código
//! (ex.: BLUS30443) e categoria. Ele fica em `PS3_GAME/PARAM.SFO` nos jogos de disco e na raiz
//! da pasta nos jogos da PSN/instalados.
//!
//! Formatos aceitos: pasta do jogo (o usuário escolhe a pasta, o EBOOT.BIN ou o PARAM.SFO)
//! e ISO (precisa estar decifrada ou ter o arquivo .dkey ao lado, exigência do RPCS3).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Lê os campos de um arquivo PARAM.SFO (formato "PSF").
pub fn parse_sfo(data: &[u8]) -> Option<HashMap<String, String>> {
    if data.len() < 20 || &data[0..4] != b"\0PSF" {
        return None;
    }
    let u16_at = |o: usize| u16::from_le_bytes([data[o], data[o + 1]]) as usize;
    let u32_at =
        |o: usize| u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]) as usize;
    let key_table = u32_at(8);
    let data_table = u32_at(12);
    let count = u32_at(16);

    let mut out = HashMap::new();
    for i in 0..count {
        let e = 20 + i * 16;
        if e + 16 > data.len() {
            break;
        }
        let key_off = key_table + u16_at(e);
        let fmt = u16_at(e + 2);
        let len = u32_at(e + 4);
        let val_off = data_table + u32_at(e + 12);
        let key_end = data[key_off..].iter().position(|&b| b == 0)? + key_off;
        let key = String::from_utf8_lossy(&data[key_off..key_end]).into_owned();
        if val_off + len > data.len() {
            continue;
        }
        let raw = &data[val_off..val_off + len];
        let value = match fmt {
            // 0x0204 = texto UTF-8 terminado em zero; 0x0004 = texto sem terminador
            0x0204 | 0x0004 => {
                String::from_utf8_lossy(raw.split(|&b| b == 0).next().unwrap_or(raw))
                    .trim()
                    .to_string()
            }
            // 0x0404 = número de 32 bits
            0x0404 if raw.len() >= 4 => {
                u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]).to_string()
            }
            _ => continue,
        };
        out.insert(key, value);
    }
    Some(out)
}

/// Informações de um jogo de PS3.
#[derive(Debug, Clone, PartialEq)]
pub struct Ps3Game {
    pub title: String,
    /// Código do jogo, ex.: "BLUS30443".
    pub title_id: Option<String>,
    /// Pasta raiz do jogo, ou o arquivo .iso.
    pub root: PathBuf,
}

/// Formata o código como os outros consoles: "BLUS30443" -> "BLUS-30443".
pub fn format_title_id(id: &str) -> String {
    if id.len() == 9 && id[..4].chars().all(|c| c.is_ascii_alphabetic()) {
        format!("{}-{}", &id[..4], &id[4..])
    } else {
        id.to_string()
    }
}

fn from_sfo(data: &[u8], root: PathBuf) -> Option<Ps3Game> {
    let sfo = parse_sfo(data)?;
    let title = sfo.get("TITLE")?.replace(['\n', '\r'], " ");
    Some(Ps3Game {
        title: title.split_whitespace().collect::<Vec<_>>().join(" "),
        title_id: sfo.get("TITLE_ID").map(|id| format_title_id(id)),
        root,
    })
}

/// A partir de qualquer coisa que o usuário escolheu (pasta, EBOOT.BIN, PARAM.SFO, .iso),
/// acha o jogo de PS3. `None` se não for um jogo de PS3.
pub fn identify(path: &Path) -> Option<Ps3Game> {
    let is_iso = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("iso"));
    if is_iso {
        let data = crate::iso9660::read_file(path, &["PS3_GAME", "PARAM.SFO"], 64 * 1024)?;
        return from_sfo(&data, path.to_path_buf());
    }
    // Sobe a partir do arquivo escolhido até achar a raiz do jogo.
    let mut dir = if path.is_dir() {
        Some(path)
    } else {
        path.parent()
    };
    for _ in 0..4 {
        let d = dir?;
        for sfo in [d.join("PS3_GAME").join("PARAM.SFO"), d.join("PARAM.SFO")] {
            if let Ok(data) = std::fs::read(&sfo) {
                // A raiz do jogo é a pasta que contém PS3_GAME/ (disco) ou o PARAM.SFO (PSN).
                // Se a pasta atual já é a PS3_GAME, a raiz é a de cima.
                let in_ps3_game = d
                    .file_name()
                    .is_some_and(|n| n.eq_ignore_ascii_case("PS3_GAME"));
                let root = match (in_ps3_game, d.parent()) {
                    (true, Some(parent)) => parent.to_path_buf(),
                    _ => d.to_path_buf(),
                };
                if let Some(game) = from_sfo(&data, root) {
                    return Some(game);
                }
            }
        }
        // Escolheu algo dentro de PS3_GAME/USRDIR: sobe.
        dir = d.parent();
    }
    None
}

/// O que o RPCS3 precisa receber para iniciar o jogo: o EBOOT.BIN ou o .iso.
pub fn boot_target(root: &Path) -> PathBuf {
    if root.is_file() {
        return root.to_path_buf();
    }
    for p in [
        root.join("PS3_GAME").join("USRDIR").join("EBOOT.BIN"),
        root.join("USRDIR").join("EBOOT.BIN"),
    ] {
        if p.exists() {
            return p;
        }
    }
    root.to_path_buf()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Monta um PARAM.SFO mínimo com campos de texto.
    pub fn fake_sfo(fields: &[(&str, &str)]) -> Vec<u8> {
        let mut keys = Vec::new();
        let mut values = Vec::new();
        let mut entries = Vec::new();
        for (k, v) in fields {
            let key_off = keys.len() as u16;
            keys.extend_from_slice(k.as_bytes());
            keys.push(0);
            let val_off = values.len() as u32;
            let mut val = v.as_bytes().to_vec();
            val.push(0);
            let len = val.len() as u32;
            values.extend_from_slice(&val);
            while values.len() % 4 != 0 {
                values.push(0);
            }
            let mut e = Vec::new();
            e.extend_from_slice(&key_off.to_le_bytes());
            e.extend_from_slice(&0x0204u16.to_le_bytes());
            e.extend_from_slice(&len.to_le_bytes());
            e.extend_from_slice(&len.to_le_bytes());
            e.extend_from_slice(&val_off.to_le_bytes());
            entries.extend(e);
        }
        while keys.len() % 4 != 0 {
            keys.push(0);
        }
        let key_table = 20 + entries.len();
        let data_table = key_table + keys.len();
        let mut out = b"\0PSF".to_vec();
        out.extend_from_slice(&0x0101u32.to_le_bytes());
        out.extend_from_slice(&(key_table as u32).to_le_bytes());
        out.extend_from_slice(&(data_table as u32).to_le_bytes());
        out.extend_from_slice(&(fields.len() as u32).to_le_bytes());
        out.extend(entries);
        out.extend(keys);
        out.extend(values);
        out
    }

    #[test]
    fn le_param_sfo() {
        let sfo = fake_sfo(&[
            ("CATEGORY", "DG"),
            ("TITLE", "Jogo de Teste"),
            ("TITLE_ID", "BLUS30443"),
        ]);
        let m = parse_sfo(&sfo).unwrap();
        assert_eq!(m["TITLE"], "Jogo de Teste");
        assert_eq!(m["TITLE_ID"], "BLUS30443");
        assert_eq!(format_title_id(&m["TITLE_ID"]), "BLUS-30443");
        assert!(parse_sfo(b"nada").is_none());
    }

    #[test]
    fn acha_jogo_em_pasta_a_partir_do_eboot() {
        let root = std::env::temp_dir()
            .join(format!("rox-ps3-{}", std::process::id()))
            .join("Jogo PS3");
        let usrdir = root.join("PS3_GAME").join("USRDIR");
        std::fs::create_dir_all(&usrdir).unwrap();
        std::fs::write(
            root.join("PS3_GAME").join("PARAM.SFO"),
            fake_sfo(&[("TITLE", "Aventura\nEspacial"), ("TITLE_ID", "BCES00001")]),
        )
        .unwrap();
        std::fs::write(usrdir.join("EBOOT.BIN"), b"self").unwrap();

        for chosen in [
            root.clone(),
            usrdir.join("EBOOT.BIN"),
            root.join("PS3_GAME").join("PARAM.SFO"),
        ] {
            let g = identify(&chosen).unwrap();
            assert_eq!(g.title, "Aventura Espacial");
            assert_eq!(g.title_id.as_deref(), Some("BCES-00001"));
            assert_eq!(g.root, root);
        }
        assert_eq!(boot_target(&root), usrdir.join("EBOOT.BIN"));
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }
}
