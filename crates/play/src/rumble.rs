//! Rumble: what a core asks for is remembered per port; this finds the ports whose request
//! changed since the last look, so the gamepad is only told about changes. Pure, so it is
//! unit-tested without a gamepad.

use vuretro_controls::PLAYERS as MAX_PORTS;

/// `[strong, weak]` strength per port.
pub type Requests = [[u16; 2]; MAX_PORTS];

#[derive(Default)]
pub struct Tracker {
    last: Requests,
}

impl Tracker {
    /// Ports whose request differs from the last call, with the new one.
    pub fn changes(&mut self, now: Requests) -> Vec<(usize, [u16; 2])> {
        let out = (0..MAX_PORTS)
            .filter(|&p| now[p] != self.last[p])
            .map(|p| (p, now[p]))
            .collect();
        self.last = now;
        out
    }

    /// Forget the last request of a port (its gamepad changed), so the next one is applied.
    pub fn forget(&mut self, port: usize) {
        if let Some(l) = self.last.get_mut(port) {
            *l = [0; 2];
        }
    }
}

/// Whether a request asks for any vibration.
pub fn is_on(r: [u16; 2]) -> bool {
    r[0] > 0 || r[1] > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_changed_ports_are_reported() {
        let mut t = Tracker::default();
        assert!(t.changes([[0; 2]; MAX_PORTS]).is_empty());
        let mut now = [[0; 2]; MAX_PORTS];
        now[1] = [30_000, 0];
        assert_eq!(t.changes(now), vec![(1, [30_000, 0])]);
        assert!(t.changes(now).is_empty(), "same request: nothing to do");
        now[1] = [0, 0];
        assert_eq!(
            t.changes(now),
            vec![(1, [0, 0])],
            "stopping is a change too"
        );
    }

    #[test]
    fn forgetting_a_port_applies_its_request_again() {
        let mut t = Tracker::default();
        let mut now = [[0; 2]; MAX_PORTS];
        now[0] = [1, 1];
        t.changes(now);
        t.forget(0);
        assert_eq!(t.changes(now), vec![(0, [1, 1])]);
        t.forget(99); // out of range: ignored
    }

    #[test]
    fn on_means_either_motor() {
        assert!(is_on([1, 0]) && is_on([0, 1]) && !is_on([0, 0]));
    }
}
