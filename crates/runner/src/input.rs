//! Keyboard and gamepad (gilrs) mapped to the libretro RetroPad, driven by
//! `controls.json` (see the `vuretro-controls` crate). Defaults match the Controls
//! screen: arrows = D-pad, Z/X/A/S = B/A/Y/X, Enter = Start, RShift = Select.

use crate::autocoin::AutoCoin;
use crate::frontend::{self, MAX_PORTS};
use crate::padkeys::{self, PadKeys};
use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Repeat, Replay, Ticks};
use gilrs::{Button, GamepadId, Gilrs};
use libretro_sys::*;
use std::collections::HashMap;
use vuretro_controls::macros::{Engine, Macro};
use vuretro_controls::{assign, ControlsConfig, PadInfo, PortSource, ACTIONS, DIAGONALS};
use vuretro_play::ports::{Hotkeys, Keyboard};
use vuretro_play::rumble;
use vuretro_play::Key;

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
            // a diagonal key presses its two directions
            for (name, parts) in DIAGONALS {
                for k in p.keyboard.get(name).into_iter().flatten() {
                    let Some(key) = parse_key(k) else {
                        log::warn!("controls: unknown key '{k}' for {name}");
                        continue;
                    };
                    for id in parts.iter().filter_map(|d| action_id(d)) {
                        b.keys.push((key, id));
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
    /// Macro buttons for this game and the triggers they listen to.
    engine: Engine,
    triggers: Vec<(Option<Key>, Option<Button>)>,
    last: [[bool; JOYPAD_BUTTON_COUNT]; MAX_PORTS],
    /// Puts a coin in before Start on arcade systems, per port; `None` when it is off.
    auto_coin: Option<[AutoCoin; MAX_PORTS]>,
    /// Frontend hotkeys (Select + button), told apart per gamepad.
    pad_keys: HashMap<GamepadId, PadKeys>,
    /// Vibrate gamepads when the game asks for it.
    rumble_on: bool,
    rumble: rumble::Tracker,
    /// The vibration running on each port; dropping one stops it.
    effects: [Option<gilrs::ff::Effect>; MAX_PORTS],
}

impl Input {
    pub fn new(cfg: ControlsConfig, macros: Vec<Macro>, fps: f64) -> Self {
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
            triggers: macros
                .iter()
                .map(|m| {
                    (
                        m.key.as_deref().and_then(parse_key),
                        m.pad.as_deref().and_then(parse_button),
                    )
                })
                .collect(),
            engine: Engine::new(macros, fps),
            last: [[false; JOYPAD_BUTTON_COUNT]; MAX_PORTS],
            auto_coin: None,
            pad_keys: HashMap::new(),
            rumble_on: true,
            rumble: rumble::Tracker::default(),
            effects: Default::default(),
        }
    }

    /// Put a coin in whenever Start is pressed (for arcade systems).
    pub fn with_auto_coin(mut self, on: bool) -> Self {
        self.auto_coin = on.then(Default::default);
        self
    }

    /// Switch gamepad vibration on or off (the `rumble` setting).
    pub fn with_rumble(mut self, on: bool) -> Self {
        self.rumble_on = on;
        self
    }

    /// Gamepad hotkeys: Select held together with another button (see `padkeys`).
    pub fn hotkeys(&mut self) -> Hotkeys {
        let Some(g) = self.gilrs.as_mut() else {
            return Hotkeys::default();
        };
        while g.next_event().is_some() {} // keep cached state fresh
        let mut out = Hotkeys::default();
        let ids: Vec<GamepadId> = g
            .gamepads()
            .filter(|(_, p)| p.is_connected())
            .map(|(id, _)| id)
            .collect();
        self.pad_keys.retain(|id, _| ids.contains(id));
        for id in ids {
            let pad = g.gamepad(id);
            let hk = self
                .pad_keys
                .entry(id)
                .or_default()
                .read(|b| pad.is_pressed(b));
            out.acts.extend(hk.acts);
            out.fast_forward |= hk.fast_forward;
            out.rewind |= hk.rewind;
        }
        out
    }

    /// Pass the core's rumble requests on to the gamepad of each port. Does nothing on
    /// systems or pads without force feedback.
    fn drive_rumble(&mut self, pads: &[(GamepadId, PadInfo)], sources: &[PortSource; MAX_PORTS]) {
        let wanted = if self.rumble_on {
            frontend::with(|fe| fe.rumble)
        } else {
            [[0; 2]; MAX_PORTS]
        };
        for (port, r) in self.rumble.changes(wanted) {
            self.effects[port] = None; // stops the old one
            if !rumble::is_on(r) {
                continue;
            }
            let (Some(idx), Some(g)) = (sources[port].pad, self.gilrs.as_mut()) else {
                self.rumble.forget(port); // no pad now: try again when there is one
                continue;
            };
            let id = pads[idx].0;
            if !g.gamepad(id).is_ff_supported() {
                continue;
            }
            let forever = Replay {
                play_for: Ticks::from_ms(1000),
                ..Default::default()
            };
            let effect = EffectBuilder::new()
                .add_effect(BaseEffect {
                    kind: BaseEffectType::Strong { magnitude: r[0] },
                    scheduling: forever,
                    envelope: Default::default(),
                })
                .add_effect(BaseEffect {
                    kind: BaseEffectType::Weak { magnitude: r[1] },
                    scheduling: forever,
                    envelope: Default::default(),
                })
                .repeat(Repeat::Infinitely)
                .gamepads(&[id])
                .finish(g);
            match effect {
                Ok(e) => {
                    if e.play().is_ok() {
                        self.effects[port] = Some(e);
                    }
                }
                Err(e) => log::debug!("rumble not available: {e}"),
            }
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

    /// Whether an enabled macro of this player is started by `key`.
    fn is_macro_key(&self, port: usize, key: Key) -> bool {
        self.triggers
            .iter()
            .zip(self.engine.macros())
            .any(|((k, _), m)| m.player == port && *k == Some(key))
    }

    /// Sample keyboard + gamepads into the frontend button table.
    pub fn poll<K: Keyboard + ?Sized>(&mut self, window: &K) {
        let mut state = [[false; JOYPAD_BUTTON_COUNT]; MAX_PORTS];
        if let Some(g) = self.gilrs.as_mut() {
            while g.next_event().is_some() {} // keep cached state fresh
        }
        let pads = self.pads();
        let infos: Vec<PadInfo> = pads.iter().map(|(_, i)| i.clone()).collect();
        let sources: [PortSource; MAX_PORTS] = assign(&self.cfg, &infos);
        // a pad whose Select is held with a hotkey button is talking to the frontend
        let chording: Vec<bool> = pads
            .iter()
            .map(|(id, _)| {
                self.gilrs
                    .as_ref()
                    .is_some_and(|g| padkeys::chord_active(|b| g.gamepad(*id).is_pressed(b)))
            })
            .collect();

        for (port, src) in sources.iter().enumerate() {
            let Some(b) = self.bindings.get(port) else {
                continue;
            };
            if src.keyboard {
                for &(key, id) in &b.keys {
                    // a key that triggers a macro of this player only starts the macro: its
                    // own button would be pressed along with it and spoil the move
                    if self.is_macro_key(port, key) {
                        continue;
                    }
                    if window.key_down(key) {
                        state[port][id as usize] = true;
                    }
                }
            }
            if let (Some(idx), Some(g)) = (src.pad.filter(|i| !chording[*i]), self.gilrs.as_ref()) {
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
        // Macro buttons: their triggers are checked here, the buttons they press are
        // added to the normal ones.
        if !self.engine.is_empty() {
            let down: Vec<bool> = self
                .triggers
                .iter()
                .zip(self.engine.macros())
                .map(|((key, button), m)| {
                    let key_down = key.is_some_and(|k| window.key_down(k));
                    let pad_down = match (button, sources.get(m.player).and_then(|s| s.pad)) {
                        (Some(b), Some(idx)) => self
                            .gilrs
                            .as_ref()
                            .is_some_and(|g| g.gamepad(pads[idx].0).is_pressed(*b)),
                        _ => false,
                    };
                    key_down || pad_down
                })
                .collect();
            for (port, p) in state.iter().enumerate() {
                self.engine.hint_facing(
                    port,
                    p[RETRO_DEVICE_ID_JOYPAD_LEFT as usize],
                    p[RETRO_DEVICE_ID_JOYPAD_RIGHT as usize],
                );
            }
            for (player, action) in self.engine.step(&down) {
                if let (Some(id), Some(port)) = (action_id(&action), state.get_mut(player)) {
                    port[id as usize] = true;
                }
            }
        }

        if let Some(coins) = self.auto_coin.as_mut() {
            for (port, ac) in coins.iter_mut().enumerate() {
                ac.apply(&mut state[port]);
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
        self.drive_rumble(&pads, &sources);
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
        // Backspace rewinds, F1 and F4 are fast-forward and the picture look, F11 changes discs
        for reserved in [
            "Escape",
            "Tab",
            "P",
            "Backspace",
            "F1",
            "F2",
            "F3",
            "F4",
            "F5",
            "F6",
            "F7",
            "F11",
        ] {
            assert!(parse_key(reserved).is_none(), "{reserved}");
        }
    }

    // ---- a keyboard key plays a move ----

    use std::collections::{HashSet, VecDeque};
    use std::sync::{Arc, Mutex};
    use vuretro_controls::macros::Kind;

    struct Keys(HashSet<Key>);
    impl Keyboard for Keys {
        fn key_down(&self, key: Key) -> bool {
            self.0.contains(&key)
        }
    }

    /// The runner keeps its state in one global: tests that poll must not overlap.
    static POLLING: Mutex<()> = Mutex::new(());

    fn move_macro(key: &str, notation: &str, mirror: bool) -> Macro {
        Macro {
            id: "m".into(),
            key: Some(key.into()),
            kind: Kind::Sequence,
            notation: Some(notation.into()),
            mirror,
            ..Macro::default()
        }
        .sanitized_for_test()
    }

    /// The buttons player 1 holds on each frame while `key` is pressed for `down_frames`.
    fn play(m: Macro, key: Key, down_frames: usize, total: usize) -> Vec<Vec<usize>> {
        frontend::init(
            "/s",
            "/v",
            Arc::new(Mutex::new(VecDeque::new())),
            HashMap::new(),
        );
        let mut input = Input::new(ControlsConfig::default(), vec![m], 60.0);
        (0..total)
            .map(|f| {
                let held = if f < down_frames { vec![key] } else { vec![] };
                input.poll(&Keys(held.into_iter().collect()));
                frontend::with(|fe| {
                    (0..JOYPAD_BUTTON_COUNT)
                        .filter(|&b| fe.buttons[0][b])
                        .collect::<Vec<usize>>()
                })
            })
            .collect()
    }

    const DOWN: usize = RETRO_DEVICE_ID_JOYPAD_DOWN as usize;
    const RIGHT: usize = RETRO_DEVICE_ID_JOYPAD_RIGHT as usize;
    const LEFT: usize = RETRO_DEVICE_ID_JOYPAD_LEFT as usize;
    // Neo Geo A is the RetroPad B button, Neo Geo C the RetroPad Y button
    const NEO_A: usize = RETRO_DEVICE_ID_JOYPAD_B as usize;
    const NEO_C: usize = RETRO_DEVICE_ID_JOYPAD_Y as usize;

    #[test]
    fn pressing_one_key_plays_the_whole_move_even_if_it_is_let_go_at_once() {
        let _guard = POLLING.lock().unwrap_or_else(|e| e.into_inner());
        // 236P: three frames each of down, down+right, then right with the punch
        let frames = play(move_macro("V", "236P", false), Key::V, 1, 12);
        assert_eq!(frames[0], vec![DOWN]);
        assert_eq!(frames[3], vec![DOWN, RIGHT]);
        assert_eq!(frames[6], vec![NEO_A, RIGHT]);
        assert!(frames[9].is_empty(), "the move is over: {frames:?}");
        assert!(frames[11].is_empty());
    }

    #[test]
    fn the_key_does_nothing_when_it_is_not_pressed_and_the_move_plays_once_per_press() {
        let _guard = POLLING.lock().unwrap_or_else(|e| e.into_inner());
        let idle = play(move_macro("V", "236P", false), Key::V, 0, 12);
        assert!(idle.iter().all(Vec::is_empty), "{idle:?}");
        // held for a long time: still one run, not a new one every few frames
        let held = play(move_macro("V", "236P", false), Key::V, 30, 30);
        let presses = held.iter().filter(|f| f.contains(&NEO_A)).count();
        assert_eq!(presses, 3, "the punch is down for its three frames only");
    }

    #[test]
    fn a_trigger_key_only_starts_the_macro_and_does_not_press_its_own_button() {
        let _guard = POLLING.lock().unwrap_or_else(|e| e.into_inner());
        // Z is the B button of the default keys: held as the trigger it must not add that
        // button to the motion (it would spoil it), but the keys of the move still work.
        let frames = play(move_macro("Z", "236P", false), Key::Z, 40, 12);
        assert_eq!(frames[0], vec![DOWN], "{frames:?}");
        assert_eq!(frames[3], vec![DOWN, RIGHT]);
        // and the key is a plain button again in a game with no macro on it
        let plain = play(move_macro("V", "236P", false), Key::Z, 1, 2);
        assert_eq!(plain[0], vec![NEO_A]);
    }

    #[test]
    fn a_move_for_a_character_facing_left_mirrors_the_direction() {
        let _guard = POLLING.lock().unwrap_or_else(|e| e.into_inner());
        let frames = play(move_macro("V", "236P", true), Key::V, 1, 12);
        assert_eq!(frames[3], vec![DOWN, LEFT]);
        assert_eq!(frames[6], vec![NEO_A, LEFT]);
    }

    #[test]
    fn a_combo_chains_its_moves_on_one_key_press() {
        let _guard = POLLING.lock().unwrap_or_else(|e| e.into_inner());
        let frames = play(move_macro("C", "cl.C > 236P", false), Key::C, 1, 24);
        assert_eq!(frames[0], vec![NEO_C], "the heavy punch first");
        let punch = frames.iter().position(|f| f.contains(&NEO_A)).unwrap();
        assert!(
            punch > 6,
            "then the fireball motion, ending in the punch: {frames:?}"
        );
    }

    #[test]
    fn a_diagonal_key_presses_both_directions_at_once() {
        let _guard = POLLING.lock().unwrap_or_else(|e| e.into_inner());
        frontend::init(
            "/s",
            "/v",
            Arc::new(Mutex::new(VecDeque::new())),
            HashMap::new(),
        );
        let mut cfg = ControlsConfig::default();
        cfg.players[0]
            .keyboard
            .insert("down_right".into(), vec!["M".into()]);
        cfg.players[0]
            .keyboard
            .insert("up_left".into(), vec!["I".into()]);
        let mut input = Input::new(cfg, vec![], 60.0);
        let held = |input: &mut Input, key: Key| {
            input.poll(&Keys([key].into_iter().collect()));
            frontend::with(|fe| {
                (0..JOYPAD_BUTTON_COUNT)
                    .filter(|&b| fe.buttons[0][b])
                    .collect::<Vec<usize>>()
            })
        };
        assert_eq!(held(&mut input, Key::M), vec![DOWN, RIGHT]);
        let up = RETRO_DEVICE_ID_JOYPAD_UP as usize;
        assert_eq!(held(&mut input, Key::I), vec![up, LEFT]);
        // the plain arrows are untouched
        assert_eq!(held(&mut input, Key::Down), vec![DOWN]);
        assert!(held(&mut input, Key::K).is_empty());
    }

    #[test]
    fn one_move_works_on_either_side_by_holding_the_direction_towards_the_opponent() {
        let _guard = POLLING.lock().unwrap_or_else(|e| e.into_inner());
        frontend::init(
            "/s",
            "/v",
            Arc::new(Mutex::new(VecDeque::new())),
            HashMap::new(),
        );
        let m = Macro {
            auto_facing: true,
            ..move_macro("V", "236P", false)
        };
        let run = |held: &[Key]| {
            let mut input = Input::new(ControlsConfig::default(), vec![m.clone()], 60.0);
            let keys: HashSet<Key> = held.iter().copied().chain([Key::V]).collect();
            (0..8)
                .map(|_| {
                    input.poll(&Keys(keys.clone()));
                    frontend::with(|fe| {
                        (0..JOYPAD_BUTTON_COUNT)
                            .filter(|&b| fe.buttons[0][b])
                            .collect::<Vec<usize>>()
                    })
                })
                .collect::<Vec<_>>()
        };
        // holding left (towards an opponent on the left): the fireball goes the other way round
        let left = run(&[Key::Left]);
        assert!(
            left[3].contains(&LEFT) && left[3].contains(&DOWN),
            "{left:?}"
        );
        // holding right: as written
        let right = run(&[Key::Right]);
        assert!(
            right[3].contains(&RIGHT) && right[3].contains(&DOWN),
            "{right:?}"
        );
    }
}
