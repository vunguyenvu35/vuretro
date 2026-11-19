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
    /// A move or combo in fighting-game notation (`236P`, `j.D, cl.C > 236P`). When it is set
    /// the steps of the sequence are made from it (see `moves`), not edited by hand.
    pub notation: Option<String>,
    /// For a character that faces left: forward and back swap in the notation.
    pub mirror: bool,
    /// Choose the side when the trigger is pressed: holding left plays the move mirrored,
    /// holding right plays it as written (`mirror` decides when neither is held).
    pub auto_facing: bool,
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
            notation: None,
            mirror: false,
            auto_facing: false,
        }
    }
}

/// A named set of macros for one game, for example one per character of a fighting game.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub macros: Vec<Macro>,
}

/// Most profiles a game can have, and the longest name.
pub const MAX_PROFILES: usize = 50;
const MAX_PROFILE_NAME: usize = 40;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MacroConfig {
    /// Played in every game.
    pub global: Vec<Macro>,
    /// Per game, by [`game_key`]: the macros every character of the game shares.
    pub games: BTreeMap<String, Vec<Macro>>,
    /// Per game: sets of macros you switch between (a character each).
    pub profiles: BTreeMap<String, Vec<Profile>>,
    /// Per game: the id of the profile in use. Without one only the shared macros play.
    pub active: BTreeMap<String, String>,
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
    /// [`Macro::sanitized`] for tests of other crates (it makes the steps of a move).
    pub fn sanitized_for_test(self) -> Self {
        self.sanitized()
    }

    fn sanitized(mut self) -> Self {
        self.player = self.player.min(PLAYERS - 1);
        self.rate_hz = self.rate_hz.clamp(1, 30);
        // A move written in notation is the source of truth for the steps of its sequence. An
        // empty notation is still a move (one that is just being written): only `None` is not.
        self.notation = self
            .notation
            .take()
            .map(|n| n.trim().chars().take(400).collect::<String>());
        if let (Some(n), Kind::Sequence) = (&self.notation, self.kind) {
            if let Ok(steps) = crate::moves::compile(n, self.mirror) {
                self.steps = steps;
            }
        }
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
        self.profiles = self
            .profiles
            .into_iter()
            .map(|(k, list)| {
                let mut seen = Vec::new();
                let list: Vec<Profile> = list
                    .into_iter()
                    .filter(|p| {
                        !p.id.is_empty() && !seen.contains(&p.id) && {
                            seen.push(p.id.clone());
                            true
                        }
                    })
                    .take(MAX_PROFILES)
                    .map(|p| Profile {
                        name: p.name.trim().chars().take(MAX_PROFILE_NAME).collect(),
                        macros: fix(p.macros),
                        ..p
                    })
                    .collect();
                (game_key(&k), list)
            })
            .filter(|(_, list)| !list.is_empty())
            .collect();
        // the profile in use has to exist
        let profiles = &self.profiles;
        self.active = self
            .active
            .into_iter()
            .map(|(k, id)| (game_key(&k), id))
            .filter(|(k, id)| {
                profiles
                    .get(k)
                    .is_some_and(|l| l.iter().any(|p| &p.id == id))
            })
            .collect();
        self
    }

    /// The macros active in a game: its own (shared by every character), then the ones of the
    /// profile in use, then the global ones.
    pub fn for_game(&self, key: &str) -> Vec<Macro> {
        let mut out = self.games.get(key).cloned().unwrap_or_default();
        if let Some(id) = self.active.get(key) {
            if let Some(p) = self
                .profiles
                .get(key)
                .and_then(|l| l.iter().find(|p| &p.id == id))
            {
                out.extend(p.macros.iter().cloned());
            }
        }
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
    /// This run plays the other-facing version of a move.
    flipped: bool,
}

/// Plays macros frame by frame. Call [`Engine::step`] once per emulated frame.
pub struct Engine {
    macros: Vec<Macro>,
    /// For a move that picks its side: the steps for the other side than `steps`.
    flipped: Vec<Option<Vec<Step>>>,
    runs: Vec<Run>,
    /// Per player: whether left and right are held right now.
    held: [(bool, bool); PLAYERS],
    fps: f64,
}

impl Engine {
    pub fn new(macros: Vec<Macro>, fps: f64) -> Self {
        let runs = vec![Run::default(); macros.len()];
        let flipped = macros
            .iter()
            .map(|m| match (&m.notation, m.kind, m.auto_facing) {
                (Some(n), Kind::Sequence, true) => crate::moves::compile(n, !m.mirror).ok(),
                _ => None,
            })
            .collect();
        Engine {
            macros,
            flipped,
            runs,
            held: [(false, false); PLAYERS],
            fps: fps.max(1.0),
        }
    }

    /// Tell the engine which of left and right a player is holding now; a move that picks
    /// its side looks at this when its trigger goes down.
    pub fn hint_facing(&mut self, player: usize, left: bool, right: bool) {
        if let Some(h) = self.held.get_mut(player) {
            *h = (left, right);
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
            let frames_normal: Vec<u32> = m.steps.iter().map(|s| self.frames(s.ms)).collect();
            let frames_flipped: Vec<u32> = self.flipped[i]
                .iter()
                .flatten()
                .map(|s| self.frames(s.ms))
                .collect();
            let (held_left, held_right) = self.held.get(m.player).copied().unwrap_or_default();
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
                        // the side the player holds decides which way the move is played
                        let flip = self.flipped[i].is_some()
                            && match (held_left, held_right) {
                                (true, false) => !m.mirror,
                                (false, true) => m.mirror,
                                _ => false,
                            };
                        *r = Run {
                            was_down: is_down,
                            playing: true,
                            flipped: flip,
                            ..Run::default()
                        };
                    }
                    if r.playing {
                        let (steps, frames_of) = match (&self.flipped[i], r.flipped) {
                            (Some(alt), true) => (alt, &frames_flipped),
                            _ => (&m.steps, &frames_normal),
                        };
                        press(&steps[r.step].actions);
                        r.in_step += 1;
                        if r.in_step >= frames_of[r.step] {
                            r.in_step = 0;
                            r.step += 1;
                            if r.step >= steps.len() {
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
    fn a_move_written_in_notation_makes_its_own_steps_when_cleaned() {
        let m = Macro {
            notation: Some("  236P ".into()),
            steps: vec![Step {
                actions: a(&["start"]),
                ms: 999,
            }],
            ..mac(Kind::Sequence)
        };
        let cfg = MacroConfig {
            global: vec![m],
            ..Default::default()
        }
        .sanitized();
        let m = &cfg.global[0];
        assert_eq!(m.notation.as_deref(), Some("236P"));
        assert_eq!(m.steps.len(), 3, "the stale steps were replaced");
        assert_eq!(m.steps[2].actions, ["right", "b"]);

        // facing left mirrors it
        let left = Macro {
            notation: Some("236P".into()),
            mirror: true,
            ..mac(Kind::Sequence)
        }
        .sanitized();
        assert_eq!(left.steps[1].actions, ["down", "left"]);
    }

    #[test]
    fn a_notation_that_cannot_be_played_keeps_the_steps_and_an_empty_one_stays_a_move() {
        let steps = vec![Step {
            actions: a(&["a"]),
            ms: 50,
        }];
        let broken = Macro {
            notation: Some("cl.C > DM".into()),
            steps: steps.clone(),
            ..mac(Kind::Sequence)
        }
        .sanitized();
        assert_eq!(broken.steps, steps);
        assert_eq!(broken.notation.as_deref(), Some("cl.C > DM"));
        // a move that is still being written has an empty notation and stays a move
        let empty = Macro {
            notation: Some("   ".into()),
            ..mac(Kind::Sequence)
        }
        .sanitized();
        assert_eq!(empty.notation.as_deref(), Some(""));
        assert!(empty.steps.is_empty());
        // `None` is what a plain sequence has
        let plain = mac(Kind::Sequence).sanitized();
        assert_eq!(plain.notation, None);
        // only a sequence plays steps: other kinds keep theirs untouched
        let combo = Macro {
            notation: Some("236P".into()),
            ..mac(Kind::Combo)
        }
        .sanitized();
        assert!(combo.steps.is_empty());
    }

    /// The buttons pressed on each of `frames` frames, with `left`/`right` held throughout and
    /// the trigger pressed on the first.
    fn play_move(m: Macro, left: bool, right: bool, frames: usize) -> Vec<Vec<String>> {
        let mut e = Engine::new(vec![m], 60.0);
        e.hint_facing(0, left, right);
        (0..frames)
            .map(|f| {
                let mut p: Vec<String> = e.step(&[f == 0]).into_iter().map(|(_, a)| a).collect();
                p.sort();
                p
            })
            .collect()
    }
    fn auto_move(mirror: bool) -> Macro {
        Macro {
            notation: Some("236P".into()),
            auto_facing: true,
            mirror,
            ..mac(Kind::Sequence)
        }
        .sanitized()
    }

    #[test]
    fn a_move_that_picks_its_side_plays_mirrored_when_left_is_held() {
        // 236P: three frames of down, then down+forward, then forward with the punch
        let right = play_move(auto_move(false), false, true, 9);
        assert_eq!(right[0], ["down"]);
        assert_eq!(right[3], ["down", "right"]);
        assert_eq!(right[6], ["b", "right"]);
        let left = play_move(auto_move(false), true, false, 9);
        assert_eq!(left[3], ["down", "left"]);
        assert_eq!(left[6], ["b", "left"]);
    }

    #[test]
    fn with_neither_side_held_the_set_facing_decides_and_both_held_counts_as_neither() {
        assert_eq!(
            play_move(auto_move(false), false, false, 9)[6],
            ["b", "right"]
        );
        assert_eq!(
            play_move(auto_move(true), false, false, 9)[6],
            ["b", "left"]
        );
        assert_eq!(
            play_move(auto_move(false), true, true, 9)[6],
            ["b", "right"]
        );
        // holding the side the move is already written for changes nothing
        assert_eq!(play_move(auto_move(true), true, false, 9)[6], ["b", "left"]);
        // and a mirrored move meets a player holding right: it plays facing right
        assert_eq!(
            play_move(auto_move(true), false, true, 9)[6],
            ["b", "right"]
        );
    }

    #[test]
    fn a_move_without_auto_facing_ignores_what_is_held() {
        let fixed = Macro {
            auto_facing: false,
            ..auto_move(false)
        };
        assert_eq!(play_move(fixed, true, false, 9)[6], ["b", "right"]);
    }

    #[test]
    fn the_side_is_chosen_when_the_trigger_goes_down_not_while_it_plays() {
        let mut e = Engine::new(vec![auto_move(false)], 60.0);
        e.hint_facing(0, true, false);
        let first: Vec<_> = e.step(&[true]);
        assert_eq!(first[0].1, "down");
        // the player lets go of left halfway: the run keeps its side
        e.hint_facing(0, false, true);
        let mut last = Vec::new();
        for _ in 0..5 {
            last = e.step(&[false]).into_iter().map(|(_, a)| a).collect();
        }
        assert!(last.contains(&"left".to_string()), "{last:?}");
    }

    fn profile(id: &str, name: &str, key: &str) -> Profile {
        Profile {
            id: id.into(),
            name: name.into(),
            macros: vec![Macro {
                id: format!("{id}-m"),
                key: Some(key.into()),
                ..mac(Kind::Combo)
            }],
        }
    }

    #[test]
    fn only_the_profile_in_use_plays_along_with_the_shared_and_global_macros() {
        let mut cfg = MacroConfig::default();
        cfg.global.push(Macro {
            id: "g".into(),
            key: Some("G".into()),
            ..mac(Kind::Combo)
        });
        cfg.games.insert(
            "fswords".into(),
            vec![Macro {
                id: "shared".into(),
                key: Some("S".into()),
                ..mac(Kind::Combo)
            }],
        );
        cfg.profiles.insert(
            "fswords".into(),
            vec![profile("h", "Haohmaru", "H"), profile("n", "Nakoruru", "N")],
        );
        let ids = |c: &MacroConfig, game: &str| -> Vec<String> {
            c.for_game(game).into_iter().map(|m| m.id).collect()
        };
        // no profile in use: the shared and the global macros only
        assert_eq!(ids(&cfg, "fswords"), ["shared", "g"]);
        cfg.active.insert("fswords".into(), "n".into());
        assert_eq!(
            ids(&cfg, "fswords"),
            ["shared", "n-m", "g"],
            "Nakoruru's, not Haohmaru's"
        );
        cfg.active.insert("fswords".into(), "h".into());
        assert_eq!(ids(&cfg, "fswords"), ["shared", "h-m", "g"]);
        // another game is not touched
        assert_eq!(ids(&cfg, "other"), ["g"]);
    }

    #[test]
    fn profiles_are_cleaned_when_saved() {
        let mut cfg = MacroConfig::default();
        cfg.profiles.insert(
            "FSwords.zip".into(),
            vec![
                profile("a", "  Haohmaru  ", "H"),
                profile("a", "same id twice", "X"),
                profile("", "no id", "X"),
                profile("b", &"n".repeat(100), "N"),
            ],
        );
        cfg.profiles.insert("empty".into(), vec![]);
        cfg.active.insert("fswords".into(), "gone".into());
        cfg.active.insert("fswords.zip".into(), "b".into());
        let cfg = cfg.sanitized();
        let list = &cfg.profiles["fswords"];
        assert_eq!(list.len(), 2, "one id once, and one without an id dropped");
        assert_eq!(list[0].name, "Haohmaru");
        assert_eq!(list[1].name.chars().count(), 40);
        assert!(!cfg.profiles.contains_key("empty"));
        assert_eq!(
            cfg.active["fswords"], "b",
            "the profile in use exists; stale ones go"
        );
        // too many profiles are cut
        let many: Vec<Profile> = (0..80)
            .map(|i| profile(&format!("p{i}"), "p", "P"))
            .collect();
        let mut big = MacroConfig::default();
        big.profiles.insert("g".into(), many);
        assert_eq!(big.sanitized().profiles["g"].len(), MAX_PROFILES);
    }

    #[test]
    fn a_config_from_before_profiles_still_loads() {
        let old: MacroConfig =
            serde_json::from_str(r#"{"global": [], "games": {"x": []}}"#).unwrap();
        assert!(old.profiles.is_empty() && old.active.is_empty());
    }

    #[test]
    fn a_new_move_from_the_screen_survives_saving_and_loading() {
        // exactly what the Macros screen sends when "+ Move" is pressed, before anything is typed
        let sent = r#"{"global":[],"games":{"fswords":[{"id":"x","name":"Move","enabled":true,
            "player":0,"key":null,"pad":null,"kind":"sequence","actions":[],"rate_hz":15,
            "steps":[],"repeat":false,"notation":"","mirror":false}]}}"#;
        let cfg: MacroConfig = serde_json::from_str(sent).unwrap();
        let dir = std::env::temp_dir().join(format!("vr-move-{}", std::process::id()));
        let path = dir.join("macros.json");
        save(&path, &cfg).unwrap();
        let back = load(&path);
        let _ = std::fs::remove_dir_all(&dir);
        let m = &back.games["fswords"][0];
        assert_eq!(
            m.notation.as_deref(),
            Some(""),
            "still a move, not turned into a sequence"
        );
        assert_eq!(m.name, "Move");
        // once something is typed it makes steps
        let typed = Macro {
            notation: Some("236P".into()),
            ..m.clone()
        }
        .sanitized();
        assert_eq!(typed.steps.len(), 3);
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
