//! Identify ROMs by hash. A DAT file (No-Intro / Redump style XML) lists every known
//! dump with its official name, size and CRC32; hashing a ROM and looking it up gives the
//! right title and shows when two files are the same game.

use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

/// One dump listed in a DAT file.
#[derive(Debug, Clone, PartialEq)]
pub struct DatEntry {
    pub name: String,
    pub size: u64,
    pub crc: u32,
}

/// Files bigger than this are not hashed (disc images; the DAT route is for cartridges).
pub const MAX_HASH_BYTES: u64 = 128 * 1024 * 1024;

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Value of `key="..."` inside one tag.
fn attr<'a>(tag: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("{key}=\"");
    let mut from = 0;
    while let Some(i) = tag[from..].find(&needle) {
        let start = from + i;
        // must be a whole attribute name, not the tail of another one
        let ok = start == 0 || tag.as_bytes()[start - 1].is_ascii_whitespace();
        let v = start + needle.len();
        if ok {
            let end = tag[v..].find('"')?;
            return Some(&tag[v..v + end]);
        }
        from = v;
    }
    None
}

/// Read the games and their ROMs out of a DAT document. Tolerant: unknown tags are skipped
/// and entries without a usable CRC are left out.
pub fn parse_dat(text: &str) -> Vec<DatEntry> {
    let mut out = Vec::new();
    let mut game = String::new();
    let mut rest = text;
    while let Some(open) = rest.find('<') {
        let Some(close) = rest[open..].find('>') else {
            break;
        };
        let tag = &rest[open + 1..open + close];
        rest = &rest[open + close + 1..];
        if tag.starts_with("game ") || tag.starts_with("machine ") {
            game = attr(tag, "name").map(unescape).unwrap_or_default();
        } else if tag.starts_with("rom ") {
            let crc = attr(tag, "crc").and_then(|c| u32::from_str_radix(c.trim(), 16).ok());
            let size = attr(tag, "size").and_then(|s| s.trim().parse::<u64>().ok());
            if let (Some(crc), Some(size)) = (crc, size) {
                let name = if game.is_empty() {
                    attr(tag, "name").map(unescape).unwrap_or_default()
                } else {
                    game.clone()
                };
                out.push(DatEntry { name, size, crc });
            }
        }
    }
    out
}

/// Look-up table from `(crc, size)` to the official name.
#[derive(Default)]
pub struct DatIndex {
    by_hash: HashMap<(u32, u64), String>,
}

impl DatIndex {
    pub fn add(&mut self, entries: Vec<DatEntry>) {
        for e in entries {
            self.by_hash.entry((e.crc, e.size)).or_insert(e.name);
        }
    }

    pub fn len(&self) -> usize {
        self.by_hash.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_hash.is_empty()
    }

    /// Official name of a ROM given its bytes. Copier headers are ignored first, because
    /// DATs list the bare cartridge data.
    pub fn identify(&self, data: &[u8]) -> Option<&str> {
        let bare = strip_header(data);
        [bare, data].into_iter().find_map(|d| {
            let crc = crc32fast::hash(d);
            self.by_hash.get(&(crc, d.len() as u64)).map(String::as_str)
        })
    }
}

/// The ROM data without a known copier/emulator header (iNES, SMC, Lynx, 7800).
pub fn strip_header(data: &[u8]) -> &[u8] {
    if data.starts_with(b"NES\x1a") && data.len() > 16 {
        &data[16..]
    } else if data.starts_with(b"LYNX") && data.len() > 64 {
        &data[64..]
    } else if data.get(1..10) == Some(b"ATARI7800") && data.len() > 128 {
        &data[128..]
    } else if data.len() % 1024 == 512 {
        &data[512..] // SNES / Genesis copier header
    } else {
        data
    }
}

