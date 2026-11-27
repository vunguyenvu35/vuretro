//! Animations (`RRAN`), the leaves of a family that hold the pictures of a thing.
//!
//! As the disc has them (offsets are counted from the start of the animation): `RRAN`, 20,
//! the offset of an info block (`ANIM`, whose frame count is at +16), the offset of a CCB
//! template, and `RRFD` at +20, then at +24 the length of the frame table, which is 28
//! bytes for each frame after a first 28 bytes of header. A frame holds, at +4, the offset
//! of a pixel chunk (the pixels start 8 bytes into it), at +8 the offset of the hotspot (two
//! `i32`: where the thing's feet are in its picture), at +16 flags, and at +20 and +24 the
//! width and height. Frames come in threes; the third is the one the roadside uses.

use crate::cel::{read_indexed, Indexed};
use std::sync::Arc;

const RRAN: u32 = 0x5252_414E;
const RRFD: u32 = 0x5252_4644;
const ANIM: u32 = 0x414E_494D;
const CCB: u32 = 0x4343_4220;

#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub texture: Option<Arc<Indexed>>,
    pub hotspot: [i32; 2],
    pub flags: u32,
}

fn word(b: &[u8], at: usize) -> Result<u32, String> {
    b.get(at..at + 4)
        .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
        .ok_or_else(|| format!("the animation is cut before byte {at}"))
}

/// The frames of the animation that starts at `offset` of `bytes`.
pub fn read(bytes: &[u8], offset: usize) -> Result<Vec<Frame>, String> {
    let at = |p: usize, n: usize| {
        let target = offset + word(bytes, p)? as usize;
        if target + n > bytes.len() {
            return Err("an offset points outside the animation".to_string());
        }
        Ok(target)
    };
    if word(bytes, offset)? != RRAN
        || word(bytes, offset + 4)? != 20
        || word(bytes, offset + 20)? != RRFD
    {
        return Err("not an animation".into());
    }
    let info = at(offset + 8, 20)?;
    if word(bytes, info)? != ANIM {
        return Err("no animation info".into());
    }
    let count = word(bytes, info + 16)? as usize;
    if count == 0 || count * 28 + 8 != word(bytes, offset + 24)? as usize {
        return Err("the frame count does not match the table".into());
    }
    let ccb = at(offset + 12, 80)?;
    if word(bytes, ccb)? != CCB {
        return Err("no CCB template".into());
    }
    let mut frames = Vec::new();
    for i in 0..count {
        let frame = offset + 28 + i * 28;
        let hotspot = if word(bytes, frame + 8)? != 0 {
            let spot = at(frame + 8, 8)?;
            [word(bytes, spot)? as i32, word(bytes, spot + 4)? as i32]
        } else {
            [0, 0]
        };
        let texture = if word(bytes, frame + 4)? != 0 {
            let body = at(frame + 4, 12)? + 8;
            // other kinds of pixels (a single word, for a flat colour) are not drawn
            read_indexed(&bytes[body..]).ok().map(Arc::new)
        } else {
            None
        };
        frames.push(Frame {
            texture,
            hotspot,
            flags: word(bytes, frame + 16)?,
        });
    }
    // a thing of one step whose first frames hold no picture shows the third everywhere; and
    // where the first picture says so (flag 1) it stands for the other two of its three
    if count == 3 && frames[0].texture.is_none() {
        let shared = frames[2].texture.clone();
        frames[0].texture = shared.clone();
        frames[1].texture = shared;
    } else if frames[0].texture.as_ref().is_some_and(|t| t.flags & 1 == 1) {
        for i in (0..count.saturating_sub(2)).step_by(3) {
            let shared = frames[i].texture.clone();
            frames[i + 1].texture = shared.clone();
            frames[i + 2].texture = shared;
        }
    }
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(b: &mut Vec<u8>, at: usize, v: u32) {
        if b.len() < at + 4 {
            b.resize(at + 4, 0);
        }
        b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    /// An animation of three frames, the third with a 2x2 picture and a hotspot.
    fn sample() -> Vec<u8> {
        let mut b = vec![0u8; 400];
        put(&mut b, 0, RRAN);
        put(&mut b, 4, 20);
        put(&mut b, 8, 200); // info
        put(&mut b, 12, 220); // ccb
        put(&mut b, 20, RRFD);
        put(&mut b, 24, 3 * 28 + 8);
        put(&mut b, 200, ANIM);
        put(&mut b, 200 + 4, 20);
        put(&mut b, 216, 3);
        put(&mut b, 220, CCB);
        // frame 2 at 28 + 56 = 84
        put(&mut b, 84 + 4, 300); // pixel chunk
        put(&mut b, 84 + 8, 380); // hotspot
        put(&mut b, 380, 5);
        put(&mut b, 384, 7);
        // the chunk at 300: 8 bytes of chunk header, then the texture
        let t = 308;
        put(&mut b, t, 0x00BC_614E);
        put(&mut b, t + 32, 2);
        put(&mut b, t + 36, 2);
        put(&mut b, t + 40, 1);
        put(&mut b, t + 56, 68 - 28); // the pixels at t + 68
        b[t + 68..t + 72].copy_from_slice(&[1, 2, 3, 4]);
        b
    }

    #[test]
    fn the_frames_the_hotspot_and_the_picture_are_read() {
        let f = read(&sample(), 0).unwrap();
        assert_eq!(f.len(), 3);
        let t = f[2].texture.as_ref().unwrap();
        assert_eq!(
            (t.width, t.height, t.pixels.clone()),
            (2, 2, vec![1, 2, 3, 4])
        );
        assert_eq!(f[2].hotspot, [5, 7]);
        // the first two have no picture of their own, so they show the third
        assert_eq!(f[0].texture, f[2].texture);
    }

    #[test]
    fn other_data_and_cut_data_are_errors_not_panics() {
        assert!(read(&[0; 10], 0).is_err());
        let good = sample();
        assert!(read(&vec![0; 400], 0).is_err());
        for cut in [4, 30, 100, 210, 310] {
            let _ = read(&good[..cut], 0);
        }
    }
}
