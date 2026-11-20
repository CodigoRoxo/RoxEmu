//! Desinstalar um jogo: apaga do disco **para sempre** (sem lixeira) só os arquivos do jogo.
//!
//! Travas de segurança, porque apagar não tem volta:
//! - só apaga o que pertence ao jogo: o arquivo escolhido e os que ele referencia
//!   (.cue -> .bin, .m3u -> discos, .iso -> .dkey ao lado) ou a pasta do jogo de PS3;
//! - uma pasta só é apagada se tiver o PARAM.SFO do PS3 (prova de que é a pasta de um jogo);
//! - nunca apaga a raiz de um disco nem pastas rasas (ex.: `E:\Jogos`);
//! - saves e memory cards ficam (estão na pasta de dados do ROXemu, não no jogo).

use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    File(PathBuf),
    Dir(PathBuf),
}

/// Quantas pastas de verdade há no caminho (sem contar "E:" e "\").
fn depth(p: &Path) -> usize {
    p.components()
        .filter(|c| matches!(c, Component::Normal(_)))
        .count()
}

fn is_ps3_game_dir(dir: &Path) -> bool {
    dir.join("PS3_GAME").join("PARAM.SFO").is_file() || dir.join("PARAM.SFO").is_file()
}

/// Lista o que será apagado para desinstalar o jogo em `path`.
pub fn targets(path: &Path) -> Result<Vec<Target>, String> {
    if !path.is_absolute() {
        return Err("caminho do jogo inválido".into());
    }
    let mut out = Vec::new();
    collect(path, &mut out, 0)?;
    // Revalida tudo antes de devolver.
    for t in &out {
        match t {
            Target::File(f) if depth(f) < 1 => {
                return Err(format!("recusei apagar {}", f.display()))
            }
            Target::Dir(d) if depth(d) < 2 || !is_ps3_game_dir(d) => {
                return Err(format!("recusei apagar a pasta {}", d.display()))
            }
            _ => {}
        }
    }
    out.dedup();
    Ok(out)
}

fn collect(path: &Path, out: &mut Vec<Target>, level: u8) -> Result<(), String> {
    if level > 3 {
        return Ok(());
    }
    if path.is_dir() {
        if !is_ps3_game_dir(path) {
            return Err(format!(
                "a pasta {} não parece ser de um jogo; não vou apagar",
                path.display()
            ));
        }
        out.push(Target::Dir(path.to_path_buf()));
        return Ok(());
    }
    if !path.is_file() {
        return Err(format!(
            "o arquivo do jogo não existe mais: {}",
            path.display()
        ));
    }
    out.push(Target::File(path.to_path_buf()));
    let dir = path.parent().unwrap_or(Path::new(""));
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "cue" => {
            let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            for f in rox_library::cue::parse_cue(&text).files {
                let bin = dir.join(&f);
                // Só arquivos ao lado do .cue (nada de "..\" escapando da pasta).
                if bin.parent() == Some(dir) && bin.is_file() {
                    out.push(Target::File(bin));
                }
            }
        }
        "m3u" => {
            let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            for line in text
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
            {
                let disc = dir.join(line);
                if disc.parent() == Some(dir) && disc.is_file() {
                    collect(&disc, out, level + 1)?;
                }
            }
        }
        "iso" => {
            let dkey = path.with_extension("dkey");
            if dkey.is_file() {
                out.push(Target::File(dkey));
            }
        }
        _ => {}
    }
    Ok(())
}

/// Apaga de verdade (não vai para a lixeira). Devolve quantos bytes foram liberados.
pub fn delete(targets: &[Target]) -> Result<u64, String> {
    let mut freed = 0u64;
    let mut errors = Vec::new();
    for t in targets {
        let (path, result) = match t {
            Target::File(f) => {
                freed += std::fs::metadata(f).map(|m| m.len()).unwrap_or(0);
                (f, std::fs::remove_file(f))
            }
            Target::Dir(d) => {
                freed += dir_size(d);
                (d, std::fs::remove_dir_all(d))
            }
        };
        if let Err(e) = result {
            errors.push(format!("{}: {e}", path.display()));
        }
    }
    if errors.is_empty() {
        Ok(freed)
    } else {
        Err(format!("Não consegui apagar tudo: {}", errors.join("; ")))
    }
}

fn dir_size(dir: &Path) -> u64 {
    std::fs::read_dir(dir)
        .map(|it| {
            it.flatten()
                .map(|e| match e.metadata() {
                    Ok(m) if m.is_dir() => dir_size(&e.path()),
                    Ok(m) => m.len(),
                    Err(_) => 0,
                })
                .sum()
        })
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir()
            .join(format!("rox-unin-{name}-{}", std::process::id()))
            .join("jogos");
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn cue_apaga_os_bins_e_mais_nada() {
        let d = tmp("cue");
        std::fs::write(
            d.join("Jogo (USA).cue"),
            "FILE \"Jogo (USA) (Track 1).bin\" BINARY\nFILE \"Jogo (USA) (Track 2).bin\" BINARY\nFILE \"..\\fora.bin\" BINARY\n",
        )
        .unwrap();
        std::fs::write(d.join("Jogo (USA) (Track 1).bin"), b"1").unwrap();
        std::fs::write(d.join("Jogo (USA) (Track 2).bin"), b"22").unwrap();
        std::fs::write(d.join("outro jogo.bin"), b"nao mexe").unwrap();
        std::fs::write(d.parent().unwrap().join("fora.bin"), b"fora").unwrap();

        let t = targets(&d.join("Jogo (USA).cue")).unwrap();
        assert_eq!(t.len(), 3, "o .cue e os dois .bin dele");
        let freed = delete(&t).unwrap();
        assert!(freed >= 3);
        assert!(!d.join("Jogo (USA) (Track 1).bin").exists());
        assert!(
            d.join("outro jogo.bin").exists(),
            "não pode apagar outro jogo"
        );
        assert!(
            d.parent().unwrap().join("fora.bin").exists(),
            "não pode sair da pasta"
        );
        let _ = std::fs::remove_dir_all(d.parent().unwrap());
    }

    #[test]
    fn pasta_so_se_for_jogo_de_ps3() {
        let d = tmp("ps3");
        let jogo = d.join("Jogo PS3");
        std::fs::create_dir_all(jogo.join("PS3_GAME").join("USRDIR")).unwrap();
        std::fs::write(jogo.join("PS3_GAME").join("PARAM.SFO"), b"\0PSF").unwrap();
        assert_eq!(targets(&jogo).unwrap(), vec![Target::Dir(jogo.clone())]);

        // Pasta qualquer (sem PARAM.SFO): recusa.
        let comum = d.join("Documentos");
        std::fs::create_dir_all(&comum).unwrap();
        assert!(targets(&comum).is_err());
        let _ = std::fs::remove_dir_all(d.parent().unwrap());
    }

    #[test]
    fn iso_leva_o_dkey_junto() {
        let d = tmp("iso");
        std::fs::write(d.join("Jogo.iso"), b"iso").unwrap();
        std::fs::write(d.join("Jogo.dkey"), b"chave").unwrap();
        let t = targets(&d.join("Jogo.iso")).unwrap();
        assert_eq!(t.len(), 2);
        let _ = std::fs::remove_dir_all(d.parent().unwrap());
    }
}
