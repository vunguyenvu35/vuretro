//! Audio output through cpal.
//!
//! The game thread pushes the core's samples into a lock-free queue ([`AudioFeed`]); the
//! sound card's thread takes them out ([`AudioSink`]), resamples them from the core's rate to
//! the device's with a band-limited filter, and follows the video clock by speeding up or
//! slowing down by up to half a percent. Neither thread ever waits for the other: a lock held
//! by the game thread while the sound card wanted its samples was the crackle this replaces.

use crate::resample::Resampler;
use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Buffer level the rate controller steers towards, in seconds of core audio.
pub const TARGET_BUFFER_SECS: f64 = 0.06;
/// The same for a core that draws with the graphics card: its frames arrive less evenly (the
/// card sometimes holds one back for a few frames), and a buffer this size rides that out
/// instead of leaving gaps in the sound, at the price of about 0.1 s more delay.
pub const GPU_BUFFER_SECS: f64 = 0.16;
/// How much more than the target may be buffered before the sound card's side skips ahead to
/// the target, so the delay cannot grow (fast-forward, or a video clock that ran away).
const BUFFER_HEADROOM_SECS: f64 = 0.19;
/// Max resampling speed change (0.5 %, inaudible).
const MAX_RATE_ADJUST: f64 = 0.005;
/// What the queue holds, in samples (two per frame): 1.4 s at 96 kHz.
const QUEUE_SAMPLES: usize = 1 << 18;
/// When the game has not delivered what the sound card needs, the last frame fades out by this
/// factor per frame (silence in about 30 ms) instead of being held, which would be a buzz.
const FADE: f32 = 0.995;
/// How fast a change of the volume or a mute is followed, in seconds (a step would click).
const GAIN_SECS: f64 = 0.005;

/// Dynamic rate control: speed multiplier for the resampler.
/// Buffer above target -> consume slightly faster, below -> slower, so audio
/// follows the video clock without crackles or growing latency.
pub fn drc_ratio(fill_frames: f64, target_frames: f64) -> f64 {
    let dist = ((fill_frames - target_frames) / target_frames).clamp(-1.0, 1.0);
    1.0 + dist * MAX_RATE_ADJUST
}

/// How many frames to skip when `fill` frames are waiting: nothing until there are more than
/// `max`, then enough to get back to `target`.
pub fn frames_to_skip(fill: usize, max: usize, target: usize) -> usize {
    if fill > max {
        fill - target.min(fill)
    } else {
        0
    }
}

/// The game's end of the queue: interleaved stereo `i16` samples.
pub struct AudioFeed {
    producer: Producer<i16>,
}

/// The sound card's end of the queue.
pub struct AudioSink {
    consumer: Consumer<i16>,
}

/// A new queue.
pub fn channel() -> (AudioFeed, AudioSink) {
    let (producer, consumer) = RingBuffer::new(QUEUE_SAMPLES);
    (AudioFeed { producer }, AudioSink { consumer })
}

impl AudioFeed {
    /// A feed nobody listens to (a run with no sound, and tests). It takes what fits and drops
    /// the rest, like any feed whose queue is full.
    pub fn detached() -> Self {
        channel().0
    }

    /// Queue whole stereo frames. What does not fit is dropped (the newest, not the oldest:
    /// the queue is only ever emptied from the other side). Returns the samples taken.
    pub fn push(&mut self, samples: &[i16]) -> usize {
        let room = self.producer.slots() & !1;
        let n = (samples.len() & !1).min(room);
        let (taken, _) = self.producer.push_partial_slice(&samples[..n]);
        taken.len()
    }
}

impl AudioSink {
    /// Throw away what is queued.
    pub fn clear(&mut self) {
        while self.consumer.pop().is_ok() {}
    }

    /// Frames waiting.
    pub fn frames(&self) -> usize {
        self.consumer.slots() / 2
    }
}

/// The queue read as stereo frames of `f32`, fading out when it runs dry.
struct Source {
    sink: AudioSink,
    last: [f32; 2],
}

impl Source {
    fn next(&mut self) -> [f32; 2] {
        if self.sink.consumer.slots() >= 2 {
            if let (Ok(l), Ok(r)) = (self.sink.consumer.pop(), self.sink.consumer.pop()) {
                self.last = [f32::from(l) / 32768.0, f32::from(r) / 32768.0];
            }
        } else {
            self.last = [self.last[0] * FADE, self.last[1] * FADE];
        }
        self.last
    }
}

