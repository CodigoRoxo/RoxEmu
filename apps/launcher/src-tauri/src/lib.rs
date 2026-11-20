//! Backend do launcher: comandos que a interface chama via `invoke`.

use rox_common::{DataDirs, SystemId};
use serde::Serialize;

#[derive(Serialize)]
struct SystemInfo {
    id: SystemId,
    name: &'static str,
    extensions: &'static [&'static str],
    default_core: &'static str,
    requires_bios: bool,
}

#[derive(Serialize)]
struct AppInfo {
    version: &'static str,
    data: DataDirs,
    systems: Vec<SystemInfo>,
}

/// Versão, pastas de dados e consoles suportados.
#[tauri::command]
fn app_info() -> Result<AppInfo, String> {
    let data = DataDirs::resolve().map_err(|e| e.to_string())?;
    data.ensure().map_err(|e| e.to_string())?;
    let systems = SystemId::ALL
        .iter()
        .map(|&id| SystemInfo {
            id,
            name: id.display_name(),
            extensions: id.extensions(),
            default_core: id.default_core(),
            requires_bios: id.requires_bios(),
        })
        .collect();
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        data,
        systems,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_info])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o ROXemu");
}
