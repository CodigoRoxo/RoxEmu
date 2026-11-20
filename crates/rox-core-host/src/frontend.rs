//! Frontend libretro: recebe vídeo, áudio e pedidos de configuração do core e entrega o input.
//!
//! A API libretro usa callbacks C sem ponteiro de contexto, então o estado do frontend vive num
//! `thread_local`. Por isso só pode existir um [`Frontend`] por thread (o runner usa um só).

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uint, c_void};
use std::path::{Path, PathBuf};

use crate::{cstr_or_empty, ffi, video, Core, CoreError};

/// Configuração que o runner passa para o frontend.
#[derive(Debug, Clone, Default)]
pub struct FrontendConfig {
    /// Pasta "system" do libretro: onde o core procura as BIOS.
    pub system_dir: PathBuf,
    /// Pasta de saves do core.
    pub save_dir: PathBuf,
    /// Opções do core forçadas pelo ROXemu (chave, valor).
    pub option_overrides: Vec<(String, String)>,
}

struct Variable {
    value: CString,
}

struct HostState {
    pixel_format: u32,
    frame: Vec<u32>,
    width: usize,
    height: usize,
    audio: Vec<i16>,
    input: [u16; 2],
    variables: HashMap<String, Variable>,
    overrides: HashMap<String, String>,
    variables_dirty: bool,
    system_dir: CString,
    save_dir: CString,
    av_info: ffi::retro_system_av_info,
    messages: Vec<String>,
}

thread_local! {
    static HOST: RefCell<Option<HostState>> = const { RefCell::new(None) };
}

fn with_host<R>(f: impl FnOnce(&mut HostState) -> R) -> Option<R> {
    HOST.with(|h| h.borrow_mut().as_mut().map(f))
}

fn path_cstring(p: &Path) -> Result<CString, CoreError> {
    CString::new(p.to_string_lossy().into_owned()).map_err(|_| CoreError::BadPath(p.to_path_buf()))
}

/// Um core rodando um jogo.
pub struct Frontend {
    core: Core,
    game_loaded: bool,
    _game_path: Option<CString>,
    _game_data: Option<Vec<u8>>,
}

impl Frontend {
    /// Instala os callbacks e inicializa o core.
    pub fn new(core: Core, config: FrontendConfig) -> Result<Self, CoreError> {
        let state = HostState {
            pixel_format: ffi::RETRO_PIXEL_FORMAT_0RGB1555,
            frame: Vec::new(),
            width: 0,
            height: 0,
            audio: Vec::with_capacity(4096),
            input: [0; 2],
            variables: HashMap::new(),
            overrides: config.option_overrides.into_iter().collect(),
            variables_dirty: false,
            system_dir: path_cstring(&config.system_dir)?,
            save_dir: path_cstring(&config.save_dir)?,
            av_info: Default::default(),
            messages: Vec::new(),
        };
        HOST.with(|h| {
            let mut h = h.borrow_mut();
            if h.is_some() {
                return Err(CoreError::AlreadyRunning);
            }
            *h = Some(state);
            Ok(())
        })?;

        let api = &core.api;
        unsafe {
            (api.set_environment)(environment_cb);
            (api.set_video_refresh)(video_cb);
            (api.set_audio_sample)(audio_sample_cb);
            (api.set_audio_sample_batch)(audio_batch_cb);
            (api.set_input_poll)(input_poll_cb);
            (api.set_input_state)(input_state_cb);
            (api.init)();
        }
        Ok(Self {
            core,
            game_loaded: false,
            _game_path: None,
            _game_data: None,
        })
    }

    pub fn core(&self) -> &Core {
        &self.core
    }

    /// Carrega o jogo e conecta um controle padrão na porta 1.
    pub fn load_game(&mut self, path: &Path) -> Result<(), CoreError> {
        let cpath = path_cstring(path)?;
        let data = if self.core.info.need_fullpath {
            None
        } else {
            Some(std::fs::read(path).map_err(|_| CoreError::LoadGame(path.to_path_buf()))?)
        };
        let info = ffi::retro_game_info {
            path: cpath.as_ptr(),
            data: data
                .as_ref()
                .map_or(std::ptr::null(), |d| d.as_ptr() as *const c_void),
            size: data.as_ref().map_or(0, Vec::len),
            meta: std::ptr::null(),
        };
        let ok = unsafe { (self.core.api.load_game)(&info) };
        if !ok {
            return Err(CoreError::LoadGame(path.to_path_buf()));
        }
        self.game_loaded = true;
        self._game_path = Some(cpath);
        self._game_data = data;

        let mut av = ffi::retro_system_av_info::default();
        unsafe {
            (self.core.api.get_system_av_info)(&mut av);
            (self.core.api.set_controller_port_device)(0, ffi::RETRO_DEVICE_JOYPAD);
        }
        with_host(|h| h.av_info = av);
        log::info!(
            "jogo carregado: {}x{} @ {:.3} fps, áudio {} Hz",
            av.geometry.base_width,
            av.geometry.base_height,
            av.timing.fps,
            av.timing.sample_rate
        );
        Ok(())
    }

