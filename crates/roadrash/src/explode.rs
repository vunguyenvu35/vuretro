//! Packed resources. A resource of `FAMILIES.RSC` starts with 16 bytes (the size once unpacked,
//! the size as stored, the CRC-32 of the unpacked bytes, and a flag that says it is packed) and
//! then the payload, which is packed in the "implode" format of PKWARE's Data Compression
//! Library (the same one that zlib's `blast` reads).
//!
//! That format, as its specification has it: the payload starts with a byte that says whether
//! the literals are coded (0: 8 bits each, 1: with a Huffman code) and a byte for the size of
//! the window (4, 5 or 6: 1024, 2048 or 4096 bytes). Then comes a stream of bits, least
//! significant first: a 1 starts a copy (a length from a Huffman code plus extra bits, then
//! a distance from another code plus low bits), a 0 starts a literal. A length of 519 ends it.
//! The Huffman codes are written as run-lengths of code lengths, and their bits are inverted.

const MAX_UNPACKED: usize = 1 << 24;

/// A canonical Huffman code, from the compact table of the format: every byte is a code length
/// in its low nibble and, in its high nibble plus one, how many symbols have that length.
struct Huffman {
    count: [u16; 14],
    symbol: Vec<u16>,
}

impl Huffman {
    fn new(compact: &[u8]) -> Huffman {
        let mut lengths = Vec::new();
        for &b in compact {
            for _ in 0..(b >> 4) + 1 {
                lengths.push(b & 15);
            }
        }
        let mut count = [0u16; 14];
        for &l in &lengths {
            count[l as usize] += 1;
        }
        let mut offset = [0u16; 14];
        for l in 1..13 {
            offset[l + 1] = offset[l] + count[l];
        }
        let mut symbol = vec![0u16; lengths.len()];
        for (s, &l) in lengths.iter().enumerate() {
            if l != 0 {
                symbol[offset[l as usize] as usize] = s as u16;
                offset[l as usize] += 1;
            }
        }
        Huffman { count, symbol }
    }
}

struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
    buffer: u32,
    left: u32,
}

