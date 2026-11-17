//! Where vuretro keeps its files. Only builds paths; creating folders is the adapters' job.

use directories::ProjectDirs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Paths {
    data_dir: PathBuf,
}

impl Paths {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Paths {
            data_dir: data_dir.into(),
        }
    }

    /// The per-user data folder of the OS (`~/Library/Application Support/dev.vuvn.vuretro`, ...).
    pub fn system_default() -> Option<Self> {
        ProjectDirs::from("dev", "vuvn", "vuretro").map(|d| Paths::new(d.data_dir()))
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
    /// BIOS files.
    pub fn system_dir(&self) -> PathBuf {
        self.data_dir.join("system")
    }
    /// Save RAM and save states.
    pub fn saves_dir(&self) -> PathBuf {
        self.data_dir.join("saves")
    }
    /// ROMs the user imported into the app.
    pub fn roms_dir(&self) -> PathBuf {
        self.data_dir.join("roms")
    }
    /// Cores installed by the user.
    pub fn cores_dir(&self) -> PathBuf {
        self.data_dir.join("cores")
    }
    pub fn screenshots_dir(&self) -> PathBuf {
        self.data_dir.join("screenshots")
    }
    pub fn config_dir(&self) -> PathBuf {
        self.data_dir.join("config")
    }
    pub fn controls_file(&self) -> PathBuf {
        self.config_dir().join("controls.json")
    }
    pub fn macros_file(&self) -> PathBuf {
        self.config_dir().join("macros.json")
    }
    pub fn game_settings_file(&self) -> PathBuf {
        self.config_dir().join("game.json")
    }
    /// Core options file for a core (`<stem>.cfg`).
    pub fn core_options_file(&self, core_stem: &str) -> PathBuf {
        self.config_dir().join(format!("{core_stem}.cfg"))
    }
    /// App settings (linked ROM folder, switched-off systems).
    pub fn settings_file(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }
    /// Per-game bookkeeping (last played, play time, favorites).
    pub fn library_file(&self) -> PathBuf {
        self.data_dir.join("library.json")
    }
    pub fn runner_log(&self) -> PathBuf {
        self.data_dir.join("logs/runner.log")
    }
    pub fn demo_rom(&self) -> PathBuf {
        self.data_dir.join("demo.test")
    }
    pub fn cover_scratch_dir(&self) -> PathBuf {
        self.data_dir.join("cover-scratch")
    }

    /// Preview image of a game (a screenshot taken from the game itself).
    pub fn cover_file(&self, rom: &Path) -> PathBuf {
        let name: String = rom
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || ".-_".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.data_dir.join("covers").join(format!("{name}.png"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_names_are_filesystem_safe() {
        let p = Paths::new("/d");
        assert_eq!(
            p.cover_file(Path::new("/x/Metal Slug (U).zip")),
            PathBuf::from("/d/covers/Metal_Slug__U_.zip.png")
        );
    }

    #[test]
    fn config_files_live_under_config() {
        let p = Paths::new("/d");
        assert_eq!(p.controls_file(), PathBuf::from("/d/config/controls.json"));
        assert_eq!(
            p.core_options_file("snes9x_libretro"),
            PathBuf::from("/d/config/snes9x_libretro.cfg")
        );
    }
}