    /// Informações de vídeo/áudio atuais (o core pode mudar durante o jogo).
    pub fn av_info(&self) -> ffi::retro_system_av_info {
        with_host(|h| h.av_info).unwrap_or_default()
    }

    /// Define os botões apertados numa porta (bits = `ffi::JOYPAD_*`).
    pub fn set_input(&self, port: usize, buttons: u16) {
        with_host(|h| {
            if let Some(slot) = h.input.get_mut(port) {
                *slot = buttons;
            }
        });
    }

    /// Roda um quadro do jogo.
    pub fn run_frame(&mut self) {
        unsafe { (self.core.api.run)() };
    }

    /// Acesso ao último quadro (pixels 0RGB, largura, altura).
    pub fn with_frame<R>(&self, f: impl FnOnce(&[u32], usize, usize) -> R) -> Option<R> {
        HOST.with(|h| {
            let h = h.borrow();
            let s = h.as_ref()?;
            (s.width > 0).then(|| f(&s.frame, s.width, s.height))
        })
    }

    /// Move as amostras de áudio acumuladas (estéreo intercalado) para `out`.
    pub fn take_audio(&self, out: &mut Vec<i16>) {
        with_host(|h| out.append(&mut h.audio));
    }

    /// Mensagens que o core pediu para mostrar ao jogador.
    pub fn take_messages(&self) -> Vec<String> {
        with_host(|h| std::mem::take(&mut h.messages)).unwrap_or_default()
    }

    /// Memória de save do jogo (ex.: memory card), se o core expõe uma.
    pub fn save_ram(&mut self) -> Option<&mut [u8]> {
        unsafe {
            let size = (self.core.api.get_memory_size)(ffi::RETRO_MEMORY_SAVE_RAM);
            let data = (self.core.api.get_memory_data)(ffi::RETRO_MEMORY_SAVE_RAM) as *mut u8;
            (size > 0 && !data.is_null()).then(|| std::slice::from_raw_parts_mut(data, size))
        }
    }

    /// Salva o estado completo do jogo (save state).
    pub fn save_state(&mut self) -> Option<Vec<u8>> {
        let size = unsafe { (self.core.api.serialize_size)() };
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size];
        let ok = unsafe { (self.core.api.serialize)(buf.as_mut_ptr() as *mut c_void, size) };
        ok.then_some(buf)
    }

    /// Carrega um save state.
    pub fn load_state(&mut self, data: &[u8]) -> bool {
        unsafe { (self.core.api.unserialize)(data.as_ptr() as *const c_void, data.len()) }
    }
}

impl Drop for Frontend {
    fn drop(&mut self) {
        unsafe {
            if self.game_loaded {
                (self.core.api.unload_game)();
            }
            (self.core.api.deinit)();
        }
        HOST.with(|h| *h.borrow_mut() = None);
    }
}

// ------------------------------------------------------------------------------------------
// Callbacks chamados pelo core
// ------------------------------------------------------------------------------------------

unsafe extern "C" fn environment_cb(cmd: c_uint, data: *mut c_void) -> bool {
    let cmd = cmd & !ffi::RETRO_ENVIRONMENT_EXPERIMENTAL;
    with_host(|h| unsafe { h.environment(cmd, data) }).unwrap_or(false)
}

