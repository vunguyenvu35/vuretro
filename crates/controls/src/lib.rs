//! Controls configuration (`<data dir>/config/controls.json`), shared by the runner
//! (which applies it) and the app (which edits it).
//!
//! Buttons are RetroPad actions (`"up"`, `"b"`, `"start"`…). Keyboard keys use the
//! names of `minifb::Key` (`"Z"`, `"Up"`, `"RightShift"`), gamepad buttons the names
//! of `gilrs::Button` (`"South"`, `"DPadUp"`, `"LeftTrigger"`). Positional gamepad
//! names mean "South" is the bottom face button on any pad.

pub mod macros;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// Number of players the frontend supports.
pub const PLAYERS: usize = 4;

/// All RetroPad actions, in display order.
pub const ACTIONS: [&str; 16] = [
    "up", "down", "left", "right", "b", "a", "y", "x", "l", "r", "l2", "r2", "l3", "r3", "select",
    "start",
];

/// Which device drives a player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Device {
    /// Player 1: keyboard plus the first free gamepad. Other players: the next free gamepad.
    Auto,
    Keyboard,
    /// A specific gamepad, by [`PadInfo::id`].
    Gamepad {
        id: String,
    },
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerConfig {
    pub device: Device,
    /// action -> key names
    pub keyboard: BTreeMap<String, Vec<String>>,
    /// action -> gamepad button names
    pub gamepad: BTreeMap<String, Vec<String>>,
    /// Stick dead zone, 0.05..=0.9.
    pub deadzone: f32,
    /// Let the left stick act as the D-pad.
    pub stick_as_dpad: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControlsConfig {
    pub players: Vec<PlayerConfig>,
}

/// A connected gamepad as seen by the runner or the app.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PadInfo {
    /// Stable id (the controller's UUID). Identical pads share it; they are told apart by order.
    pub id: String,
    pub name: String,
}

/// What feeds one emulated port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortSource {
    pub keyboard: bool,
    /// Index into the connected pad list.
    pub pad: Option<usize>,
}

fn map(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    pairs
        .iter()
        .map(|(a, k)| (a.to_string(), k.iter().map(|s| s.to_string()).collect()))
        .collect()
}

pub fn default_gamepad() -> BTreeMap<String, Vec<String>> {
    map(&[
        ("up", &["DPadUp"]),
        ("down", &["DPadDown"]),
        ("left", &["DPadLeft"]),
        ("right", &["DPadRight"]),
        ("b", &["South"]),
        ("a", &["East"]),
        ("y", &["West"]),
        ("x", &["North"]),
        ("l", &["LeftTrigger"]),
        ("r", &["RightTrigger"]),
        ("l2", &["LeftTrigger2"]),
        ("r2", &["RightTrigger2"]),
        ("l3", &["LeftThumb"]),
        ("r3", &["RightThumb"]),
        ("select", &["Select"]),
        ("start", &["Start"]),
    ])
}

pub fn default_keyboard(player: usize) -> BTreeMap<String, Vec<String>> {
    if player == 0 {
        map(&[
            ("up", &["Up"]),
            ("down", &["Down"]),
            ("left", &["Left"]),
            ("right", &["Right"]),
            ("b", &["Z"]),
            ("a", &["X"]),
            ("y", &["A"]),
            ("x", &["S"]),
            ("l", &["Q"]),
            ("r", &["E"]),
            ("l2", &["W"]),
            ("r2", &["R"]),
            ("select", &["RightShift", "Key5"]),
            ("start", &["Enter", "Key1"]),
        ])
    } else if player == 1 {
        map(&[
            ("up", &["I"]),
            ("down", &["K"]),
            ("left", &["J"]),
            ("right", &["L"]),
            ("b", &["N"]),
            ("a", &["M"]),
            ("y", &["H"]),
            ("x", &["G"]),
            ("select", &["Key6"]),
            ("start", &["Key2"]),
        ])
    } else if player == 2 {
        // Numeric keypad
        map(&[
            ("up", &["NumPad8"]),
            ("down", &["NumPad5"]),
            ("left", &["NumPad4"]),
            ("right", &["NumPad6"]),
            ("b", &["NumPad1"]),
            ("a", &["NumPad2"]),
            ("y", &["NumPad0"]),
            ("x", &["NumPad3"]),
            ("select", &["NumPad7"]),
            ("start", &["NumPad9"]),
        ])
    } else {
        // Player 4 is meant for a gamepad; bind keys yourself if you need them.
        BTreeMap::new()
    }
}

