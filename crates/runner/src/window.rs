//! Window-independent rules of a game session: actions, title text, size limits, frame
//! pacing and the paused picture. Pure, so they are unit-tested without a window.

pub const SLOTS: u8 = vuretro_core::model::SAVE_SLOTS;
const NOTICE_FOR: std::time::Duration = std::time::Duration::from_secs(2);

/// Output level in percent: 0 when sound is off or the game is paused.
pub fn gain_for(sound_on: bool, volume: u8, holding: bool) -> u32 {
    if sound_on && !holding {
        volume as u32
    } else {
        0
    }
}

/// The paused picture: the last frame at half brightness with a pause symbol on it.
pub fn paused_overlay(buf: &[u32], w: usize, h: usize) -> Vec<u32> {
    let mut out: Vec<u32> = buf.iter().map(|p| (p >> 1) & 0x007F_7F7F).collect();
    let (bar_w, bar_h) = ((w / 18).max(2), (h / 4).max(4));
    let (x0, y0) = ((w - 3 * bar_w) / 2, (h - bar_h) / 2);
    for y in y0..y0 + bar_h {
        for x in (x0..x0 + bar_w).chain(x0 + 2 * bar_w..x0 + 3 * bar_w) {
            out[y * w + x] = 0x00FF_FFFF;
        }
    }
    out
}

/// Things the player can do while a game runs: by hotkey or from the menu bar.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Act {
    SaveState,
    LoadState,
    NextSlot,
    Slot(u8),
    Reset,
    Pause,
    Mute,
    SizeStep(i32),
    Size(u8),
    Screenshot,
    Crt,
    Lcd,
    /// Scale2x smoothing on or off.
    Scale2x,
    NextDisc,
    /// Fast-forward stays on until pressed again.
    FastForwardToggle,
    SlowMotion,
    /// Choose the fast-forward multiple (2, 3, 4 or 8).
    Speed(u8),
    Fullscreen,
    /// One slot back (the gamepad shortcut for the previous slot).
    PrevSlot,
    Quit,
}

/// Menu item ids are plain numbers: map them to actions and back.
pub fn act_id(a: Act) -> usize {
    match a {
        Act::SaveState => 1,
        Act::LoadState => 2,
        Act::NextSlot => 3,
        Act::Slot(n) => 10 + n as usize,
        Act::Reset => 4,
        Act::Pause => 5,
        Act::Mute => 6,
        Act::SizeStep(-1) => 7,
        Act::SizeStep(_) => 8,
        Act::Size(n) => 20 + n as usize,
        Act::Screenshot => 9,
        Act::Crt => 40,
        Act::NextDisc => 41,
        Act::FastForwardToggle => 42,
        Act::SlowMotion => 43,
        Act::Fullscreen => 44,
        Act::Lcd => 45,
        Act::Scale2x => 47,
        Act::PrevSlot => 46,
        Act::Speed(n) => 50 + n as usize,
        Act::Quit => 30,
    }
}

pub fn act_from_id(id: usize) -> Option<Act> {
    let all = [
        Act::SaveState,
        Act::LoadState,
        Act::NextSlot,
        Act::Reset,
        Act::Pause,
        Act::Mute,
        Act::SizeStep(-1),
        Act::SizeStep(1),
        Act::Screenshot,
        Act::Crt,
        Act::Lcd,
        Act::Scale2x,
        Act::NextDisc,
        Act::FastForwardToggle,
        Act::SlowMotion,
        Act::Fullscreen,
        Act::PrevSlot,
        Act::Quit,
    ];
    all.into_iter()
        .chain((1..=SLOTS).map(Act::Slot))
        .chain((1..=4).map(Act::Size))
        .chain(
            vuretro_settings::FAST_FORWARD_SPEEDS
                .into_iter()
                .map(Act::Speed),
        )
        .find(|a| act_id(*a) == id)
}

