//! Pastas de dados do usuário.
//!
//! Padrão: `%APPDATA%\ROXemu\`. Modo portátil: uma pasta `portable-data` ao lado do
//! executável (ativado quando existe um arquivo `portable.txt` ao lado do .exe).

use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DataDirs {
    pub root: PathBuf,
    pub bios: PathBuf,
    pub saves: PathBuf,
    pub states: PathBuf,
    pub covers: PathBuf,
    pub cores: PathBuf,
    pub database: PathBuf,
    pub config: PathBuf,
    pub portable: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum PathsError {
    #[error("não foi possível descobrir a pasta de dados do usuário")]
    NoHome,
    #[error("falha ao criar a pasta {path}: {source}")]
    Create {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl DataDirs {
    /// Monta as pastas a partir de uma raiz qualquer (útil em testes).
    pub fn from_root(root: impl Into<PathBuf>, portable: bool) -> Self {
        let root = root.into();
        Self {
            bios: root.join("bios"),
            saves: root.join("saves"),
            states: root.join("states"),
            covers: root.join("covers"),
            cores: root.join("cores"),
            database: root.join("games.db"),
            config: root.join("config.toml"),
            root,
            portable,
        }
    }

    /// Descobre as pastas certas para esta instalação.
    pub fn resolve() -> Result<Self, PathsError> {
        if let Some(exe_dir) = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
        {
            if exe_dir.join("portable.txt").exists() {
                return Ok(Self::from_root(exe_dir.join("portable-data"), true));
            }
        }
        let dirs = directories::ProjectDirs::from("", "", "ROXemu").ok_or(PathsError::NoHome)?;
        Ok(Self::from_root(dirs.config_dir(), false))
    }

    /// Cria as pastas que ainda não existem.
    pub fn ensure(&self) -> Result<(), PathsError> {
        for dir in [
            &self.root,
            &self.bios,
            &self.saves,
            &self.states,
            &self.covers,
            &self.cores,
        ] {
            std::fs::create_dir_all(dir).map_err(|source| PathsError::Create {
                path: dir.clone(),
                source,
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subpastas_ficam_dentro_da_raiz() {
        let d = DataDirs::from_root("X:/rox", false);
        assert!(d.bios.starts_with(&d.root));
        assert!(d.database.ends_with("games.db"));
    }
}
