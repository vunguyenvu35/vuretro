//! Keyboard (minifb) and gamepad (gilrs) mapped to the libretro RetroPad.
//!
//! Default keyboard layout (port 0), matching the Controls screen:
//!   Arrows = D-pad, Enter/1 = Start, RShift/5 = Select
//!   Z = B, X = A, A = Y, S = X   (Neo Geo via FBNeo: A, B, C, D)
//!   Q = L, E = R, W = L2, R = R2

use crate::frontend::{self, MAX_PORTS};
use gilrs::{Button, Gilrs};
use libretro_sys::*;
use minifb::{Key, Window};

const KEYBOARD_MAP: &[(Key, u32)] = &[
    (Key::Up, RETRO_DEVICE_ID_JOYPAD_UP),
    (Key::Down, RETRO_DEVICE_ID_JOYPAD_DOWN),
    (Key::Left, RETRO_DEVICE_ID_JOYPAD_LEFT),
    (Key::Right, RETRO_DEVICE_ID_JOYPAD_RIGHT),
    (Key::Enter, RETRO_DEVICE_ID_JOYPAD_START),
    (Key::Key1, RETRO_DEVICE_ID_JOYPAD_START),
    (Key::RightShift, RETRO_DEVICE_ID_JOYPAD_SELECT),
    (Key::Key5, RETRO_DEVICE_ID_JOYPAD_SELECT),
    (Key::Z, RETRO_DEVICE_ID_JOYPAD_B),
    (Key::X, RETRO_DEVICE_ID_JOYPAD_A),
    (Key::A, RETRO_DEVICE_ID_JOYPAD_Y),
    (Key::S, RETRO_DEVICE_ID_JOYPAD_X),
    (Key::Q, RETRO_DEVICE_ID_JOYPAD_L),
    (Key::E, RETRO_DEVICE_ID_JOYPAD_R),
    (Key::W, RETRO_DEVICE_ID_JOYPAD_L2),
    (Key::R, RETRO_DEVICE_ID_JOYPAD_R2),
];

/// gilrs uses positional names (South = bottom face button).
const GAMEPAD_MAP: &[(Button, u32)] = &[
    (Button::DPadUp, RETRO_DEVICE_ID_JOYPAD_UP),
    (Button::DPadDown, RETRO_DEVICE_ID_JOYPAD_DOWN),
    (Button::DPadLeft, RETRO_DEVICE_ID_JOYPAD_LEFT),
    (Button::DPadRight, RETRO_DEVICE_ID_JOYPAD_RIGHT),
    (Button::South, RETRO_DEVICE_ID_JOYPAD_B),
    (Button::East, RETRO_DEVICE_ID_JOYPAD_A),
    (Button::West, RETRO_DEVICE_ID_JOYPAD_Y),
    (Button::North, RETRO_DEVICE_ID_JOYPAD_X),
    (Button::LeftTrigger, RETRO_DEVICE_ID_JOYPAD_L),
    (Button::RightTrigger, RETRO_DEVICE_ID_JOYPAD_R),
    (Button::LeftTrigger2, RETRO_DEVICE_ID_JOYPAD_L2),
    (Button::RightTrigger2, RETRO_DEVICE_ID_JOYPAD_R2),
    (Button::LeftThumb, RETRO_DEVICE_ID_JOYPAD_L3),
    (Button::RightThumb, RETRO_DEVICE_ID_JOYPAD_R3),
    (Button::Select, RETRO_DEVICE_ID_JOYPAD_SELECT),
    (Button::Start, RETRO_DEVICE_ID_JOYPAD_START),
];

const STICK_DEADZONE: f32 = 0.35;

pub struct Input {
    gilrs: Option<Gilrs>,
}

impl Input {
    pub fn new() -> Self {
        let gilrs = match Gilrs::new() {
            Ok(g) => {
                for (_, pad) in g.gamepads() {
                    log::info!("gamepad: {}", pad.name());
                }
                Some(g)
            }
            Err(e) => {
                log::warn!("gamepad support disabled: {e}");
                None
            }
        };
        Input { gilrs }
    }

    /// Sample keyboard + gamepads into the frontend button table.
    pub fn poll(&mut self, window: &Window) {
        let mut state = [[false; JOYPAD_BUTTON_COUNT]; MAX_PORTS];

        for &(key, id) in KEYBOARD_MAP {
            if window.is_key_down(key) {
                state[0][id as usize] = true;
            }
        }

        if let Some(g) = self.gilrs.as_mut() {
            while g.next_event().is_some() {} // keep cached state fresh
            for (port, (_, pad)) in g.gamepads().take(MAX_PORTS).enumerate() {
                for &(btn, id) in GAMEPAD_MAP {
                    if pad.is_pressed(btn) {
                        state[port][id as usize] = true;
                    }
                }
                // Left stick doubles as D-pad.
                let x = pad.value(gilrs::Axis::LeftStickX);
                let y = pad.value(gilrs::Axis::LeftStickY);
                let p = &mut state[port];
                if x < -STICK_DEADZONE {
                    p[RETRO_DEVICE_ID_JOYPAD_LEFT as usize] = true;
                }
                if x > STICK_DEADZONE {
                    p[RETRO_DEVICE_ID_JOYPAD_RIGHT as usize] = true;
                }
                if y > STICK_DEADZONE {
                    p[RETRO_DEVICE_ID_JOYPAD_UP as usize] = true;
                }
                if y < -STICK_DEADZONE {
                    p[RETRO_DEVICE_ID_JOYPAD_DOWN as usize] = true;
                }
            }
        }

        frontend::with(|fe| fe.buttons = state);
    }
}
