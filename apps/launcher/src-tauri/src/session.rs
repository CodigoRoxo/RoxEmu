//! Controle da sessão de jogo: qual processo está rodando, fechar o jogo pelo ROXemu e
//! o atalho do controle para "voltar ao console" (como o botão PS do PS5).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// O jogo foi fechado de propósito (pelo ROXemu ou pelo controle), não caiu sozinho.
static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Lê e limpa o aviso de "fechado de propósito".
pub fn take_stop_requested() -> bool {
    STOP_REQUESTED.swap(false, Ordering::SeqCst)
}

use gilrs::{Button, Gilrs};
use tauri::{AppHandle, Manager};

/// PID do emulador/runner do jogo aberto agora.
#[derive(Default)]
pub struct RunningPid(pub Mutex<Option<u32>>);

pub fn set_pid(app: &AppHandle, pid: Option<u32>) {
    if let Ok(mut slot) = app.state::<RunningPid>().0.lock() {
        *slot = pid;
    }
}

fn current_pid(app: &AppHandle) -> Option<u32> {
    app.state::<RunningPid>().0.lock().ok().and_then(|g| *g)
}

/// Fecha o jogo aberto (o processo e os filhos dele). A thread que espera o processo avisa
/// a interface ("exited") e soma o tempo jogado.
pub fn stop_game(app: &AppHandle) -> Result<(), String> {
    let pid = current_pid(app).ok_or("Nenhum jogo aberto.")?;
    STOP_REQUESTED.store(true, Ordering::SeqCst);
    let mut cmd = std::process::Command::new("taskkill");
    cmd.args(["/PID", &pid.to_string(), "/T", "/F"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // sem janela de console
    }
    let status = cmd.status().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Não consegui fechar o jogo.".into());
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
    Ok(())
}

/// Enquanto o jogo `pid` estiver aberto, observa o controle (mesmo com o jogo em primeiro
/// plano): soltar o botão PS/Guide, ou segurar SELECT + START por 1 segundo, fecha o jogo
/// e volta para o ROXemu.
pub fn watch_close_combo(app: &AppHandle, pid: u32) {
    let app = app.clone();
    std::thread::spawn(move || {
        let Ok(mut gilrs) = Gilrs::new() else {
            log::warn!("atalho de fechar jogo indisponível (sem acesso aos controles)");
            return;
        };
        let mut guide_prev = false;
        let mut combo_since: Option<Instant> = None;
        while current_pid(&app) == Some(pid) {
            while gilrs.next_event().is_some() {}
            let (mut guide, mut combo) = (false, false);
            for (_, gp) in gilrs.gamepads() {
                guide |= gp.is_pressed(Button::Mode);
                combo |= gp.is_pressed(Button::Select) && gp.is_pressed(Button::Start);
            }
            combo_since = if combo {
                Some(combo_since.unwrap_or_else(Instant::now))
            } else {
                None
            };
            let fire = (guide_prev && !guide)
                || combo_since.is_some_and(|t| t.elapsed() >= Duration::from_secs(1));
            guide_prev = guide;
            if fire {
                log::info!("controle pediu para fechar o jogo");
                if let Err(e) = stop_game(&app) {
                    log::warn!("fechar jogo: {e}");
                }
                break;
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    });
}
