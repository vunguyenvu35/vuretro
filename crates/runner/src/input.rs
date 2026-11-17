//! Keyboard (minifb) and gamepad (gilrs) mapped to the libretro RetroPad, driven by
//! `controls.json` (see the `vuretro-controls` crate). Defaults match the Controls
//! screen: arrows = D-pad, Z/X/A/S = B/A/Y/X, Enter = Start, RShift = Select.

use crate::frontend::{self, MAX_PORTS};
use gilrs::{Button, Gilrs};
use libretro_sys::*;
use minifb::{Key, Window};
use vuretro_controls::{assign, ControlsConfig, PadInfo, PortSource, ACTIONS};

fn action_id(name: &str) -> Option<u32> {
    Some(match name {
        "up" => RETRO_DEVICE_ID_JOYPAD_UP,
        "down" => RETRO_DEVICE_ID_JOYPAD_DOWN,
        "left" => RETRO_DEVICE_ID_JOYPAD_LEFT,
        "right" => RETRO_DEVICE_ID_JOYPAD_RIGHT,
        "b" => RETRO_DEVICE_ID_JOYPAD_B,
        "a" => RETRO_DEVICE_ID_JOYPAD_A,
        "y" => RETRO_DEVICE_ID_JOYPAD_Y,
        "x" => RETRO_DEVICE_ID_JOYPAD_X,
        "l" => RETRO_DEVICE_ID_JOYPAD_L,
        "r" => RETRO_DEVICE_ID_JOYPAD_R,
        "l2" => RETRO_DEVICE_ID_JOYPAD_L2,
        "r2" => RETRO_DEVICE_ID_JOYPAD_R2,
        "l3" => RETRO_DEVICE_ID_JOYPAD_L3,
        "r3" => RETRO_DEVICE_ID_JOYPAD_R3,
        "select" => RETRO_DEVICE_ID_JOYPAD_SELECT,
        "start" => RETRO_DEVICE_ID_JOYPAD_START,
        _ => return None,
    })
}

/// Keys that can be bound. Esc, Tab, P and the F-keys are reserved for runner hotkeys.
const KEYS: &[(&str, Key)] = &[
    ("Key0", Key::Key0),
    ("Key1", Key::Key1),
    ("Key2", Key::Key2),
    ("Key3", Key::Key3),
    ("Key4", Key::Key4),
    ("Key5", Key::Key5),
    ("Key6", Key::Key6),
    ("Key7", Key::Key7),
    ("Key8", Key::Key8),
    ("Key9", Key::Key9),
    ("A", Key::A),
    ("B", Key::B),
    ("C", Key::C),
    ("D", Key::D),
    ("E", Key::E),
    ("F", Key::F),
    ("G", Key::G),
    ("H", Key::H),
    ("I", Key::I),
    ("J", Key::J),
    ("K", Key::K),
    ("L", Key::L),
    ("M", Key::M),
    ("N", Key::N),
    ("O", Key::O),
    ("Q", Key::Q),
    ("R", Key::R),
    ("S", Key::S),
    ("T", Key::T),
    ("U", Key::U),
    ("V", Key::V),
    ("W", Key::W),
    ("X", Key::X),
    ("Y", Key::Y),
    ("Z", Key::Z),
    ("Up", Key::Up),
    ("Down", Key::Down),
    ("Left", Key::Left),
    ("Right", Key::Right),
    ("Enter", Key::Enter),
    ("Space", Key::Space),
    ("Backspace", Key::Backspace),
    ("LeftShift", Key::LeftShift),
    ("RightShift", Key::RightShift),
    ("LeftCtrl", Key::LeftCtrl),
    ("RightCtrl", Key::RightCtrl),
    ("LeftAlt", Key::LeftAlt),
    ("RightAlt", Key::RightAlt),
    ("Comma", Key::Comma),
    ("Period", Key::Period),
    ("Slash", Key::Slash),
    ("Semicolon", Key::Semicolon),
    ("Apostrophe", Key::Apostrophe),
    ("LeftBracket", Key::LeftBracket),
    ("RightBracket", Key::RightBracket),
    ("Minus", Key::Minus),
    ("Equal", Key::Equal),
    ("Backslash", Key::Backslash),
    ("Insert", Key::Insert),
    ("Delete", Key::Delete),
    ("Home", Key::Home),
    ("End", Key::End),
    ("PageUp", Key::PageUp),
    ("PageDown", Key::PageDown),
    ("NumPad0", Key::NumPad0),
    ("NumPad1", Key::NumPad1),
    ("NumPad2", Key::NumPad2),
    ("NumPad3", Key::NumPad3),
    ("NumPad4", Key::NumPad4),
    ("NumPad5", Key::NumPad5),
    ("NumPad6", Key::NumPad6),
    ("NumPad7", Key::NumPad7),
    ("NumPad8", Key::NumPad8),
    ("NumPad9", Key::NumPad9),
];

