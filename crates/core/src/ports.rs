//! Ports: what the use cases need from the outside world. Adapters implement them
//! (`vuretro-library::adapters` for disk and processes); tests use fakes.

use crate::model::{
    BiosStatus, CoreOptionDef, GameEntry, GameMeta, ImportedBios, ImportedRom, MachineInfo,
    Settings, SlotInfo,
};
use crate::protocol::Event;
use crate::registry::System;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use vuretro_controls::{macros::MacroConfig, ControlsConfig};
use vuretro_settings::scoped::Overrides;
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
    /// Per-system and per-game choices layered over the general settings.
    fn load_overrides(&self) -> Overrides;
    fn save_overrides(&self, overrides: &Overrides) -> Result<(), String>;
    /// Key bindings of one system, if it has its own.
    fn load_system_controls(&self, system: &str) -> Option<ControlsConfig>;
    fn save_system_controls(&self, system: &str, controls: &ControlsConfig) -> Result<(), String>;
    /// Key bindings of one game, if it has its own.
    fn load_game_controls(&self, rom: &Path) -> Option<ControlsConfig>;
    fn save_game_controls(&self, rom: &Path, controls: &ControlsConfig) -> Result<(), String>;
    fn remove_game_controls(&self, rom: &Path) -> Result<(), String>;
    /// Go back to the general key bindings for this system.
    fn remove_system_controls(&self, system: &str) -> Result<(), String>;
    /// Window size, sound and the like, shared with the runner.
    fn load_game_settings(&self) -> GameSettings;
    fn save_game_settings(&self, settings: &GameSettings) -> Result<(), String>;

    /// The save slots of a game that hold a save, in slot order.
    fn list_slots(&self, rom: &Path) -> Vec<SlotInfo>;
    fn read_slot_thumb(&self, rom: &Path, slot: u8) -> Option<Vec<u8>>;

    /// Keep downloaded cover art for a game (it wins over the screenshot preview).
    fn save_art(&self, rom: &Path, png: &[u8]) -> Result<(), String>;
    fn art_exists(&self, rom: &Path) -> bool;
    /// Use an image file the user chose as the game's cover (PNG or JPEG, validated).
    fn import_art(&self, rom: &Path, src: &Path) -> Result<(), String>;
    /// Forget downloaded or chosen cover art.
    fn remove_art(&self, rom: &Path);
    /// Bytes of a game's preview image, if it has one.
    fn read_cover(&self, rom: &Path) -> Option<Vec<u8>>;
    fn cover_exists(&self, rom: &Path) -> bool;
    fn remove_cover(&self, rom: &Path);
    /// Delete the temporary files of a preview run.
    fn clear_cover_scratch(&self);
    /// Delete every screenshot preview (downloaded or chosen cover art is kept); returns how
    /// many were removed.
    fn remove_all_previews(&self) -> usize;
    /// The cheat file text of a game (empty if none).
    fn load_cheats(&self, rom: &Path) -> String;
    fn save_cheats(&self, rom: &Path, text: &str) -> Result<(), String>;
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
    /// BIOS files of known systems found in a folder (subfolders included), so an import of a
    /// whole collection can install them too.
    fn find_bios(&self, dir: &Path) -> Vec<PathBuf>;
    /// Copy one ROM into the app's library. `system` settles ambiguous archives.
    fn import_rom(&self, src: &Path, system: Option<&str>) -> Result<ImportedRom, String>;
    /// Copy every recognised ROM of a folder into the library.
    fn import_folder(&self, dir: &Path) -> Vec<(PathBuf, Result<ImportedRom, String>)>;
    /// Apply an IPS, UPS or BPS patch to a game and add the patched copy to the library.
    /// The original file is never changed.
    fn apply_patch(&self, rom: &Path, patch: &Path) -> Result<ImportedRom, String>;
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
    /// Save slot to load as the game starts.
    pub load_slot: Option<u8>,
    /// Id of the system, so the runner applies that system's settings and key bindings.
    pub system: String,
}

/// A program of its own to start (see [`crate::model::NativeGame`]).
#[derive(Debug, Clone)]
pub struct NativeSpec {
    pub exe: PathBuf,
    pub args: Vec<String>,
    /// The folder to start it in; the folder of the program when `None`.
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    /// A command (program and arguments, run with the same environment) that ends what the
    /// program started besides itself, for when the game is stopped from the app: Wine keeps a
    /// server running for the game.
    pub stop: Option<Vec<String>>,
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
    /// Block until it ends. Whatever it reported is available from `events` afterwards.
    fn wait(&mut self);
    /// End the game now (the user asked to stop it). A process that cannot be asked ignores it.
    fn stop(&mut self) {}
    /// What the game has reported since the last call (see [`crate::protocol`]), oldest first.
    /// A process that reports nothing gives an empty list.
    fn events(&mut self) -> Vec<Event> {
        Vec::new()
    }
}

