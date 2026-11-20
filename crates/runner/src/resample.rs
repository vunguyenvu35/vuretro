//! A band-limited resampler for the sound: Kaiser-windowed sinc, with the ratio free to change
//! a little at every output frame (the audio follows the video clock through it).
//!
//! It is plain arithmetic on preallocated memory, so it is safe to run in the audio thread,
//! and it takes its input from a function, so a test can feed it a sine wave and measure what
//! comes out.

/// Positions between two input frames the kernel is tabulated at. In between, the kernel is
/// interpolated linearly, which at this density is far below what the ear can tell.
const PHASES: usize = 512;
/// Kaiser window shape: about 80 dB of stop band.
const KAISER_BETA: f64 = 8.0;
/// Part of the input's (or, when shrinking, the output's) Nyquist rate that is kept. The rest
/// is the transition band of the filter.
const PASS: f64 = 0.97;

/// Zeroth order modified Bessel function (the Kaiser window needs it).
fn bessel_i0(x: f64) -> f64 {
    let (mut sum, mut term) = (1.0, 1.0);
    let q = x * x / 4.0;
    for k in 1..60 {
        term *= q / (k * k) as f64;
        sum += term;
        if term < sum * 1e-17 {
            break;
        }
    }
    sum
}

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        1.0
    } else {
        let px = std::f64::consts::PI * x;
        px.sin() / px
    }
}

pub struct Resampler {
    /// Taps of the filter, even.
    taps: usize,
    /// `PHASES + 1` rows of `taps` weights; each row sums to 1 (no change in level).
    table: Vec<f32>,
    /// The last `taps` input frames, oldest first.
    window: Vec<[f32; 2]>,
    /// Where the output sample sits between the two middle frames of the window, 0..1 (and
    /// above 1 while input frames are still to be taken).
    pos: f64,
}

impl Resampler {
    /// `nominal_step` is input frames per output frame the stream is expected to run at (the
    /// input rate over the output rate). Above 1 the sound is shrunk, and the filter is made
    /// longer and its cut-off lower so that nothing above the new Nyquist rate folds back.
    pub fn new(nominal_step: f64) -> Self {
        let shrink = nominal_step.max(1.0);
        let taps = ((32.0 * shrink).ceil() as usize)
            .next_multiple_of(2)
            .min(256);
        let cutoff = PASS / shrink;
        let half = taps as f64 / 2.0;
        let centre = half - 1.0; // the output point lies between window[half - 1] and window[half]
        let mut table = vec![0f32; (PHASES + 1) * taps];
        for p in 0..=PHASES {
            let frac = p as f64 / PHASES as f64;
            let row = &mut table[p * taps..(p + 1) * taps];
            let mut weights: Vec<f64> = (0..taps)
                .map(|k| {
                    let x = k as f64 - centre - frac; // distance from the output point
                    let r = x / half;
                    let window = if r.abs() < 1.0 {
                        bessel_i0(KAISER_BETA * (1.0 - r * r).sqrt()) / bessel_i0(KAISER_BETA)
                    } else {
                        0.0
                    };
                    cutoff * sinc(cutoff * x) * window
                })
                .collect();
            let sum: f64 = weights.iter().sum();
            for w in &mut weights {
                *w /= sum;
            }
            for (dst, w) in row.iter_mut().zip(weights) {
                *dst = w as f32;
            }
        }
        Resampler {
            taps,
            table,
            window: vec![[0.0; 2]; taps],
            pos: 0.0,
        }
    }

    /// Half the filter: how many frames the sound is late by.
    #[cfg(test)]
    pub fn delay_frames(&self) -> usize {
        self.taps / 2
    }

