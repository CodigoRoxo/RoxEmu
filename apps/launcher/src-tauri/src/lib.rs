//! Backend do launcher: comandos que a interface chama via `invoke`.

mod art;
mod covers;
mod ini;
mod launch;
mod library;
mod pcsx2;
mod settings;

use std::sync::Mutex;

use rox_common::{DataDirs, SystemId};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use library::{Library, LibraryGame};
use settings::SettingsStore;

pub struct AppState {
    pub data: DataDirs,
    pub library: Mutex<Library>,
    pub settings: Mutex<SettingsStore>,
}

/// O que a interface pode saber das configurações (a chave em si nunca volta para a tela).
#[derive(Serialize)]
struct SettingsView {
    has_steamgriddb_key: bool,
    steamgriddb_key_hint: Option<String>,
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Result<SettingsView, String> {
    let s = state.settings.lock().map_err(|e| e.to_string())?;
    let key = s.steamgriddb_key();
    Ok(SettingsView {
        has_steamgriddb_key: key.is_some(),
        steamgriddb_key_hint: key.map(|k| format!("••••{}", &k[k.len().saturating_sub(4)..])),
    })
}

/// Salva a chave do SteamGridDB (depois de testar) e busca as artes de todos os jogos.
/// Chave vazia = remove.
#[tauri::command]
async fn set_steamgriddb_key(app: AppHandle, key: String) -> Result<(), String> {
    let key = key.trim().to_string();
    let state = app.state::<AppState>();
    if !key.is_empty() {
        rox_metadata::SteamGridDb::new(&key, &state.data.covers)
            .check_key()
            .map_err(|e| e.to_string())?;
    }
    {
        let mut s = state.settings.lock().map_err(|e| e.to_string())?;
        s.current.steamgriddb_key = (!key.is_empty()).then_some(key);
        s.save()?;
    }
    art::fetch_in_background(&app, None, true);
    Ok(())
}

/// Busca de novo a arte de fundo de um jogo.
#[tauri::command]
fn refresh_art(app: AppHandle, id: i64) {
    art::fetch_in_background(&app, Some(id), true);
}

/// Abre no navegador padrão a página para gerar a chave do SteamGridDB (só esse endereço).
#[tauri::command]
fn open_steamgriddb_page() -> Result<(), String> {
    std::process::Command::new("explorer")
        .arg("https://www.steamgriddb.com/profile/preferences/api")
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct SystemInfo {
    id: SystemId,
    name: &'static str,
    extensions: &'static [&'static str],
    default_core: &'static str,
    requires_bios: bool,
    has_bios: bool,
}

#[derive(Serialize)]
struct AppInfo {
    version: &'static str,
    data: DataDirs,
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
            has_bios: launch::has_bios(&state.data.bios, id),
        })
        .collect();
    AppInfo {
        version: env!("CARGO_PKG_VERSION"),
        data: state.data.clone(),
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
    let mut exts: Vec<&str> = [SystemId::Ps1, SystemId::Ps2]
        .iter()
        .flat_map(|s| s.extensions().iter().copied())
        .collect();
    exts.sort_unstable();
    exts.dedup();
    let Some(file) = app
        .dialog()
        .file()
        .set_title("Escolha um jogo de PlayStation ou PlayStation 2")
        .add_filter("Jogos de PS1 e PS2", &exts)
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = file.into_path().map_err(|e| e.to_string())?;
    let game = {
        let state = app.state::<AppState>();
        let mut lib = state.library.lock().map_err(|e| e.to_string())?;
        lib.add(&path)?
    };
    covers::fetch_in_background(&app, Some(game.id), false);
    art::fetch_in_background(&app, Some(game.id), false);
    Ok(Some(game))
}

/// Tenta de novo buscar a capa na internet.
#[tauri::command]
fn refresh_cover(app: AppHandle, id: i64) {
    covers::fetch_in_background(&app, Some(id), true);
}

/// Deixa o usuário escolher uma imagem própria como capa. `false` = cancelou.
#[tauri::command]
async fn choose_cover(app: AppHandle, id: i64) -> Result<bool, String> {
    let Some(file) = app
        .dialog()
        .file()
        .set_title("Escolha a imagem da capa")
        .add_filter("Imagens", &["png", "jpg", "jpeg", "webp"])
        .blocking_pick_file()
    else {
        return Ok(false);
    };
    let image = file.into_path().map_err(|e| e.to_string())?;
    let state = app.state::<AppState>();
    let cover = covers::import_custom(&state.data.covers, id, &image)?;
    state
        .library
        .lock()
        .map_err(|e| e.to_string())?
        .set_cover(id, Some(cover))?;
    Ok(true)
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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let data = DataDirs::resolve().expect("não foi possível achar a pasta de dados do ROXemu");
    data.ensure()
        .expect("não foi possível criar as pastas do ROXemu");
    let library = Library::open(data.root.join("library.json"));
    let settings = SettingsStore::open(data.root.join("settings.json"));
    let covers_dir = data.covers.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            data,
            library: Mutex::new(library),
            settings: Mutex::new(settings),
        })
        .manage(launch::Running::default())
        .setup(move |app| {
            // A interface só pode ler imagens da pasta de capas (nada mais do disco).
            app.asset_protocol_scope()
                .allow_directory(&covers_dir, true)?;
            covers::fetch_in_background(app.handle(), None, false);
            art::fetch_in_background(app.handle(), None, false);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_info,
            list_games,
            add_game,
            remove_game,
            set_favorite,
            launch_game,
            open_bios_folder,
            refresh_cover,
            choose_cover,
            get_settings,
            set_steamgriddb_key,
            refresh_art,
            open_steamgriddb_page
        ])
        .run(tauri::generate_context!())
        .expect("erro ao iniciar o ROXemu");
}
