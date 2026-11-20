//! Tipos da API C do libretro (`libretro.h`), escritos à mão.
//! Só o necessário até agora; cresce junto com a Fase 1.

#![allow(non_camel_case_types)]

use std::os::raw::c_char;

pub const RETRO_API_VERSION: u32 = 1;

#[repr(C)]
#[derive(Debug)]
pub struct retro_system_info {
    pub library_name: *const c_char,
    pub library_version: *const c_char,
    pub valid_extensions: *const c_char,
    pub need_fullpath: bool,
    pub block_extract: bool,
}

impl Default for retro_system_info {
    fn default() -> Self {
        Self {
            library_name: std::ptr::null(),
            library_version: std::ptr::null(),
            valid_extensions: std::ptr::null(),
            need_fullpath: false,
            block_extract: false,
        }
    }
}