impl PlayerConfig {
    pub fn default_for(player: usize) -> Self {
        PlayerConfig {
            device: Device::Auto,
            keyboard: default_keyboard(player),
            gamepad: default_gamepad(),
            deadzone: 0.35,
            stick_as_dpad: true,
        }
    }
}

impl Default for ControlsConfig {
    fn default() -> Self {
        ControlsConfig {
            players: (0..PLAYERS).map(PlayerConfig::default_for).collect(),
        }
    }
}

impl ControlsConfig {
    /// Clamp values and make sure there are exactly [`PLAYERS`] players.
    pub fn sanitized(mut self) -> Self {
        self.players.truncate(PLAYERS);
        while self.players.len() < PLAYERS {
            self.players
                .push(PlayerConfig::default_for(self.players.len()));
        }
        for p in &mut self.players {
            p.deadzone = if p.deadzone.is_finite() {
                p.deadzone.clamp(0.05, 0.9)
            } else {
                0.35
            };
            p.keyboard.retain(|a, _| ACTIONS.contains(&a.as_str()));
            p.gamepad.retain(|a, _| ACTIONS.contains(&a.as_str()));
        }
        self
    }

    /// Swap the devices of players `a` and `b` (bindings stay with the player's slot,
    /// only the device changes hands).
    pub fn swap_devices(&mut self, a: usize, b: usize) {
        if a != b && a < self.players.len() && b < self.players.len() {
            let da = self.players[a].device.clone();
            self.players[a].device = std::mem::replace(&mut self.players[b].device, da);
        }
    }
}

/// Decide which keyboard/gamepad feeds each port.
///
/// Explicit `Gamepad{id}` choices are resolved first (the first not-yet-used pad with
/// that id), then `Auto` players take the remaining pads in connection order.
pub fn assign(cfg: &ControlsConfig, pads: &[PadInfo]) -> [PortSource; PLAYERS] {
    let mut out = [PortSource {
        keyboard: false,
        pad: None,
    }; PLAYERS];
    let mut used = vec![false; pads.len()];

    for (i, p) in cfg.players.iter().take(PLAYERS).enumerate() {
        match &p.device {
            Device::Keyboard => out[i].keyboard = true,
            Device::Gamepad { id } => {
                if let Some(idx) = (0..pads.len()).find(|&n| !used[n] && pads[n].id == *id) {
                    used[idx] = true;
                    out[i].pad = Some(idx);
                }
            }
            Device::Auto | Device::None => {}
        }
    }
    for (i, p) in cfg.players.iter().take(PLAYERS).enumerate() {
        if p.device == Device::Auto {
            out[i].keyboard = i == 0;
            if let Some(idx) = (0..pads.len()).find(|&n| !used[n]) {
                used[idx] = true;
                out[i].pad = Some(idx);
            }
        }
    }
    out
}

pub fn load(path: &Path) -> ControlsConfig {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<ControlsConfig>(&t).ok())
        .unwrap_or_default()
        .sanitized()
}

