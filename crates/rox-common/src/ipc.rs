//! Protocolo entre o launcher e o `rox-runner`.
//!
//! O launcher inicia o runner como processo filho. O runner escreve um evento JSON por linha
//! no stdout ("JSON lines"); o launcher lê e atualiza a interface (jogo iniciou, tempo jogado,
//! saiu, erro).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::SystemId;

/// O que o launcher pede para o runner abrir.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchRequest {
    pub game_id: i64,
    pub system: SystemId,
    pub core_path: PathBuf,
    pub game_path: PathBuf,
    pub bios_dir: Option<PathBuf>,
    /// Carregar este save state logo após iniciar.
    pub load_state: Option<PathBuf>,
}

/// Eventos que o runner envia de volta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum RunnerEvent {
    Started {
        game_id: i64,
        core: String,
        core_version: String,
    },
    /// Enviado periodicamente para o launcher contar o tempo jogado.
    Heartbeat {
        game_id: i64,
        played_secs: u64,
    },
    Exited {
        game_id: i64,
        played_secs: u64,
    },
    Error {
        message: String,
    },
}

impl RunnerEvent {
    /// Serializa como uma linha JSON (sem quebra de linha no fim).
    pub fn to_line(&self) -> String {
        serde_json::to_string(self).expect("RunnerEvent sempre serializa")
    }

    pub fn from_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line.trim())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_e_volta_em_json_lines() {
        let ev = RunnerEvent::Heartbeat {
            game_id: 7,
            played_secs: 120,
        };
        let line = ev.to_line();
        assert_eq!(
            line,
            r#"{"event":"heartbeat","game_id":7,"played_secs":120}"#
        );
        assert_eq!(RunnerEvent::from_line(&line).unwrap(), ev);
    }
}