/// Everything the sound card's thread does, apart from talking to the sound card.
pub struct Player {
    source: Source,
    resampler: Resampler,
    /// Input frames per output frame at normal speed.
    step: f64,
    target_frames: usize,
    max_frames: usize,
    /// The level now, moving towards what was asked for.
    level: f32,
    /// How much of the way to the asked level one frame goes.
    level_rate: f32,
}

impl Player {
    pub fn new(sink: AudioSink, core_rate: f64, device_rate: f64, target_secs: f64) -> Self {
        let step = core_rate / device_rate;
        Player {
            source: Source {
                sink,
                last: [0.0; 2],
            },
            resampler: Resampler::new(step),
            step,
            target_frames: (core_rate * target_secs) as usize,
            max_frames: (core_rate * (target_secs + BUFFER_HEADROOM_SECS)) as usize,
            level: 0.0,
            level_rate: (1.0 - (-1.0 / (GAIN_SECS * device_rate)).exp()) as f32,
        }
    }

    /// Fill `out` (frames of `channels` samples). `gain` is the level in percent and `speed`
    /// the game speed in percent: fast-forwarding delivers more audio per second, which is
    /// played that much faster (and higher).
    pub fn render(&mut self, out: &mut [f32], channels: usize, gain: u32, speed: u32) {
        let wanted = gain as f32 / 100.0;
        let skip = frames_to_skip(
            self.source.sink.frames(),
            self.max_frames,
            self.target_frames,
        );
        for _ in 0..skip * 2 {
            let _ = self.source.sink.consumer.pop();
        }
        let speed_step = self.step * f64::from(speed.max(1)) / 100.0;
        let target = self.target_frames as f64;
        for frame in out.chunks_mut(channels.max(1)) {
            let fill = self.source.sink.frames() as f64;
            let [l, r] = self
                .resampler
                .next(speed_step * drc_ratio(fill, target), &mut || {
                    self.source.next()
                });
            self.level += (wanted - self.level) * self.level_rate;
            let (l, r) = (l * self.level, r * self.level);
            if frame.len() == 1 {
                frame[0] = (l + r) * 0.5;
            } else {
                for (i, s) in frame.iter_mut().enumerate() {
                    *s = if i % 2 == 0 { l } else { r };
                }
            }
        }
    }
}

pub struct AudioOut {
    _stream: cpal::Stream,
}

