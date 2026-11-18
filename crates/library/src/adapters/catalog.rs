use crate::dat::{self, DatIndex};
use crate::{bios, roms, scan};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use vuretro_core::model::{BiosStatus, GameEntry, ImportedBios, ImportedRom};
use vuretro_core::paths::Paths;
use vuretro_core::ports::Catalog;
use vuretro_core::registry::System;

/// The game collection and BIOS files on the local disk.
pub struct FsCatalog {
    paths: Paths,
    /// The DAT files in `<data dir>/dats`, read on first use and after a DAT is added.
    dats: Mutex<Option<DatIndex>>,
}

impl FsCatalog {
    pub fn new(paths: Paths) -> Self {
        FsCatalog {
            paths,
            dats: Mutex::new(None),
        }
    }

    fn with_index<T>(&self, f: impl FnOnce(&DatIndex) -> T) -> T {
        let mut slot = self.dats.lock().unwrap();
        let index = slot.get_or_insert_with(|| {
            let mut index = DatIndex::default();
            if let Ok(dir) = std::fs::read_dir(self.paths.dats_dir()) {
                for file in dir.flatten() {
                    if let Ok(text) = std::fs::read_to_string(file.path()) {
                        index.add(dat::parse_dat(&text));
                    }
                }
            }
            index
        });
        f(index)
    }
}

impl Catalog for FsCatalog {
    fn scan(&self, dir: &Path) -> Vec<GameEntry> {
        scan::scan(dir)
    }
    fn bios_status(&self, system: &'static System) -> Vec<BiosStatus> {
        bios::check(system, &self.paths.system_dir())
    }
    fn import_bios(&self, src: &Path) -> Result<ImportedBios, String> {
        bios::import(src, &self.paths.system_dir())
    }
    fn is_folder(&self, path: &Path) -> bool {
        path.is_dir()
    }
    fn import_rom(&self, src: &Path, system: Option<&str>) -> Result<ImportedRom, String> {
        roms::import_file(src, &self.paths.roms_dir(), system)
    }
    fn import_folder(&self, dir: &Path) -> Vec<(PathBuf, Result<ImportedRom, String>)> {
        roms::import_folder(dir, &self.paths.roms_dir())
    }
    fn remove_rom(&self, path: &Path) -> Result<(), String> {
        roms::remove(&self.paths.roms_dir(), path)
    }

    fn identify(&self, path: &Path) -> Option<String> {
        let bytes = dat::read_rom_bytes(path)?;
        self.with_index(|idx| idx.identify(&bytes).map(str::to_string))
    }

    fn import_dat(&self, src: &Path) -> Result<usize, String> {
        let text = std::fs::read_to_string(src)
            .map_err(|e| format!("cannot read {}: {e}", src.display()))?;
        let n = dat::parse_dat(&text).len();
        if n == 0 {
            return Err("no ROM entries found: is this a No-Intro / Redump DAT (XML)?".into());
        }
        let dir = self.paths.dats_dir();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let name = src
            .file_name()
            .ok_or("not a file")?
            .to_string_lossy()
            .into_owned();
        std::fs::write(dir.join(name), text).map_err(|e| e.to_string())?;
        *self.dats.lock().unwrap() = None; // read again next time
        Ok(n)
    }

    fn dat_entries(&self) -> usize {
        self.with_index(DatIndex::len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dat_names_games_by_hash_and_empty_ones_are_refused() {
        let dir = tempfile::tempdir().unwrap();
        let cat = FsCatalog::new(Paths::new(dir.path().join("data")));
        assert_eq!(cat.dat_entries(), 0);

        let bad = dir.path().join("bad.dat");
        std::fs::write(&bad, "<datafile/>").unwrap();
        assert!(cat.import_dat(&bad).unwrap_err().contains("no ROM entries"));

        let dat = dir.path().join("snes.dat");
        std::fs::write(
            &dat,
            r#"<datafile><game name="Hello Game (World)"><rom name="h.sfc" size="5" crc="3610a686"/></game></datafile>"#,
        )
        .unwrap();
        assert_eq!(cat.import_dat(&dat).unwrap(), 1);
        assert_eq!(cat.dat_entries(), 1);

        let rom = dir.path().join("weird-name.sfc");
        std::fs::write(&rom, b"hello").unwrap();
        assert_eq!(cat.identify(&rom).as_deref(), Some("Hello Game (World)"));
        std::fs::write(&rom, b"other").unwrap();
        assert_eq!(cat.identify(&rom), None);

        // a new catalog (app restart) reads the saved DAT from disk
        let again = FsCatalog::new(Paths::new(dir.path().join("data")));
        assert_eq!(again.dat_entries(), 1);
    }
}
