//! Macro buttons (`<data dir>/config/macros.json`): one key or pad button that does
//! more than one thing. Shared by the runner (which plays them) and the app (which
//! edits and records them).
//!
//! * **Combo**: while the trigger is held, press several buttons together.
//! * **Turbo**: while the trigger is held, tap the buttons over and over.
//! * **Sequence**: when the trigger is pressed, play a list of steps, each a set of
//!   buttons held for some milliseconds. Optionally keep repeating while it is held.
//!
//! Macros are either global or belong to one game (keyed by the ROM file name without
//! extension, lower case). A game's own macros are played together with the global ones.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

use crate::{ACTIONS, PLAYERS};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    #[default]
    Combo,
    Turbo,
    Sequence,
}

/// One step of a sequence: these buttons are held for `ms` milliseconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Step {
    pub actions: Vec<String>,
    pub ms: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Macro {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    /// Which player's port the buttons are pressed on (0-based).
    pub player: usize,
    /// Trigger: a keyboard key name and/or a gamepad button name.
    pub key: Option<String>,
    pub pad: Option<String>,
    pub kind: Kind,
    /// Buttons for a combo or turbo macro.
    pub actions: Vec<String>,
    /// Turbo speed in presses per second.
    pub rate_hz: u32,
    /// Steps of a sequence.
    pub steps: Vec<Step>,
    /// Sequence: start over again while the trigger is still held.
    pub repeat: bool,
}