/// `gain` is the output level in percent (0 = muted); it can change while playing.
/// `speed` is the game speed in percent (100 = normal): while fast-forwarding the core
/// delivers more audio per second, so it is played that much faster (and higher).
pub fn start(
    sink: AudioSink,
    core_rate: f64,
    gain: Arc<AtomicU32>,
    speed: Arc<AtomicU32>,
    target_secs: f64,
) -> Result<AudioOut> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow!("no audio output device"))?;
    let supported = device.default_output_config()?;
    let channels = supported.channels() as usize;
    let device_rate = supported.sample_rate().0 as f64;
    let config: cpal::StreamConfig = supported.into();
    log::info!("audio: core {core_rate:.0} Hz -> device {device_rate:.0} Hz, {channels} ch");

    let mut player = Player::new(sink, core_rate, device_rate, target_secs);
    let stream = device.build_output_stream(
        &config,
        move |out: &mut [f32], _| {
            player.render(
                out,
                channels,
                gain.load(Ordering::Relaxed),
                speed.load(Ordering::Relaxed),
            )
        },
        |err| log::error!("audio stream error: {err}"),
        None,
    )?;
    stream.play()?;
    Ok(AudioOut { _stream: stream })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 440 Hz at 44.1 kHz, `frames` stereo frames.
    fn tone(frames: usize) -> Vec<i16> {
        (0..frames)
            .flat_map(|i| {
                let v = ((i as f64 * 2.0 * std::f64::consts::PI * 440.0 / 44_100.0).sin() * 8000.0)
                    as i16;
                [v, v]
            })
            .collect()
    }

    fn player(frames_queued: usize) -> (Player, AudioFeed) {
        let (mut feed, sink) = channel();
        assert_eq!(feed.push(&tone(frames_queued)), frames_queued * 2);
        (
            Player::new(sink, 44_100.0, 48_000.0, TARGET_BUFFER_SECS),
            feed,
        )
    }

    #[test]
    fn drc_steers_towards_target() {
        assert_eq!(drc_ratio(100.0, 100.0), 1.0);
        assert!(drc_ratio(200.0, 100.0) > 1.0);
        assert!(drc_ratio(0.0, 100.0) < 1.0);
        assert!((drc_ratio(1e9, 100.0) - (1.0 + MAX_RATE_ADJUST)).abs() < 1e-12);
        assert!((drc_ratio(0.0, 100.0) - (1.0 - MAX_RATE_ADJUST)).abs() < 1e-12);
    }

    #[test]
    fn a_core_that_draws_with_the_gpu_gets_a_bigger_buffer_and_room_above_it() {
        let (_feed, sink) = channel();
        let normal = Player::new(sink, 44_100.0, 48_000.0, TARGET_BUFFER_SECS);
        let (_feed, sink) = channel();
        let gpu = Player::new(sink, 44_100.0, 48_000.0, GPU_BUFFER_SECS);
        assert!(gpu.target_frames > normal.target_frames * 2);
        // the sound is cut back to the target only well above it: the delay still cannot grow
        assert!(gpu.max_frames > gpu.target_frames);
        assert!(
            gpu.max_frames as f64 / 44_100.0 < 0.4,
            "{} frames",
            gpu.max_frames
        );
    }

    #[test]
    fn the_queue_takes_whole_frames_and_drops_the_rest_when_full() {
        let (mut feed, mut sink) = channel();
        assert_eq!(feed.push(&[1, 2, 3]), 2, "an odd sample is not a frame");
        assert_eq!(sink.frames(), 1);
        let big = vec![0i16; QUEUE_SAMPLES * 2];
        let taken = feed.push(&big);
        assert_eq!(taken + 2, QUEUE_SAMPLES, "it holds what it holds");
        assert_eq!(feed.push(&[1, 2]), 0, "full: dropped, nothing waits");
        sink.clear();
        assert_eq!(sink.frames(), 0);
        assert_eq!(feed.push(&[1, 2]), 2);
    }

    #[test]
    fn a_detached_feed_never_blocks_or_fails() {
        let mut feed = AudioFeed::detached();
        for _ in 0..10 {
            feed.push(&vec![0i16; 100_000]);
        }
    }

    #[test]
    fn skipping_ahead_only_happens_past_the_limit_and_lands_on_the_target() {
        assert_eq!(frames_to_skip(5_000, 11_025, 2_646), 0);
        assert_eq!(frames_to_skip(11_025, 11_025, 2_646), 0);
        assert_eq!(frames_to_skip(20_000, 11_025, 2_646), 17_354);
        assert_eq!(frames_to_skip(10, 5, 100), 0, "never more than there is");
    }

    #[test]
    fn a_queue_that_has_grown_too_long_is_cut_back_to_the_target() {
        // 0.6 s waiting is more than the 0.25 s allowed
        let (mut p, _feed) = player(26_000);
        let mut out = vec![0f32; 2 * 64];
        p.render(&mut out, 2, 100, 100);
        let left = p.source.sink.frames();
        assert!(
            (2_000..3_000).contains(&left),
            "{left} frames left, the target is 2646"
        );
    }

    #[test]
    fn a_fuller_queue_is_played_a_little_faster_and_an_emptier_one_slower() {
        let eaten = |queued: usize| {
            let (mut p, mut _feed) = player(queued);
            let mut out = vec![0f32; 2 * 4800];
            p.render(&mut out, 2, 100, 100);
            queued - p.source.sink.frames()
        };
        // 0.1 s of output at 44.1/48 kHz takes 4410 frames, give or take half a percent
        let (full, empty) = (eaten(8_000), eaten(100));
        let level = (4410.0 * 0.995) as usize..=(4410.0 * 1.005) as usize + 40;
        assert!(level.contains(&full), "{full}");
        assert!(full > empty + 10, "full {full}, empty {empty}");
    }

    #[test]
    fn the_sound_fades_out_when_the_game_falls_behind_and_does_not_buzz() {
        // 2000 frames in are about 2180 frames out at 44.1 -> 48 kHz; ask for 6000
        let (mut p, _feed) = player(2_000);
        let mut out = vec![0f32; 2 * 6_000];
        p.render(&mut out, 2, 100, 100);
        let peak = |from: usize, to: usize| {
            out[2 * from..2 * to]
                .iter()
                .fold(0f32, |m, v| m.max(v.abs()))
        };
        assert!(
            peak(1_500, 1_600) > 0.05,
            "still playing: {}",
            peak(1_500, 1_600)
        );
        assert!(
            peak(5_900, 6_000) < 0.002,
            "silent at the end: {}",
            peak(5_900, 6_000)
        );
        // a smooth way down: the tone's own slope, never a step to zero
        let step = out[2 * 1_000..]
            .windows(2)
            .fold(0f32, |m, w| m.max((w[1] - w[0]).abs()));
        assert!(step < 0.03, "largest step {step}");
    }

    #[test]
    fn muting_and_unmuting_ramps_instead_of_clicking() {
        let (mut p, mut feed) = player(8_000);
        let mut out = vec![0f32; 2 * 1_000];
        p.render(&mut out, 2, 100, 100);
        let before = out[2 * 999].abs().max(out[2 * 998].abs());
        assert!(before > 0.01, "playing");
        // mute while the tone is running (a little more is queued, but not past the limit)
        feed.push(&tone(2_000));
        let mut muted = vec![0f32; 2 * 1_000];
        p.render(&mut muted, 2, 0, 100);
        let step = muted
            .windows(2)
            .fold(0f32, |m, w| m.max((w[1] - w[0]).abs()));
        assert!(step < 0.03, "largest step {step}, a click is far more");
        let tail = muted[2 * 900..].iter().fold(0f32, |m, v| m.max(v.abs()));
        // the volume follows with a 5 ms time constant: 19 ms on, a fiftieth of the 0.24 tone
        assert!(tail < 0.01, "nearly silent by the end: {tail}");
        // and back up without a step
        let mut back = vec![0f32; 2 * 1_000];
        p.render(&mut back, 2, 100, 100);
        let step = back
            .windows(2)
            .fold(0f32, |m, w| m.max((w[1] - w[0]).abs()));
        assert!(step < 0.03, "largest step {step} on unmuting");
    }

    #[test]
    fn the_delay_stays_put_when_the_game_runs_a_little_fast_or_slow() {
        // The game delivers 100 times a second, the sound card asks 100 times a second, and
        // the game's clock is off by up to 0.3 %: the queue must neither empty nor fill up.
        // (With only a proportional correction it settles at an offset from the target that
        // grows with the error: about 60 % of it for 0.3 %. That is what is checked for.)
        for error in [-0.003, 0.0, 0.003] {
            let (mut feed, sink) = channel();
            let mut p = Player::new(sink, 44_100.0, 48_000.0, TARGET_BUFFER_SECS);
            let target = p.target_frames;
            feed.push(&tone(target)); // the game is a little ahead of the sound when it starts
            let mut owed = 0.0f64;
            let mut out = vec![0f32; 2 * 480];
            let (mut lowest, mut highest) = (usize::MAX, 0usize);
            for tick in 0..4_000 {
                // 10 ms per tick: 441 frames in (a little off), 480 out
                owed += 441.0 * (1.0 + error);
                let whole = owed.floor();
                owed -= whole;
                feed.push(&tone(whole as usize));
                p.render(&mut out, 2, 100, 100);
                if tick >= 3_000 {
                    let fill = p.source.sink.frames();
                    lowest = lowest.min(fill);
                    highest = highest.max(fill);
                }
            }
            let t = target as f64;
            assert!(
                lowest as f64 > t * 0.3 && (highest as f64) < t * 1.9,
                "error {error}: queue between {lowest} and {highest} frames (target {target})"
            );
            if error == 0.0 {
                // on time it stays close to where it started
                assert!((lowest as f64 - t).abs() < t * 0.1, "{lowest}");
            }
        }
    }

    #[test]
    fn mono_devices_get_both_channels_mixed_and_wider_ones_alternate() {
        let (mut feed, sink) = channel();
        feed.push(&[8000i16, -8000].repeat(4_000)); // left up, right down: cancels in mono
        let mut p = Player::new(sink, 48_000.0, 48_000.0, TARGET_BUFFER_SECS);
        let mut mono = vec![0f32; 2_000];
        p.render(&mut mono, 1, 100, 100);
        assert!(mono[1_999].abs() < 1e-3, "{}", mono[1_999]);
        let mut six = vec![0f32; 6 * 2_000];
        p.render(&mut six, 6, 100, 100);
        let f = &six[6 * 1_999..];
        assert!(f[0] > 0.1 && f[1] < -0.1 && f[2] > 0.1 && f[3] < -0.1);
    }
}
