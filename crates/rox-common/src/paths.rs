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
        let base = directories::BaseDirs::new().ok_or(PathsError::NoHome)?;
        let root = base.config_dir().join("ROXemu");
        migrate_legacy(&root);
        Ok(Self::from_root(root, false))
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

/// Versões até 0.1.0 guardavam tudo em `ROXemu\config\` (efeito colateral da biblioteca de
/// pastas). Move o conteúdo para `ROXemu\`, sem sobrescrever nada que já exista lá.
fn migrate_legacy(root: &Path) {
    let old = root.join("config");
    let Ok(entries) = std::fs::read_dir(&old) else {
        return;
    };
    for entry in entries.flatten() {
        let target = root.join(entry.file_name());
        if !target.exists() {
            if let Err(e) = std::fs::rename(entry.path(), &target) {
                eprintln!("ROXemu: não consegui mover {}: {e}", entry.path().display());
            }
        }
    }
    // Só apaga a pasta antiga se ficou vazia (remove_dir falha se tiver algo).
    let _ = std::fs::remove_dir(&old);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migra_pasta_config_antiga_sem_sobrescrever() {
        let root = std::env::temp_dir().join(format!("rox-migra-{}", std::process::id()));
        let old = root.join("config");
        std::fs::create_dir_all(old.join("bios")).unwrap();
        std::fs::write(old.join("bios").join("scph1001.bin"), b"bios").unwrap();
        std::fs::write(old.join("library.json"), b"antigo").unwrap();
        std::fs::write(root.join("library.json"), b"novo").unwrap();

        migrate_legacy(&root);

        assert!(root.join("bios").join("scph1001.bin").exists());
        assert_eq!(std::fs::read(root.join("library.json")).unwrap(), b"novo");
        assert!(old.join("library.json").exists()); // conflito: o antigo fica onde estava
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn subpastas_ficam_dentro_da_raiz() {
        let d = DataDirs::from_root("X:/rox", false);
        assert!(d.bios.starts_with(&d.root));
        assert!(d.database.ends_with("games.db"));
    }
}
