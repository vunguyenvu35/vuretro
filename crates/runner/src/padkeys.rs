//! Frontend hotkeys on a gamepad: hold **Select** and press another button. Pure rules over
//! "which buttons are down", so they are unit-tested without a gamepad.
//!
//! | With Select held | Does |
//! |---|---|
//! | Start | close the game |
//! | R1 / L1 | save state / load state |
//! | D-pad right / left | next / previous slot |
//! | R2 (hold) | fast-forward |
//! | L2 (hold) | rewind |
//! | East (○) | fast-forward on / off |
//! | North (△) | pause |
//! | West (□) | screenshot |
//! | South (✕) | sound on / off |

use crate::ports::Hotkeys;
use crate::window::Act;
use gilrs::Button;
use std::collections::HashSet;

/// The button that turns the others into hotkeys.
pub const MODIFIER: Button = Button::Select;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Does {
    /// Once per press.
    Act(Act),
    FastForward,
    Rewind,
}

const CHORDS: [(Button, Does); 11] = [
    (Button::Start, Does::Act(Act::Quit)),
    (Button::RightTrigger, Does::Act(Act::SaveState)),
    (Button::LeftTrigger, Does::Act(Act::LoadState)),
    (Button::DPadRight, Does::Act(Act::NextSlot)),
    (Button::DPadLeft, Does::Act(Act::PrevSlot)),
    (Button::RightTrigger2, Does::FastForward),
    (Button::LeftTrigger2, Does::Rewind),
    (Button::East, Does::Act(Act::FastForwardToggle)),
    (Button::North, Does::Act(Act::Pause)),
    (Button::West, Does::Act(Act::Screenshot)),
    (Button::South, Does::Act(Act::Mute)),
];

/// True while Select is held together with one of the hotkey buttons. The game should not
/// see those presses.
pub fn chord_active(pressed: impl Fn(Button) -> bool) -> bool {
    pressed(MODIFIER) && CHORDS.iter().any(|(b, _)| pressed(*b))
}

/// Remembers which hotkey buttons were already down, so a held button acts once.
#[derive(Default)]
pub struct PadKeys {
    down: HashSet<Button>,
}

impl PadKeys {
    pub fn read(&mut self, pressed: impl Fn(Button) -> bool) -> Hotkeys {
        let mut out = Hotkeys::default();
        if !pressed(MODIFIER) {
            self.down.clear();
            return out;
        }
        for (button, does) in CHORDS {
            let is_down = pressed(button);
            match does {
                Does::Act(act) => {
                    if is_down && self.down.insert(button) {
                        out.acts.push(act);
                    }
                }
                Does::FastForward => out.fast_forward |= is_down,
                Does::Rewind => out.rewind |= is_down,
            }
            if !is_down {
                self.down.remove(&button);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(buttons: &'static [Button]) -> impl Fn(Button) -> bool {
        move |b| buttons.contains(&b)
    }

    #[test]
    fn nothing_happens_without_select() {
        let mut k = PadKeys::default();
        let hk = k.read(held(&[Button::Start, Button::RightTrigger2]));
        assert_eq!(hk, Hotkeys::default());
        assert!(!chord_active(held(&[Button::Start])));
    }

    #[test]
    fn select_plus_a_button_acts_once_per_press() {
        let mut k = PadKeys::default();
        let down = held(&[Button::Select, Button::RightTrigger]);
        assert_eq!(k.read(&down).acts, vec![Act::SaveState]);
        assert!(k.read(&down).acts.is_empty(), "still held: no repeat");
        // let go of the button but keep Select, then press again
        assert!(k.read(held(&[Button::Select])).acts.is_empty());
        assert_eq!(k.read(&down).acts, vec![Act::SaveState]);
        // letting go of Select resets everything
        k.read(held(&[]));
        assert_eq!(k.read(&down).acts, vec![Act::SaveState]);
    }

    #[test]
    fn fast_forward_and_rewind_are_held_not_pressed() {
        let mut k = PadKeys::default();
        let hk = k.read(held(&[Button::Select, Button::RightTrigger2]));
        assert!(hk.fast_forward && !hk.rewind && hk.acts.is_empty());
        let hk = k.read(held(&[Button::Select, Button::RightTrigger2]));
        assert!(hk.fast_forward, "stays on while held");
        let hk = k.read(held(&[Button::Select, Button::LeftTrigger2]));
        assert!(hk.rewind && !hk.fast_forward);
    }

    #[test]
    fn each_chord_maps_to_its_action() {
        for (button, expected) in [
            (Button::Start, Act::Quit),
            (Button::LeftTrigger, Act::LoadState),
            (Button::DPadRight, Act::NextSlot),
            (Button::DPadLeft, Act::PrevSlot),
            (Button::East, Act::FastForwardToggle),
            (Button::North, Act::Pause),
            (Button::West, Act::Screenshot),
            (Button::South, Act::Mute),
        ] {
            let mut k = PadKeys::default();
            let hk = k.read(move |b| b == Button::Select || b == button);
            assert_eq!(hk.acts, vec![expected], "{button:?}");
        }
    }

    #[test]
    fn the_game_does_not_see_a_chord() {
        assert!(chord_active(held(&[Button::Select, Button::North])));
        assert!(
            !chord_active(held(&[Button::Select])),
            "select alone is a game button"
        );
        assert!(!chord_active(held(&[Button::North])));
        // a button that is not a hotkey is the game's even with Select held
        assert!(!chord_active(held(&[Button::Select, Button::C])));
    }
}