const BUTTONS: &[(&str, Button)] = &[
    ("South", Button::South),
    ("East", Button::East),
    ("North", Button::North),
    ("West", Button::West),
    ("C", Button::C),
    ("Z", Button::Z),
    ("LeftTrigger", Button::LeftTrigger),
    ("LeftTrigger2", Button::LeftTrigger2),
    ("RightTrigger", Button::RightTrigger),
    ("RightTrigger2", Button::RightTrigger2),
    ("Select", Button::Select),
    ("Start", Button::Start),
    ("Mode", Button::Mode),
    ("LeftThumb", Button::LeftThumb),
    ("RightThumb", Button::RightThumb),
    ("DPadUp", Button::DPadUp),
    ("DPadDown", Button::DPadDown),
    ("DPadLeft", Button::DPadLeft),
    ("DPadRight", Button::DPadRight),
];

pub fn parse_key(name: &str) -> Option<Key> {
    KEYS.iter().find(|(n, _)| *n == name).map(|(_, k)| *k)
}
pub fn parse_button(name: &str) -> Option<Button> {
    BUTTONS.iter().find(|(n, _)| *n == name).map(|(_, b)| *b)
}

/// Bindings of one player, resolved to concrete keys/buttons.
#[derive(Default)]
struct Bindings {
    keys: Vec<(Key, u32)>,
    buttons: Vec<(Button, u32)>,
    deadzone: f32,
    stick_as_dpad: bool,
}

fn resolve(cfg: &ControlsConfig) -> Vec<Bindings> {
    cfg.players
        .iter()
        .map(|p| {
            let mut b = Bindings {
                deadzone: p.deadzone,
                stick_as_dpad: p.stick_as_dpad,
                ..Default::default()
            };
            for action in ACTIONS {
                let Some(id) = action_id(action) else {
                    continue;
                };
                for k in p.keyboard.get(action).into_iter().flatten() {
                    match parse_key(k) {
                        Some(key) => b.keys.push((key, id)),
                        None => log::warn!("controls: unknown key '{k}' for {action}"),
                    }
                }
                for n in p.gamepad.get(action).into_iter().flatten() {
                    match parse_button(n) {
                        Some(btn) => b.buttons.push((btn, id)),
                        None => log::warn!("controls: unknown button '{n}' for {action}"),
                    }
                }
            }
            b
        })
        .collect()
}

fn uuid_hex(u: [u8; 16]) -> String {
    u.iter().map(|b| format!("{b:02x}")).collect()
}

pub struct Input {
    gilrs: Option<Gilrs>,
    bindings: Vec<Bindings>,
    cfg: ControlsConfig,
    last: [[bool; JOYPAD_BUTTON_COUNT]; MAX_PORTS],
}

