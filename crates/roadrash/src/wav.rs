//! The sound effects of the disc (`AUDIO/EFFECTS/*.WAV`): plain WAV files of uncompressed
//! samples (8-bit unsigned or 16-bit signed, one or two channels). They are read into mono
//! samples between -1 and 1.

/// A sound ready to be played.
#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    /// Samples per second.
    pub rate: f32,
    pub samples: Vec<f32>,
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    b.get(at..at + 2).map(|w| u16::from_le_bytes([w[0], w[1]]))
}
fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4)
        .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
}

pub fn parse(file: &[u8]) -> Result<Clip, String> {
    if file.get(..4) != Some(b"RIFF") || file.get(8..12) != Some(b"WAVE") {
        return Err("not a WAV file".into());
    }
    let (mut channels, mut rate, mut bits) = (0u16, 0u32, 0u16);
    let mut at = 12;
    while at + 8 <= file.len() {
        let len = u32_at(file, at + 4).unwrap_or(0) as usize;
        let body = file
            .get(at + 8..(at + 8 + len).min(file.len()))
            .unwrap_or(&[]);
        match &file[at..at + 4] {
            b"fmt " => {
                if u16_at(body, 0) != Some(1) {
                    return Err("only uncompressed (PCM) sound is read".into());
                }
                channels = u16_at(body, 2).unwrap_or(0);
                rate = u32_at(body, 4).unwrap_or(0);
                bits = u16_at(body, 14).unwrap_or(0);
            }
            b"data" => {
                if channels == 0 || channels > 2 || rate == 0 || !(bits == 8 || bits == 16) {
                    return Err(format!(
                        "{channels} channels, {bits} bits, {rate} Hz is not read"
                    ));
                }
                let width = (bits / 8) as usize;
                let frame = width * channels as usize;
                let samples = body
                    .chunks_exact(frame)
                    .map(|f| {
                        let sum: f32 = f
                            .chunks_exact(width)
                            .map(|s| {
                                if width == 1 {
                                    (s[0] as f32 - 128.0) / 128.0
                                } else {
                                    i16::from_le_bytes([s[0], s[1]]) as f32 / 32768.0
                                }
                            })
                            .sum();
                        sum / channels as f32
                    })
                    .collect();
                return Ok(Clip {
                    rate: rate as f32,
                    samples,
                });
            }
            _ => {}
        }
        at += 8 + len + (len & 1);
    }
    Err("the file has no sound data".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(channels: u16, bits: u16, data: &[u8]) -> Vec<u8> {
        let mut fmt = 1u16.to_le_bytes().to_vec();
        fmt.extend_from_slice(&channels.to_le_bytes());
        fmt.extend_from_slice(&11025u32.to_le_bytes());
        fmt.extend_from_slice(&(11025 * channels as u32 * bits as u32 / 8).to_le_bytes());
        fmt.extend_from_slice(&(channels * bits / 8).to_le_bytes());
        fmt.extend_from_slice(&bits.to_le_bytes());
        let mut w = b"RIFF".to_vec();
        w.extend_from_slice(&0u32.to_le_bytes());
        w.extend_from_slice(b"WAVEfmt ");
        w.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
        w.extend(fmt);
        w.extend_from_slice(b"data");
        w.extend_from_slice(&(data.len() as u32).to_le_bytes());
        w.extend_from_slice(data);
        w
    }

    #[test]
    fn eight_bit_samples_are_centred_on_128() {
        let c = parse(&wav(1, 8, &[128, 255, 0])).unwrap();
        assert_eq!(c.rate, 11025.0);
        assert_eq!(c.samples, vec![0.0, 127.0 / 128.0, -1.0]);
    }

    #[test]
    fn sixteen_bit_stereo_is_mixed_down_to_mono() {
        let mut data = 16384i16.to_le_bytes().to_vec();
        data.extend_from_slice(&(-16384i16).to_le_bytes()); // left +0.5, right -0.5
        data.extend_from_slice(&32767i16.to_le_bytes());
        data.extend_from_slice(&32767i16.to_le_bytes());
        let c = parse(&wav(2, 16, &data)).unwrap();
        assert_eq!(c.samples.len(), 2);
        assert_eq!(c.samples[0], 0.0);
        assert!(c.samples[1] > 0.99);
    }

    #[test]
    fn other_files_and_formats_are_errors() {
        assert!(parse(b"nope").is_err());
        assert!(parse(&wav(1, 8, &[])[..30]).is_err());
        assert!(parse(&wav(3, 8, &[1, 2, 3])).is_err());
        let mut adpcm = wav(1, 8, &[1]);
        adpcm[20] = 0x11; // format tag: not PCM
        assert!(parse(&adpcm).is_err());
    }
}
