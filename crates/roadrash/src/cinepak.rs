//! Cinepak video (`cvid`), the codec of the disc's AVI files.
//!
//! A frame is a 10-byte header (flags, 24-bit length, width, height, strip count; the sizes
//! are big-endian) and then strips: each has a 12-byte header (id, size, the strip's top,
//! left, bottom and right) and chunks. The picture is made of 4x4 blocks; each block is
//! either kept from the previous frame, or painted from a "V1" code book (one entry, a 2x2
//! cell made twice as big) or from a "V4" code book (four entries, one per 2x2 corner). An
//! entry is 4 luma values and two chroma values. The code books survive from frame to frame
//! and are changed by the chunks. All of this is kept in a [`Decoder`].

/// Decoder state: the picture so far and the two code books.
pub struct Decoder {
    width: usize,
    height: usize,
    /// The picture, 3 bytes per pixel.
    rgb: Vec<u8>,
    /// The code books of each strip (a frame has at most 32). A strip starts from the books
    /// of the strip above it, or, in a frame whose flag bit 0 is set, from its own books of
    /// the frame before.
    books: Vec<Books>,
}

#[derive(Clone)]
struct Books {
    v1: Vec<[u8; 6]>,
    v4: Vec<[u8; 6]>,
}

impl Books {
    fn empty() -> Books {
        Books {
            v1: vec![[0; 6]; 256],
            v4: vec![[0; 6]; 256],
        }
    }
}

struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let s = self
            .data
            .get(self.at..self.at + n)
            .ok_or("the frame is cut short")?;
        self.at += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<usize, String> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]) as usize)
    }
    fn u32(&mut self) -> Result<u32, String> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
}

/// A run of bits, taken from 32-bit words, most significant bit first.
struct Bits {
    word: u32,
    left: u32,
}
impl Bits {
    fn next(&mut self, r: &mut Reader) -> Result<bool, String> {
        if self.left == 0 {
            self.word = r.u32()?;
            self.left = 32;
        }
        self.left -= 1;
        Ok(self.word >> self.left & 1 == 1)
    }
}