impl Input {
    pub fn new(cfg: ControlsConfig) -> Self {
        let gilrs = match Gilrs::new() {
            Ok(g) => {
                for (_, pad) in g.gamepads() {
                    log::info!("gamepad: {} ({})", pad.name(), uuid_hex(pad.uuid()));
                }
                Some(g)
            }
            Err(e) => {
                log::warn!("gamepad support disabled: {e}");
                None
            }
        };
        let bindings = resolve(&cfg);
        Input {
            gilrs,
            bindings,
            cfg,
            last: [[false; JOYPAD_BUTTON_COUNT]; MAX_PORTS],
        }
    }

    fn pads(&self) -> Vec<(gilrs::GamepadId, PadInfo)> {
        self.gilrs
            .as_ref()
            .map(|g| {
                g.gamepads()
                    .map(|(id, p)| {
                        (
                            id,
                            PadInfo {
                                id: uuid_hex(p.uuid()),
                                name: p.name().to_string(),
                            },
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Sample keyboard + gamepads into the frontend button table.
    pub fn poll(&mut self, window: &Window) {
        let mut state = [[false; JOYPAD_BUTTON_COUNT]; MAX_PORTS];
        if let Some(g) = self.gilrs.as_mut() {
            while g.next_event().is_some() {} // keep cached state fresh
        }
        let pads = self.pads();
        let infos: Vec<PadInfo> = pads.iter().map(|(_, i)| i.clone()).collect();
        let sources: [PortSource; MAX_PORTS] = assign(&self.cfg, &infos);

        for (port, src) in sources.iter().enumerate() {
            let Some(b) = self.bindings.get(port) else {
                continue;
            };
            if src.keyboard {
                for &(key, id) in &b.keys {
                    if window.is_key_down(key) {
                        state[port][id as usize] = true;
                    }
                }
            }
            if let (Some(idx), Some(g)) = (src.pad, self.gilrs.as_ref()) {
                let pad = g.gamepad(pads[idx].0);
                for &(btn, id) in &b.buttons {
                    if pad.is_pressed(btn) {
                        state[port][id as usize] = true;
                    }
                }
                if b.stick_as_dpad {
                    let x = pad.value(gilrs::Axis::LeftStickX);
                    let y = pad.value(gilrs::Axis::LeftStickY);
                    let p = &mut state[port];
                    p[RETRO_DEVICE_ID_JOYPAD_LEFT as usize] |= x < -b.deadzone;
                    p[RETRO_DEVICE_ID_JOYPAD_RIGHT as usize] |= x > b.deadzone;
                    p[RETRO_DEVICE_ID_JOYPAD_UP as usize] |= y > b.deadzone;
                    p[RETRO_DEVICE_ID_JOYPAD_DOWN as usize] |= y < -b.deadzone;
                }
            }
        }
        if state != self.last {
            // Visible with RUST_LOG=debug: shows what the runner actually sees.
            for (port, p) in state.iter().enumerate() {
                let down: Vec<usize> = (0..JOYPAD_BUTTON_COUNT).filter(|&i| p[i]).collect();
                log::debug!("input: port {port} buttons down {down:?}");
            }
            self.last = state;
        }
        frontend::with(|fe| fe.buttons = state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_bindable_name_parses_and_defaults_resolve() {
        for (name, key) in KEYS {
            assert_eq!(parse_key(name), Some(*key));
        }
        for (name, btn) in BUTTONS {
            assert_eq!(parse_button(name), Some(*btn));
        }
        // Every key/button used by the default config must be known.
        let b = resolve(&ControlsConfig::default());
        let defaults = ControlsConfig::default();
        for (i, p) in defaults.players.iter().enumerate() {
            let want_keys: usize = p.keyboard.values().map(Vec::len).sum();
            let want_btns: usize = p.gamepad.values().map(Vec::len).sum();
            assert_eq!(b[i].keys.len(), want_keys, "player {i} keys");
            assert_eq!(b[i].buttons.len(), want_btns, "player {i} buttons");
        }
    }

    #[test]
    fn reserved_keys_are_not_bindable() {
        for reserved in ["Escape", "Tab", "P", "F2", "F3", "F5", "F6", "F7"] {
            assert!(parse_key(reserved).is_none(), "{reserved}");
        }
    }
}
