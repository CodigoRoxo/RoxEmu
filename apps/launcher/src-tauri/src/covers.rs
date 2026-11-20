//! Busca de capas em segundo plano. A interface é avisada pelo evento `library-changed`.

use std::path::{Path, PathBuf};

use rox_metadata::CoverFetcher;
use tauri::{AppHandle, Emitter, Manager};

use crate::library::{now_secs, LibraryGame};
use crate::AppState;

#[derive(Clone, serde::Serialize)]
struct CoverMissing {
    title: String,
    error: Option<String>,
}

/// Depois de uma busca sem sucesso, espera 1 dia para tentar de novo sozinho.
const RETRY_AFTER_SECS: u64 = 24 * 3600;

fn needs_cover(g: &LibraryGame) -> bool {
    g.cover.as_ref().is_none_or(|p| !p.exists())
        && g.cover_checked_at
            .is_none_or(|t| now_secs().saturating_sub(t) > RETRY_AFTER_SECS)
}

/// Busca capas que faltam. `only` = só um jogo; `force` = tenta mesmo se já tentou hoje.
pub fn fetch_in_background(app: &AppHandle, only: Option<i64>, force: bool) {
    let app = app.clone();
    std::thread::spawn(move || {
        let state = app.state::<AppState>();
        let todo: Vec<LibraryGame> = match state.library.lock() {
            Ok(lib) => lib
                .games()
                .iter()
                .filter(|g| only.is_none_or(|id| g.id == id))
                .filter(|g| force || needs_cover(g))
                .cloned()
                .collect(),
            Err(_) => return,
        };
        if todo.is_empty() {
            return;
        }
        let fetcher = CoverFetcher::new(&state.data.covers);
        for game in todo {
            let stem = game
                .path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            let (found, error) =
                match fetcher.fetch_boxart(game.system, &stem, &game.title, game.region.as_deref())
                {
                    Ok(found) => (found, None),
                    Err(e) => {
                        log::warn!("capa de {}: {e}", game.title);
                        (None, Some(e.to_string()))
                    }
                };
            let ok = found.is_some();
            if let Ok(mut lib) = state.library.lock() {
                let _ = lib.set_cover(game.id, found);
            }
            let _ = app.emit("library-changed", game.id);
            // Só avisa a interface quando o pedido foi de um jogo específico (não na abertura).
            if only.is_some() && !ok {
                let _ = app.emit(
                    "cover-missing",
                    CoverMissing {
                        title: game.title.clone(),
                        error,
                    },
                );
            }
        }
    });
}

/// Copia uma imagem escolhida pelo usuário para a pasta de capas e devolve o novo caminho.
pub fn import_custom(covers_dir: &Path, game_id: i64, image: &Path) -> Result<PathBuf, String> {
    let ext = image
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .filter(|e| ["png", "jpg", "jpeg", "webp"].contains(&e.as_str()))
        .ok_or("Use uma imagem PNG, JPG ou WEBP.")?;
    let dir = covers_dir.join("custom");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // Nome com horário: força a interface a recarregar mesmo trocando a capa do mesmo jogo.
    let out = dir.join(format!("{game_id}-{}.{ext}", now_secs()));
    std::fs::copy(image, &out).map_err(|e| e.to_string())?;
    Ok(out)
}
