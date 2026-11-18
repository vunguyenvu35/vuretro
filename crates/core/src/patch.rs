//! ROM patches in the three common formats: **IPS**, **UPS** and **BPS**. Fan translations
//! and ROM hacks are shared as patches, because a patch holds none of the original game.
//! Applying one never changes the original: the caller gets the patched bytes.
//!
//! Pure bytes in, bytes out, no I/O, so it is unit-tested without files. Malformed patches
//! return an error and never panic.

/// Patched results larger than this are refused (a patch that claims more is broken).
const MAX_OUTPUT: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Ips,
    Ups,
    Bps,
}

/// Whether a file name looks like a patch (by extension).
pub fn is_patch_name(name: &str) -> bool {
    matches!(
        name.rsplit('.')
            .next()
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("ips" | "ups" | "bps")
    ) && name.contains('.')
}

/// Recognise a patch by its header.
pub fn detect(patch: &[u8]) -> Option<Format> {
    if patch.starts_with(b"PATCH") {
        Some(Format::Ips)
    } else if patch.starts_with(b"UPS1") {
        Some(Format::Ups)
    } else if patch.starts_with(b"BPS1") {
        Some(Format::Bps)
    } else {
        None
    }
}

/// Apply `patch` to `rom` and return the patched bytes.
pub fn apply(rom: &[u8], patch: &[u8]) -> Result<Vec<u8>, String> {
    match detect(patch) {
        Some(Format::Ips) => apply_ips(rom, patch),
        Some(Format::Ups) => apply_ups(rom, patch),
        Some(Format::Bps) => apply_bps(rom, patch),
        None => Err("not a patch file (expected IPS, UPS or BPS)".into()),
    }
}

// ---------- CRC-32 (the checksum UPS and BPS carry) ----------

const fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
}

static CRC_TABLE: [u32; 256] = crc_table();

pub fn crc32(data: &[u8]) -> u32 {
    !data.iter().fold(!0u32, |c, &b| {
        CRC_TABLE[((c ^ u32::from(b)) & 0xFF) as usize] ^ (c >> 8)
    })
}

// ---------- reading helpers ----------

/// A cursor over the patch that fails cleanly at the end instead of panicking.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8], pos: usize) -> Self {
        Reader { data, pos }
    }
    fn byte(&mut self) -> Result<u8, String> {
        let b = *self.data.get(self.pos).ok_or("the patch is cut short")?;
        self.pos += 1;
        Ok(b)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let end = self.pos.checked_add(n).ok_or("the patch is damaged")?;
        let s = self
            .data
            .get(self.pos..end)
            .ok_or("the patch is cut short")?;
        self.pos = end;
        Ok(s)
    }
    fn be(&mut self, n: usize) -> Result<usize, String> {
        Ok(self
            .take(n)?
            .iter()
            .fold(0, |v, &b| (v << 8) | usize::from(b)))
    }
    /// The variable-length number UPS and BPS use.
    fn varint(&mut self) -> Result<u64, String> {
        let (mut data, mut shift) = (0u64, 1u64);
        loop {
            let x = self.byte()?;
            data = data
                .checked_add(
                    u64::from(x & 0x7F)
                        .checked_mul(shift)
                        .ok_or("number too big")?,
                )
                .ok_or("number too big")?;
            if x & 0x80 != 0 {
                return Ok(data);
            }
            shift = shift.checked_shl(7).ok_or("number too big")?;
            data = data.checked_add(shift).ok_or("number too big")?;
        }
    }
}

fn check_size(n: u64) -> Result<usize, String> {
    usize::try_from(n)
        .ok()
        .filter(|n| *n <= MAX_OUTPUT)
        .ok_or_else(|| "the patch asks for a file that is too large".to_string())
}