    /// The next output frame. `step` is input frames per output frame right now (close to the
    /// nominal one). `input` gives the next input frame whenever one is needed.
    pub fn next(&mut self, step: f64, input: &mut impl FnMut() -> [f32; 2]) -> [f32; 2] {
        while self.pos >= 1.0 {
            self.window.rotate_left(1);
            self.window[self.taps - 1] = input();
            self.pos -= 1.0;
        }
        let at = self.pos * PHASES as f64;
        let i = (at as usize).min(PHASES - 1);
        let t = (at - i as f64) as f32;
        let (a, b) = (
            &self.table[i * self.taps..(i + 1) * self.taps],
            &self.table[(i + 1) * self.taps..(i + 2) * self.taps],
        );
        let mut out = [0f32; 2];
        for (k, frame) in self.window.iter().enumerate() {
            let w = a[k] + (b[k] - a[k]) * t;
            out[0] += w * frame[0];
            out[1] += w * frame[1];
        }
        self.pos += step;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Resample `n_out` frames of a sine wave of `freq` Hz sampled at `rate_in`, to `rate_out`.
    fn sine_through(freq: f64, rate_in: f64, rate_out: f64, n_out: usize) -> Vec<f32> {
        let mut rs = Resampler::new(rate_in / rate_out);
        let mut n = 0usize;
        let mut input = || {
            let v = (2.0 * std::f64::consts::PI * freq * n as f64 / rate_in).sin() as f32;
            n += 1;
            [v, v]
        };
        (0..n_out)
            .map(|_| rs.next(rate_in / rate_out, &mut input)[0])
            .collect()
    }

    /// Amplitude of the `freq` tone in `x` (sampled at `rate`) and what is left over when it is
    /// taken out, as a fraction of that amplitude. The first `skip` frames (the filter
    /// settling) are left out.
    fn tone_and_residue(x: &[f32], freq: f64, rate: f64, skip: usize) -> (f64, f64) {
        let x = &x[skip..];
        let (mut s, mut c) = (0.0, 0.0);
        for (i, &v) in x.iter().enumerate() {
            let ph = 2.0 * std::f64::consts::PI * freq * (i + skip) as f64 / rate;
            s += v as f64 * ph.sin();
            c += v as f64 * ph.cos();
        }
        let (s, c) = (2.0 * s / x.len() as f64, 2.0 * c / x.len() as f64);
        let amp = (s * s + c * c).sqrt();
        let mut resid = 0.0;
        for (i, &v) in x.iter().enumerate() {
            let ph = 2.0 * std::f64::consts::PI * freq * (i + skip) as f64 / rate;
            let e = v as f64 - (s * ph.sin() + c * ph.cos());
            resid += e * e;
        }
        let resid = (resid / x.len() as f64).sqrt();
        (amp, resid / amp.max(1e-12))
    }

    #[test]
    fn a_steady_level_stays_the_same_level() {
        for (a, b) in [
            (44_100.0, 48_000.0),
            (48_000.0, 44_100.0),
            (32_000.0, 48_000.0),
        ] {
            let mut rs = Resampler::new(a / b);
            let mut input = || [0.5, -0.25];
            let out: Vec<_> = (0..2000).map(|_| rs.next(a / b, &mut input)).collect();
            let last = out[1999];
            assert!((last[0] - 0.5).abs() < 1e-4, "{a}->{b}: {}", last[0]);
            assert!((last[1] + 0.25).abs() < 1e-4);
        }
    }

    #[test]
    fn a_tone_keeps_its_level_and_stays_clean_when_the_rate_goes_up() {
        // 44.1 kHz to 48 kHz, 1 kHz and 10 kHz
        for freq in [1_000.0, 10_000.0] {
            let out = sine_through(freq, 44_100.0, 48_000.0, 20_000);
            let (amp, residue) = tone_and_residue(&out, freq, 48_000.0, 1000);
            assert!((amp - 1.0).abs() < 0.01, "{freq} Hz: level {amp}");
            assert!(
                residue < 0.001,
                "{freq} Hz: {residue} of noise (-60 dB is 0.001)"
            );
        }
    }

    #[test]
    fn it_is_much_cleaner_than_the_linear_one_it_replaces() {
        let freq = 10_000.0;
        let (rate_in, rate_out) = (44_100.0, 48_000.0);
        // the old way: two neighbouring frames, mixed by position
        let n = 20_000;
        let mut linear = Vec::with_capacity(n);
        let mut pos = 0.0f64;
        for _ in 0..n {
            let i = pos.floor();
            let t = (pos - i) as f32;
            let f = |k: f64| (2.0 * std::f64::consts::PI * freq * k / rate_in).sin() as f32;
            linear.push(f(i) + (f(i + 1.0) - f(i)) * t);
            pos += rate_in / rate_out;
        }
        let (_, linear_residue) = tone_and_residue(&linear, freq, rate_out, 1000);
        let out = sine_through(freq, rate_in, rate_out, n);
        let (_, residue) = tone_and_residue(&out, freq, rate_out, 1000);
        assert!(
            residue * 20.0 < linear_residue,
            "linear {linear_residue}, sinc {residue}"
        );
    }

    #[test]
    fn a_tone_above_the_new_nyquist_rate_does_not_fold_back_when_the_rate_goes_down() {
        // 96 kHz to 48 kHz: a 40 kHz tone is not representable and must be gone, not
        // turned into an 8 kHz one
        let out = sine_through(40_000.0, 96_000.0, 48_000.0, 20_000);
        let peak = out[2000..].iter().fold(0f32, |m, v| m.max(v.abs()));
        assert!(peak < 0.003, "left {peak} of it (-50 dB is 0.003)");
        // while a tone below it passes
        let out = sine_through(5_000.0, 96_000.0, 48_000.0, 20_000);
        let (amp, residue) = tone_and_residue(&out, 5_000.0, 48_000.0, 2000);
        assert!(
            (amp - 1.0).abs() < 0.01 && residue < 0.001,
            "{amp} {residue}"
        );
    }

    #[test]
    fn the_ratio_can_drift_while_playing_without_a_click() {
        // what the audio follows the video clock with: up to half a percent faster or slower
        let (a, b) = (44_100.0, 48_000.0);
        let mut rs = Resampler::new(a / b);
        let mut n = 0usize;
        let mut input = || {
            let v = (2.0 * std::f64::consts::PI * 1_000.0 * n as f64 / a).sin() as f32;
            n += 1;
            [v, v]
        };
        let out: Vec<f32> = (0..20_000)
            .map(|i| {
                let drift = 1.0 + 0.005 * (i as f64 / 700.0).sin();
                rs.next(a / b * drift, &mut input)[0]
            })
            .collect();
        // the largest jump from one frame to the next of a 1 kHz tone at 48 kHz is
        // 2*pi*1000/48000 = 0.13; a click would be several times that
        let jump = out[1000..]
            .windows(2)
            .fold(0f32, |m, w| m.max((w[1] - w[0]).abs()));
        assert!(jump < 0.16, "largest step {jump}");
    }

    #[test]
    fn the_filter_gets_longer_when_the_sound_is_shrunk() {
        assert_eq!(Resampler::new(0.9).delay_frames(), 16);
        assert_eq!(Resampler::new(2.0).delay_frames(), 32);
    }

    /// `cargo test -p vuretro-runner quality_report -- --ignored --nocapture`: how clean the
    /// sound is, in dB below the tone, next to the linear resampler this one replaced.
    #[test]
    #[ignore]
    fn quality_report() {
        let (rate_in, rate_out) = (44_100.0, 48_000.0);
        println!("44.1 kHz -> 48 kHz, noise and distortion below the tone:");
        for freq in [1_000.0, 5_000.0, 10_000.0, 15_000.0, 18_000.0] {
            let n = 30_000;
            let mut linear = Vec::with_capacity(n);
            let mut pos = 0.0f64;
            let f = |k: f64| (2.0 * std::f64::consts::PI * freq * k / rate_in).sin() as f32;
            for _ in 0..n {
                let i = pos.floor();
                linear.push(f(i) + (f(i + 1.0) - f(i)) * (pos - i) as f32);
                pos += rate_in / rate_out;
            }
            let (_, lin) = tone_and_residue(&linear, freq, rate_out, 1000);
            let out = sine_through(freq, rate_in, rate_out, n);
            let (_, new) = tone_and_residue(&out, freq, rate_out, 1000);
            println!(
                "  {freq:>6.0} Hz: linear {:>6.1} dB   sinc {:>6.1} dB",
                20.0 * lin.log10(),
                20.0 * new.log10()
            );
        }
    }
}