/// The bytes of a ROM: the file itself, or the one file inside a zip. `None` for archives
/// with several files (arcade sets), unreadable or oversized files.
pub fn read_rom_bytes(path: &Path) -> Option<Vec<u8>> {
    let is_zip = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("zip"));
    if is_zip {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(path).ok()?).ok()?;
        let files: Vec<usize> = (0..zip.len())
            .filter(|&i| zip.by_index(i).is_ok_and(|f| f.is_file()))
            .collect();
        if files.len() != 1 {
            return None;
        }
        let f = zip.by_index(files[0]).ok()?;
        if f.size() > MAX_HASH_BYTES {
            return None;
        }
        let mut buf = Vec::with_capacity(f.size() as usize);
        f.take(MAX_HASH_BYTES).read_to_end(&mut buf).ok()?;
        Some(buf)
    } else {
        if std::fs::metadata(path).ok()?.len() > MAX_HASH_BYTES {
            return None;
        }
        std::fs::read(path).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const DAT: &str = r#"<?xml version="1.0"?>
<datafile>
  <header><name>Nintendo - Super Nintendo</name></header>
  <game name="Super &amp; Test (USA)">
    <description>Super &amp; Test (USA)</description>
    <rom name="Super &amp; Test (USA).sfc" size="5" crc="3610A686" md5="x" sha1="y"/>
  </game>
  <game name="Broken (USA)">
    <rom name="b.sfc" size="3" crc="zzzz"/>
  </game>
  <machine name="pacman"><rom name="a.bin" size="4" crc="deadbeef"/></machine>
</datafile>"#;

    #[test]
    fn parses_games_with_entities_and_skips_bad_entries() {
        let e = parse_dat(DAT);
        assert_eq!(e.len(), 2);
        assert_eq!(
            e[0],
            DatEntry {
                name: "Super & Test (USA)".into(),
                size: 5,
                crc: 0x3610A686
            }
        );
        assert_eq!(e[1].name, "pacman");
        assert_eq!(e[1].crc, 0xdeadbeef);
    }

    #[test]
    fn attribute_lookup_matches_whole_names_only() {
        assert_eq!(attr(r#"rom name="a" size="1""#, "size"), Some("1"));
        assert_eq!(attr(r#"rom xsize="9" size="1""#, "size"), Some("1"));
        assert_eq!(attr(r#"rom name="a""#, "crc"), None);
    }

    #[test]
    fn identifies_by_crc_and_size_even_with_a_copier_header() {
        let mut idx = DatIndex::default();
        idx.add(parse_dat(DAT));
        assert_eq!(idx.len(), 2);
        // "hello" has CRC32 3610a686
        assert_eq!(idx.identify(b"hello"), Some("Super & Test (USA)"));
        assert_eq!(idx.identify(b"hellp"), None);
        // an iNES header in front is ignored
        let mut with_header = b"NES\x1a".to_vec();
        with_header.extend_from_slice(&[0; 12]);
        with_header.extend_from_slice(b"hello");
        assert_eq!(idx.identify(&with_header), Some("Super & Test (USA)"));
        // a 512-byte copier header too
        let mut smc = vec![0u8; 512];
        smc.extend_from_slice(b"hello");
        assert_eq!(
            idx.identify(&smc),
            None,
            "5 + 512 bytes is not a multiple of 1024 + 512"
        );
        let smc = vec![0u8; 1536]; // 1024 + 512: a copier header and 1 KiB of data
        assert_eq!(strip_header(&smc).len(), 1024);
    }

    #[test]
    fn reads_plain_files_and_single_file_zips_but_not_sets() {
        let dir = tempfile::tempdir().unwrap();
        let plain = dir.path().join("a.sfc");
        std::fs::write(&plain, b"hello").unwrap();
        assert_eq!(read_rom_bytes(&plain).unwrap(), b"hello");

        let make = |name: &str, files: &[&str]| {
            let p = dir.path().join(name);
            let mut w = zip::ZipWriter::new(std::fs::File::create(&p).unwrap());
            for f in files {
                w.start_file(*f, zip::write::SimpleFileOptions::default())
                    .unwrap();
                w.write_all(b"hello").unwrap();
            }
            w.finish().unwrap();
            p
        };
        assert_eq!(
            read_rom_bytes(&make("one.zip", &["a.sfc"])).unwrap(),
            b"hello"
        );
        assert!(read_rom_bytes(&make("set.zip", &["a.bin", "b.bin"])).is_none());
        assert!(read_rom_bytes(&dir.path().join("missing.sfc")).is_none());
    }
}
