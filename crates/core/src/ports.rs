//! Ports: what the use cases need from the outside world. Adapters implement them
//! (`vuretro-library::adapters` for disk and processes); tests use fakes.

use crate::model::{
    BiosStatus, CoreOptionDef, GameEntry, GameMeta, ImportedBios, ImportedRom, Settings, SlotInfo,
};
use crate::registry::System;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use vuretro_controls::{macros::MacroConfig, ControlsConfig};
use vuretro_settings::GameSettings;

/// Persistence of the app's own data.
pub trait Store: Send + Sync {
    fn load_settings(&self) -> Settings;
    fn save_settings(&self, settings: &Settings);
    fn load_meta(&self) -> HashMap<String, GameMeta>;
    fn save_meta(&self, meta: &HashMap<String, GameMeta>);

    /// Key bindings and devices. Missing or broken files give the defaults.
    fn load_controls(&self) -> ControlsConfig;
    fn save_controls(&self, controls: &ControlsConfig) -> Result<(), String>;
    fn load_macros(&self) -> MacroConfig;
    fn save_macros(&self, macros: &MacroConfig) -> Result<(), String>;
    /// Window size, sound and the like, shared with the runner.
    fn load_game_settings(&self) -> GameSettings;
    fn save_game_settings(&self, settings: &GameSettings) -> Result<(), String>;

    /// The save slots of a game that hold a save, in slot order.
    fn list_slots(&self, rom: &Path) -> Vec<SlotInfo>;
    fn read_slot_thumb(&self, rom: &Path, slot: u8) -> Option<Vec<u8>>;

    /// Keep downloaded cover art for a game (it wins over the screenshot preview).
    fn save_art(&self, rom: &Path, png: &[u8]) -> Result<(), String>;
    fn art_exists(&self, rom: &Path) -> bool;
    /// Bytes of a game's preview image, if it has one.
    fn read_cover(&self, rom: &Path) -> Option<Vec<u8>>;
    fn cover_exists(&self, rom: &Path) -> bool;
    fn remove_cover(&self, rom: &Path);
    /// Delete the temporary files of a preview run.
    fn clear_cover_scratch(&self);
    /// Option overrides saved for a core (`key = value` file shared with the runner).
    fn load_core_options(&self, stem: &str) -> BTreeMap<String, String>;
    fn save_core_options(
        &self,
        stem: &str,
        options: &BTreeMap<String, String>,
    ) -> Result<(), String>;
    /// Make the little file the fake core plays, and return its path.
    fn ensure_demo_rom(&self) -> Result<PathBuf, String>;
}

/// The game collection and BIOS files on disk.
pub trait Catalog: Send + Sync {
    /// Games found under `dir` (empty if it does not exist).
    fn scan(&self, dir: &Path) -> Vec<GameEntry>;
    fn bios_status(&self, system: &'static System) -> Vec<BiosStatus>;
    fn import_bios(&self, src: &Path) -> Result<ImportedBios, String>;
    fn is_folder(&self, path: &Path) -> bool;
    /// Copy one ROM into the app's library. `system` settles ambiguous archives.
    fn import_rom(&self, src: &Path, system: Option<&str>) -> Result<ImportedRom, String>;
    /// Copy every recognised ROM of a folder into the library.
    fn import_folder(&self, dir: &Path) -> Vec<(PathBuf, Result<ImportedRom, String>)>;
    /// Official name of a ROM according to the user's DAT files, if they know it.
    fn identify(&self, path: &Path) -> Option<String>;
    /// Add a DAT file; returns how many dumps it lists.
    fn import_dat(&self, src: &Path) -> Result<usize, String>;
    /// Dumps known from the DAT files added so far.
    fn dat_entries(&self) -> usize;
    /// Delete a ROM from the library (never anything outside it).
    fn remove_rom(&self, path: &Path) -> Result<(), String>;
}

/// Finds the executables and cores the app needs.
pub trait Locator: Send + Sync {
    fn runner(&self) -> Option<PathBuf>;
    fn core(&self, stem: &str) -> Option<PathBuf>;
    /// The fake core used for the demo.
    fn test_core(&self) -> Option<PathBuf>;
}

/// Everything needed to start one game.
#[derive(Debug, Clone)]
pub struct LaunchSpec {
    pub runner: PathBuf,
    pub core: PathBuf,
    pub rom: PathBuf,
    pub system_dir: PathBuf,
    /// Where the runner stores the game's preview image on exit.
    pub cover: PathBuf,
    /// Save slot to load as the game starts.
    pub load_slot: Option<u8>,
}

/// A request to make a preview image without opening a window.
#[derive(Debug, Clone)]
pub struct CoverJob {
    /// The game's key (its path), echoed back to the UI.
    pub key: String,
    pub runner: PathBuf,
    pub core: PathBuf,
    pub rom: PathBuf,
    pub system_dir: PathBuf,
    pub scratch_dir: PathBuf,
    pub out: PathBuf,
}

/// How a game process ended.
#[derive(Debug, Clone, PartialEq)]
pub struct Exit {
    pub success: bool,
    /// What to tell the user when it failed (the tail of the runner's log).
    pub detail: String,
}

/// A running game.
pub trait GameProcess: Send {
    /// `Some` once the process has ended.
    fn poll(&mut self) -> Result<Option<Exit>, String>;
    /// Block until it ends.
    fn wait(&mut self);
}

/// Starts runner processes.
pub trait Launcher: Send + Sync {
    fn spawn(&self, spec: &LaunchSpec) -> Result<Box<dyn GameProcess>, String>;
    /// Run a game headless and keep its best frame. True when the image exists afterwards.
    fn capture_cover(&self, job: &CoverJob) -> bool;
    /// Ask a core which options it offers (loads it without a game).
    fn list_core_options(&self, runner: &Path, core: &Path) -> Result<Vec<CoreOptionDef>, String>;
}

/// Where cover art comes from (libretro-thumbnails).
pub trait ArtSource: Send + Sync {
    /// Box art PNG for `name` on `system_folder`; `Ok(None)` when the server has none.
    fn boxart(&self, system_folder: &str, name: &str) -> Result<Option<Vec<u8>>, String>;
}

/// Downloads cores into the app's own cores folder.
pub trait CoreInstaller: Send + Sync {
    /// Fetch and unpack core `stem` (e.g. `snes9x_libretro`). `progress` gets 0..=100.
    /// Returns the path of the installed file.
    fn install(&self, stem: &str, progress: &dyn Fn(u8)) -> Result<PathBuf, String>;
}
