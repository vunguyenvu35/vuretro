//! Game settings (`<data dir>/config/game.json`), shared by the runner (which applies
//! them when a game starts) and the app (which edits them).

pub mod cheats;
pub mod core_options;
pub mod scoped;

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const MIN_SCALE: u8 = 1;
pub const MAX_SCALE: u8 = 8;

/// Fast-forward speeds the player can choose, as multiples of normal speed.
pub const FAST_FORWARD_SPEEDS: [u8; 4] = [2, 3, 4, 8];
pub const DEFAULT_FAST_FORWARD: u8 = 4;

/// The allowed speed closest to `n` that is not faster than asked (at least the slowest).
pub fn snap_fast_forward(n: u8) -> u8 {
    FAST_FORWARD_SPEEDS
        .iter()
        .copied()
        .filter(|s| *s <= n)
        .max()
        .unwrap_or(FAST_FORWARD_SPEEDS[0])
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GameSettings {
    /// Window size as a multiple of the game's native resolution, 1..=8.
    pub window_scale: u8,
    /// Sound on or off.
    pub sound: bool,
    /// Volume in percent, 0..=100.
    pub volume: u8,
    /// Run video at the display's refresh rate when it is close to the game's own
    /// (smoother motion; the game may run up to 5 % faster or slower than the original).
    pub sync_display: bool,
    /// Pause the game while its window is not the active one.
    pub pause_on_blur: bool,
    /// Scanline look (CRT effect).
    pub crt: bool,
    /// Keep recent states so a game can be rewound (hold Backspace). Costs memory and CPU.
    pub rewind: bool,
    /// Speed while the fast-forward key is held (or toggled): 2, 3, 4 or 8 times normal.
    pub fast_forward: u8,
    /// Keep the sound on while fast-forwarding (it plays faster and higher). Off: muted.
    pub ff_sound: bool,
    /// Save the game when the window closes and carry on from there next time.
    pub resume: bool,
    /// Handheld LCD look: a pixel grid. CRT wins when both are on.
    pub lcd: bool,
    /// Scale2x: doubles the picture and smooths the stairs on diagonal edges (CPU filter).
    pub scale2x: bool,
    /// Scale the picture by whole numbers only, with black borders instead of blur.
    pub integer_scale: bool,
    /// Start in a full-screen window.
    pub fullscreen: bool,
    /// Make the gamepad rumble when the game asks for it (not supported on every OS).
    pub rumble: bool,
    /// On arcade systems (Neo Geo, MAME): put a coin in whenever Start is pressed, so Start
    /// always begins a game (on by default). Does nothing on systems without coins.
    pub auto_coin: bool,
}

impl Default for GameSettings {
    fn default() -> Self {
        GameSettings {
            window_scale: 2,
            sound: true,
            volume: 100,
            sync_display: true,
            pause_on_blur: false,
            crt: false,
            rewind: false,
            fast_forward: DEFAULT_FAST_FORWARD,
            ff_sound: false,
            resume: false,
            lcd: false,
            scale2x: false,
            integer_scale: false,
            fullscreen: false,
            rumble: true,
            auto_coin: true,
        }
    }
}

impl GameSettings {
    pub fn sanitized(mut self) -> Self {
        self.window_scale = self.window_scale.clamp(MIN_SCALE, MAX_SCALE);
        self.volume = self.volume.min(100);
        self.fast_forward = snap_fast_forward(self.fast_forward);
        self
    }

    /// Output gain in percent that the audio callback applies (0 when muted).
    pub fn gain_percent(&self) -> u32 {
        if self.sound {
            self.volume as u32
        } else {
            0
        }
    }
}

pub fn load(path: &Path) -> GameSettings {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<GameSettings>(&t).ok())
        .unwrap_or_default()
        .sanitized()
}

pub fn save(path: &Path, settings: &GameSettings) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text =
        serde_json::to_string_pretty(&settings.clone().sanitized()).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_sanitize() {
        let d = GameSettings::default();
        assert_eq!((d.window_scale, d.sound, d.volume), (2, true, 100));
        let s = GameSettings {
            window_scale: 99,
            sound: true,
            volume: 250,
            sync_display: true,
            pause_on_blur: false,
            fast_forward: 7,
            ..GameSettings::default()
        }
        .sanitized();
        assert_eq!((s.window_scale, s.volume), (MAX_SCALE, 100));
        assert_eq!(s.fast_forward, 4, "7x is not offered: the next lower one");
        assert_eq!(
            GameSettings {
                window_scale: 0,
                ..d
            }
            .sanitized()
            .window_scale,
            1
        );
    }

    #[test]
    fn fast_forward_snaps_to_an_offered_speed() {
        assert_eq!(snap_fast_forward(0), 2);
        assert_eq!(snap_fast_forward(2), 2);
        assert_eq!(snap_fast_forward(3), 3);
        assert_eq!(snap_fast_forward(5), 4);
        assert_eq!(snap_fast_forward(200), 8);
    }

    #[test]
    fn gain_follows_sound_switch() {
        let mut s = GameSettings {
            volume: 40,
            ..Default::default()
        };
        assert_eq!(s.gain_percent(), 40);
        s.sound = false;
        assert_eq!(s.gain_percent(), 0);
    }

    #[test]
    fn roundtrip_and_partial_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config/game.json");
        assert_eq!(load(&path), GameSettings::default());
        let s = GameSettings {
            window_scale: 3,
            sound: false,
            volume: 70,
            sync_display: false,
            pause_on_blur: true,
            crt: true,
            rewind: true,
            fast_forward: 8,
            ff_sound: true,
            resume: true,
            lcd: true,
            scale2x: true,
            integer_scale: true,
            fullscreen: true,
            rumble: false,
            auto_coin: false,
        };
        save(&path, &s).unwrap();
        assert_eq!(load(&path), s);
        // a file written by an older version with missing keys still loads
        std::fs::write(&path, r#"{ "window_scale": 4 }"#).unwrap();
        let p = load(&path);
        assert_eq!((p.window_scale, p.sound, p.volume), (4, true, 100));
        assert!(p.sync_display && !p.pause_on_blur);
        // settings added later fall back to their defaults
        assert_eq!((p.fast_forward, p.ff_sound, p.resume), (4, false, false));
        assert!(p.rumble && !p.fullscreen && !p.lcd && !p.integer_scale && p.auto_coin);
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(load(&path), GameSettings::default());
    }
}
