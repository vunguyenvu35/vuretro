//! A disc image in the ISO 9660 format, read in place: the files are found and read in the
//! image itself, nothing is unpacked first.
//!
//! The layout, as the standard has it: sectors of 2048 bytes; at sector 16 the volume
//! descriptor (`01 "CD001" 01`) holds at byte 156 the record of the root folder. A folder is a
//! run of records: a length byte, an extended-attribute length, the first sector and the size
//! (each written twice, little-endian then big-endian), seven bytes of date, the flags (bit 1
//! is a folder), and at byte 32 the length of the name and then the name. A record never
//! crosses a sector; a zero length means the rest of the sector is unused. Names end with a
//! version (`NAME.EXT;1`), which is dropped here. Long (Joliet) names are not used.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

const SECTOR: u64 = 2048;

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub sector: u32,
    pub size: u32,
    pub folder: bool,
}

pub struct Iso {
    file: Mutex<File>,
    length: u64,
    root: Entry,
}

fn le32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

impl Iso {
    pub fn open(path: &Path) -> Result<Iso, String> {
        let mut file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let length = file.metadata().map_err(|e| e.to_string())?.len();
        let mut descriptor = [0u8; 2048];
        file.seek(SeekFrom::Start(16 * SECTOR))
            .and_then(|_| file.read_exact(&mut descriptor))
            .map_err(|_| "the file is too short to be a disc image")?;
        if descriptor[0] != 1 || &descriptor[1..6] != b"CD001" {
            return Err("not an ISO 9660 disc image".into());
        }
        let root = &descriptor[156..190];
        Ok(Iso {
            file: Mutex::new(file),
            length,
            root: Entry {
                name: String::new(),
                sector: le32(root, 2),
                size: le32(root, 10),
                folder: true,
            },
        })
    }

    fn bytes(&self, sector: u32, size: u32) -> Result<Vec<u8>, String> {
        let start = sector as u64 * SECTOR;
        if start + size as u64 > self.length {
            return Err("the disc image ends before the data does".into());
        }
        let mut data = vec![0u8; size as usize];
        let mut file = self.file.lock().map_err(|_| "the disc image is busy")?;
        file.seek(SeekFrom::Start(start))
            .and_then(|_| file.read_exact(&mut data))
            .map_err(|e| e.to_string())?;
        Ok(data)
    }

    /// What a folder holds.
    pub fn list(&self, folder: &Entry) -> Result<Vec<Entry>, String> {
        if !folder.folder {
            return Err(format!("{} is not a folder", folder.name));
        }
        let data = self.bytes(folder.sector, folder.size)?;
        let mut entries = Vec::new();
        let mut at = 0usize;
        while at < data.len() {
            let len = data[at] as usize;
            if len == 0 {
                at = (at / SECTOR as usize + 1) * SECTOR as usize; // the rest of the sector
                continue;
            }
            if len < 34 || at + len > data.len() {
                return Err("a folder record is cut or too short".into());
            }
            let name_len = data[at + 32] as usize;
            if 33 + name_len > len {
                return Err("a name runs past its record".into());
            }
            let raw = &data[at + 33..at + 33 + name_len];
            // the records for "." and ".." have the names 0 and 1
            if raw != [0] && raw != [1] {
                let name = String::from_utf8_lossy(raw).into_owned();
                let name = name
                    .split(';')
                    .next()
                    .unwrap_or("")
                    .trim_end_matches('.')
                    .to_string();
                entries.push(Entry {
                    name,
                    sector: le32(&data, at + 2),
                    size: le32(&data, at + 10),
                    folder: data[at + 25] & 2 != 0,
                });
            }
            at += len;
        }
        Ok(entries)
    }

    /// The file or folder at `path` (names separated by `/`, upper or lower case).
    pub fn find(&self, path: &str) -> Result<Entry, String> {
        let mut here = self.root.clone();
        for part in path.split('/').filter(|p| !p.is_empty()) {
            here = self
                .list(&here)?
                .into_iter()
                .find(|e| e.name.eq_ignore_ascii_case(part))
                .ok_or_else(|| format!("{path} is not on the disc"))?;
        }
        Ok(here)
    }

