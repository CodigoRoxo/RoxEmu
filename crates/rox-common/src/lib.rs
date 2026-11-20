//! Tipos compartilhados entre o launcher, o runner e as bibliotecas do ROXemu.

pub mod ipc;
pub mod paths;
pub mod system;

pub use paths::DataDirs;
pub use system::{Engine, SystemId};
