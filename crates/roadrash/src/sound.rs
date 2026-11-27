//! Sound for the ride: the engine, which goes round and round and rises in pitch with the
//! revs, and effects that play once. [`Mixer`] makes the samples; [`Audio`] sends them to the
//! sound card (through `cpal`, as the runner does).

use crate::wav::Clip;
use std::sync::{Arc, Mutex};

/// The revs at which the engine clip is played at its own pitch.
const REFERENCE_REVS: f32 = 6000.0;

pub struct Mixer {
    engine: Clip,
    effects: Vec<Clip>,
    out_rate: f32,
    engine_at: f64,
    shots: Vec<(usize, f64)>,
    /// The music of the ride (goes round and round) and where it is.
    music: Option<Clip>,
    music_at: f64,
    /// An effect that goes round and round (the siren): which, where it is, how loud.
    looping: Option<(usize, f64, f32)>,
    /// Turns of the engine in a minute; 0 silences it.
    pub revs: f32,
    pub volume: f32,
    /// How loud the music is compared with the rest.
    pub music_volume: f32,
}

impl Mixer {
    pub fn new(engine: Clip, effects: Vec<Clip>, out_rate: f32) -> Mixer {
        Mixer {
            engine,
            effects,
            out_rate,
            engine_at: 0.0,
            shots: Vec::new(),
            music: None,
            music_at: 0.0,
            looping: None,
            revs: 0.0,
            volume: 0.8,
            music_volume: 0.35,
        }
    }

    /// Play this music from the start, again and again; `None` stops it.
    pub fn set_music(&mut self, music: Option<Clip>) {
        self.music = music;
        self.music_at = 0.0;
    }

    /// Add a sound while the mixer is running; its number is returned.
    pub fn add_effect(&mut self, clip: Clip) -> usize {
        self.effects.push(clip);
        self.effects.len() - 1
    }

    /// Start (or change the loudness of) the effect that goes round, or stop it with `None`.
    pub fn set_loop(&mut self, which: Option<(usize, f32)>) {
        self.looping = match (which, self.looping) {
            (Some((n, volume)), _) if n >= self.effects.len() || volume <= 0.0 => {
                let _ = volume;
                None
            }
            (Some((n, volume)), Some((m, at, _))) if n == m => Some((n, at, volume)),
            (Some((n, volume)), _) => Some((n, 0.0, volume)),
            (None, _) => None,
        };
    }

    /// Play effect number `n` once. A sound that is not there is ignored.
    pub fn play(&mut self, n: usize) {
        if n < self.effects.len() && self.shots.len() < 16 {
            self.shots.push((n, 0.0));
        }
    }

    /// Make the next `out.len()` samples (mono).
    pub fn fill(&mut self, out: &mut [f32]) {
        let len = self.engine.samples.len();
        let engine_step = (self.engine.rate / self.out_rate * self.revs / REFERENCE_REVS) as f64;
        for sample in out.iter_mut() {
            let mut value = 0.0;
            if self.revs > 0.0 && len > 1 {
                let (i, f) = (self.engine_at as usize, self.engine_at.fract() as f32);
                let (a, b) = (self.engine.samples[i], self.engine.samples[(i + 1) % len]);
                value += (a + (b - a) * f) * 0.6;
                self.engine_at = (self.engine_at + engine_step) % len as f64;
            }
            if let Some(m) = &self.music {
                if !m.samples.is_empty() {
                    value +=
                        m.samples[self.music_at as usize % m.samples.len()] * self.music_volume;
                    self.music_at =
                        (self.music_at + (m.rate / self.out_rate) as f64) % m.samples.len() as f64;
                }
            }
            if let Some((n, at, volume)) = &mut self.looping {
                let clip = &self.effects[*n];
                if !clip.samples.is_empty() {
                    value += clip.samples[*at as usize % clip.samples.len()] * *volume;
                    *at = (*at + (clip.rate / self.out_rate) as f64) % clip.samples.len() as f64;
                }
            }
            for (n, at) in &mut self.shots {
                let clip = &self.effects[*n];
                let i = *at as usize;
                if i < clip.samples.len() {
                    value += clip.samples[i];
                    *at += (clip.rate / self.out_rate) as f64;
                }
            }
            *sample = (value * self.volume).clamp(-1.0, 1.0);
        }
        let effects = &self.effects;
        self.shots
            .retain(|(n, at)| (*at as usize) < effects[*n].samples.len());
    }
}

/// A sound card that plays what the mixer makes. Keep it alive for as long as there is to be
/// sound.
pub struct Audio {
    pub mixer: Arc<Mutex<Mixer>>,
    _stream: cpal::Stream,
}

