//! Game speed: normal, fast-forward (2, 3, 4 or 8 times) and slow motion. Pure rules, so
//! they are unit-tested without a window or a core.

/// Slow motion plays at half speed.
pub const SLOW_PERCENT: u32 = 50;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Normal,
    Fast,
    Slow,
}

/// What a single pass of the frame loop does at a given speed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Plan {
    /// How many frames the core runs in this pass.
    pub runs: usize,
    /// How many normal frame times this pass lasts (2 means half speed).
    pub stretch: u32,
}

/// Which speed is in force. Fast-forward (held or toggled on) beats slow motion.
pub fn mode(fast: bool, slow: bool) -> Mode {
    if fast {
        Mode::Fast
    } else if slow {
        Mode::Slow
    } else {
        Mode::Normal
    }
}

/// Speed in percent of normal for a mode; `ff` is the chosen fast-forward multiple.
pub fn percent(mode: Mode, ff: u8) -> u32 {
    match mode {
        Mode::Normal => 100,
        Mode::Fast => 100 * u32::from(vuretro_settings::snap_fast_forward(ff)),
        Mode::Slow => SLOW_PERCENT,
    }
}

/// Faster than normal runs the core several times per pass; slower stretches the pass.
pub fn plan(percent: u32) -> Plan {
    let percent = percent.max(1);
    if percent >= 100 {
        Plan {
            runs: (percent / 100) as usize,
            stretch: 1,
        }
    } else {
        Plan {
            runs: 1,
            stretch: (100 / percent).max(1),
        }
    }
}

/// Whether the sound is silenced at this speed: slow motion always (the audio could not
/// keep up), fast-forward unless the player wants to hear it.
pub fn silent(mode: Mode, ff_sound: bool) -> bool {
    match mode {
        Mode::Normal => false,
        Mode::Fast => !ff_sound,
        Mode::Slow => true,
    }
}

pub fn label(mode: Mode, ff: u8) -> String {
    match mode {
        Mode::Normal => "normal speed".to_string(),
        Mode::Fast => format!("fast-forward {ff}×"),
        Mode::Slow => "slow motion".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_forward_beats_slow_motion() {
        assert_eq!(mode(false, false), Mode::Normal);
        assert_eq!(mode(true, true), Mode::Fast);
        assert_eq!(mode(false, true), Mode::Slow);
    }

    #[test]
    fn percent_follows_the_mode_and_snaps_the_multiple() {
        assert_eq!(percent(Mode::Normal, 8), 100);
        assert_eq!(percent(Mode::Fast, 2), 200);
        assert_eq!(percent(Mode::Fast, 8), 800);
        assert_eq!(percent(Mode::Fast, 5), 400, "5 is not offered");
        assert_eq!(percent(Mode::Slow, 4), 50);
    }

    #[test]
    fn plans_run_more_frames_or_stretch_time() {
        assert_eq!(
            plan(100),
            Plan {
                runs: 1,
                stretch: 1
            }
        );
        assert_eq!(
            plan(200),
            Plan {
                runs: 2,
                stretch: 1
            }
        );
        assert_eq!(
            plan(800),
            Plan {
                runs: 8,
                stretch: 1
            }
        );
        assert_eq!(
            plan(50),
            Plan {
                runs: 1,
                stretch: 2
            }
        );
        assert_eq!(plan(0).runs, 1, "never a plan that runs nothing");
    }

    #[test]
    fn sound_is_silenced_in_slow_motion_and_by_choice_when_fast() {
        assert!(!silent(Mode::Normal, false));
        assert!(silent(Mode::Fast, false));
        assert!(!silent(Mode::Fast, true));
        assert!(silent(Mode::Slow, true));
    }
}
