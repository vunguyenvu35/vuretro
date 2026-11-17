//! Audio output via cpal with a linear resampler from core rate to device rate.

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Buffer level the rate controller steers towards, in seconds of core audio.
pub const TARGET_BUFFER_SECS: f64 = 0.06;
/// Max resampling speed change (0.5 %%, inaudible).
const MAX_RATE_ADJUST: f64 = 0.005;

/// Dynamic rate control: speed multiplier for the resampler.
/// Buffer above target -> consume slightly faster, below -> slower, so audio
/// follows the video clock without crackles or growing latency.
pub fn drc_ratio(fill_frames: f64, target_frames: f64) -> f64 {
    let dist = ((fill_frames - target_frames) / target_frames).clamp(-1.0, 1.0);
    1.0 + dist * MAX_RATE_ADJUST
}

pub struct AudioOut {
    _stream: cpal::Stream,
}

pub fn start(buffer: Arc<Mutex<VecDeque<i16>>>, core_rate: f64) -> Result<AudioOut> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow!("no audio output device"))?;
    let supported = device.default_output_config()?;
    let channels = supported.channels() as usize;
    let device_rate = supported.sample_rate().0 as f64;
    let config: cpal::StreamConfig = supported.into();
    let step = core_rate / device_rate;
    let target_frames = core_rate * TARGET_BUFFER_SECS;
    log::info!("audio: core {core_rate:.0} Hz -> device {device_rate:.0} Hz, {channels} ch");

    let mut pos = 0.0f64; // fractional read position in stereo frames
    let mut prev = [0f32; 2];
    let stream = device.build_output_stream(
        &config,
        move |out: &mut [f32], _| {
            let mut q = buffer.lock().unwrap();
            let fill = (q.len() / 2) as f64;
            for frame in out.chunks_mut(channels) {
                // Consume whole input frames passed by the read position.
                while pos >= 1.0 {
                    if q.len() >= 2 {
                        prev = [q[0] as f32 / 32768.0, q[1] as f32 / 32768.0];
                        q.drain(..2);
                    }
                    pos -= 1.0;
                }
                let next = if q.len() >= 2 {
                    [q[0] as f32 / 32768.0, q[1] as f32 / 32768.0]
                } else {
                    prev
                };
                let t = pos as f32;
                let l = prev[0] + (next[0] - prev[0]) * t;
                let r = prev[1] + (next[1] - prev[1]) * t;
                for (i, s) in frame.iter_mut().enumerate() {
                    *s = if i % 2 == 0 { l } else { r };
                }
                pos += step * drc_ratio(fill, target_frames);
            }
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

    #[test]
    fn drc_steers_towards_target() {
        assert_eq!(drc_ratio(100.0, 100.0), 1.0);
        assert!(drc_ratio(200.0, 100.0) > 1.0);
        assert!(drc_ratio(0.0, 100.0) < 1.0);
        assert!((drc_ratio(1e9, 100.0) - (1.0 + MAX_RATE_ADJUST)).abs() < 1e-12);
        assert!((drc_ratio(0.0, 100.0) - (1.0 - MAX_RATE_ADJUST)).abs() < 1e-12);
    }
}
