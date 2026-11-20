//! Where the OS keeps per-user app data.

use directories::ProjectDirs;
use vuretro_core::paths::Paths;

/// The per-user data folder of the OS (`~/Library/Application Support/dev.vuvn.vuretro`, ...).
pub fn system_paths() -> Option<Paths> {
    ProjectDirs::from("dev", "vuvn", "vuretro").map(|d| Paths::new(d.data_dir()))
}
