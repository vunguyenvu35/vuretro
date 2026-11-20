//! Is the game keeping up? Each frame the core must be run and shown within the time one
//! frame lasts (16.7 ms at 60 fps). When that work takes longer on average the game runs
//! slower than it should, and the player should be told, not left guessing. Pure, so it is
//! unit-tested.

use std::collections::VecDeque;
use std::time::Duration;

/// How many recent frames are averaged: two seconds at 60 fps.
pub const WINDOW: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Status {
    Smooth,
    /// Frames take longer than they should; roughly this many fit in a second.
    Slow {
        fps: u32,
    },
}

pub struct PerfMonitor {
    target: Duration,
    window: usize,
    samples: VecDeque<Duration>,
    sum: Duration,
    status: Status,
}

impl PerfMonitor {
    pub fn new(target: Duration, window: usize) -> Self {
        PerfMonitor {
            target,
            window: window.max(1),
            samples: VecDeque::new(),
            sum: Duration::ZERO,
            status: Status::Smooth,
        }
    }

    #[cfg(test)]
    pub fn status(&self) -> Status {
        self.status
    }

    /// Record how long one frame's work took. Returns the new status when it changed.
    /// Nothing is judged until a full window has been seen, and the way back to "smooth"
    /// needs a 10 % margin so the verdict does not flicker around the limit.
    pub fn record(&mut self, work: Duration) -> Option<Status> {
        self.samples.push_back(work);
        self.sum += work;
        if self.samples.len() > self.window {
            self.sum -= self.samples.pop_front().unwrap_or_default();
        }
        if self.samples.len() < self.window {
            return None;
        }
        let avg = self.sum / self.samples.len() as u32;
        let next = match self.status {
            Status::Smooth if avg > self.target => Status::Slow {
                fps: (1.0 / avg.as_secs_f64().max(1e-6)).round() as u32,
            },
            Status::Slow { .. } if avg < self.target.mul_f64(0.9) => Status::Smooth,
            Status::Slow { .. } => Status::Slow {
                fps: (1.0 / avg.as_secs_f64().max(1e-6)).round() as u32,
            },
            Status::Smooth => Status::Smooth,
        };
        let changed = std::mem::discriminant(&next) != std::mem::discriminant(&self.status);
        self.status = next;
        changed.then_some(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: fn(u64) -> Duration = Duration::from_millis;

    fn feed(m: &mut PerfMonitor, ms: u64, n: usize) -> Vec<Status> {
        (0..n).filter_map(|_| m.record(MS(ms))).collect()
    }

    #[test]
    fn nothing_is_said_before_a_full_window_or_while_frames_are_on_time() {
        let mut m = PerfMonitor::new(MS(16), 10);
        assert!(feed(&mut m, 40, 9).is_empty(), "window not full yet");
        let mut ok = PerfMonitor::new(MS(16), 10);
        assert!(feed(&mut ok, 10, 100).is_empty());
        assert_eq!(ok.status(), Status::Smooth);
    }

    #[test]
    fn sustained_slow_frames_are_reported_once_with_the_real_frame_rate() {
        let mut m = PerfMonitor::new(MS(16), 10);
        let changes = feed(&mut m, 40, 50);
        assert_eq!(
            changes,
            vec![Status::Slow { fps: 25 }],
            "one report, 1/40 ms = 25 fps"
        );
        assert_eq!(m.status(), Status::Slow { fps: 25 });
    }

    #[test]
    fn a_short_stutter_does_not_count_but_recovery_does() {
        let mut m = PerfMonitor::new(MS(16), 10);
        feed(&mut m, 10, 10);
        assert!(
            feed(&mut m, 60, 1).is_empty(),
            "one bad frame in ten averages out"
        );
        let mut slow = PerfMonitor::new(MS(16), 10);
        feed(&mut slow, 40, 20);
        // 15 ms is under the limit but not under the 10 % margin: still slow
        assert!(feed(&mut slow, 16, 30).is_empty());
        let back = feed(&mut slow, 10, 30);
        assert_eq!(back, vec![Status::Smooth]);
    }
}
