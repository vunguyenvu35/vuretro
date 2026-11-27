//! Where the game's files come from: a folder that was unpacked, or the `.iso` itself. The
//! rest of the crate asks for a path like `DATA/PALETTE.RAW` and does not care which.

use crate::iso::Iso;
use std::path::{Path, PathBuf};

pub enum Disc {
    Folder(PathBuf),
    /// A disc image, and the folder of it that holds the game (`ROADRASH` on the retail disc,
    /// where the game sits beside its installer; empty when the game is at the top).
    Iso(Iso, String),
}

impl Disc {
    /// A folder, or a disc image (any file).
    pub fn open(path: &Path) -> Result<Disc, String> {
        if path.is_dir() {
            Ok(Disc::Folder(path.to_path_buf()))
        } else {
            let iso = Iso::open(path)?;
            let prefix = if iso.find("ROADRASH.EXE").is_ok() {
                ""
            } else if iso.find("ROADRASH/ROADRASH.EXE").is_ok() {
                "ROADRASH"
            } else {
                return Err("this disc image does not hold Road Rash (no ROADRASH.EXE)".into());
            };
            Ok(Disc::Iso(iso, prefix.to_string()))
        }
    }

    pub fn read(&self, path: &str) -> Result<Vec<u8>, String> {
        match self {
            Disc::Folder(root) => {
                std::fs::read(root.join(path)).map_err(|e| format!("{path}: {e}"))
            }
            Disc::Iso(iso, prefix) if prefix.is_empty() => iso.read(path),
            Disc::Iso(iso, prefix) => iso.read(&format!("{prefix}/{path}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_is_read_by_its_paths() {
        let dir = std::env::temp_dir().join(format!("vuretro-disc-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("DATA")).unwrap();
        std::fs::write(dir.join("DATA/X.BIN"), [1, 2]).unwrap();
        let disc = Disc::open(&dir).unwrap();
        assert_eq!(disc.read("DATA/X.BIN").unwrap(), vec![1, 2]);
        assert!(disc.read("DATA/NOPE").is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn something_that_is_neither_is_an_error() {
        assert!(Disc::open(Path::new("/nowhere")).is_err());
    }
}
