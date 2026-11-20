//! PS3: o ROXemu embute o RPCS3 e configura ele antes de cada jogo — tela cheia, sem a janela
//! principal, fecha sozinho quando o jogo termina, sistema em português e controle mapeado.
//!
//! O PS3 precisa do firmware oficial (PS3UPDAT.PUP), que a Sony oferece de graça no site dela;
//! o ROXemu não distribui, só instala o arquivo que o usuário baixou.

use std::path::{Path, PathBuf};
use std::process::Command;

const EXE: &str = "rpcs3.exe";

/// Página oficial da Sony com o firmware do PS3.
pub const FIRMWARE_PAGE: &str =
    "https://www.playstation.com/pt-br/support/hardware/ps3/system-software/";

/// Controle conectado, informado pela interface (ela vê o controle pela Gamepad API).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadKind {
    DualSense,
    DualShock4,
    Xbox,
    None,
}

impl PadKind {
    pub fn from_ui(s: Option<&str>) -> Self {
        match s {
            Some("dualsense") => PadKind::DualSense,
            Some("ds4") => PadKind::DualShock4,
            Some("xbox") => PadKind::Xbox,
            _ => PadKind::None,
        }
    }
}

pub fn find_dir(exe_dir: &Path) -> Option<PathBuf> {
    let mut candidates = vec![exe_dir.join("emulators").join("rpcs3")];
    if let Some(repo) = option_env!("CARGO_MANIFEST_DIR") {
        candidates.push(Path::new(repo).join("../../../emulators/bin/rpcs3"));
    }
    candidates.into_iter().find(|d| d.join(EXE).exists())
}

/// Versão do firmware instalado ("4.92"), ou `None` se ainda não foi instalado.
pub fn firmware_version(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("dev_flash/vsh/etc/version.txt")).ok()?;
    let release = text
        .lines()
        .find_map(|l| l.strip_prefix("release:"))?
        .split(':')
        .next()?
        .trim()
        .to_string();
    // "04.9200" -> "4.92"
    let mut parts = release.split('.');
    let major = parts.next()?.trim_start_matches('0');
    let minor: String = parts.next().unwrap_or("").chars().take(2).collect();
    Some(format!(
        "{}.{minor}",
        if major.is_empty() { "0" } else { major }
    ))
}

/// Troca `chave: valor` dentro de uma seção de primeiro nível do YAML do RPCS3, preservando o resto.
fn set_yaml(text: &str, section: &str, key: &str, value: &str) -> String {
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let header = format!("{section}:");
    let start = lines.iter().position(|l| l == &header);
    let Some(start) = start else {
        lines.push(header);
        lines.push(format!("  {key}: {value}"));
        return lines.join("\n") + "\n";
    };
    let end = lines[start + 1..]
        .iter()
        .position(|l| !l.is_empty() && !l.starts_with(' '))
        .map_or(lines.len(), |p| start + 1 + p);
    let prefix = format!("  {key}:");
    match lines[start + 1..end]
        .iter()
        .position(|l| l.starts_with(&prefix))
    {
        Some(i) => lines[start + 1 + i] = format!("  {key}: {value}"),
        None => lines.insert(end, format!("  {key}: {value}")),
    }
    lines.join("\n") + "\n"
}

/// Mapeamento do controle do jogador 1 para o RPCS3 (nomes exatos dos botões do RPCS3).
fn input_config(pad: PadKind) -> Option<String> {
    let (handler, device, buttons): (&str, &str, [(&str, &str); 17]) = match pad {
        PadKind::DualSense | PadKind::DualShock4 => {
            let (h, d) = if pad == PadKind::DualSense {
                ("DualSense", "DualSense Pad #1")
            } else {
                ("DualShock 4", "DS4 Pad #1")
            };
            (
                h,
                d,
                [
                    ("Start", "Options"),
                    ("Select", "Share"),
                    ("PS Button", "PS Button"),
                    ("Square", "Square"),
                    ("Cross", "Cross"),
                    ("Circle", "Circle"),
                    ("Triangle", "Triangle"),
                    ("Left", "Left"),
                    ("Down", "Down"),
                    ("Right", "Right"),
                    ("Up", "Up"),
                    ("R1", "R1"),
                    ("R2", "R2"),
                    ("R3", "R3"),
                    ("L1", "L1"),
                    ("L2", "L2"),
                    ("L3", "L3"),
                ],
            )
        }
        PadKind::Xbox => (
            "XInput",
            "XInput Pad #1",
            [
                ("Start", "Start"),
                ("Select", "Back"),
                ("PS Button", "Guide"),
                ("Square", "X"),
                ("Cross", "A"),
                ("Circle", "B"),
                ("Triangle", "Y"),
                ("Left", "Left"),
                ("Down", "Down"),
                ("Right", "Right"),
                ("Up", "Up"),
                ("R1", "RB"),
                ("R2", "RT"),
                ("R3", "RS"),
                ("L1", "LB"),
                ("L2", "LT"),
                ("L3", "LS"),
            ],
        ),
        PadKind::None => return None,
    };
    let mut yml = String::from("# Gerado pelo ROXemu a partir do controle conectado.\n");
    yml.push_str("Player 1 Input:\n");
    // Aspas obrigatórias: em YAML " #" começa um comentário, e "DS4 Pad #1" virava "DS4 Pad".
    yml.push_str(&format!(
        "  Handler: \"{handler}\"\n  Device: \"{device}\"\n  Config:\n"
    ));
    for (stick, axis) in [
        ("Left Stick Left", "LS X-"),
        ("Left Stick Down", "LS Y-"),
        ("Left Stick Right", "LS X+"),
        ("Left Stick Up", "LS Y+"),
        ("Right Stick Left", "RS X-"),
        ("Right Stick Down", "RS Y-"),
        ("Right Stick Right", "RS X+"),
        ("Right Stick Up", "RS Y+"),
    ] {
        yml.push_str(&format!("    {stick}: {axis}\n"));
    }
    for (button, key) in buttons {
        yml.push_str(&format!("    {button}: {key}\n"));
    }
    yml.push_str("  Buddy Device: \"\"\n");
    Some(yml)
}