/// Title bar text: the game, its state and a short-lived message about what just happened.
pub fn window_title(
    base: &str,
    holding: bool,
    slow_fps: Option<u32>,
    speed: Option<&str>,
    notice: &Option<(String, std::time::Instant)>,
) -> String {
    let mut t = base.to_string();
    if let Some(speed) = speed {
        t.push_str(&format!(" ({speed})"));
    }
    if let Some(fps) = slow_fps {
        t.push_str(&format!(" (slow: about {fps} fps)"));
    }
    if holding {
        t.push_str(" (paused, press P to resume)");
    }
    if let Some((msg, at)) = notice {
        if at.elapsed() < NOTICE_FOR {
            t.push_str(" · ");
            t.push_str(msg);
        }
    }
    t
}

pub fn clamp_scale(n: u8) -> usize {
    n.clamp(vuretro_settings::MIN_SCALE, vuretro_settings::MAX_SCALE) as usize
}

/// Sleep until the next frame deadline (sleep most of it, spin the rest for accuracy).
/// If we fell far behind (window dragged, machine suspended) resync instead of
/// racing to catch up.
pub fn pace(deadline: &mut std::time::Instant, frame_time: std::time::Duration) {
    use std::time::{Duration, Instant};
    *deadline += frame_time;
    let now = Instant::now();
    if *deadline > now {
        let wait = *deadline - now;
        if wait > Duration::from_millis(2) {
            std::thread::sleep(wait - Duration::from_millis(1));
        }
        while Instant::now() < *deadline {
            std::hint::spin_loop();
        }
    } else if now - *deadline > frame_time * 4 {
        *deadline = now;
    }
}

#[cfg(test)]
mod pause_tests {
    use super::*;

    #[test]
    fn every_action_has_its_own_menu_id_and_maps_back() {
        let mut acts = vec![
            Act::SaveState,
            Act::LoadState,
            Act::NextSlot,
            Act::Reset,
            Act::Pause,
            Act::Mute,
            Act::SizeStep(-1),
            Act::SizeStep(1),
            Act::Screenshot,
            Act::Crt,
            Act::Lcd,
            Act::Scale2x,
            Act::NextDisc,
            Act::FastForwardToggle,
            Act::SlowMotion,
            Act::Fullscreen,
            Act::PrevSlot,
            Act::Quit,
        ];
        acts.extend((1..=SLOTS).map(Act::Slot));
        acts.extend((1..=4).map(Act::Size));
        acts.extend(
            vuretro_settings::FAST_FORWARD_SPEEDS
                .into_iter()
                .map(Act::Speed),
        );
        let mut ids = std::collections::HashSet::new();
        for a in acts {
            assert!(
                ids.insert(act_id(a)),
                "{a:?} shares its id with another action"
            );
            assert_eq!(act_from_id(act_id(a)), Some(a), "{a:?}");
        }
    }

    #[test]
    fn title_shows_speed_pause_and_notice() {
        let none: Option<(String, std::time::Instant)> = None;
        assert_eq!(window_title("G", false, None, None, &none), "G");
        assert_eq!(
            window_title("G", false, None, Some("fast-forward 4×"), &none),
            "G (fast-forward 4×)"
        );
        assert!(window_title("G", true, None, None, &none).contains("paused"));
    }

    #[test]
    fn gain_is_zero_when_muted_or_paused() {
        assert_eq!(gain_for(true, 80, false), 80);
        assert_eq!(gain_for(false, 80, false), 0);
        assert_eq!(gain_for(true, 80, true), 0);
    }

    #[test]
    fn paused_picture_is_dimmed_with_a_symbol() {
        let buf = vec![0x00FF_FFFFu32; 64 * 48];
        let out = paused_overlay(&buf, 64, 48);
        assert_eq!(out.len(), buf.len());
        assert_eq!(out[0], 0x007F_7F7F); // half brightness
        assert!(out.contains(&0x00FF_FFFF)); // the bars
    }
}