/// Starts runner processes.
pub trait Launcher: Send + Sync {
    fn spawn(&self, spec: &LaunchSpec) -> Result<Box<dyn GameProcess>, String>;
    /// Start a program of its own, with no runner and no core.
    fn spawn_native(&self, _spec: &NativeSpec) -> Result<Box<dyn GameProcess>, String> {
        Err("this build cannot start native games".into())
    }
    /// Run a game headless and keep its best frame. True when the image exists afterwards.
    fn capture_cover(&self, job: &CoverJob) -> bool;
    /// Ask a core which options it offers. Some cores announce most of them only once a game
    /// is loaded, so a sample `rom` of the core's system can be given.
    fn list_core_options(
        &self,
        runner: &Path,
        core: &Path,
        rom: Option<&Path>,
    ) -> Result<Vec<CoreOptionDef>, String>;
}

/// Sets up a native game on this computer from its source (the Road Rash port): the tools it
/// needs, and the build. Everything slow reports what it is doing through `log`.
pub trait NativeInstaller: Send + Sync {
    /// The tools the build needs that this computer lacks (empty when it is ready).
    fn missing_tools(&self) -> Vec<String>;
    /// Install the missing tools, as far as this computer allows.
    fn install_tools(&self, log: &dyn Fn(&str)) -> Result<(), String>;
    /// Fetch the source, build it and import the game's data from `disc` (a disc image or the
    /// folder it was unpacked to). Afterwards the game is under `<data dir>/native/road-rash`
    /// with a `game.json`.
    fn install_road_rash(&self, disc: &Path, log: &dyn Fn(&str)) -> Result<(), String>;
    /// Whether the runtime for Windows games (Wine) is on this computer.
    fn pc_ready(&self) -> bool;
    /// Fetch the runtime for Windows games. `log` gets progress lines, `  40%` among them.
    fn install_pc_runtime(&self, log: &dyn Fn(&str)) -> Result<(), String>;
    /// Make a Windows game from a disc image ready to start (the first time: an environment
    /// for it and its files from the disc; later: nothing to do) and say how to start it.
    fn prepare_pc_game(&self, disc: &Path, log: &dyn Fn(&str)) -> Result<NativeSpec, String>;
    /// Run the installer of a Windows game's disc (the user follows it in its window) and make the
    /// program that it leaves the one that Play starts. Gives the name of that program.
    fn run_pc_installer(&self, disc: &Path, log: &dyn Fn(&str)) -> Result<String, String>;
}

/// Measures the computer the app runs on.
pub trait Machine: Send + Sync {
    fn info(&self) -> MachineInfo;
}

/// What to look up.
pub struct ArtQuery<'a> {
    /// Official game name (from a DAT file).
    pub name: &'a str,
    /// libretro-thumbnails folder of the system (empty: unknown).
    pub thumbnails: &'a str,
    /// ScreenScraper system id (0: unknown).
    pub screenscraper_id: u32,
    /// The user's ScreenScraper login, if they gave one.
    pub login: Option<(&'a str, &'a str)>,
}

/// A place cover art can be downloaded from.
pub trait ArtSource: Send + Sync {
    fn id(&self) -> &'static str;
    /// Box art (PNG or JPEG bytes); `Ok(None)` when this source has none or cannot be asked.
    fn boxart(&self, q: &ArtQuery) -> Result<Option<Vec<u8>>, String>;
}

/// Downloads cores into the app's own cores folder.
pub trait CoreInstaller: Send + Sync {
    /// Fetch and unpack core `stem` (e.g. `snes9x_libretro`). `progress` gets 0..=100.
    /// Returns the path of the installed file.
    fn install(&self, stem: &str, progress: &dyn Fn(u8)) -> Result<PathBuf, String>;
}

/// Backup files and exports written to a place the user chose.
pub trait Archive: Send + Sync {
    /// Zip the given data-folder entries into `dest`.
    fn create_backup(
        &self,
        dest: &Path,
        includes: &[&str],
        app_version: &str,
        created: u64,
    ) -> Result<crate::backup::BackupSummary, String>;
    /// Read a backup's manifest without restoring anything.
    fn inspect_backup(&self, src: &Path) -> Result<crate::backup::BackupManifest, String>;
    /// Write the backup's files into the data folder (only safe, allowed entries).
    fn restore_backup(&self, src: &Path) -> Result<crate::backup::BackupSummary, String>;
    fn write_text(&self, dest: &Path, text: &str) -> Result<(), String>;
}
