//! Rewind: a bounded stack of recent save states. The newest state is taken first, so
//! holding the rewind key walks the game backwards. Pure, so it is unit-tested.

use std::collections::VecDeque;

pub struct Rewind {
    states: VecDeque<Vec<u8>>,
    bytes: usize,
    cap_bytes: usize,
}

impl Rewind {
    pub fn new(cap_bytes: usize) -> Self {
        Rewind {
            states: VecDeque::new(),
            bytes: 0,
            cap_bytes,
        }
    }

    /// Keep `state` as the newest; the oldest ones go when the memory cap is exceeded.
    /// A single state bigger than the cap is not kept at all.
    pub fn push(&mut self, state: Vec<u8>) {
        if state.len() > self.cap_bytes {
            return;
        }
        self.bytes += state.len();
        self.states.push_back(state);
        while self.bytes > self.cap_bytes {
            if let Some(old) = self.states.pop_front() {
                self.bytes -= old.len();
            }
        }
    }

    /// The newest state, removed.
    pub fn pop(&mut self) -> Option<Vec<u8>> {
        let s = self.states.pop_back()?;
        self.bytes -= s.len();
        Some(s)
    }

    pub fn len(&self) -> usize {
        self.states.len()
    }

    pub fn clear(&mut self) {
        self.states.clear();
        self.bytes = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newest_comes_out_first() {
        let mut r = Rewind::new(100);
        for n in 1..=3u8 {
            r.push(vec![n; 4]);
        }
        assert_eq!(r.pop(), Some(vec![3; 4]));
        assert_eq!(r.pop(), Some(vec![2; 4]));
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn the_oldest_states_are_dropped_to_stay_under_the_cap() {
        let mut r = Rewind::new(10);
        for n in 1..=5u8 {
            r.push(vec![n; 4]); // 4 bytes each: only two fit
        }
        assert_eq!(r.len(), 2);
        assert_eq!(r.pop(), Some(vec![5; 4]));
        assert_eq!(r.pop(), Some(vec![4; 4]));
        assert_eq!(r.pop(), None);
    }

    #[test]
    fn a_state_larger_than_the_cap_is_ignored_and_clear_empties() {
        let mut r = Rewind::new(3);
        r.push(vec![0; 4]);
        assert_eq!(r.len(), 0);
        r.push(vec![0; 2]);
        r.clear();
        assert_eq!(r.pop(), None);
        r.push(vec![1; 3]);
        assert_eq!(r.len(), 1, "byte count was reset by clear");
    }
}
