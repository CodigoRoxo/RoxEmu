//! Biblioteca de jogos do usuário, salva em `library.json` na pasta de dados.
//! (Na Fase 2 vira SQLite com scanner de pastas e capas.)

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rox_common::SystemId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryGame {
    pub id: i64,
    pub system: SystemId,
    pub title: String,
    pub region: Option<String>,
    pub serial: Option<String>,
    pub format: String,
    pub discs: u32,
    pub size_bytes: u64,
    pub path: PathBuf,
    pub added_at: u64,
    pub last_played: Option<u64>,
    pub playtime_secs: u64,
    pub favorite: bool,
    /// Imagem da capa (baixada da web ou escolhida pelo usuário).
    #[serde(default)]
    pub cover: Option<PathBuf>,
    /// Quando a busca de capa rodou pela última vez (para não insistir toda hora).
    #[serde(default)]
    pub cover_checked_at: Option<u64>,
    /// Arte de fundo em alta resolução (SteamGridDB).
    #[serde(default)]
    pub hero: Option<PathBuf>,
    /// Logotipo do jogo com fundo transparente.
    #[serde(default)]
    pub logo: Option<PathBuf>,
    /// A arte de fundo é de outro jogo da mesma franquia.
    #[serde(default)]
    pub art_from_franchise: bool,
    #[serde(default)]
    pub art_checked_at: Option<u64>,
    /// Arte quadrada para o ícone da fileira (SteamGridDB).
    #[serde(default)]
    pub icon: Option<PathBuf>,
    /// Versão da busca de artes que já rodou para o jogo. Quando a busca ganha algo novo
    /// (ex.: ícones na versão 2), os jogos antigos são buscados de novo automaticamente.
    #[serde(default)]
    pub art_version: u32,
}

/// Versão atual da busca de artes (2 = hero + logo + ícone quadrado).
pub const ART_VERSION: u32 = 2;

#[derive(Debug, Default, Serialize, Deserialize)]
struct LibraryFile {
    next_id: i64,
    games: Vec<LibraryGame>,
}

pub struct Library {
    file: PathBuf,
    data: LibraryFile,
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

impl Library {
    pub fn open(file: PathBuf) -> Self {
        let data = std::fs::read(&file)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self { file, data }
    }

    fn save(&self) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(&self.data).map_err(|e| e.to_string())?;
        // Grava num arquivo temporário e troca, para nunca deixar a biblioteca pela metade.
        let tmp = self.file.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.file).map_err(|e| e.to_string())
    }

    pub fn games(&self) -> &[LibraryGame] {
        &self.data.games
    }

    pub fn get(&self, id: i64) -> Option<&LibraryGame> {
        self.data.games.iter().find(|g| g.id == id)
    }

    /// Adiciona um jogo. Se o arquivo já está na biblioteca, devolve o existente.
    pub fn add(&mut self, path: &Path) -> Result<LibraryGame, String> {
        if let Some(existing) = self.data.games.iter().find(|g| g.path == path) {
            return Ok(existing.clone());
        }
        let info = rox_library::inspect(path).map_err(|e| e.to_string())?;
        self.data.next_id += 1;
        let game = LibraryGame {
            id: self.data.next_id,
            system: info.system,
            title: info.title,
            region: info.region,
            serial: info.serial,
            format: info.format,
            discs: info.discs,
            size_bytes: info.size_bytes,
            path: info.launch_path,
            added_at: now_secs(),
            last_played: None,
            playtime_secs: 0,
            favorite: false,
            cover: None,
            cover_checked_at: None,
            hero: None,
            logo: None,
            art_from_franchise: false,
            art_checked_at: None,
            icon: None,
            art_version: 0,
        };
        self.data.games.push(game.clone());
        self.save()?;
        Ok(game)
    }

    pub fn remove(&mut self, id: i64) -> Result<(), String> {
        self.data.games.retain(|g| g.id != id);
        self.save()
    }

    pub fn set_favorite(&mut self, id: i64, favorite: bool) -> Result<(), String> {
        if let Some(g) = self.data.games.iter_mut().find(|g| g.id == id) {
            g.favorite = favorite;
        }
        self.save()
    }

    /// Registra o resultado de uma busca de capa (`None` = não achou, mantém a anterior).
    pub fn set_cover(&mut self, id: i64, cover: Option<PathBuf>) -> Result<(), String> {
        if let Some(g) = self.data.games.iter_mut().find(|g| g.id == id) {
            if cover.is_some() {
                g.cover = cover;
            }
            g.cover_checked_at = Some(now_secs());
        }
        self.save()
    }

    /// Registra o resultado da busca de arte de fundo e logo.
    pub fn set_art(&mut self, id: i64, art: rox_metadata::HeroArt) -> Result<(), String> {
        if let Some(g) = self.data.games.iter_mut().find(|g| g.id == id) {
            if art.hero.is_some() {
                g.hero = art.hero;
                g.art_from_franchise = art.from_franchise;
            }
            if art.logo.is_some() {
                g.logo = art.logo;
            }
            if art.icon.is_some() {
                g.icon = art.icon;
            }
            g.art_checked_at = Some(now_secs());
            g.art_version = ART_VERSION;
        }
        self.save()
    }

    /// Soma tempo jogado e marca a última vez.
    pub fn record_session(&mut self, id: i64, played_secs: u64) -> Result<(), String> {
        if let Some(g) = self.data.games.iter_mut().find(|g| g.id == id) {
            g.playtime_secs += played_secs;
            g.last_played = Some(now_secs());
        }
        self.save()
    }
}
