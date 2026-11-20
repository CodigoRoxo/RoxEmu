//! Teclado e controle -> botões do PS1 (no padrão RetroPad do libretro).
//!
//! | PS1        | Controle (Xbox / PlayStation) | Teclado        |
//! |------------|-------------------------------|----------------|
//! | Direcional | D-pad ou analógico esquerdo   | Setas ou WASD  |
//! | ✕ Cross    | A / ✕                         | K              |
//! | ○ Circle   | B / ○                         | L              |
//! | □ Square   | X / □                         | J              |
//! | △ Triangle | Y / △                         | I              |
//! | L1 / R1    | LB / RB                       | Q / E          |
//! | L2 / R2    | LT / RT                       | 1 / 3          |
//! | Start      | Start / Options               | Enter          |
//! | Select     | Back / Share                  | Backspace      |

use gilrs::{Axis, Button, Gilrs};
use minifb::{Key, Window};
use rox_core_host::ffi;

/// Ações do ROXemu (não vão para o jogo).
#[derive(Debug, Default, Clone, Copy)]
pub struct Hotkeys {
    pub quit: bool,
    pub fast_forward: bool,
    pub save_state: bool,
    pub load_state: bool,
}

pub struct Input {
    gilrs: Option<Gilrs>,
    prev_guide: bool,
}

const KEYS: &[(Key, u32)] = &[
    (Key::Up, ffi::JOYPAD_UP),
    (Key::Down, ffi::JOYPAD_DOWN),
    (Key::Left, ffi::JOYPAD_LEFT),
    (Key::Right, ffi::JOYPAD_RIGHT),
    (Key::W, ffi::JOYPAD_UP),
    (Key::S, ffi::JOYPAD_DOWN),
    (Key::A, ffi::JOYPAD_LEFT),
    (Key::D, ffi::JOYPAD_RIGHT),
    (Key::K, ffi::JOYPAD_B),
    (Key::L, ffi::JOYPAD_A),
    (Key::J, ffi::JOYPAD_Y),
    (Key::I, ffi::JOYPAD_X),
    (Key::Q, ffi::JOYPAD_L),
    (Key::E, ffi::JOYPAD_R),
    (Key::Key1, ffi::JOYPAD_L2),
    (Key::Key3, ffi::JOYPAD_R2),
    (Key::Enter, ffi::JOYPAD_START),
    (Key::Backspace, ffi::JOYPAD_SELECT),
];

const PAD: &[(Button, u32)] = &[
    (Button::DPadUp, ffi::JOYPAD_UP),
    (Button::DPadDown, ffi::JOYPAD_DOWN),
    (Button::DPadLeft, ffi::JOYPAD_LEFT),
    (Button::DPadRight, ffi::JOYPAD_RIGHT),
    (Button::South, ffi::JOYPAD_B),
    (Button::East, ffi::JOYPAD_A),
    (Button::West, ffi::JOYPAD_Y),
    (Button::North, ffi::JOYPAD_X),
    (Button::LeftTrigger, ffi::JOYPAD_L),
    (Button::RightTrigger, ffi::JOYPAD_R),
    (Button::LeftTrigger2, ffi::JOYPAD_L2),
    (Button::RightTrigger2, ffi::JOYPAD_R2),
    (Button::LeftThumb, ffi::JOYPAD_L3),
    (Button::RightThumb, ffi::JOYPAD_R3),
    (Button::Start, ffi::JOYPAD_START),
    (Button::Select, ffi::JOYPAD_SELECT),
];

impl Input {
    pub fn new() -> Self {
        let gilrs = match Gilrs::new() {
            Ok(g) => {
                for (_, gp) in g.gamepads() {
                    log::info!("controle conectado: {}", gp.name());
                }
                Some(g)
            }
            Err(e) => {
                log::warn!("controles indisponíveis: {e}");
                None
            }
        };
        Self {
            gilrs,
            prev_guide: false,
        }
    }

    /// Lê teclado e controle. Retorna os botões do jogo (bits) e as teclas de atalho.
    pub fn poll(&mut self, window: &Window) -> (u16, Hotkeys) {
        let mut bits = 0u16;
        for &(key, id) in KEYS {
            if window.is_key_down(key) {
                bits |= 1 << id;
            }
        }
        let mut hot = Hotkeys {
            quit: window.is_key_down(Key::Escape),
            fast_forward: window.is_key_down(Key::Space),
            save_state: window.is_key_pressed(Key::F2, minifb::KeyRepeat::No),
            load_state: window.is_key_pressed(Key::F4, minifb::KeyRepeat::No),
        };

        if let Some(gilrs) = self.gilrs.as_mut() {
            while let Some(ev) = gilrs.next_event() {
                if let gilrs::EventType::Connected = ev.event {
                    log::info!("controle conectado: {}", gilrs.gamepad(ev.id).name());
                }
            }
            let mut guide = false;
            for (_, gp) in gilrs.gamepads() {
                for &(btn, id) in PAD {
                    if gp.is_pressed(btn) {
                        bits |= 1 << id;
                    }
                }
                let (x, y) = (gp.value(Axis::LeftStickX), gp.value(Axis::LeftStickY));
                if x < -0.5 {
                    bits |= 1 << ffi::JOYPAD_LEFT;
                }
                if x > 0.5 {
                    bits |= 1 << ffi::JOYPAD_RIGHT;
                }
                if y > 0.5 {
                    bits |= 1 << ffi::JOYPAD_UP;
                }
                if y < -0.5 {
                    bits |= 1 << ffi::JOYPAD_DOWN;
                }
                guide |= gp.is_pressed(Button::Mode);
            }
            // Botão Guide/PS: volta para o ROXemu (ao soltar, para não vazar para o launcher).
            if self.prev_guide && !guide {
                hot.quit = true;
            }
            self.prev_guide = guide;
        }
        (bits, hot)
    }
}