/// Ajustes conhecidos por jogo (o RPCS3 precisa de configuração especial para alguns títulos).
/// Cada linha: (seção YAML, chave, valor); "Video/Vulkan" é uma subseção.
/// Fontes: fórum e wiki do RPCS3. Quando um jogo novo travar, a correção entra aqui.
fn game_fixes(title_id: &str) -> &'static [(&'static str, &'static str, &'static str)] {
    match title_id {
        // Call of Duty: Black Ops II (EUA/Europa). Configuração recomendada pelo wiki do RPCS3.
        // Atenção: o jogo ainda pode travar ("Hang detected") por um bug do RPCS3 no SPURS
        // (issue RPCS3/rpcs3#16426, aberta); nenhuma configuração resolve isso por completo.
        "BLUS31141" | "BLUS31011" | "BLES01717" | "BLES01718" | "BLES01719" | "BLES01720" => &[
            ("Core", "RSX FIFO Fetch Accuracy", "Atomic"),
            ("Core", "SPU Block Size", "Safe"),
        ],
        _ => &[],
    }
}

/// Monta o YAML de ajustes por jogo (agrupado por seção e subseção).
fn game_fixes_yaml(title_id: &str, fixes: &[(&str, &str, &str)]) -> String {
    let mut out = format!("# Ajustes do ROXemu para {title_id}.\n");
    let mut sections: Vec<&str> = fixes
        .iter()
        .map(|(s, _, _)| s.split('/').next().unwrap_or(s))
        .collect();
    sections.dedup();
    for top in sections {
        out.push_str(&format!("{top}:\n"));
        for (s, k, v) in fixes.iter().filter(|(s, _, _)| *s == top) {
            let _ = s;
            out.push_str(&format!("  {k}: {v}\n"));
        }
        let mut subs: Vec<&str> = fixes
            .iter()
            .filter_map(|(s, _, _)| s.strip_prefix(&format!("{top}/")))
            .collect();
        subs.dedup();
        for sub in subs {
            out.push_str(&format!("  {sub}:\n"));
            let full = format!("{top}/{sub}");
            for (_, k, v) in fixes.iter().filter(|(s, _, _)| *s == full) {
                out.push_str(&format!("    {k}: {v}\n"));
            }
        }
    }
    out
}

/// Escreve (ou remove) o arquivo de ajustes por jogo do RPCS3.
fn apply_game_fixes(config_dir: &Path, serial: Option<&str>) -> Result<(), String> {
    let Some(serial) = serial else { return Ok(()) };
    let title_id: String = serial
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let custom_dir = config_dir.join("custom_configs");
    let path = custom_dir.join(format!("config_{title_id}.yml"));
    // Só mexe em arquivo do ROXemu: ajustes feitos à mão no RPCS3 são respeitados.
    let ours = std::fs::read_to_string(&path)
        .map(|t| t.starts_with("# Ajustes do ROXemu"))
        .unwrap_or(true);
    if !ours {
        return Ok(());
    }
    let fixes = game_fixes(&title_id);
    if fixes.is_empty() {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    }
    std::fs::create_dir_all(&custom_dir).map_err(|e| e.to_string())?;
    log::info!("RPCS3: aplicando ajustes conhecidos para {title_id}");
    std::fs::write(&path, game_fixes_yaml(&title_id, fixes)).map_err(|e| e.to_string())
}

