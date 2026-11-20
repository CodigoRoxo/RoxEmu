//! Host libretro.
//!
//! Fase 0: carrega a `.dll` de um core, confere a versão da API e lê as informações do core.
//! Fase 1: callbacks de vídeo, áudio, input e environment, e o loop `retro_run`.

pub mod ffi;

use std::ffi::CStr;
use std::path::{Path, PathBuf};

use libloading::Library;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("não foi possível carregar o core {path}: {source}")]
    Load {
        path: PathBuf,
        source: libloading::Error,
    },
    #[error("o core não exporta a função {0}")]
    MissingSymbol(&'static str),
    #[error("versão da API libretro incompatível: o core usa {0}, o ROXemu espera {expected}", expected = ffi::RETRO_API_VERSION)]
    ApiVersion(u32),
}

/// Informações que o core declara sobre si mesmo.
#[derive(Debug, Clone)]
pub struct CoreInfo {
    pub name: String,
    pub version: String,
    /// Extensões aceitas, ex.: `["cue", "chd", "pbp"]`.
    pub extensions: Vec<String>,
    /// O core precisa do caminho do arquivo em vez do conteúdo em memória.
    pub need_fullpath: bool,
}

/// Um core libretro carregado na memória.
pub struct Core {
    _lib: Library,
    pub info: CoreInfo,
    pub path: PathBuf,
}

impl Core {
    /// Carrega o core e lê as informações básicas.
    ///
    /// # Segurança
    /// Carregar uma `.dll` executa código dela. O ROXemu só carrega cores compilados pelo
    /// próprio projeto (ver `cores/cores.lock.toml`); a verificação de hash entra antes desta
    /// chamada no runner.
    pub fn load(path: &Path) -> Result<Self, CoreError> {
        let lib = unsafe { Library::new(path) }.map_err(|source| CoreError::Load {
            path: path.to_path_buf(),
            source,
        })?;

        let api_version: libloading::Symbol<unsafe extern "C" fn() -> u32> =
            unsafe { lib.get(b"retro_api_version\0") }
                .map_err(|_| CoreError::MissingSymbol("retro_api_version"))?;
        let version = unsafe { api_version() };
        if version != ffi::RETRO_API_VERSION {
            return Err(CoreError::ApiVersion(version));
        }

        let get_info: libloading::Symbol<unsafe extern "C" fn(*mut ffi::retro_system_info)> =
            unsafe { lib.get(b"retro_get_system_info\0") }
                .map_err(|_| CoreError::MissingSymbol("retro_get_system_info"))?;
        let mut raw = ffi::retro_system_info::default();
        unsafe { get_info(&mut raw) };

        let info = CoreInfo {
            name: unsafe { cstr_or_empty(raw.library_name) },
            version: unsafe { cstr_or_empty(raw.library_version) },
            extensions: unsafe { cstr_or_empty(raw.valid_extensions) }
                .split('|')
                .filter(|s| !s.is_empty())
                .map(str::to_ascii_lowercase)
                .collect(),
            need_fullpath: raw.need_fullpath,
        };
        log::info!("core carregado: {} {}", info.name, info.version);

        Ok(Self {
            _lib: lib,
            info,
            path: path.to_path_buf(),
        })
    }
}

unsafe fn cstr_or_empty(ptr: *const std::os::raw::c_char) -> String {
    if ptr.is_null() {
        String::new()
    } else {
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erro_claro_quando_o_core_nao_existe() {
        let err = Core::load(Path::new("nao_existe_libretro.dll"))
            .err()
            .unwrap();
        assert!(matches!(err, CoreError::Load { .. }));
    }
}
