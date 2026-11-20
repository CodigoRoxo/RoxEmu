//! PS2: o ROXemu embute o PCSX2 e configura ele antes de cada jogo, para abrir direto em tela
//! cheia, com a BIOS e as pastas de save do ROXemu, sem assistente e sem atualizar sozinho.

use std::path::{Path, PathBuf};
use std::process::Command;

use rox_common::DataDirs;

use crate::ini::Ini;

const EXE: &str = "pcsx2-qt.exe";

/// Botões do DualShock 2 -> controle (SDL). O PCSX2 aceita várias teclas por botão, então o
/// teclado continua funcionando junto.
const PAD_SDL: &[(&str, &str)] = &[
    ("Up", "SDL-0/DPadUp"),
    ("Right", "SDL-0/DPadRight"),
    ("Down", "SDL-0/DPadDown"),
    ("Left", "SDL-0/DPadLeft"),
    ("Triangle", "SDL-0/Y"),
    ("Circle", "SDL-0/B"),
    ("Cross", "SDL-0/A"),
    ("Square", "SDL-0/X"),
    ("Select", "SDL-0/Back"),
    ("Start", "SDL-0/Start"),
    ("L1", "SDL-0/LeftShoulder"),
    ("R1", "SDL-0/RightShoulder"),
    ("L2", "SDL-0/+LeftTrigger"),
    ("R2", "SDL-0/+RightTrigger"),
    ("L3", "SDL-0/LeftStick"),
    ("R3", "SDL-0/RightStick"),
    ("LUp", "SDL-0/-LeftY"),
    ("LRight", "SDL-0/+LeftX"),
    ("LDown", "SDL-0/+LeftY"),
    ("LLeft", "SDL-0/-LeftX"),
    ("RUp", "SDL-0/-RightY"),
    ("RRight", "SDL-0/+RightX"),
    ("RDown", "SDL-0/+RightY"),
    ("RLeft", "SDL-0/-RightX"),
];

/// Onde está o PCSX2. Instalado: `emulators/pcsx2/` ao lado do ROXemu. Em dev: `emulators/bin/pcsx2`.
pub fn find_dir(exe_dir: &Path) -> Option<PathBuf> {
    let mut candidates = vec![exe_dir.join("emulators").join("pcsx2")];
    if let Some(repo) = option_env!("CARGO_MANIFEST_DIR") {
        candidates.push(Path::new(repo).join("../../../emulators/bin/pcsx2"));
    }
    candidates.into_iter().find(|d| d.join(EXE).exists())
}

fn template(exe_dir: &Path) -> Option<String> {
    let mut candidates = vec![exe_dir.join("emulators").join("PCSX2.default.ini")];
    if let Some(repo) = option_env!("CARGO_MANIFEST_DIR") {
        candidates.push(Path::new(repo).join("../../../emulators/pcsx2/PCSX2.default.ini"));
    }
    candidates
        .into_iter()
        .find_map(|p| std::fs::read_to_string(p).ok())
}

/// Ajusta o `inis/PCSX2.ini` (modo portátil) para o jogo abrir pronto.
pub fn configure(
    pcsx2_dir: &Path,
    exe_dir: &Path,
    data: &DataDirs,
    bios_file: &Path,
) -> Result<(), String> {
    let inis = pcsx2_dir.join("inis");
    std::fs::create_dir_all(&inis).map_err(|e| e.to_string())?;
    let ini_path = inis.join("PCSX2.ini");
    let text = std::fs::read_to_string(&ini_path)
        .ok()
        .or_else(|| template(exe_dir))
        .unwrap_or_default();
    let mut ini = Ini::parse(&text);

    let memcards = data.saves.join("ps2");
    let states = data.states.join("ps2");
    for d in [&memcards, &states] {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let p = |path: &Path| path.to_string_lossy().into_owned();

    ini.set("UI", "SetupWizardIncomplete", "false");
    ini.set("UI", "StartFullscreen", "true");
    ini.set("UI", "ConfirmShutdown", "false");
    ini.set("UI", "HideMouseCursor", "true");
    ini.set("AutoUpdater", "CheckAtStartup", "false");
    ini.set("Folders", "Bios", &p(&data.bios));
    ini.set("Folders", "MemoryCards", &p(&memcards));
    ini.set("Folders", "Savestates", &p(&states));
    let bios_name = bios_file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    ini.set("Filenames", "BIOS", &bios_name);

    // Controle: só na primeira vez, para respeitar ajustes que o usuário fizer depois.
    if ini.get("ROXemu", "ControllerSetup").is_none() {
        for (button, sdl) in PAD_SDL {
            let keyboard = ini.get("Pad1", button);
            let mut binds: Vec<&str> = keyboard
                .iter()
                .map(String::as_str)
                .filter(|v| !v.is_empty())
                .collect();
            binds.push(sdl);
            ini.set_all("Pad1", button, &binds);
        }
        // Botão Guide/PS abre o menu de pausa (de onde se sai do jogo), além do Esc.
        ini.set_all(
            "Hotkeys",
            "OpenPauseMenu",
            &["Keyboard/Escape", "SDL-0/Guide"],
        );
        ini.set("ROXemu", "ControllerSetup", "1");
    }

    std::fs::write(&ini_path, ini.to_text()).map_err(|e| e.to_string())
}

/// Monta o comando: sem a janela principal do PCSX2, em tela cheia, boot rápido.
pub fn command(pcsx2_dir: &Path, game: &Path) -> Command {
    let mut cmd = Command::new(pcsx2_dir.join(EXE));
    cmd.current_dir(pcsx2_dir)
        .arg("-nogui")
        .arg("-fullscreen")
        .arg("-fastboot")
        .arg("--")
        .arg(game);
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configura_bios_tela_cheia_e_controle() {
        let root = std::env::temp_dir().join(format!("rox-pcsx2-{}", std::process::id()));
        let pcsx2_dir = root.join("pcsx2");
        std::fs::create_dir_all(pcsx2_dir.join("inis")).unwrap();
        // Começa do modelo real gerado pelo PCSX2 v2.8.2.
        let template = include_str!("../../../../emulators/pcsx2/PCSX2.default.ini");
        std::fs::write(pcsx2_dir.join("inis/PCSX2.ini"), template).unwrap();
        let data = DataDirs::from_root(root.join("data"), true);

        configure(&pcsx2_dir, &root, &data, &data.bios.join("scph39001.bin")).unwrap();
        // Rodar de novo não pode duplicar o mapeamento do controle.
        configure(&pcsx2_dir, &root, &data, &data.bios.join("scph39001.bin")).unwrap();

        let ini = Ini::parse(&std::fs::read_to_string(pcsx2_dir.join("inis/PCSX2.ini")).unwrap());
        assert_eq!(
            ini.get("UI", "SetupWizardIncomplete").as_deref(),
            Some("false")
        );
        assert_eq!(ini.get("UI", "StartFullscreen").as_deref(), Some("true"));
        assert_eq!(
            ini.get("AutoUpdater", "CheckAtStartup").as_deref(),
            Some("false")
        );
        assert_eq!(
            ini.get("Filenames", "BIOS").as_deref(),
            Some("scph39001.bin")
        );
        assert_eq!(
            ini.get("Folders", "Bios"),
            Some(data.bios.to_string_lossy().into_owned())
        );
        let text = ini.to_text();
        assert!(text.contains("Cross = Keyboard/K\r\nCross = SDL-0/A\r\n"));
        assert_eq!(text.matches("Cross = SDL-0/A").count(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}
