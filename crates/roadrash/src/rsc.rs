//! `.RSC`: a container of resources (`DATA/GLOBAL.RSC`, `BIKESPEC.RSC`, `FAMILIES.RSC`).
//!
//! What the files show (checked on all three of them):
//!
//! ```text
//! 0x00  "CRSR"                     magic
//! 0x04  u32, u32                   not understood yet (0x30 and a small number)
//! 0x0C  descriptor of the table:   tag "LBTR", u32 table offset, u32 table size, ...
//! table at that offset:
//!       tag "LBTR", u32 size, u32 entry count, u32 0
//!       then `count` entries of 32 bytes:
//!       tag (4 bytes, written backwards: "MINA" is ANIM), u32 id, u32 offset, u32 size,
//!       16 more bytes, all zero in the files seen
//! ```
//!
//! All numbers are little-endian. The bytes at an entry's offset are not read here yet.

/// One resource of the container.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// What kind of resource it is, as four letters in reading order (e.g. `ANIM`).
    pub tag: String,
    pub id: u32,
    pub offset: u32,
    pub size: u32,
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, String> {
    bytes
        .get(at..at + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .ok_or_else(|| format!("the file ends before byte {at}"))
}

/// The tag as it is written in the file is reversed: `MINA` reads as `ANIM`.
fn tag_at(bytes: &[u8], at: usize) -> Result<String, String> {
    let b = bytes
        .get(at..at + 4)
        .ok_or_else(|| format!("the file ends before byte {at}"))?;
    let letters: Vec<u8> = b.iter().rev().copied().collect();
    Ok(String::from_utf8_lossy(&letters).trim().to_string())
}

/// The list of resources in a container.
pub fn entries(bytes: &[u8]) -> Result<Vec<Entry>, String> {
    if bytes.get(..4) != Some(b"CRSR") {
        return Err("not a resource container (no CRSR at the start)".into());
    }
    if tag_at(bytes, 0x0C)? != "RTBL" {
        return Err("the table descriptor is not where it was in every file seen".into());
    }
    let table = u32_at(bytes, 0x10)? as usize;
    let table_size = u32_at(bytes, 0x14)? as usize;
    if tag_at(bytes, table)? != "RTBL" {
        return Err("no table at the place the descriptor gives".into());
    }
    let count = u32_at(bytes, table + 8)? as usize;
    let first = table + 16;
    if 16 + count * 32 > table_size {
        return Err(format!(
            "{count} entries do not fit a table of {table_size} bytes"
        ));
    }
    (0..count)
        .map(|i| {
            let at = first + i * 32;
            let entry = Entry {
                tag: tag_at(bytes, at)?,
                id: u32_at(bytes, at + 4)?,
                offset: u32_at(bytes, at + 8)?,
                size: u32_at(bytes, at + 12)?,
            };
            if entry.offset as usize + entry.size as usize > bytes.len() {
                return Err(format!("{} {} lies outside the file", entry.tag, entry.id));
            }
            Ok(entry)
        })
        .collect()
}

impl Entry {
    /// The bytes of this resource in the container's file.
    pub fn bytes<'a>(&self, file: &'a [u8]) -> &'a [u8] {
        &file[self.offset as usize..(self.offset + self.size) as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A container with one resource, built the way the files are.
    fn sample() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"CRSR");
        b.extend_from_slice(&[0x30, 0, 0, 0, 1, 0, 0, 0]);
        b.extend_from_slice(b"LBTR"); // descriptor at 0x0C
        b.extend_from_slice(&0x30u32.to_le_bytes()); // table at 0x30
        b.extend_from_slice(&0x30u32.to_le_bytes()); // 16 + 1 entry of 32
        b.resize(0x30, 0);
        b.extend_from_slice(b"LBTR");
        b.extend_from_slice(&0x28u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(b"MINA"); // entry
        b.extend_from_slice(&7u32.to_le_bytes());
        b.extend_from_slice(&0x60u32.to_le_bytes());
        b.extend_from_slice(&4u32.to_le_bytes());
        b.resize(0x60, 0);
        b.extend_from_slice(&[1, 2, 3, 4]);
        b
    }

    #[test]
    fn the_table_is_read_and_tags_come_out_in_reading_order() {
        let e = entries(&sample()).unwrap();
        assert_eq!(
            e,
            vec![Entry {
                tag: "ANIM".into(),
                id: 7,
                offset: 0x60,
                size: 4
            }]
        );
    }

    #[test]
    fn other_files_and_cut_files_are_errors_not_panics() {
        assert!(entries(b"nope").is_err());
        let s = sample();
        for cut in [4, 0x0D, 0x20, 0x44, 0x5F] {
            assert!(entries(&s[..cut]).is_err(), "cut at {cut}");
        }
    }

    #[test]
    fn an_entry_outside_the_file_is_refused() {
        let mut s = sample();
        s.truncate(0x62); // the resource now runs past the end
        assert!(entries(&s).is_err());
    }
}