impl Default for Macro {
    fn default() -> Self {
        Macro {
            id: String::new(),
            name: "Macro".into(),
            enabled: true,
            player: 0,
            key: None,
            pad: None,
            kind: Kind::Combo,
            actions: Vec::new(),
            rate_hz: 15,
            steps: Vec::new(),
            repeat: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MacroConfig {
    /// Played in every game.
    pub global: Vec<Macro>,
    /// Per game, by [`game_key`].
    pub games: BTreeMap<String, Vec<Macro>>,
}

/// The key a game's macros are stored under: its file name without extension, lower case.
pub fn game_key(rom: &str) -> String {
    let name = rom.rsplit(['/', '\\']).next().unwrap_or(rom);
    let stem = name.rsplit_once('.').map_or(name, |(s, _)| s);
    stem.to_ascii_lowercase()
}

fn clean_actions(list: &mut Vec<String>) {
    list.retain(|a| ACTIONS.contains(&a.as_str()));
    let mut seen = Vec::new();
    list.retain(|a| {
        let new = !seen.contains(a);
        seen.push(a.clone());
        new
    });
}

impl Macro {
    fn sanitized(mut self) -> Self {
        self.player = self.player.min(PLAYERS - 1);
        self.rate_hz = self.rate_hz.clamp(1, 30);
        clean_actions(&mut self.actions);
        for s in &mut self.steps {
            clean_actions(&mut s.actions);
            s.ms = s.ms.clamp(10, 10_000);
        }
        self.steps.truncate(200);
        self.name.truncate(60);
        self.key = self.key.filter(|k| !k.is_empty());
        self.pad = self.pad.filter(|k| !k.is_empty());
        self
    }
}

impl MacroConfig {
    pub fn sanitized(mut self) -> Self {
        let fix = |v: Vec<Macro>| -> Vec<Macro> { v.into_iter().map(Macro::sanitized).collect() };
        self.global = fix(self.global);
        self.games = self
            .games
            .into_iter()
            .map(|(k, v)| (game_key(&k), fix(v)))
            .filter(|(_, v)| !v.is_empty())
            .collect();
        self
    }

    /// The macros active in a game: its own first, then the global ones.
    pub fn for_game(&self, key: &str) -> Vec<Macro> {
        let mut out = self.games.get(key).cloned().unwrap_or_default();
        out.extend(self.global.iter().cloned());
        out.retain(|m| m.enabled && (m.key.is_some() || m.pad.is_some()));
        out
    }
}

pub fn load(path: &Path) -> MacroConfig {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<MacroConfig>(&t).ok())
        .unwrap_or_default()
        .sanitized()
}

pub fn save(path: &Path, cfg: &MacroConfig) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&cfg.clone().sanitized()).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

// ---------------------------------------------------------------- playback

#[derive(Default, Clone)]
struct Run {
    was_down: bool,
    /// Frames since the trigger went down (turbo).
    held_for: u32,
    /// Sequence playback: current step and frames spent in it.
    playing: bool,
    step: usize,
    in_step: u32,
}

/// Plays macros frame by frame. Call [`Engine::step`] once per emulated frame.
pub struct Engine {
    macros: Vec<Macro>,
    runs: Vec<Run>,
    fps: f64,
}

impl Engine {
    pub fn new(macros: Vec<Macro>, fps: f64) -> Self {
        let runs = vec![Run::default(); macros.len()];
        Engine {
            macros,
            runs,
            fps: fps.max(1.0),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.macros.is_empty()
    }

    /// The macros, so the caller can look at their triggers.
    pub fn macros(&self) -> &[Macro] {
        &self.macros
    }

    fn frames(&self, ms: u32) -> u32 {
        ((ms as f64 * self.fps / 1000.0).round() as u32).max(1)
    }

    /// Advance one frame. `down[i]` says whether macro `i`'s trigger is held. Returns the
    /// buttons to press this frame as (player, action) pairs.
    pub fn step(&mut self, down: &[bool]) -> Vec<(usize, String)> {
        let mut out = Vec::new();
        for i in 0..self.macros.len() {
            let is_down = down.get(i).copied().unwrap_or(false);
            let m = &self.macros[i];
            let frames_of: Vec<u32> = m.steps.iter().map(|s| self.frames(s.ms)).collect();
            let half = ((self.fps / (2.0 * m.rate_hz as f64)).round() as u32).max(1);
            let r = &mut self.runs[i];
            let pressed_now = is_down && !r.was_down;
            r.was_down = is_down;
            let mut press = |actions: &[String]| {
                out.extend(actions.iter().map(|a| (m.player, a.clone())));
            };
            match m.kind {
                Kind::Combo => {
                    if is_down {
                        press(&m.actions);
                    }
                }
                Kind::Turbo => {
                    if is_down {
                        if (r.held_for / half).is_multiple_of(2) {
                            press(&m.actions);
                        }
                        r.held_for += 1;
                    } else {
                        r.held_for = 0;
                    }
                }
                Kind::Sequence => {
                    if m.steps.is_empty() {
                        continue;
                    }
                    if !r.playing && pressed_now {
                        *r = Run {
                            was_down: is_down,
                            playing: true,
                            ..Run::default()
                        };
                    }
                    if r.playing {
                        press(&m.steps[r.step].actions);
                        r.in_step += 1;
                        if r.in_step >= frames_of[r.step] {
                            r.in_step = 0;
                            r.step += 1;
                            if r.step >= m.steps.len() {
                                r.step = 0;
                                r.playing = m.repeat && is_down;
                            }
                        }
                    }
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }
    fn mac(kind: Kind) -> Macro {
        Macro {
            id: "m".into(),
            key: Some("F".into()),
            kind,
            ..Macro::default()
        }
    }
    fn pressed(out: &[(usize, String)]) -> Vec<&str> {
        out.iter().map(|(_, s)| s.as_str()).collect()
    }

    #[test]
    fn combo_presses_all_buttons_while_held() {
        let m = Macro {
            actions: a(&["b", "a", "y"]),
            ..mac(Kind::Combo)
        };
        let mut e = Engine::new(vec![m], 60.0);
        assert!(e.step(&[false]).is_empty());
        assert_eq!(pressed(&e.step(&[true])), ["b", "a", "y"]);
        assert!(e.step(&[false]).is_empty());
    }

    #[test]
    fn turbo_taps_at_the_rate() {
        let m = Macro {
            actions: a(&["b"]),
            rate_hz: 15, // 60 fps -> 2 frames down, 2 frames up
            ..mac(Kind::Turbo)
        };
        let mut e = Engine::new(vec![m], 60.0);
        let frames: Vec<bool> = (0..8).map(|_| !e.step(&[true]).is_empty()).collect();
        assert_eq!(frames, [true, true, false, false, true, true, false, false]);
        assert!(e.step(&[false]).is_empty());
        // pressing again starts with a tap
        assert!(!e.step(&[true]).is_empty());
    }

    #[test]
    fn sequence_plays_once_even_if_released_and_can_repeat() {
        let steps = vec![
            Step {
                actions: a(&["down"]),
                ms: 50, // 3 frames at 60 fps
            },
            Step {
                actions: a(&["down", "right"]),
                ms: 17, // 1 frame
            },
            Step {
                actions: a(&["a"]),
                ms: 33, // 2 frames
            },
        ];
        let once = Macro {
            steps: steps.clone(),
            ..mac(Kind::Sequence)
        };
        let mut e = Engine::new(vec![once], 60.0);
        let mut got = vec![pressed(&e.step(&[true])).join("+")];
        for _ in 0..8 {
            got.push(pressed(&e.step(&[false])).join("+")); // released at once: still plays
        }
        assert_eq!(
            got,
            ["down", "down", "down", "down+right", "a", "a", "", "", ""]
        );

        let looped = Macro {
            steps,
            repeat: true,
            ..mac(Kind::Sequence)
        };
        let mut e = Engine::new(vec![looped], 60.0);
        let first: Vec<_> = (0..7)
            .map(|_| pressed(&e.step(&[true])).join("+"))
            .collect();
        assert_eq!(first[6], "down"); // started over while still held
        let _ = e.step(&[false]);
    }

    #[test]
    fn players_and_config_rules() {
        let m = Macro {
            player: 1,
            actions: a(&["b", "bogus", "b"]),
            ..mac(Kind::Combo)
        };
        let mut cfg = MacroConfig::default();
        cfg.global.push(Macro {
            rate_hz: 500,
            ..mac(Kind::Turbo)
        });
        cfg.games.insert("MSlug2.zip".into(), vec![m.clone()]);
        let cfg = cfg.sanitized();
        assert!(cfg.games.contains_key("mslug2"));
        assert_eq!(cfg.global[0].rate_hz, 30);
        assert_eq!(cfg.games["mslug2"][0].actions, a(&["b"]));
        // a game's own macros come first, then the global ones
        let list = cfg.for_game("mslug2");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].player, 1);
        assert_eq!(cfg.for_game("aof").len(), 1);
        let mut e = Engine::new(vec![list[0].clone()], 60.0);
        assert_eq!(e.step(&[true]), [(1, "b".to_string())]);
        assert_eq!(game_key("/x/y/Metal Slug.ZIP"), "metal slug");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config/macros.json");
        assert_eq!(load(&path), MacroConfig::default());
        let mut cfg = MacroConfig::default();
        cfg.games.insert(
            "aof".into(),
            vec![Macro {
                actions: a(&["a", "b"]),
                ..mac(Kind::Combo)
            }],
        );
        save(&path, &cfg).unwrap();
        assert_eq!(load(&path), cfg);
        std::fs::write(&path, "{ nope").unwrap();
        assert_eq!(load(&path), MacroConfig::default());
    }
}