impl Audio {
    /// Open the default output. `Err` when there is none or it is not of a kind this reads
    /// (the ride then goes on without sound).
    pub fn start(engine: Clip, effects: Vec<Clip>) -> Result<Audio, String> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("no sound output")?;
        let config = device.default_output_config().map_err(|e| e.to_string())?;
        if config.sample_format() != cpal::SampleFormat::F32 {
            return Err(format!(
                "sound output of kind {:?} is not used",
                config.sample_format()
            ));
        }
        let channels = config.channels() as usize;
        let mixer = Arc::new(Mutex::new(Mixer::new(
            engine,
            effects,
            config.sample_rate().0 as f32,
        )));
        let shared = mixer.clone();
        let mut mono = Vec::new();
        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _| {
                    mono.resize(data.len() / channels, 0.0);
                    if let Ok(mut m) = shared.lock() {
                        m.fill(&mut mono);
                    } else {
                        mono.fill(0.0);
                    }
                    for (frame, s) in data.chunks_mut(channels).zip(&mono) {
                        frame.fill(*s);
                    }
                },
                |e| eprintln!("sound: {e}"),
                None,
            )
            .map_err(|e| e.to_string())?;
        stream.play().map_err(|e| e.to_string())?;
        Ok(Audio {
            mixer,
            _stream: stream,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(rate: f32, samples: Vec<f32>) -> Clip {
        Clip { rate, samples }
    }
    /// A ramp 0, 0.01, 0.02 ... so the sample number can be read back from the value.
    fn ramp(n: usize) -> Clip {
        clip(1000.0, (0..n).map(|i| i as f32 / 100.0).collect())
    }

    #[test]
    fn the_engine_is_silent_until_it_has_revs_and_goes_round() {
        let mut m = Mixer::new(ramp(10), vec![], 1000.0);
        m.volume = 1.0;
        let mut out = [9.0; 4];
        m.fill(&mut out);
        assert_eq!(out, [0.0; 4], "no revs, no sound");
        m.revs = 6000.0; // the clip's own pitch
        let mut out = [0.0; 12];
        m.fill(&mut out);
        assert!(
            (out[1] - 0.006).abs() < 1e-6,
            "0.6 of the sample value: {out:?}"
        );
        assert_eq!(out[10], 0.0, "after ten samples the clip starts again");
        assert!((out[11] - out[1]).abs() < 1e-6);
        assert!(out.iter().all(|v| v.abs() <= 1.0));
    }

    #[test]
    fn more_revs_play_the_clip_faster() {
        let used = |revs| {
            let mut m = Mixer::new(ramp(1000), vec![], 1000.0);
            m.revs = revs;
            m.volume = 1.0;
            m.fill(&mut [0.0; 100]);
            m.engine_at
        };
        assert!(used(12000.0) > used(6000.0) * 1.9);
        assert!(used(3000.0) < used(6000.0) * 0.6);
    }

    #[test]
    fn an_effect_plays_once_and_stops() {
        let mut m = Mixer::new(ramp(10), vec![clip(1000.0, vec![0.5; 5])], 1000.0);
        m.volume = 1.0;
        m.play(0);
        m.play(7); // not there: ignored
        let mut out = [0.0; 8];
        m.fill(&mut out);
        assert_eq!(out, [0.5, 0.5, 0.5, 0.5, 0.5, 0.0, 0.0, 0.0]);
        assert!(m.shots.is_empty(), "finished sounds are dropped");
    }

    #[test]
    fn a_sound_at_half_the_output_rate_lasts_twice_as_long() {
        let mut m = Mixer::new(ramp(10), vec![clip(500.0, vec![0.5; 5])], 1000.0);
        m.volume = 1.0;
        m.play(0);
        let mut out = [0.0; 12];
        m.fill(&mut out);
        assert_eq!(out.iter().filter(|v| **v > 0.0).count(), 10);
    }

    #[test]
    fn loud_sounds_are_kept_in_range() {
        let mut m = Mixer::new(ramp(10), vec![clip(1000.0, vec![1.0; 5]); 3], 1000.0);
        m.volume = 1.0;
        for _ in 0..3 {
            m.play(0);
        }
        let mut out = [0.0; 5];
        m.fill(&mut out);
        assert!(out.iter().all(|v| *v == 1.0));
    }

    #[test]
    fn a_looped_effect_goes_round_until_it_is_stopped_and_can_be_made_louder() {
        let mut m = Mixer::new(ramp(10), vec![clip(1000.0, vec![0.1, 0.2, 0.3])], 1000.0);
        m.volume = 1.0;
        m.set_loop(Some((0, 1.0)));
        let mut out = [0.0; 7];
        m.fill(&mut out);
        let want = [0.1, 0.2, 0.3, 0.1, 0.2, 0.3, 0.1];
        assert!(
            out.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-6),
            "{out:?}"
        );
        m.set_loop(Some((0, 0.5)));
        let mut louder = [0.0; 2];
        m.fill(&mut louder);
        assert!(
            (louder[0] - 0.1).abs() < 1e-6,
            "went on where it was, half as loud: {louder:?}"
        );
        m.set_loop(None);
        let mut quiet = [9.0; 3];
        m.fill(&mut quiet);
        assert_eq!(quiet, [0.0; 3]);
        m.set_loop(Some((5, 1.0))); // not there
        m.fill(&mut quiet);
        assert_eq!(quiet, [0.0; 3]);
    }

    #[test]
    fn a_sound_added_later_can_be_played() {
        let mut m = Mixer::new(ramp(10), vec![], 1000.0);
        m.volume = 1.0;
        let n = m.add_effect(clip(1000.0, vec![0.25; 2]));
        m.play(n);
        let mut out = [0.0; 3];
        m.fill(&mut out);
        assert_eq!(out, [0.25, 0.25, 0.0]);
    }

    #[test]
    fn music_goes_round_at_its_own_loudness_and_can_be_stopped() {
        let mut m = Mixer::new(ramp(10), vec![], 1000.0);
        m.volume = 1.0;
        m.music_volume = 0.5;
        m.set_music(Some(clip(1000.0, vec![0.2, 0.4])));
        let mut out = [0.0; 5];
        m.fill(&mut out);
        let want = [0.1, 0.2, 0.1, 0.2, 0.1];
        assert!(
            out.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-6),
            "{out:?}"
        );
        m.set_music(None);
        m.fill(&mut out);
        assert_eq!(out, [0.0; 5]);
    }
}