/// Ajusta o RPCS3 para o jogo abrir pronto.
pub fn configure(dir: &Path, pad: PadKind, serial: Option<&str>) -> Result<(), String> {
    apply_game_fixes(&dir.join("config"), serial)?;
    let config_dir = dir.join("config");
    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
    let cfg_path = config_dir.join("config.yml");
    let mut text = std::fs::read_to_string(&cfg_path).unwrap_or_default();
    for (section, key, value) in [
        ("Miscellaneous", "Exit RPCS3 when process finishes", "true"),
        ("Miscellaneous", "Start games in fullscreen mode", "true"),
        (
            "Miscellaneous",
            "Automatically start games after boot",
            "true",
        ),
        (
            "Miscellaneous",
            "Prevent display sleep while running games",
            "true",
        ),
        ("System", "Language", "Portuguese (Brazil)"),
        ("System", "Enter button assignment", "Enter with cross"),
    ] {
        text = set_yaml(&text, section, key, value);
    }
    std::fs::write(&cfg_path, text).map_err(|e| e.to_string())?;

    // Controle: só reescreve se o arquivo for do ROXemu (respeita ajustes feitos no RPCS3).
    let input_dir = config_dir.join("input_configs").join("global");
    let input_path = input_dir.join("Default.yml");
    let ours = std::fs::read_to_string(&input_path)
        .map(|t| t.starts_with("# Gerado pelo ROXemu"))
        .unwrap_or(true);
    if ours {
        match input_config(pad) {
            Some(yml) => {
                std::fs::create_dir_all(&input_dir).map_err(|e| e.to_string())?;
                std::fs::write(&input_path, yml).map_err(|e| e.to_string())?;
            }
            // Sem controle: volta ao padrão do RPCS3 (teclado).
            None => {
                let _ = std::fs::remove_file(&input_path);
            }
        }
    }
    Ok(())
}

/// Abre o jogo: sem a janela principal do RPCS3, em tela cheia.
pub fn command(dir: &Path, target: &Path) -> Command {
    let mut cmd = Command::new(dir.join(EXE));
    cmd.current_dir(dir)
        .arg("--no-gui")
        .arg("--fullscreen")
        .arg(target);
    cmd
}

/// Instala o firmware (abre o RPCS3 mostrando o progresso; o usuário fecha ao terminar).
pub fn install_firmware_command(dir: &Path, pup: &Path) -> Command {
    let mut cmd = Command::new(dir.join(EXE));
    cmd.current_dir(dir).arg("--installfw").arg(pup);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edita_yaml_por_secao() {
        let yml = "Core:\n  PPU Decoder: Recompiler (LLVM)\nMiscellaneous:\n  Exit RPCS3 when process finishes: false\n  Show trophy popups: true\nLog: {}\n";
        let out = set_yaml(
            yml,
            "Miscellaneous",
            "Exit RPCS3 when process finishes",
            "true",
        );
        assert!(
            out.contains("  Exit RPCS3 when process finishes: true\n  Show trophy popups: true")
        );
        let out = set_yaml(&out, "System", "Language", "Portuguese (Brazil)");
        assert!(out.ends_with("System:\n  Language: Portuguese (Brazil)\n"));
        let out = set_yaml(&out, "Miscellaneous", "Nova chave", "1");
        assert!(out.contains("  Show trophy popups: true\n  Nova chave: 1\nLog: {}"));
    }

    #[test]
    fn controle_por_tipo() {
        let ds = input_config(PadKind::DualSense).unwrap();
        assert!(ds.contains("Handler: \"DualSense\"\n  Device: \"DualSense Pad #1\""));
        let ds4 = input_config(PadKind::DualShock4).unwrap();
        assert!(
            ds4.contains("Device: \"DS4 Pad #1\""),
            "o # precisa estar dentro de aspas"
        );
        assert!(ds.contains("    Cross: Cross\n"));
        let xb = input_config(PadKind::Xbox).unwrap();
        assert!(xb.contains("Device: \"XInput Pad #1\""));
        assert!(xb.contains("    Cross: A\n") && xb.contains("    Circle: B\n"));
        assert!(input_config(PadKind::None).is_none());
    }

    #[test]
    fn ajustes_do_black_ops_2() {
        let dir = std::env::temp_dir().join(format!("rox-fix-{}", std::process::id()));
        apply_game_fixes(&dir, Some("BLUS-31141")).unwrap();
        let yml = std::fs::read_to_string(dir.join("custom_configs/config_BLUS31141.yml")).unwrap();
        assert!(yml.contains("Core:\n  RSX FIFO Fetch Accuracy: Atomic\n  SPU Block Size: Safe\n"));
        // Subseções continuam funcionando (ex.: Video/Vulkan).
        let nested = game_fixes_yaml("X", &[("Video", "A", "1"), ("Video/Vulkan", "B", "2")]);
        assert!(nested.contains("Video:\n  A: 1\n  Vulkan:\n    B: 2\n"));
        // Jogo sem ajustes: nenhum arquivo.
        apply_game_fixes(&dir, Some("BLUS30000")).unwrap();
        assert!(!dir.join("custom_configs/config_BLUS30000.yml").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn le_versao_do_firmware() {
        let dir = std::env::temp_dir().join(format!("rox-rpcs3-{}", std::process::id()));
        let etc = dir.join("dev_flash/vsh/etc");
        std::fs::create_dir_all(&etc).unwrap();
        assert_eq!(firmware_version(&dir), None);
        std::fs::write(
            etc.join("version.txt"),
            "release:04.9200:\nbuild:12345,67890:\n",
        )
        .unwrap();
        assert_eq!(firmware_version(&dir).as_deref(), Some("4.92"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