fn clamp(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

impl Decoder {
    pub fn new(width: usize, height: usize) -> Decoder {
        Decoder {
            width,
            height,
            rgb: vec![0; width * height * 3],
            books: vec![Books::empty(); 32],
        }
    }

    /// The picture as it is now: `width * height * 3` bytes.
    pub fn picture(&self) -> &[u8] {
        &self.rgb
    }

    /// Paint one 2x2 cell of the picture from a code book entry's luma value `which` (0..4).
    fn put(&mut self, x: usize, y: usize, entry: &[u8; 6], which: usize) {
        if x >= self.width || y >= self.height {
            return;
        }
        let luma = entry[which] as i32;
        let (u, v) = (entry[4] as i8 as i32, entry[5] as i8 as i32);
        let at = (y * self.width + x) * 3;
        self.rgb[at] = clamp(luma + 2 * v);
        self.rgb[at + 1] = clamp(luma - u / 2 - v);
        self.rgb[at + 2] = clamp(luma + 2 * u);
    }

    fn paint_cell(&mut self, x: usize, y: usize, entry: [u8; 6]) {
        for (n, (dx, dy)) in [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().enumerate() {
            self.put(x + dx, y + dy, &entry, n);
        }
    }

    /// Decode one frame into the picture.
    pub fn frame(&mut self, data: &[u8]) -> Result<(), String> {
        let mut r = Reader { data, at: 0 };
        let flags = r.u8()?;
        r.take(3)?; // the length: the data is as long as the chunk it came in
        let (_w, _h) = (r.u16()?, r.u16()?);
        let strips = r.u16()?;
        if strips == 0 || strips > self.books.len() {
            return Err(format!("{strips} strips"));
        }
        let mut top = 0usize;
        for strip in 0..strips {
            if strip > 0 && flags & 1 == 0 {
                self.books[strip] = self.books[strip - 1].clone();
            }
            let strip_start = r.at;
            let _id = r.u16()?;
            let size = r.u16()?;
            let (_y1, _x1, y2, _x2) = (r.u16()?, r.u16()?, r.u16()?, r.u16()?);
            if size < 12 {
                return Err("a strip is too small".into());
            }
            // the strips follow each other; a strip's bottom edge is as the header says
            let bottom = if y2 > top { y2 } else { top + y2 };
            let area = (top, bottom.min(self.height));
            let end = strip_start + size;
            while r.at + 4 <= end {
                let id = r.u16()? >> 8; // the kind is in the high byte; the low one is 0
                let len = r.u16()?;
                if len < 4 {
                    return Err("a chunk is too small".into());
                }
                let body = r.take(len - 4)?;
                self.chunk(strip, id, body, area)?;
            }
            r.at = end;
            top = area.1;
        }
        Ok(())
    }

    fn chunk(
        &mut self,
        strip: usize,
        id: usize,
        body: &[u8],
        area: (usize, usize),
    ) -> Result<(), String> {
        let mut r = Reader { data: body, at: 0 };
        match id {
            0x20..=0x27 => {
                let colour = id & 4 == 0;
                let size = if colour { 6 } else { 4 };
                let books = &mut self.books[strip];
                let book = if id & 2 == 0 {
                    &mut books.v4
                } else {
                    &mut books.v1
                };
                let update = id & 1 == 1;
                let mut bits = Bits { word: 0, left: 0 };
                let mut i = 0;
                while i < 256 && r.at < body.len() {
                    if !update || bits.next(&mut r)? {
                        let e = r.take(size)?;
                        let mut entry = [0u8; 6];
                        entry[..size].copy_from_slice(e);
                        if !colour {
                            entry[4] = 0; // grey: no chroma
                            entry[5] = 0;
                        }
                        book[i] = entry;
                    }
                    i += 1;
                }
                Ok(())
            }
            0x30..=0x32 => self.blocks(strip, id, &mut r, area),
            _ => Ok(()), // not understood: skipped, the strip carries its own length
        }
    }

    fn blocks(
        &mut self,
        strip: usize,
        id: usize,
        r: &mut Reader,
        area: (usize, usize),
    ) -> Result<(), String> {
        let inter = id == 0x31;
        let v1_only = id == 0x32;
        let mut bits = Bits { word: 0, left: 0 };
        for y in (area.0..area.1).step_by(4) {
            for x in (0..self.width).step_by(4) {
                if inter && !bits.next(r)? {
                    continue; // this block stays as it was
                }
                if v1_only || !bits.next(r)? {
                    let entry = self.books[strip].v1[r.u8()? as usize];
                    for (dx, dy) in [(0, 0), (2, 0), (0, 2), (2, 2)] {
                        // one V1 entry covers the block: each luma value fills a 2x2 cell
                        let which = (dy / 2) * 2 + dx / 2;
                        for (cx, cy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                            self.put(x + dx + cx, y + dy + cy, &entry, which);
                        }
                    }
                } else {
                    for (dx, dy) in [(0, 0), (2, 0), (0, 2), (2, 2)] {
                        let entry = self.books[strip].v4[r.u8()? as usize];
                        self.paint_cell(x + dx, y + dy, entry);
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame of one strip holding the given chunks, as the files lay it out.
    fn frame(width: u16, height: u16, chunks: &[(u16, Vec<u8>)]) -> Vec<u8> {
        let mut strip = Vec::new();
        for (id, body) in chunks {
            strip.extend_from_slice(&(id << 8).to_be_bytes());
            strip.extend_from_slice(&((body.len() + 4) as u16).to_be_bytes());
            strip.extend_from_slice(body);
        }
        let mut s = 0x1000u16.to_be_bytes().to_vec();
        s.extend_from_slice(&((strip.len() + 12) as u16).to_be_bytes());
        for v in [0, 0, height, width] {
            s.extend_from_slice(&(v as u16).to_be_bytes());
        }
        s.extend(strip);
        let mut f = vec![0, 0, 0, 0];
        f.extend_from_slice(&width.to_be_bytes());
        f.extend_from_slice(&height.to_be_bytes());
        f.extend_from_slice(&1u16.to_be_bytes());
        f.extend(s);
        f
    }

    #[test]
    fn a_v1_block_fills_four_cells_from_one_entry() {
        // V1 book entry 0: luma 10,20,30,40 and no chroma; a V1-only block uses entry 0
        let book = vec![10, 20, 30, 40, 0, 0];
        let mut d = Decoder::new(4, 4);
        d.frame(&frame(4, 4, &[(0x22, book), (0x32, vec![0])]))
            .unwrap();
        let px = |x: usize, y: usize| d.picture()[(y * 4 + x) * 3];
        assert_eq!((px(0, 0), px(1, 1)), (10, 10), "top left cell");
        assert_eq!((px(2, 0), px(3, 1)), (20, 20), "top right cell");
        assert_eq!((px(0, 2), px(1, 3)), (30, 30), "bottom left cell");
        assert_eq!((px(2, 2), px(3, 3)), (40, 40), "bottom right cell");
    }

    #[test]
    fn an_inter_block_that_is_skipped_keeps_the_last_frame() {
        let mut d = Decoder::new(4, 4);
        d.frame(&frame(
            4,
            4,
            &[(0x22, vec![200, 200, 200, 200, 0, 0]), (0x32, vec![0])],
        ))
        .unwrap();
        // inter chunk: one flag word whose first bit (skip) is 0
        d.frame(&frame(4, 4, &[(0x31, vec![0, 0, 0, 0])])).unwrap();
        assert_eq!(d.picture()[0], 200);
    }

    #[test]
    fn cut_or_empty_frames_are_errors_not_panics() {
        let mut d = Decoder::new(4, 4);
        assert!(d.frame(&[]).is_err());
        let good = frame(4, 4, &[(0x22, vec![1; 6]), (0x32, vec![0])]);
        for cut in 0..good.len() {
            let _ = d.frame(&good[..cut]);
        }
        // a V1-only chunk that runs out of block bytes
        assert!(Decoder::new(8, 8)
            .frame(&frame(8, 8, &[(0x32, vec![0])]))
            .is_err());
    }
}
