//! `CEL ` resources: pictures in the format of the 3DO, as a list of chunks. Each chunk is a
//! tag word, its own length (tag and length included) and a body; all words are little-endian
//! and the tags are written backwards in the file, so they are compared here as numbers.
//!
//! Chunks found: `CCB ` (the picture's size), `PLUT` (a palette of 16-bit colours) and the
//! pixel chunk, whose indexed form starts with the word `0x00BC614E` and holds the picture and
//! its smaller copies (mip levels). Only the full-size level of the indexed form is read.

const CCB: u32 = 0x4343_4220;
const PDAT: u32 = 0x5044_4154;
const INDEXED: u32 = 0x00BC_614E;

/// An indexed picture: one palette index per pixel.
#[derive(Debug, Clone, PartialEq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub indices: Vec<u8>,
}

fn word(b: &[u8], at: usize) -> Result<u32, String> {
    b.get(at..at + 4)
        .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
        .ok_or_else(|| format!("cut before byte {at}"))
}

/// The indexed picture of a `CEL ` resource, `None` when it has another kind of pixels.
pub fn read_texture(resource: &[u8]) -> Result<Option<Texture>, String> {
    let mut at = 0;
    let mut has_ccb = false;
    while at < resource.len() {
        let (tag, extent) = (word(resource, at)?, word(resource, at + 4)? as usize);
        if extent < 8 || at + extent > resource.len() {
            return Err(format!("a chunk at byte {at} has an impossible length"));
        }
        if tag == CCB {
            has_ccb = true;
        } else if tag == PDAT || tag == 0x5250_4454 || tag == 0x5252_5044 {
            if !has_ccb {
                return Err("pixels come before the picture's header".into());
            }
            let body = &resource[at + 8..at + extent];
            if word(body, 0)? != INDEXED {
                return Ok(None);
            }
            let (width, height) = (word(body, 32)?, word(body, 36)?);
            if width == 0 || width > 256 || height == 0 || height > 512 {
                return Err(format!("a picture of {width}x{height} is not believable"));
            }
            // the first mip level: its offset is in the table at 56, counted from byte 28
            let start = word(body, 56)? as usize + 28;
            let size = (width * height) as usize;
            let indices = body
                .get(start..start + size)
                .ok_or("the pixels run past the chunk")?;
            return Ok(Some(Texture {
                width,
                height,
                indices: indices.to_vec(),
            }));
        }
        at += extent;
    }
    Err("no pixel chunk".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(tag: u32, body: &[u8]) -> Vec<u8> {
        let mut c = tag.to_le_bytes().to_vec();
        c.extend_from_slice(&((body.len() + 8) as u32).to_le_bytes());
        c.extend_from_slice(body);
        c
    }

    /// A 2x2 indexed picture the way the files hold one.
    fn indexed() -> Vec<u8> {
        let mut body = vec![0u8; 68];
        body[..4].copy_from_slice(&INDEXED.to_le_bytes());
        body[32..36].copy_from_slice(&2u32.to_le_bytes());
        body[36..40].copy_from_slice(&2u32.to_le_bytes());
        body[40..44].copy_from_slice(&1u32.to_le_bytes());
        body[56..60].copy_from_slice(&(68u32 - 28).to_le_bytes());
        body.extend_from_slice(&[1, 2, 3, 4]);
        let mut r = chunk(CCB, &[0; 80]);
        r.extend(chunk(PDAT, &body));
        r
    }

    #[test]
    fn the_indexed_picture_is_found_among_the_chunks() {
        let t = read_texture(&indexed()).unwrap().unwrap();
        assert_eq!((t.width, t.height, t.indices), (2, 2, vec![1, 2, 3, 4]));
    }

    #[test]
    fn other_pixels_are_none_and_broken_files_are_errors() {
        let mut r = chunk(CCB, &[0; 80]);
        r.extend(chunk(PDAT, &[9, 9, 9, 9]));
        assert_eq!(read_texture(&r).unwrap(), None);
        let good = indexed();
        for cut in [3, 9, 50, good.len() - 2] {
            assert!(read_texture(&good[..cut]).is_err(), "cut at {cut}");
        }
    }
}