pub fn save(path: &Path, cfg: &ControlsConfig) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&cfg.clone().sanitized()).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(id: &str) -> PadInfo {
        PadInfo {
            id: id.into(),
            name: format!("pad {id}"),
        }
    }
    fn with(p1: Device, p2: Device) -> ControlsConfig {
        let mut c = ControlsConfig::default();
        c.players[0].device = p1;
        c.players[1].device = p2;
        c
    }

    #[test]
    fn auto_matches_previous_behaviour() {
        let c = ControlsConfig::default();
        let none = assign(&c, &[]);
        assert!(none[0].keyboard && none[0].pad.is_none() && !none[1].keyboard);
        let two = assign(&c, &[pad("a"), pad("b")]);
        assert_eq!((two[0].pad, two[1].pad), (Some(0), Some(1)));
        assert!(two[0].keyboard && !two[1].keyboard);
    }

    #[test]
    fn explicit_pad_wins_over_auto_and_survives_order_changes() {
        // P2 insists on pad "b" even though it is connected second; P1 auto gets "a".
        let c = with(Device::Auto, Device::Gamepad { id: "b".into() });
        let s = assign(&c, &[pad("a"), pad("b")]);
        assert_eq!((s[0].pad, s[1].pad), (Some(0), Some(1)));
        // Order flipped: P2 still gets "b", P1 auto takes the remaining one.
        let s = assign(&c, &[pad("b"), pad("a")]);
        assert_eq!((s[0].pad, s[1].pad), (Some(1), Some(0)));
    }

    #[test]
    fn switching_devices() {
        let mut c = with(Device::Keyboard, Device::Gamepad { id: "a".into() });
        c.swap_devices(0, 1);
        assert_eq!(c.players[0].device, Device::Gamepad { id: "a".into() });
        assert_eq!(c.players[1].device, Device::Keyboard);
        let s = assign(&c, &[pad("a")]);
        assert_eq!((s[0].pad, s[0].keyboard), (Some(0), false));
        assert!(s[1].keyboard);
    }

    #[test]
    fn four_players_take_pads_in_order() {
        let c = ControlsConfig::default();
        assert_eq!(c.players.len(), 4);
        let s = assign(&c, &[pad("a"), pad("b"), pad("c"), pad("d")]);
        let got: Vec<_> = s.iter().map(|x| x.pad).collect();
        assert_eq!(got, [Some(0), Some(1), Some(2), Some(3)]);
        // only player 1 gets the keyboard when Auto
        assert_eq!(s.iter().filter(|x| x.keyboard).count(), 1);
        // an old 2-player config file is padded up to 4 players
        let mut old = ControlsConfig::default();
        old.players.truncate(2);
        assert_eq!(old.sanitized().players.len(), 4);
    }

    #[test]
    fn identical_pads_and_unplugged_pads() {
        let c = with(
            Device::Gamepad { id: "x".into() },
            Device::Gamepad { id: "x".into() },
        );
        let s = assign(&c, &[pad("x"), pad("x")]);
        assert_eq!((s[0].pad, s[1].pad), (Some(0), Some(1)));
        let s = assign(&c, &[pad("x")]);
        assert_eq!((s[0].pad, s[1].pad), (Some(0), None)); // second one is unplugged
        let c = with(Device::None, Device::Auto);
        let s = assign(&c, &[pad("a")]);
        assert!(!s[0].keyboard && s[0].pad.is_none() && s[1].pad == Some(0));
    }

    #[test]
    fn roundtrip_and_sanitize() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config/controls.json");
        assert_eq!(load(&path), ControlsConfig::default()); // missing file -> defaults

        let mut c = ControlsConfig::default();
        c.players[0].deadzone = 5.0;
        c.players[0]
            .keyboard
            .insert("bogus".into(), vec!["Z".into()]);
        c.players[0].device = Device::Gamepad { id: "u".into() };
        save(&path, &c).unwrap();
        let back = load(&path);
        assert_eq!(back.players[0].deadzone, 0.9);
        assert!(!back.players[0].keyboard.contains_key("bogus"));
        assert_eq!(back.players[0].device, Device::Gamepad { id: "u".into() });

        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(load(&path), ControlsConfig::default());
    }
}
