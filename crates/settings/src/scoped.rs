//! Settings in three layers: everything, one system, one game. A layer only stores what it
//! changes; the rest is inherited from the layer above, so a later change to the general
//! settings still reaches every system that did not override it.
//!
//! Only the settings that differ in a useful way between systems can be overridden: window
//! size (a Game Boy needs a bigger window than a PlayStation), the picture look (CRT, LCD,
//! whole-number scaling), rewind, fast-forward speed and resuming.
//! Sound and focus behaviour are habits of the player, not of a system, so they stay general.

use crate::GameSettings;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// The overridable settings of one layer; `None` means "inherit".
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Override {
    pub window_scale: Option<u8>,
    pub crt: Option<bool>,
    pub rewind: Option<bool>,
    pub fast_forward: Option<u8>,
    pub lcd: Option<bool>,
    pub integer_scale: Option<bool>,
    pub resume: Option<bool>,
    pub auto_coin: Option<bool>,
}

impl Override {
    pub fn is_empty(&self) -> bool {
        *self == Override::default()
    }

    /// `base` with this layer's choices put on top.
    pub fn apply(&self, mut base: GameSettings) -> GameSettings {
        if let Some(v) = self.window_scale {
            base.window_scale = v;
        }
        if let Some(v) = self.crt {
            base.crt = v;
        }
        if let Some(v) = self.rewind {
            base.rewind = v;
        }
        if let Some(v) = self.fast_forward {
            base.fast_forward = v;
        }
        if let Some(v) = self.lcd {
            base.lcd = v;
        }
        if let Some(v) = self.integer_scale {
            base.integer_scale = v;
        }
        if let Some(v) = self.resume {
            base.resume = v;
        }
        if let Some(v) = self.auto_coin {
            base.auto_coin = v;
        }
        base.sanitized()
    }

    pub fn sanitized(mut self) -> Self {
        self.window_scale = self
            .window_scale
            .map(|v| v.clamp(crate::MIN_SCALE, crate::MAX_SCALE));
        self.fast_forward = self.fast_forward.map(crate::snap_fast_forward);
        self
    }
}

/// Which system and game a session is for (the game is its file name without extension).
#[derive(Debug, Clone, Copy)]
pub struct Scope<'a> {
    pub system: &'a str,
    pub game: &'a str,
}

/// One change made while playing (a hotkey).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Change {
    WindowScale(u8),
    Crt(bool),
    Lcd(bool),
    FastForward(u8),
    /// Full screen is a habit of the player, not of a system: always the general setting.
    Fullscreen(bool),
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Overrides {
    pub systems: BTreeMap<String, Override>,
    pub games: BTreeMap<String, Override>,
}

impl Overrides {
    /// What actually applies to a game: general, then its system, then the game itself.
    pub fn resolve(&self, base: &GameSettings, scope: Scope) -> GameSettings {
        let mut s = base.clone();
        if let Some(o) = self.systems.get(scope.system) {
            s = o.apply(s);
        }
        if let Some(o) = self.games.get(scope.game) {
            s = o.apply(s);
        }
        s.sanitized()
    }

    /// Replace a system's override; an empty one removes the entry.
    pub fn set_system(&mut self, system: &str, ov: Override) {
        let ov = ov.sanitized();
        if ov.is_empty() {
            self.systems.remove(system);
        } else {
            self.systems.insert(system.to_string(), ov);
        }
    }

    pub fn set_game(&mut self, game: &str, ov: Override) {
        let ov = ov.sanitized();
        if ov.is_empty() {
            self.games.remove(game);
        } else {
            self.games.insert(game.to_string(), ov);
        }
    }

    /// Record a change made during play at the most specific layer that already overrides
    /// that setting (game, then system), or in the general settings when none does. This
    /// way a hotkey changes what the player sees on the next start, whatever layer owns it.
    pub fn record_change(&mut self, base: &mut GameSettings, scope: Scope, change: Change) {
        let owns = |o: Option<&Override>| match change {
            Change::WindowScale(_) => o.is_some_and(|o| o.window_scale.is_some()),
            Change::Crt(_) => o.is_some_and(|o| o.crt.is_some()),
            Change::Lcd(_) => o.is_some_and(|o| o.lcd.is_some()),
            Change::FastForward(_) => o.is_some_and(|o| o.fast_forward.is_some()),
            Change::Fullscreen(_) => false,
        };
        let set = |o: &mut Override| match change {
            Change::WindowScale(v) => o.window_scale = Some(v),
            Change::Crt(v) => o.crt = Some(v),
            Change::Lcd(v) => o.lcd = Some(v),
            Change::FastForward(v) => o.fast_forward = Some(crate::snap_fast_forward(v)),
            Change::Fullscreen(_) => {}
        };
        if owns(self.games.get(scope.game)) {
            set(self.games.get_mut(scope.game).unwrap());
        } else if owns(self.systems.get(scope.system)) {
            set(self.systems.get_mut(scope.system).unwrap());
        } else {
            match change {
                Change::WindowScale(v) => base.window_scale = v,
                Change::Crt(v) => base.crt = v,
                Change::Lcd(v) => base.lcd = v,
                Change::FastForward(v) => base.fast_forward = crate::snap_fast_forward(v),
                Change::Fullscreen(v) => base.fullscreen = v,
            }
        }
    }
}

