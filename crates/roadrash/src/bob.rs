//! Sprite sets (`DATA/BIKERS/*.DAT`): frames one after the other, each a width byte, a height
//! byte and then `width * height` palette indices, row after row. A set has no count: it
//! runs to the end of the file. (Plain `IMAGES/*.BOB` files have no header and are read with
//! [`crate::image::decode_bob`].)

/// One sprite: palette indices, index 0 being the see-through colour (to be confirmed).
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    pub indices: Vec<u8>,
}

/// Every frame of a set. A set that does not end exactly at a frame's end is an error.
pub fn read_frames(bytes: &[u8]) -> Result<Vec<Frame>, String> {
    let mut frames = Vec::new();
    let mut at = 0usize;
    while at < bytes.len() {
        let header = bytes
            .get(at..at + 2)
            .ok_or_else(|| format!("frame {} is cut before its size", frames.len()))?;
        let (width, height) = (header[0] as u32, header[1] as u32);
        let end = at + 2 + (width * height) as usize;
        let indices = bytes
            .get(at + 2..end)
            .ok_or_else(|| format!("frame {} is cut in its pixels", frames.len()))?;
        frames.push(Frame {
            width,
            height,
            indices: indices.to_vec(),
        });
        at = end;
    }
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_follow_each_other_to_the_end() {
        let frames = read_frames(&[2, 1, 7, 8, 0, 0, 1, 1, 9]).unwrap();
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].indices, vec![7, 8]);
        assert_eq!(
            (frames[1].width, frames[1].height),
            (0, 0),
            "an empty frame is allowed"
        );
        assert_eq!(frames[2].indices, vec![9]);
    }

    #[test]
    fn a_cut_file_is_an_error() {
        assert!(read_frames(&[2, 2, 1, 2, 3]).is_err());
        assert!(read_frames(&[2]).is_err());
        assert!(read_frames(&[]).unwrap().is_empty());
    }
}
