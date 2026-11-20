//! What the game runner tells the app while it plays.
//!
//! The runner is a separate process, so the two talk through its standard output: one JSON
//! object per line, started by `--events`. This is the shape of those lines, written by the
//! runner and read by the app, so the two cannot drift apart.
//!
//! ```text
//! {"vuretro":1,"event":"ready","core":"snes9x","core_version":"1.62","width":256,...}
//! {"vuretro":1,"event":"state","action":"saved","slot":1}
//! {"vuretro":1,"event":"exit","played_secs":1832}
//! ```
//!
//! Cores are C code that may print to the same output, so a reader skips every line that is
//! not one of these, and a writer starts its line on a fresh one. A line from a newer
//! runner with an event or a version this reader does not know is skipped too, not an error.

use serde::{Deserialize, Serialize};

/// The version of the line format. A reader only takes lines of this version.
pub const VERSION: u32 = 1;

/// What happened to a save slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StateAction {
    Saved,
    Loaded,
}

/// One thing the runner reports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    /// The game is loaded and its window is open: from here on it counts as started.
    Ready {
        core: String,
        core_version: String,
        /// Size of the picture and the frame rate the core runs at.
        width: u32,
        height: u32,
        fps: f64,
        /// `opengl` or `basic`.
        window: String,
        /// The core draws with the graphics card (a 3D system).
        hardware_render: bool,
    },
    /// The game has been played this long so far (time the game was running, not paused).
    /// Sent every half minute, so a crash loses little.
    Progress { played_secs: u64 },
    /// A save slot was written or read.
    State { action: StateAction, slot: u8 },
    /// A screenshot was saved.
    Screenshot { path: String },
    /// The runner could not go on. `message` is what to show the player.
    Error { message: String },
    /// The game ended normally.
    Exit { played_secs: u64 },
}

/// The line as it is written: the event with the version beside it.
#[derive(Serialize, Deserialize)]
struct Line {
    vuretro: u32,
    #[serde(flatten)]
    event: Event,
}

/// The text of one event, without a line break.
pub fn encode(event: &Event) -> String {
    // serializing plain data into a string cannot fail
    serde_json::to_string(&Line {
        vuretro: VERSION,
        event: event.clone(),
    })
    .unwrap_or_default()
}

/// The event on `line`, or `None` when the line is something else (a core's own output, a
/// blank line, an event or version from the future).
pub fn parse(line: &str) -> Option<Event> {
    let line = line.trim();
    if !line.starts_with('{') {
        return None;
    }
    let parsed: Line = serde_json::from_str(line).ok()?;
    (parsed.vuretro == VERSION).then_some(parsed.event)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_event() -> Vec<Event> {
        vec![
            Event::Ready {
                core: "snes9x".into(),
                core_version: "1.62".into(),
                width: 256,
                height: 224,
                fps: 60.0988,
                window: "opengl".into(),
                hardware_render: false,
            },
            Event::Progress { played_secs: 30 },
            Event::State {
                action: StateAction::Saved,
                slot: 2,
            },
            Event::State {
                action: StateAction::Loaded,
                slot: 1,
            },
            Event::Screenshot {
                path: "/a b/\"c\".png".into(),
            },
            Event::Error {
                message: "missing neogeo.zip\nsecond line, with \"quotes\"".into(),
            },
            Event::Exit { played_secs: 1832 },
        ]
    }

    #[test]
    fn every_event_survives_the_trip() {
        for e in every_event() {
            let text = encode(&e);
            assert!(!text.contains('\n'), "one line: {text}");
            assert_eq!(parse(&text), Some(e), "{text}");
        }
    }

    #[test]
    fn the_line_looks_the_way_the_documentation_says() {
        assert_eq!(
            encode(&Event::Exit { played_secs: 7 }),
            r#"{"vuretro":1,"event":"exit","played_secs":7}"#
        );
        assert_eq!(
            encode(&Event::State {
                action: StateAction::Saved,
                slot: 1
            }),
            r#"{"vuretro":1,"event":"state","action":"saved","slot":1}"#
        );
    }

    #[test]
    fn what_a_core_prints_is_not_an_event() {
        for line in [
            "",
            "   ",
            "loading rom...",
            "[1, 2, 3]",
            r#"{"unrelated": true}"#,
            r#"{"vuretro":1,"event":"exit"}"#, // a field missing
            "{not json",
        ] {
            assert_eq!(parse(line), None, "{line:?}");
        }
    }

    #[test]
    fn an_event_may_follow_a_cores_unfinished_line_on_a_line_of_its_own() {
        // the runner starts its line with a line break, so this is what the reader sees
        let out = format!("partial output from a core{}", "\n");
        let all = format!("{out}{}\n", encode(&Event::Exit { played_secs: 1 }));
        let events: Vec<_> = all.lines().filter_map(parse).collect();
        assert_eq!(events, vec![Event::Exit { played_secs: 1 }]);
    }

    #[test]
    fn a_newer_runner_does_not_break_an_older_app() {
        // an event this reader does not know
        assert_eq!(parse(r#"{"vuretro":1,"event":"achievement","id":5}"#), None);
        // a newer line format
        assert_eq!(
            parse(r#"{"vuretro":2,"event":"exit","played_secs":1}"#),
            None
        );
        // a known event with a field added later is still understood
        assert_eq!(
            parse(r#"{"vuretro":1,"event":"exit","played_secs":9,"frames":12}"#),
            Some(Event::Exit { played_secs: 9 })
        );
    }

    #[test]
    fn leading_and_trailing_white_space_is_ignored() {
        let text = format!("  {}\r", encode(&Event::Progress { played_secs: 3 }));
        assert_eq!(parse(&text), Some(Event::Progress { played_secs: 3 }));
    }
}