fn le32(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

/// The three checksums at the end of a UPS or BPS patch: input, output and the patch itself.
fn footer(patch: &[u8]) -> Result<(u32, u32, usize), String> {
    if patch.len() < 12 {
        return Err("the patch is cut short".into());
    }
    let end = patch.len() - 12;
    let (input, output, own) = (
        le32(&patch[end..]),
        le32(&patch[end + 4..]),
        le32(&patch[end + 8..]),
    );
    if crc32(&patch[..patch.len() - 4]) != own {
        return Err("the patch file is damaged (its checksum does not match)".into());
    }
    Ok((input, output, end))
}

// ---------- IPS ----------

fn apply_ips(rom: &[u8], patch: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = rom.to_vec();
    let mut r = Reader::new(patch, 5);
    loop {
        let head = r.take(3)?;
        if head == b"EOF" {
            break;
        }
        let offset = head.iter().fold(0usize, |v, &b| (v << 8) | usize::from(b));
        let size = r.be(2)?;
        let (len, fill, data): (usize, Option<u8>, &[u8]) = if size == 0 {
            let n = r.be(2)?;
            (n, Some(r.byte()?), &[])
        } else {
            (size, None, r.take(size)?)
        };
        let end = offset
            .checked_add(len)
            .filter(|e| *e <= MAX_OUTPUT)
            .ok_or("the patch asks for a file that is too large")?;
        if out.len() < end {
            out.resize(end, 0);
        }
        match fill {
            Some(b) => out[offset..end].fill(b),
            None => out[offset..end].copy_from_slice(data),
        }
    }
    // some tools append the length to cut the result to
    if r.pos + 3 == patch.len() {
        let n = r.be(3)?;
        out.truncate(n);
    }
    Ok(out)
}

// ---------- UPS ----------

fn apply_ups(rom: &[u8], patch: &[u8]) -> Result<Vec<u8>, String> {
    let (crc_in, crc_out, end) = footer(patch)?;
    let mut r = Reader::new(patch, 4);
    let (a, b) = (check_size(r.varint()?)?, check_size(r.varint()?)?);
    // a UPS patch works both ways: the same XOR turns the new file back into the old one
    let rom_crc = crc32(rom);
    let (want_len, want_crc) = if rom_crc == crc_in && rom.len() == a {
        (b, crc_out)
    } else if rom_crc == crc_out && rom.len() == b {
        (a, crc_in)
    } else {
        return Err("this patch is for a different version of the game".into());
    };
    let mut out = rom.to_vec();
    out.resize(want_len, 0);
    let mut pos = 0usize;
    while r.pos < end {
        pos = pos
            .checked_add(usize::try_from(r.varint()?).map_err(|_| "the patch is damaged")?)
            .ok_or("the patch is damaged")?;
        loop {
            let x = r.byte()?;
            if let Some(slot) = out.get_mut(pos) {
                *slot ^= x;
            }
            pos += 1;
            if x == 0 {
                break;
            }
        }
    }
    if crc32(&out) != want_crc {
        return Err("the patched file does not match what the patch expects".into());
    }
    Ok(out)
}

// ---------- BPS ----------

fn apply_bps(rom: &[u8], patch: &[u8]) -> Result<Vec<u8>, String> {
    let (crc_src, crc_dst, end) = footer(patch)?;
    let mut r = Reader::new(patch, 4);
    let src_len = check_size(r.varint()?)?;
    let dst_len = check_size(r.varint()?)?;
    let meta = check_size(r.varint()?)?;
    r.take(meta)?;
    if rom.len() != src_len || crc32(rom) != crc_src {
        return Err("this patch is for a different version of the game".into());
    }
    let mut out: Vec<u8> = Vec::with_capacity(dst_len);
    let (mut src_rel, mut dst_rel) = (0i64, 0i64);
    while r.pos < end {
        let data = r.varint()?;
        let len = check_size((data >> 2) + 1)?;
        if out.len() + len > dst_len {
            return Err("the patch writes past the end of the result".into());
        }
        match data & 3 {
            0 => {
                // the same bytes of the original, at the same place
                let at = out.len();
                let s = rom
                    .get(at..at + len)
                    .ok_or("the patch reads past the original")?;
                out.extend_from_slice(s);
            }
            1 => out.extend_from_slice(r.take(len)?),
            2 | 3 => {
                let d = r.varint()?;
                let delta = i64::try_from(d >> 1).map_err(|_| "the patch is damaged")?;
                let delta = if d & 1 != 0 { -delta } else { delta };
                if data & 3 == 2 {
                    src_rel += delta;
                    let from = usize::try_from(src_rel).map_err(|_| "the patch is damaged")?;
                    let s = rom
                        .get(from..from + len)
                        .ok_or("the patch reads past the original")?;
                    out.extend_from_slice(s);
                    src_rel += len as i64;
                } else {
                    dst_rel += delta;
                    let from = usize::try_from(dst_rel).map_err(|_| "the patch is damaged")?;
                    // may overlap what it is writing: byte by byte
                    for i in 0..len {
                        let b = *out
                            .get(from + i)
                            .ok_or("the patch reads what is not written yet")?;
                        out.push(b);
                    }
                    dst_rel += len as i64;
                }
            }
            _ => unreachable!(),
        }
    }
    if out.len() != dst_len || crc32(&out) != crc_dst {
        return Err("the patched file does not match what the patch expects".into());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_standard_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn names_and_headers_are_recognised() {
        assert!(is_patch_name("Game (Translation).ips"));
        assert!(is_patch_name("x.BPS"));
        assert!(is_patch_name("x.ups"));
        assert!(!is_patch_name("game.gba"));
        assert!(
            !is_patch_name("ips"),
            "a bare word is not a file with an extension"
        );
        assert_eq!(detect(b"PATCH..."), Some(Format::Ips));
        assert_eq!(detect(b"UPS1"), Some(Format::Ups));
        assert_eq!(detect(b"BPS1"), Some(Format::Bps));
        assert_eq!(detect(b"nope"), None);
        assert!(apply(b"rom", b"nope").unwrap_err().contains("not a patch"));
    }

    // ----- IPS -----

    fn ips(records: &[(usize, &[u8])]) -> Vec<u8> {
        let mut p = b"PATCH".to_vec();
        for (offset, data) in records {
            p.extend_from_slice(&(*offset as u32).to_be_bytes()[1..]);
            p.extend_from_slice(&(data.len() as u16).to_be_bytes());
            p.extend_from_slice(data);
        }
        p.extend_from_slice(b"EOF");
        p
    }

    #[test]
    fn ips_overwrites_bytes_and_can_grow_the_file() {
        let rom = vec![1u8; 8];
        let patched = apply(&rom, &ips(&[(2, &[9, 9]), (7, &[5, 6, 7])])).unwrap();
        assert_eq!(patched, vec![1, 1, 9, 9, 1, 1, 1, 5, 6, 7]);
        assert_eq!(rom, vec![1u8; 8], "the input is untouched");
    }

    #[test]
    fn ips_run_length_records_fill_a_range() {
        let mut p = b"PATCH".to_vec();
        p.extend_from_slice(&[0, 0, 1]); // offset 1
        p.extend_from_slice(&[0, 0]); // size 0 = run
        p.extend_from_slice(&[0, 3]); // 3 bytes
        p.push(0xAB);
        p.extend_from_slice(b"EOF");
        assert_eq!(apply(&[0; 6], &p).unwrap(), vec![0, 0xAB, 0xAB, 0xAB, 0, 0]);
    }

    #[test]
    fn ips_can_truncate_with_a_trailing_length() {
        let mut p = ips(&[(0, &[7])]);
        p.extend_from_slice(&[0, 0, 3]);
        assert_eq!(apply(&[1, 2, 3, 4, 5], &p).unwrap(), vec![7, 2, 3]);
    }

    #[test]
    fn ips_that_is_cut_short_or_absurd_is_an_error_not_a_panic() {
        let p = ips(&[(0, &[1, 2, 3])]);
        assert!(apply(&[0; 4], &p[..p.len() - 6]).is_err());
        // a record that claims to end at 16 MB + : refused by the size guard only if huge
        let mut huge = b"PATCH".to_vec();
        huge.extend_from_slice(&[0xFF, 0xFF, 0xFE, 0, 2, 1, 2]);
        huge.extend_from_slice(b"EOF");
        assert!(apply(&[], &huge).is_ok(), "16 MB is allowed");
        assert!(apply(&[0; 4], b"PATCH").is_err(), "no EOF");
    }

    // ----- helpers to build UPS and BPS patches in the tests -----

    fn varint(mut n: u64) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let x = (n & 0x7F) as u8;
            n >>= 7;
            if n == 0 {
                out.push(0x80 | x);
                return out;
            }
            out.push(x);
            n -= 1;
        }
    }

    fn seal(mut body: Vec<u8>, src: &[u8], dst: &[u8]) -> Vec<u8> {
        body.extend_from_slice(&crc32(src).to_le_bytes());
        body.extend_from_slice(&crc32(dst).to_le_bytes());
        let own = crc32(&body);
        body.extend_from_slice(&own.to_le_bytes());
        body
    }

    #[test]
    fn varints_round_trip() {
        for n in [
            0u64,
            1,
            127,
            128,
            129,
            16_511,
            16_512,
            1 << 30,
            u32::MAX as u64,
        ] {
            let v = varint(n);
            assert_eq!(Reader::new(&v, 0).varint().unwrap(), n, "{n}");
        }
    }

    // ----- UPS -----

    fn ups(src: &[u8], dst: &[u8]) -> Vec<u8> {
        let mut p = b"UPS1".to_vec();
        p.extend(varint(src.len() as u64));
        p.extend(varint(dst.len() as u64));
        let len = src.len().max(dst.len());
        let at = |s: &[u8], i: usize| s.get(i).copied().unwrap_or(0);
        let mut last = 0usize;
        let mut i = 0;
        while i < len {
            if at(src, i) == at(dst, i) {
                i += 1;
                continue;
            }
            p.extend(varint((i - last) as u64));
            while i < len && at(src, i) != at(dst, i) {
                p.push(at(src, i) ^ at(dst, i));
                i += 1;
            }
            p.push(0); // end of the run (this byte is also "consumed" as position)
            last = i + 1;
            i += 1;
        }
        seal(p, src, dst)
    }

    #[test]
    fn ups_changes_bytes_and_can_be_run_backwards() {
        let old: Vec<u8> = (0..64).collect();
        let mut new = old.clone();
        new[3] = 99;
        new[4] = 98;
        new[40] = 7;
        new.extend_from_slice(&[1, 2, 3]);
        let p = ups(&old, &new);
        assert_eq!(apply(&old, &p).unwrap(), new);
        assert_eq!(
            apply(&new, &p).unwrap(),
            old,
            "applied to the new file it undoes"
        );
    }

    #[test]
    fn ups_for_another_game_or_damaged_is_refused() {
        let old: Vec<u8> = (0..32).collect();
        let mut new = old.clone();
        new[1] = 0;
        let p = ups(&old, &new);
        assert!(apply(&[9; 32], &p)
            .unwrap_err()
            .contains("different version"));
        let mut broken = p.clone();
        broken[6] ^= 0xFF;
        assert!(apply(&old, &broken).unwrap_err().contains("damaged"));
        assert!(apply(&old, &p[..5]).is_err());
    }

    // ----- BPS -----

    /// A tiny encoder: copy what is the same from the source, write the rest as new bytes.
    fn bps(src: &[u8], dst: &[u8]) -> Vec<u8> {
        let mut p = b"BPS1".to_vec();
        p.extend(varint(src.len() as u64));
        p.extend(varint(dst.len() as u64));
        p.extend(varint(0)); // no metadata
        let mut i = 0;
        while i < dst.len() {
            let same = (i..dst.len())
                .take_while(|&k| src.get(k) == Some(&dst[k]))
                .count();
            if same > 0 {
                p.extend(varint((same as u64 - 1) << 2)); // SourceRead (command 0)
                i += same;
            } else {
                let new = (i..dst.len())
                    .take_while(|&k| src.get(k) != Some(&dst[k]))
                    .count();
                p.extend(varint(((new as u64 - 1) << 2) | 1)); // TargetRead
                p.extend_from_slice(&dst[i..i + new]);
                i += new;
            }
        }
        seal(p, src, dst)
    }

    #[test]
    fn bps_reads_source_and_writes_new_bytes() {
        let old: Vec<u8> = (0..100).collect();
        let mut new = old.clone();
        new[10] = 200;
        new[11] = 201;
        new.extend_from_slice(&[5, 5, 5]);
        assert_eq!(apply(&old, &bps(&old, &new)).unwrap(), new);
    }

    #[test]
    fn bps_copy_commands_use_relative_offsets() {
        let src: Vec<u8> = (0..16).collect();
        // result: src[8..12], then src[2..4] again (a backwards SourceCopy), then a
        // TargetCopy of the first two result bytes (overlap-safe, byte by byte)
        let mut dst = Vec::new();
        dst.extend_from_slice(&src[8..12]);
        dst.extend_from_slice(&src[2..4]);
        dst.extend_from_slice(&dst.clone()[0..2]);
        let mut p = b"BPS1".to_vec();
        p.extend(varint(src.len() as u64));
        p.extend(varint(dst.len() as u64));
        p.extend(varint(0));
        p.extend(varint((3 << 2) | 2)); // SourceCopy 4 bytes
        p.extend(varint(8 << 1)); // forward by 8
        p.extend(varint((1 << 2) | 2)); // SourceCopy 2 bytes
        p.extend(varint((10 << 1) | 1)); // backwards by 10: 12 - 10 = 2
        p.extend(varint((1 << 2) | 3)); // TargetCopy 2 bytes
        p.extend(varint(0)); // from the start of the result
        assert_eq!(apply(&src, &seal(p, &src, &dst)).unwrap(), dst);
    }

    #[test]
    fn bps_checks_the_original_and_the_result() {
        let old: Vec<u8> = (0..40).collect();
        let mut new = old.clone();
        new[5] = 1;
        let p = bps(&old, &new);
        assert!(apply(&[0; 40], &p)
            .unwrap_err()
            .contains("different version"));
        assert!(apply(&old[..39], &p)
            .unwrap_err()
            .contains("different version"));
        let mut broken = p.clone();
        let n = broken.len();
        broken[n - 5] ^= 1; // the result checksum, with the patch checksum no longer fitting
        assert!(apply(&old, &broken).is_err());
    }

    #[test]
    fn bps_that_points_outside_is_an_error_not_a_panic() {
        let src = vec![1u8; 8];
        let mut p = b"BPS1".to_vec();
        p.extend(varint(8));
        p.extend(varint(20));
        p.extend(varint(0));
        p.extend(varint(19 << 2)); // SourceRead (command 0) of 20 bytes from an 8-byte file
        let dst = vec![1u8; 20];
        assert!(apply(&src, &seal(p, &src, &dst)).is_err());
    }
}
