//! Backend do launcher: comandos que a interface chama via `invoke`.

mod launch;
mod library;

use std::sync::Mutex;

use rox_common::{DataDirs, SystemId};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use library::{Library, LibraryGame};

pub struct AppState {
    pub data: DataDirs,
    pub library: Mutex<Library>,
}

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
    ps1_bios: bool,
    systems: Vec<SystemInfo>,
}

/// Versão, pastas de dados, situação da BIOS e consoles suportados.
#[tauri::command]
fn app_info(state: State<AppState>) -> AppInfo {
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
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        data: state.data.clone(),
        ps1_bios: launch::has_ps1_bios(&state.data.bios),
        systems,
    }
}

#[tauri::command]
fn list_games(state: State<AppState>) -> Result<Vec<LibraryGame>, String> {
    let lib = state.library.lock().map_err(|e| e.to_string())?;
    Ok(lib.games().to_vec())
}

/// Abre o seletor de arquivos e adiciona o jogo escolhido. `None` = usuário cancelou.
#[tauri::command]
async fn add_game(app: AppHandle) -> Result<Option<LibraryGame>, String> {
    let exts = SystemId::Ps1.extensions();
    let Some(file) = app
        .dialog()
        .file()
        .set_title("Escolha um jogo de PlayStation")
        .add_filter("Jogos de PlayStation", exts)
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = file.into_path().map_err(|e| e.to_string())?;
    let state = app.state::<AppState>();
    let mut lib = state.library.lock().map_err(|e| e.to_string())?;
    lib.add(&path).map(Some)
}

#[tauri::command]
fn remove_game(state: State<AppState>, id: i64) -> Result<(), String> {
    state.library.lock().map_err(|e| e.to_string())?.remove(id)
}

#[tauri::command]
fn set_favorite(state: State<AppState>, id: i64, favorite: bool) -> Result<(), String> {
    state
        .library
        .lock()
        .map_err(|e| e.to_string())?
        .set_favorite(id, favorite)
}

/// Abre o jogo no runner. O progresso chega pelo evento `runner`.
#[tauri::command]
fn launch_game(app: AppHandle, id: i64) -> Result<(), String> {
    let game = {
        let state = app.state::<AppState>();
        let lib = state.library.lock().map_err(|e| e.to_string())?;
        lib.get(id)
            .cloned()
            .ok_or("Jogo não encontrado na biblioteca.")?
    };
    launch::launch(&app, game)
}

/// Abre a pasta da BIOS no Explorer.
#[tauri::command]
fn open_bios_folder(state: State<AppState>) -> Result<(), String> {
    std::process::Command::new("explorer")
        .arg(&state.data.bios)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data = DataDirs::resolve().expect("não foi possível achar a pasta de dados do ROXemu");
    data.ensure()
        .expect("não foi possível criar as pastas do ROXemu");
    let library = Library::open(data.root.join("library.json"));

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            data,
            library: Mutex::new(library),
        })
        .manage(launch::Running::default())
        .invoke_handler(tauri::generate_handler![
            app_info,
            list_games,
            add_game,
            remove_game,
            set_favorite,
            launch_game,
            open_bios_folder
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o ROXemu");
}
