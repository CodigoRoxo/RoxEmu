//! Configurações do usuário, salvas em `settings.json` na pasta de dados (só neste PC).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    /// Chave pessoal da API do SteamGridDB (artes de fundo e logos). Nunca vai para log.
    #[serde(default)]
    pub steamgriddb_key: Option<String>,
}

pub struct SettingsStore {
    file: PathBuf,
    pub current: Settings,
}

impl SettingsStore {
    pub fn open(file: PathBuf) -> Self {
        let current = std::fs::read(&file)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        Self { file, current }
    }

    pub fn save(&self) -> Result<(), String> {
        let json = serde_json::to_vec_pretty(&self.current).map_err(|e| e.to_string())?;
        std::fs::write(&self.file, json).map_err(|e| e.to_string())
    }

    pub fn steamgriddb_key(&self) -> Option<String> {
        self.current
            .steamgriddb_key
            .clone()
            .filter(|k| !k.trim().is_empty())
    }
}
