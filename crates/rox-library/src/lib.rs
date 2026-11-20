//! Biblioteca de jogos.
//!
//! Fase 0: formatos, leitura de `.cue` e extração do serial do PS1.
//! Fase 2: scanner de pastas, leitura ISO9660/CHD e banco SQLite.

pub mod cue;
pub mod formats;
pub mod ps1;

pub use formats::{detect_format, GameFormat};
