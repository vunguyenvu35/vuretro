use crate::{bios, roms, scan};
use std::path::{Path, PathBuf};
use vuretro_core::model::{BiosStatus, GameEntry, ImportedBios, ImportedRom};
use vuretro_core::paths::Paths;
use vuretro_core::ports::Catalog;
use vuretro_core::registry::System;

/// The game collection and BIOS files on the local disk.
pub struct FsCatalog {
    paths: Paths,
}

impl FsCatalog {
    pub fn new(paths: Paths) -> Self {
        FsCatalog { paths }
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
}
