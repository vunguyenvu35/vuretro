//! Auto coin for arcade systems. An arcade game ignores Start until a coin is in (the Select
//! button), which surprises people used to consoles. With this on, a new press of Start first
//! puts a coin in, waits a moment for the credit to count, and then lets Start through.
//! Pure, so it is unit-tested without a game.

use libretro_sys::{RETRO_DEVICE_ID_JOYPAD_SELECT, RETRO_DEVICE_ID_JOYPAD_START};

const START: usize = RETRO_DEVICE_ID_JOYPAD_START as usize;
const COIN: usize = RETRO_DEVICE_ID_JOYPAD_SELECT as usize;

/// How many polls (about one per frame) the coin is held, the pause that follows, and the
/// least time Start is held afterwards (so a quick tap still counts).
pub const COIN_POLLS: u32 = 3;
pub const GAP_POLLS: u32 = 15;
pub const START_POLLS: u32 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
enum Phase {
    #[default]
    Idle,
    Coin(u32),
    Gap(u32),
    /// Start goes through; the number is how many polls it is still forced on.
    Start(u32),
}

/// One per port.
#[derive(Default)]
pub struct AutoCoin {
    phase: Phase,
    was_down: bool,
}

impl AutoCoin {
    /// Change this poll's buttons of one port: Start is the player's, Select (the coin) is
    /// added. Call once per poll, in order.
    pub fn apply<const N: usize>(&mut self, buttons: &mut [bool; N]) {
        let down = buttons[START];
        let pressed = down && !self.was_down;
        self.was_down = down;
        if pressed && self.phase == Phase::Idle {
            self.phase = Phase::Coin(COIN_POLLS);
        }
        self.phase = match self.phase {
            Phase::Idle => Phase::Idle,
            Phase::Coin(n) => {
                buttons[COIN] = true;
                buttons[START] = false;
                if n > 1 {
                    Phase::Coin(n - 1)
                } else {
                    Phase::Gap(GAP_POLLS)
                }
            }
            Phase::Gap(n) => {
                buttons[START] = false;
                if n > 1 {
                    Phase::Gap(n - 1)
                } else {
                    Phase::Start(START_POLLS)
                }
            }
            Phase::Start(n) => {
                buttons[START] = down || n > 0;
                if down || n > 1 {
                    Phase::Start(n.saturating_sub(1))
                } else {
                    Phase::Idle
                }
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feed a sequence of physical Start states; return (coin, start) the game sees.
    fn run(start: &[bool]) -> Vec<(bool, bool)> {
        let mut ac = AutoCoin::default();
        start
            .iter()
            .map(|&s| {
                let mut b = [false; 16];
                b[START] = s;
                ac.apply(&mut b);
                (b[COIN], b[START])
            })
            .collect()
    }

    const C: usize = COIN_POLLS as usize;
    const G: usize = GAP_POLLS as usize;
    const S: usize = START_POLLS as usize;

    #[test]
    fn a_press_of_start_puts_a_coin_in_first_and_start_follows() {
        let out = run(&[true; C + G + S + 4]);
        assert!(
            out[..C].iter().all(|&o| o == (true, false)),
            "the coin goes in first"
        );
        assert!(
            out[C..C + G].iter().all(|&o| o == (false, false)),
            "then a pause for the credit to count"
        );
        assert!(
            out[C + G..].iter().all(|&o| o == (false, true)),
            "then Start goes through: {out:?}"
        );
    }

    #[test]
    fn a_quick_tap_still_starts_the_game() {
        // pressed for one poll only, then released
        let mut input = vec![true];
        input.extend(vec![false; C + G + S + 6]);
        let out = run(&input);
        assert_eq!(out[0], (true, false));
        let starts: Vec<usize> = (0..out.len()).filter(|&i| out[i].1).collect();
        assert_eq!(
            starts.len(),
            S,
            "Start is held long enough to count: {out:?}"
        );
        assert_eq!(starts[0], C + G, "and only after the coin and the pause");
    }

    #[test]
    fn nothing_changes_while_start_is_not_pressed() {
        assert!(run(&[false; 10]).iter().all(|&(c, s)| !c && !s));
    }

    #[test]
    fn every_new_press_puts_another_coin_in() {
        let long = C + G + S + 2;
        let mut input = vec![true; long];
        input.extend(vec![false; 6]);
        input.extend(vec![true; long]);
        let out = run(&input);
        let rises = out.windows(2).filter(|w| !w[0].0 && w[1].0).count() + out[0].0 as usize;
        assert_eq!(rises, 2, "one coin per press: {out:?}");
    }

    #[test]
    fn the_players_own_select_is_left_alone() {
        let mut ac = AutoCoin::default();
        let mut b = [false; 16];
        b[COIN] = true; // the player inserts a coin by hand
        ac.apply(&mut b);
        assert!(b[COIN] && !b[START]);
        // and holding Start for a long time does not keep adding coins
        let out = run(&[true; 40]);
        assert_eq!(out.iter().filter(|&&(c, _)| c).count() as u32, COIN_POLLS);
    }
}