    /// The contents of the file at `path`.
    pub fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        let entry = self.find(path)?;
        if entry.folder {
            return Err(format!("{path} is a folder"));
        }
        self.bytes(entry.sector, entry.size)
    }

    /// Every file under a folder (or on the whole disc with `""`), as paths from that folder.
    pub fn files_under(&self, folder: &str) -> Result<Vec<String>, String> {
        fn walk(
            iso: &Iso,
            folder: &Entry,
            prefix: &str,
            out: &mut Vec<String>,
        ) -> Result<(), String> {
            for e in iso.list(folder)? {
                let path = if prefix.is_empty() {
                    e.name.clone()
                } else {
                    format!("{prefix}/{}", e.name)
                };
                if e.folder {
                    walk(iso, &e, &path, out)?;
                } else {
                    out.push(path);
                }
            }
            Ok(())
        }
        let mut out = Vec::new();
        walk(self, &self.find(folder)?, "", &mut out)?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(name: &[u8], sector: u32, size: u32, folder: bool) -> Vec<u8> {
        let mut r = vec![0u8; 33 + name.len()];
        r[2..6].copy_from_slice(&sector.to_le_bytes());
        r[6..10].copy_from_slice(&sector.to_be_bytes());
        r[10..14].copy_from_slice(&size.to_le_bytes());
        r[14..18].copy_from_slice(&size.to_be_bytes());
        r[25] = if folder { 2 } else { 0 };
        r[32] = name.len() as u8;
        r[33..].copy_from_slice(name);
        if r.len() % 2 == 1 {
            r.push(0);
        }
        r[0] = r.len() as u8;
        r
    }

    /// A small disc: `A.TXT;1` and `data/b.bin;1` (names in lower case on the disc).
    fn image(path: &Path) {
        let mut disc = vec![0u8; 22 * 2048];
        let root = [
            record(&[0], 18, 2048, true),
            record(&[1], 18, 2048, true),
            record(b"A.TXT;1", 20, 5, false),
            record(b"data", 19, 2048, true),
        ]
        .concat();
        let data = [
            record(&[0], 19, 2048, true),
            record(&[1], 18, 2048, true),
            record(b"b.bin;1", 21, 3, false),
        ]
        .concat();
        disc[16 * 2048] = 1;
        disc[16 * 2048 + 1..16 * 2048 + 6].copy_from_slice(b"CD001");
        disc[16 * 2048 + 156..16 * 2048 + 156 + 34]
            .copy_from_slice(&record(&[0], 18, 2048, true)[..34]);
        disc[18 * 2048..18 * 2048 + root.len()].copy_from_slice(&root);
        disc[19 * 2048..19 * 2048 + data.len()].copy_from_slice(&data);
        disc[20 * 2048..20 * 2048 + 5].copy_from_slice(b"hello");
        disc[21 * 2048..21 * 2048 + 3].copy_from_slice(&[7, 8, 9]);
        std::fs::write(path, disc).unwrap();
    }

    fn temp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("vuretro-iso-{}-{name}", std::process::id()))
    }

    #[test]
    fn files_are_found_by_path_in_any_case_and_read() {
        let path = temp("a.iso");
        image(&path);
        let iso = Iso::open(&path).unwrap();
        assert_eq!(iso.read("a.txt").unwrap(), b"hello");
        assert_eq!(iso.read("DATA/B.BIN").unwrap(), vec![7, 8, 9]);
        assert_eq!(iso.files_under("").unwrap(), vec!["A.TXT", "data/b.bin"]);
        assert_eq!(iso.files_under("data").unwrap(), vec!["b.bin"]);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn missing_things_and_other_files_are_errors() {
        let path = temp("b.iso");
        image(&path);
        let iso = Iso::open(&path).unwrap();
        assert!(iso.read("nope.txt").is_err());
        assert!(iso.read("data").is_err(), "a folder is not a file");
        assert!(iso.find("a.txt/x").is_err(), "a file is not a folder");
        std::fs::write(&path, vec![0u8; 100]).unwrap();
        assert!(Iso::open(&path).is_err(), "too short");
        std::fs::write(&path, vec![0u8; 20 * 2048]).unwrap();
        assert!(Iso::open(&path).is_err(), "no CD001");
        assert!(Iso::open(Path::new("/nowhere.iso")).is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn a_cut_image_is_an_error_not_a_panic() {
        let path = temp("c.iso");
        image(&path);
        let whole = std::fs::read(&path).unwrap();
        std::fs::write(&path, &whole[..20 * 2048 + 2]).unwrap(); // A.TXT is cut
        let iso = Iso::open(&path).unwrap();
        assert!(iso.read("a.txt").is_err());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn an_image_whose_size_is_not_a_whole_number_of_sectors_still_opens() {
        let path = temp("d.iso");
        image(&path);
        let mut whole = std::fs::read(&path).unwrap();
        whole.extend_from_slice(&[0u8; 156]); // as on the disc of the game
        std::fs::write(&path, whole).unwrap();
        assert_eq!(Iso::open(&path).unwrap().read("a.txt").unwrap(), b"hello");
        std::fs::remove_file(path).unwrap();
    }
}
