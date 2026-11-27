//! IMA ADPCM as the AVI sound blocks hold it (format 0x0011). A block starts with a header per
//! channel (a 16-bit starting sample, a step index and a spare byte); the 4-bit codes follow,
//! 8 codes (4 bytes) of one channel at a time, the channels taking turns.

const STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];
const INDEX_CHANGE: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

struct Channel {
    predictor: i32,
    index: i32,
}

impl Channel {
    fn decode(&mut self, code: u8) -> i16 {
        let step = STEPS[self.index as usize];
        let mut diff = step >> 3;
        if code & 1 != 0 {
            diff += step >> 2;
        }
        if code & 2 != 0 {
            diff += step >> 1;
        }
        if code & 4 != 0 {
            diff += step;
        }
        self.predictor = (self.predictor + if code & 8 != 0 { -diff } else { diff })
            .clamp(i16::MIN as i32, i16::MAX as i32);
        self.index = (self.index + INDEX_CHANGE[(code & 7) as usize]).clamp(0, 88);
        self.predictor as i16
    }
}

/// Decode one block into interleaved 16-bit samples.
pub fn decode_block(block: &[u8], channels: usize) -> Result<Vec<i16>, String> {
    if channels == 0 || channels > 2 || block.len() < 4 * channels {
        return Err("a block needs a 4-byte header for each channel".into());
    }
    let mut state = Vec::new();
    for c in 0..channels {
        let h = &block[c * 4..c * 4 + 4];
        let index = h[2] as i32;
        if index > 88 {
            return Err(format!("step index {index} is out of range"));
        }
        state.push(Channel {
            predictor: i16::from_le_bytes([h[0], h[1]]) as i32,
            index,
        });
    }
    let per_channel: Vec<Vec<i16>> = {
        let mut out: Vec<Vec<i16>> = state.iter().map(|s| vec![s.predictor as i16]).collect();
        let body = &block[4 * channels..];
        for group in body.chunks(4 * channels) {
            for (c, bytes) in group.chunks(4).enumerate().take(channels) {
                for &byte in bytes {
                    out[c].push(state[c].decode(byte & 15));
                    out[c].push(state[c].decode(byte >> 4));
                }
            }
        }
        out
    };
    let frames = per_channel.iter().map(Vec::len).min().unwrap_or(0);
    let mut samples = Vec::with_capacity(frames * channels);
    for i in 0..frames {
        for channel in &per_channel {
            samples.push(channel[i]);
        }
    }
    Ok(samples)
}

/// Write 16-bit samples as a WAV file.
pub fn wav(samples: &[i16], channels: u16, rate: u32) -> Vec<u8> {
    let data_len = samples.len() as u32 * 2;
    let mut w = b"RIFF".to_vec();
    w.extend_from_slice(&(36 + data_len).to_le_bytes());
    w.extend_from_slice(b"WAVEfmt ");
    w.extend_from_slice(&16u32.to_le_bytes());
    w.extend_from_slice(&1u16.to_le_bytes());
    w.extend_from_slice(&channels.to_le_bytes());
    w.extend_from_slice(&rate.to_le_bytes());
    w.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
    w.extend_from_slice(&(channels * 2).to_le_bytes());
    w.extend_from_slice(&16u16.to_le_bytes());
    w.extend_from_slice(b"data");
    w.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        w.extend_from_slice(&s.to_le_bytes());
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_block_gives_the_header_sample_then_two_samples_per_byte() {
        // mono: starting sample 1000, step index 0, then one byte with two zero codes
        let s = decode_block(&[0xE8, 0x03, 0, 0, 0x00], 1).unwrap();
        assert_eq!(s.len(), 3);
        assert_eq!(s[0], 1000);
        // a zero code adds step/8 = 0 (7 >> 3), so the sample holds
        assert_eq!(s[1], 1000);
    }

    #[test]
    fn a_sign_bit_goes_down_and_the_channels_are_interleaved() {
        // stereo: left starts at 0, right at 0, step index 20 (step 50)
        let block = [0, 0, 20, 0, 0, 0, 20, 0, 0x0F, 0, 0, 0, 0x07, 0, 0, 0];
        let s = decode_block(&block, 2).unwrap();
        assert_eq!(s[0..2], [0, 0], "the header samples, left then right");
        assert!(s[2] < 0, "left: code 15 has the sign bit: {}", s[2]);
        assert!(s[3] > 0, "right: code 7 goes up: {}", s[3]);
    }

    #[test]
    fn bad_blocks_are_errors_and_the_wav_header_is_right() {
        assert!(decode_block(&[0, 0], 1).is_err());
        assert!(decode_block(&[0, 0, 99, 0], 1).is_err());
        assert!(decode_block(&[0; 4], 3).is_err());
        let w = wav(&[1, -1], 2, 22050);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(w.len(), 44 + 4);
    }
}