pub fn load_overrides(path: &Path) -> Overrides {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Save the overrides; an empty set removes the file.
pub fn save_overrides(path: &Path, overrides: &Overrides) -> Result<(), String> {
    if *overrides == Overrides::default() {
        return match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                Err(format!("cannot remove {}: {e}", path.display()))
            }
            _ => Ok(()),
        };
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(overrides).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SNES: Scope = Scope {
        system: "snes",
        game: "mario",
    };

    fn ov(scale: Option<u8>, crt: Option<bool>) -> Override {
        Override {
            window_scale: scale,
            crt,
            ..Override::default()
        }
    }

    #[test]
    fn layers_stack_and_only_override_what_they_set() {
        let base = GameSettings {
            window_scale: 2,
            volume: 70,
            ..Default::default()
        };
        let mut o = Overrides::default();
        assert_eq!(
            o.resolve(&base, SNES),
            base,
            "no overrides: the general settings"
        );

        o.set_system("snes", ov(Some(4), Some(true)));
        let s = o.resolve(&base, SNES);
        assert_eq!((s.window_scale, s.crt, s.volume), (4, true, 70));
        // another system is untouched
        let other = Scope {
            system: "psx",
            game: "crash",
        };
        assert_eq!(o.resolve(&base, other), base);

        // the game wins over its system, for the fields it sets
        o.set_game("mario", ov(Some(6), None));
        let s = o.resolve(&base, SNES);
        assert_eq!((s.window_scale, s.crt), (6, true));
    }

    #[test]
    fn new_fields_stack_like_the_old_ones() {
        let base = GameSettings::default();
        let mut o = Overrides::default();
        o.set_system(
            "gb",
            Override {
                fast_forward: Some(8),
                lcd: Some(true),
                integer_scale: Some(true),
                resume: Some(true),
                auto_coin: Some(true),
                ..Override::default()
            },
        );
        let gb = Scope {
            system: "gb",
            game: "tetris",
        };
        let s = o.resolve(&base, gb);
        assert_eq!(
            (s.fast_forward, s.lcd, s.integer_scale, s.resume),
            (8, true, true, true)
        );
        assert!(s.auto_coin);
        assert_eq!(o.resolve(&base, SNES), base);
        // an offered speed is enforced even on a hand-edited file
        o.set_system(
            "gb",
            Override {
                fast_forward: Some(5),
                ..Override::default()
            },
        );
        assert_eq!(o.systems["gb"].fast_forward, Some(4));
    }

    #[test]
    fn hotkey_changes_to_speed_and_lcd_follow_ownership_and_fullscreen_stays_general() {
        let mut base = GameSettings::default();
        let mut o = Overrides::default();
        o.record_change(&mut base, SNES, Change::FastForward(2));
        o.record_change(&mut base, SNES, Change::Lcd(true));
        assert_eq!((base.fast_forward, base.lcd), (2, true));

        o.set_system(
            "snes",
            Override {
                fast_forward: Some(3),
                ..Override::default()
            },
        );
        o.record_change(&mut base, SNES, Change::FastForward(8));
        assert_eq!(
            (base.fast_forward, o.systems["snes"].fast_forward),
            (2, Some(8))
        );

        // even with a system override of something else, fullscreen is the general setting
        o.record_change(&mut base, SNES, Change::Fullscreen(true));
        assert!(base.fullscreen);
    }

    #[test]
    fn empty_overrides_are_dropped_and_scales_are_clamped() {
        let mut o = Overrides::default();
        o.set_system("snes", ov(Some(99), None));
        assert_eq!(o.systems["snes"].window_scale, Some(crate::MAX_SCALE));
        o.set_system("snes", Override::default());
        assert!(o.systems.is_empty());
        assert_eq!(o, Overrides::default());
    }

    #[test]
    fn a_hotkey_change_lands_on_the_layer_that_owns_the_setting() {
        let mut base = GameSettings::default();
        let mut o = Overrides::default();
        // nobody overrides the window size: the general setting changes
        o.record_change(&mut base, SNES, Change::WindowScale(3));
        assert_eq!(base.window_scale, 3);
        assert!(o.systems.is_empty());

        // the system overrides it: the system changes, general stays
        o.set_system("snes", ov(Some(4), None));
        o.record_change(&mut base, SNES, Change::WindowScale(5));
        assert_eq!(
            (base.window_scale, o.systems["snes"].window_scale),
            (3, Some(5))
        );

        // a game override wins over the system's
        o.set_game("mario", ov(Some(2), None));
        o.record_change(&mut base, SNES, Change::WindowScale(7));
        assert_eq!(o.games["mario"].window_scale, Some(7));
        assert_eq!(o.systems["snes"].window_scale, Some(5));

        // CRT is a different field: still general, because no layer overrides it
        o.record_change(&mut base, SNES, Change::Crt(true));
        assert!(base.crt && o.systems["snes"].crt.is_none());
    }

    #[test]
    fn files_round_trip_and_an_empty_set_removes_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config/overrides.json");
        assert_eq!(load_overrides(&path), Overrides::default());
        let mut o = Overrides::default();
        o.set_system("gba", ov(Some(5), Some(false)));
        save_overrides(&path, &o).unwrap();
        assert_eq!(load_overrides(&path), o);
        // an older or partial file still loads
        std::fs::write(&path, r#"{ "systems": { "nes": { "crt": true } } }"#).unwrap();
        assert_eq!(load_overrides(&path).systems["nes"], ov(None, Some(true)));
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(load_overrides(&path), Overrides::default());
        save_overrides(&path, &Overrides::default()).unwrap();
        assert!(!path.exists());
    }
}
