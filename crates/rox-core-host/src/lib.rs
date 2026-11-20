//! Host libretro: carrega um core (.dll) e conversa com ele via FFI.
//!
//! - [`Core::load`] abre a .dll, confere a versão da API e lê as informações do core.
//! - [`Frontend`] implementa os callbacks (vídeo, áudio, input, environment) e roda o jogo.

pub mod ffi;
mod frontend;
mod video;

use std::ffi::CStr;
use std::os::raw::{c_char, c_uint, c_void};
use std::path::{Path, PathBuf};

use libloading::Library;

pub use frontend::{Frontend, FrontendConfig};

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
    #[error("o core recusou o jogo {0} (confira a BIOS e o arquivo)")]
    LoadGame(PathBuf),
    #[error("caminho inválido: {0}")]
    BadPath(PathBuf),
    #[error("já existe um core rodando nesta thread")]
    AlreadyRunning,
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

/// Ponteiros para as funções exportadas pelo core.
#[allow(clippy::type_complexity)]
pub(crate) struct CoreApi {
    pub set_environment: unsafe extern "C" fn(ffi::retro_environment_t),
    pub set_video_refresh: unsafe extern "C" fn(ffi::retro_video_refresh_t),
    pub set_audio_sample: unsafe extern "C" fn(ffi::retro_audio_sample_t),
    pub set_audio_sample_batch: unsafe extern "C" fn(ffi::retro_audio_sample_batch_t),
    pub set_input_poll: unsafe extern "C" fn(ffi::retro_input_poll_t),
    pub set_input_state: unsafe extern "C" fn(ffi::retro_input_state_t),
    pub init: unsafe extern "C" fn(),
    pub deinit: unsafe extern "C" fn(),
    pub get_system_av_info: unsafe extern "C" fn(*mut ffi::retro_system_av_info),
    pub set_controller_port_device: unsafe extern "C" fn(c_uint, c_uint),
    pub run: unsafe extern "C" fn(),
    pub load_game: unsafe extern "C" fn(*const ffi::retro_game_info) -> bool,
    pub unload_game: unsafe extern "C" fn(),
    pub serialize_size: unsafe extern "C" fn() -> usize,
    pub serialize: unsafe extern "C" fn(*mut c_void, usize) -> bool,
    pub unserialize: unsafe extern "C" fn(*const c_void, usize) -> bool,
    pub get_memory_data: unsafe extern "C" fn(c_uint) -> *mut c_void,
    pub get_memory_size: unsafe extern "C" fn(c_uint) -> usize,
}

/// Um core libretro carregado na memória.
pub struct Core {
    pub info: CoreInfo,
    pub path: PathBuf,
    pub(crate) api: CoreApi,
    // Precisa ficar por último: a .dll só pode ser descarregada depois de tudo acima.
    _lib: Library,
}

macro_rules! sym {
    ($lib:expr, $name:literal) => {
        *unsafe { $lib.get(concat!($name, "\0").as_bytes()) }
            .map_err(|_| CoreError::MissingSymbol($name))?
    };
}

impl Core {
    /// Carrega o core e lê as informações básicas.
    ///
    /// # Segurança
    /// Carregar uma `.dll` executa código dela. O ROXemu só carrega cores compilados pelo
    /// próprio projeto (ver `cores/cores.lock.toml`).
    pub fn load(path: &Path) -> Result<Self, CoreError> {
        let lib = unsafe { Library::new(path) }.map_err(|source| CoreError::Load {
            path: path.to_path_buf(),
            source,
        })?;

        let api_version: unsafe extern "C" fn() -> u32 = sym!(lib, "retro_api_version");
        let version = unsafe { api_version() };
        if version != ffi::RETRO_API_VERSION {
            return Err(CoreError::ApiVersion(version));
        }

        let get_info: unsafe extern "C" fn(*mut ffi::retro_system_info) =
            sym!(lib, "retro_get_system_info");
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

        let api = CoreApi {
            set_environment: sym!(lib, "retro_set_environment"),
            set_video_refresh: sym!(lib, "retro_set_video_refresh"),
            set_audio_sample: sym!(lib, "retro_set_audio_sample"),
            set_audio_sample_batch: sym!(lib, "retro_set_audio_sample_batch"),
            set_input_poll: sym!(lib, "retro_set_input_poll"),
            set_input_state: sym!(lib, "retro_set_input_state"),
            init: sym!(lib, "retro_init"),
            deinit: sym!(lib, "retro_deinit"),
            get_system_av_info: sym!(lib, "retro_get_system_av_info"),
            set_controller_port_device: sym!(lib, "retro_set_controller_port_device"),
            run: sym!(lib, "retro_run"),
            load_game: sym!(lib, "retro_load_game"),
            unload_game: sym!(lib, "retro_unload_game"),
            serialize_size: sym!(lib, "retro_serialize_size"),
            serialize: sym!(lib, "retro_serialize"),
            unserialize: sym!(lib, "retro_unserialize"),
            get_memory_data: sym!(lib, "retro_get_memory_data"),
            get_memory_size: sym!(lib, "retro_get_memory_size"),
        };

        log::info!("core carregado: {} {}", info.name, info.version);
        Ok(Self {
            info,
            path: path.to_path_buf(),
            api,
            _lib: lib,
        })
    }
}

pub(crate) unsafe fn cstr_or_empty(ptr: *const c_char) -> String {
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