impl Bits<'_> {
    fn get(&mut self, n: u32) -> Result<u32, String> {
        while self.left < n {
            let byte = *self
                .data
                .get(self.pos)
                .ok_or("the packed data ends early")?;
            self.pos += 1;
            self.buffer |= (byte as u32) << self.left;
            self.left += 8;
        }
        let v = self.buffer & ((1u32 << n) - 1);
        self.buffer >>= n;
        self.left -= n;
        Ok(v)
    }

    fn decode(&mut self, h: &Huffman) -> Result<usize, String> {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..=13usize {
            code |= (self.get(1)? ^ 1) as i32;
            let count = h.count[len] as i32;
            if code < first + count {
                return Ok(h.symbol[(index + code - first) as usize] as usize);
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        Err("a code that is not in the table".into())
    }
}

const LITERALS: [u8; 98] = [
    11, 124, 8, 7, 28, 7, 188, 13, 76, 4, 10, 8, 12, 10, 12, 10, 8, 23, 8, 9, 7, 6, 7, 8, 7, 6, 55,
    8, 23, 24, 12, 11, 7, 9, 11, 12, 6, 7, 22, 5, 7, 24, 6, 11, 9, 6, 7, 22, 7, 11, 38, 7, 9, 8,
    25, 11, 8, 11, 9, 12, 8, 12, 5, 38, 5, 38, 5, 11, 7, 5, 6, 21, 6, 10, 53, 8, 7, 24, 10, 27, 44,
    253, 253, 253, 252, 252, 252, 13, 12, 45, 12, 45, 12, 61, 12, 45, 44, 173,
];
const LENGTHS: [u8; 6] = [2, 35, 36, 53, 38, 23];
const DISTANCES: [u8; 7] = [2, 20, 53, 230, 247, 151, 248];
const LENGTH_BASE: [u32; 16] = [3, 2, 4, 5, 6, 7, 8, 9, 10, 12, 16, 24, 40, 72, 136, 264];
const LENGTH_EXTRA: [u32; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8];

/// Unpack `data`, which must come to exactly `expected` bytes.
pub fn explode(data: &[u8], expected: usize) -> Result<Vec<u8>, String> {
    if expected > MAX_UNPACKED {
        return Err("the unpacked size is too big".into());
    }
    let (literals, lengths, distances) = (
        Huffman::new(&LITERALS),
        Huffman::new(&LENGTHS),
        Huffman::new(&DISTANCES),
    );
    let mut bits = Bits {
        data,
        pos: 0,
        buffer: 0,
        left: 0,
    };
    let (coded, window) = (bits.get(8)?, bits.get(8)?);
    if coded > 1 {
        return Err("the literal mode is not 0 or 1".into());
    }
    if !(4..=6).contains(&window) {
        return Err("the window size is not 4, 5 or 6".into());
    }
    let mut out: Vec<u8> = Vec::with_capacity(expected.min(65536));
    loop {
        if bits.get(1)? == 1 {
            let s = bits.decode(&lengths)?;
            let len = LENGTH_BASE[s] + bits.get(LENGTH_EXTRA[s])?;
            if len == 519 {
                break;
            }
            let extra = if len == 2 { 2 } else { window };
            // the two reads are separate: the high part comes first in the stream
            let high = (bits.decode(&distances)? as u32) << extra;
            let distance = (high + bits.get(extra)? + 1) as usize;
            if distance > out.len() {
                return Err("a copy reaches back before the start".into());
            }
            if out.len() + len as usize > expected {
                return Err("the data is longer than it says".into());
            }
            for _ in 0..len {
                out.push(out[out.len() - distance]);
            }
        } else {
            if out.len() >= expected {
                return Err("the data is longer than it says".into());
            }
            out.push(if coded == 1 {
                bits.decode(&literals)? as u8
            } else {
                bits.get(8)? as u8
            });
        }
    }
    if out.len() != expected {
        return Err(format!(
            "{} bytes came out, {expected} were said",
            out.len()
        ));
    }
    Ok(out)
}

/// The CRC-32 of the usual kind (reflected, polynomial 0xEDB88320).
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                0xEDB8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// The bytes of a resource of `FAMILIES.RSC`, unpacked when they are packed.
pub fn unpack(resource: &[u8]) -> Result<Vec<u8>, String> {
    let word = |at: usize| {
        resource
            .get(at..at + 4)
            .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
            .ok_or("the resource is cut before its header ends")
    };
    let (unpacked, stored, crc, packed) =
        (word(0)? as usize, word(4)? as usize, word(8)?, word(12)?);
    let payload = resource
        .get(16..16 + stored)
        .ok_or("the resource is shorter than it says")?;
    if packed == 0 {
        if stored != unpacked {
            return Err("an unpacked resource has two sizes".into());
        }
        return Ok(payload.to_vec());
    }
    let out = explode(payload, unpacked)?;
    if crc32(&out) != crc {
        return Err("the CRC of the unpacked bytes is not the one in the header".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crc_is_the_usual_one() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    /// The smallest packed stream: plain literals (mode 0), window 4, the bytes "hi", the end
    /// mark (a copy of length 519: length code 15 with 8 extra bits of 255).
    fn hi() -> Vec<u8> {
        let mut bits: Vec<u8> = Vec::new();
        let mut put = |v: u32, n: u32| bits.extend((0..n).map(|i| ((v >> i) & 1) as u8));
        put(0, 8);
        put(4, 8);
        for b in *b"hi" {
            put(0, 1);
            put(b as u32, 8);
        }
        put(1, 1);
        // length symbol 15: its code is the last of the length table, written inverted
        let lengths = Huffman::new(&LENGTHS);
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        let mut path = Vec::new();
        'search: for len in 1..=13usize {
            let count = lengths.count[len] as i32;
            for c in 0..count {
                if lengths.symbol[(index + c) as usize] == 15 {
                    let value = first + c;
                    path = (0..len).rev().map(|i| ((value >> i) & 1) as u8).collect();
                    break 'search;
                }
            }
            let _ = &mut code;
            index += count;
            first = (first + count) << 1;
        }
        for b in path {
            put((b ^ 1) as u32, 1);
        }
        put(255, 8);
        bits.chunks(8)
            .map(|c| c.iter().enumerate().fold(0u8, |a, (i, &b)| a | (b << i)))
            .collect()
    }

    #[test]
    fn a_small_packed_stream_comes_out_as_what_it_holds() {
        assert_eq!(explode(&hi(), 2).unwrap(), b"hi");
        assert!(explode(&hi(), 3).is_err(), "it says more than there is");
        assert!(explode(&hi(), 1).is_err(), "it says less than there is");
    }

    #[test]
    fn broken_streams_are_errors_not_panics() {
        assert!(explode(&[], 0).is_err());
        assert!(explode(&[2, 4], 0).is_err(), "literal mode 2");
        assert!(explode(&[0, 9], 0).is_err(), "window 9");
        let good = hi();
        for cut in 0..good.len() {
            let _ = explode(&good[..cut], 2);
        }
    }

    #[test]
    fn a_resource_that_is_not_packed_is_copied_and_a_wrong_crc_is_refused() {
        let mut r = Vec::new();
        for w in [3u32, 3, 0, 0] {
            r.extend_from_slice(&w.to_le_bytes());
        }
        r.extend_from_slice(&[7, 8, 9]);
        assert_eq!(unpack(&r).unwrap(), vec![7, 8, 9]);
        let packed = |crc: u32| {
            let mut p = Vec::new();
            for w in [2u32, hi().len() as u32, crc, 1] {
                p.extend_from_slice(&w.to_le_bytes());
            }
            p.extend(hi());
            p
        };
        assert_eq!(unpack(&packed(crc32(b"hi"))).unwrap(), b"hi");
        assert!(unpack(&packed(1)).is_err());
        assert!(unpack(&[1, 2]).is_err());
    }
}