impl HostState {
    unsafe fn environment(&mut self, cmd: c_uint, data: *mut c_void) -> bool {
        match cmd {
            ffi::ENV_GET_CAN_DUPE => {
                *(data as *mut bool) = true;
                true
            }
            ffi::ENV_SET_PIXEL_FORMAT => {
                let fmt = *(data as *const u32);
                if fmt <= ffi::RETRO_PIXEL_FORMAT_RGB565 {
                    self.pixel_format = fmt;
                    true
                } else {
                    false
                }
            }
            ffi::ENV_GET_SYSTEM_DIRECTORY => {
                *(data as *mut *const c_char) = self.system_dir.as_ptr();
                true
            }
            ffi::ENV_GET_SAVE_DIRECTORY => {
                *(data as *mut *const c_char) = self.save_dir.as_ptr();
                true
            }
            ffi::ENV_SET_VARIABLES => {
                self.set_variables(data as *const ffi::retro_variable);
                true
            }
            ffi::ENV_GET_VARIABLE => {
                let var = &mut *(data as *mut ffi::retro_variable);
                if var.key.is_null() {
                    return false;
                }
                let key = CStr::from_ptr(var.key).to_string_lossy();
                match self.variables.get(key.as_ref()) {
                    Some(v) => {
                        var.value = v.value.as_ptr();
                        true
                    }
                    None => {
                        var.value = std::ptr::null();
                        false
                    }
                }
            }
            ffi::ENV_GET_VARIABLE_UPDATE => {
                *(data as *mut bool) = std::mem::take(&mut self.variables_dirty);
                true
            }
            ffi::ENV_SET_GEOMETRY => {
                self.av_info.geometry = *(data as *const ffi::retro_game_geometry);
                true
            }
            ffi::ENV_SET_SYSTEM_AV_INFO => {
                self.av_info = *(data as *const ffi::retro_system_av_info);
                true
            }
            ffi::ENV_SET_MESSAGE => {
                let msg = &*(data as *const ffi::retro_message);
                let text = cstr_or_empty(msg.msg);
                log::info!("[core] {text}");
                self.messages.push(text);
                true
            }
            ffi::ENV_SET_INPUT_DESCRIPTORS
            | ffi::ENV_SET_CONTROLLER_INFO
            | ffi::ENV_SET_PERFORMANCE_LEVEL
            | ffi::ENV_SET_SUPPORT_NO_GAME => true,
            // Fase 1 usa só o renderer por software; aceleração por GPU vem na Fase 4.
            ffi::ENV_SET_HW_RENDER => false,
            other => {
                log::trace!("environment não suportado: {other}");
                false
            }
        }
    }

    /// Lê a lista de opções do core ("Descrição; padrão|opção2|...") e escolhe os valores.
    unsafe fn set_variables(&mut self, mut list: *const ffi::retro_variable) {
        while !list.is_null() && !(*list).key.is_null() {
            let key = cstr_or_empty((*list).key);
            let spec = cstr_or_empty((*list).value);
            let choices: Vec<String> = spec
                .split_once(';')
                .map(|(_, opts)| opts.trim().split('|').map(str::to_string).collect())
                .unwrap_or_default();
            let chosen = self
                .overrides
                .get(&key)
                .cloned()
                .or_else(|| prefer_software(&key, &choices))
                .or_else(|| choices.first().cloned())
                .unwrap_or_default();
            log::debug!("opção {key} = {chosen}");
            if let Ok(value) = CString::new(chosen) {
                self.variables.insert(key, Variable { value });
            }
            list = list.add(1);
        }
        self.variables_dirty = true;
    }
}

/// Enquanto não temos renderização por GPU, força o renderer por software quando o core oferece.
fn prefer_software(key: &str, choices: &[String]) -> Option<String> {
    if !key.to_ascii_lowercase().contains("renderer") {
        return None;
    }
    choices
        .iter()
        .find(|c| c.eq_ignore_ascii_case("software"))
        .cloned()
}

unsafe extern "C" fn video_cb(data: *const c_void, width: c_uint, height: c_uint, pitch: usize) {
    if data.is_null() {
        return; // quadro repetido: mantém o anterior
    }
    with_host(|h| unsafe {
        video::convert_frame(
            h.pixel_format,
            data as *const u8,
            width as usize,
            height as usize,
            pitch,
            &mut h.frame,
        );
        h.width = width as usize;
        h.height = height as usize;
    });
}

unsafe extern "C" fn audio_sample_cb(left: i16, right: i16) {
    with_host(|h| h.audio.extend_from_slice(&[left, right]));
}

unsafe extern "C" fn audio_batch_cb(data: *const i16, frames: usize) -> usize {
    if data.is_null() {
        return 0;
    }
    let samples = std::slice::from_raw_parts(data, frames * 2);
    with_host(|h| h.audio.extend_from_slice(samples));
    frames
}

unsafe extern "C" fn input_poll_cb() {}

unsafe extern "C" fn input_state_cb(
    port: c_uint,
    device: c_uint,
    _index: c_uint,
    id: c_uint,
) -> i16 {
    if device & 0xFF != ffi::RETRO_DEVICE_JOYPAD || id >= 16 {
        return 0;
    }
    with_host(|h| {
        h.input
            .get(port as usize)
            .map_or(0, |bits| ((bits >> id) & 1) as i16)
    })
    .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escolhe_software_quando_existe() {
        let choices = vec!["Auto".to_string(), "Hardware".into(), "Software".into()];
        assert_eq!(
            prefer_software("swanstation_GPU_Renderer", &choices).as_deref(),
            Some("Software")
        );
        assert_eq!(prefer_software("swanstation_Audio_Volume", &choices), None);
    }
}
