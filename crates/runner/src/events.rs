//! Telling the app what happens, with `--events` (see `vuretro_core::protocol`).
//!
//! One JSON line per event on the standard output. A line starts on a fresh line, so what a core
//! printed without a line break before it does not spoil it, and a failed write (the app is
//! gone) is ignored, never a reason to stop the game.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use vuretro_core::protocol::{self, Event};
use vuretro_play::ports::Events;

static ENABLED: AtomicBool = AtomicBool::new(false);

/// Switch the reports on or off (off unless `--events` was given: a person running the runner
/// from a terminal does not want JSON between the log lines).
pub fn enable(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

/// Write `event` to `out` as the reader expects it.
fn write_event(out: &mut impl Write, event: &Event) {
    let _ = out
        .write_all(format!("\n{}\n", protocol::encode(event)).as_bytes())
        .and_then(|_| out.flush());
}

/// Report `event` if reports are on.
pub fn emit(event: &Event) {
    if ENABLED.load(Ordering::Relaxed) {
        write_event(&mut std::io::stdout().lock(), event);
    }
}

/// What the game session reports, sent on.
pub struct RunnerEvents;

impl Events for RunnerEvents {
    fn emit(&self, event: Event) {
        emit(&event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vuretro_core::protocol::{parse, StateAction};

    #[test]
    fn an_event_is_a_whole_line_of_its_own() {
        let mut out = Vec::new();
        out.extend_from_slice(b"half a line from a core");
        let first = Event::State {
            action: StateAction::Saved,
            slot: 3,
        };
        let second = Event::Exit { played_secs: 9 };
        write_event(&mut out, &first);
        write_event(&mut out, &second);
        let text = String::from_utf8(out).unwrap();
        let read: Vec<_> = text.lines().filter_map(parse).collect();
        assert_eq!(read, vec![first, second]);
    }

    struct Gone;
    impl Write for Gone {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
    }

    #[test]
    fn a_reader_that_has_gone_does_not_stop_the_game() {
        write_event(&mut Gone, &Event::Progress { played_secs: 1 });
    }
}
