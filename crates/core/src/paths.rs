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
    /// DAT files the user added (official names of known dumps).
    pub fn dats_dir(&self) -> PathBuf {
        self.data_dir.join("dats")
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
    /// Key bindings of one game (`config/controls/games/<file name>.json`), the most
    /// specific layer: used before the system's and the general ones.
    pub fn game_controls_file(&self, rom: &Path) -> PathBuf {
        let stem: String = rom
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || " .-_()".contains(c) {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.config_dir()
            .join("controls")
            .join("games")
            .join(format!("{stem}.json"))
    }
    /// Per-system and per-game overrides of the game settings.
    pub fn overrides_file(&self) -> PathBuf {
        self.config_dir().join("overrides.json")
    }
    /// Key bindings of one system (`config/controls/<system>.json`); used instead of the
    /// general ones when it exists. The id is reduced to safe characters.
    pub fn system_controls_file(&self, system: &str) -> PathBuf {
        let safe: String = system
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        self.config_dir()
            .join("controls")
            .join(format!("{safe}.json"))
    }
    pub fn game_settings_file(&self) -> PathBuf {
        self.config_dir().join("game.json")
    }
    /// Core options file for a core (`<stem>.cfg`).
    pub fn core_options_file(&self, core_stem: &str) -> PathBuf {
        self.config_dir().join(format!("{core_stem}.cfg"))
    }
    /// Cheat codes of a game (`config/cheats/<rom name>.cht`).
    pub fn cheats_file(&self, rom: &Path) -> PathBuf {
        let stem = rom
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.config_dir().join("cheats").join(format!("{stem}.cht"))
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

    /// Save-state file of a game's slot, as the runner writes it.
    pub fn state_file(&self, rom: &Path, slot: u8) -> PathBuf {
        let stem = rom
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.saves_dir().join(format!("{stem}.state{slot}"))
    }

    /// The picture taken when the slot was saved.
    pub fn state_thumb(&self, rom: &Path, slot: u8) -> PathBuf {
        let mut p = self.state_file(rom, slot).into_os_string();
        p.push(".png");
        PathBuf::from(p)
    }

    /// Downloaded box art of a game.
    pub fn art_file(&self, rom: &Path) -> PathBuf {
        let mut p = self.cover_file(rom).into_os_string();
        p.push(".art");
        PathBuf::from(p)
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
    fn override_files_live_under_config_and_system_ids_are_made_safe() {
        let p = Paths::new("/d");
        assert_eq!(
            p.overrides_file(),
            PathBuf::from("/d/config/overrides.json")
        );
        assert_eq!(
            p.system_controls_file("snes"),
            PathBuf::from("/d/config/controls/snes.json")
        );
        assert_eq!(
            p.system_controls_file("../../etc"),
            PathBuf::from("/d/config/controls/etc.json")
        );
    }

    #[test]
    fn game_key_bindings_are_named_after_the_game_file_and_made_safe() {
        let p = Paths::new("/d");
        assert_eq!(
            p.game_controls_file(Path::new("/roms/Super Game (U).sfc")),
            PathBuf::from("/d/config/controls/games/Super Game (U).json")
        );
        assert_eq!(
            p.game_controls_file(Path::new("/roms/a:b/c*d.nes")),
            PathBuf::from("/d/config/controls/games/c_d.json")
        );
    }

    #[test]
    fn slot_files_match_the_runner_naming() {
        let p = Paths::new("/d");
        let rom = Path::new("/roms/Super Game (U).sfc");
        assert_eq!(
            p.state_file(rom, 2),
            Path::new("/d/saves/Super Game (U).state2")
        );
        assert_eq!(
            p.state_thumb(rom, 2),
            Path::new("/d/saves/Super Game (U).state2.png")
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
