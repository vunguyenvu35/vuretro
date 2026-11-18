//! Game settings (`<data dir>/config/game.json`), shared by the runner (which applies
//! them when a game starts) and the app (which edits them).

pub mod core_options;

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const MIN_SCALE: u8 = 1;
pub const MAX_SCALE: u8 = 8;

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
}

impl Default for GameSettings {
    fn default() -> Self {
        GameSettings {
            window_scale: 2,
            sound: true,
            volume: 100,
            sync_display: true,
            pause_on_blur: false,
        }
    }
}

impl GameSettings {
    pub fn sanitized(mut self) -> Self {
        self.window_scale = self.window_scale.clamp(MIN_SCALE, MAX_SCALE);
        self.volume = self.volume.min(100);
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
        }
        .sanitized();
        assert_eq!((s.window_scale, s.volume), (MAX_SCALE, 100));
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
        };
        save(&path, &s).unwrap();
        assert_eq!(load(&path), s);
        // a file written by an older version with missing keys still loads
        std::fs::write(&path, r#"{ "window_scale": 4 }"#).unwrap();
        let p = load(&path);
        assert_eq!((p.window_scale, p.sound, p.volume), (4, true, 100));
        assert!(p.sync_display && !p.pause_on_blur);
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(load(&path), GameSettings::default());
    }
}
