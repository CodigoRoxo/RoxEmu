//! Artes de fundo (hero 2K/4K) e logos do SteamGridDB, buscadas em segundo plano.
//! Só funciona com a chave do usuário configurada; sem ela, a estante usa a capa desfocada.

use rox_metadata::steamgriddb::file_key;
use rox_metadata::SteamGridDb;
use tauri::{AppHandle, Emitter, Manager};

use crate::library::{now_secs, LibraryGame};
use crate::AppState;

/// Sem arte encontrada: tenta de novo sozinho depois de 7 dias.
const RETRY_AFTER_SECS: u64 = 7 * 24 * 3600;

fn needs_art(g: &LibraryGame) -> bool {
    g.hero.as_ref().is_none_or(|p| !p.exists())
        && g.art_checked_at
            .is_none_or(|t| now_secs().saturating_sub(t) > RETRY_AFTER_SECS)
}

/// Busca artes que faltam. `only` = só um jogo; `force` = ignora o "já tentei".
pub fn fetch_in_background(app: &AppHandle, only: Option<i64>, force: bool) {
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let Some(key) = state.settings.lock().ok().and_then(|s| s.steamgriddb_key()) else {
            return;
        };
        let todo: Vec<LibraryGame> = match state.library.lock() {
            Ok(lib) => lib
                .games()
                .iter()
                .filter(|g| only.is_none_or(|id| g.id == id))
                .filter(|g| force || needs_art(g))
                .cloned()
                .collect(),
            Err(_) => return,
        };
        let sgdb = SteamGridDb::new(&key, &state.data.covers);
        for game in todo {
            let _ = app.emit("art-loading", game.id);
            let system = serde_json::to_value(game.system)
                .ok()
                .and_then(|v| v.as_str().map(str::to_string))
                .unwrap_or_default();
            let art = match sgdb.fetch(&game.title, &file_key(&system, &game.title)) {
                Ok(art) => art,
                Err(e) => {
                    log::warn!("arte de {}: {e}", game.title);
                    let _ = app.emit("art-error", e.to_string());
                    if e.to_string().contains("chave") {
                        return; // chave inválida: não adianta tentar os outros jogos
                    }
                    Default::default()
                }
            };
            if let Ok(mut lib) = state.library.lock() {
                let _ = lib.set_art(game.id, art.hero, art.logo, art.from_franchise);
            }
            let _ = app.emit("library-changed", game.id);
        }
    });
}
