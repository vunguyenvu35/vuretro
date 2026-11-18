use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use vuretro_controls::{macros, macros::MacroConfig, ControlsConfig};
use vuretro_core::model::{GameMeta, Settings};
use vuretro_core::paths::Paths;
use vuretro_core::ports::Store;
use vuretro_settings::GameSettings;

/// Keeps the app's data as JSON files (and preview images) under the data folder.
pub struct JsonStore {
    paths: Paths,
}

impl JsonStore {
    pub fn new(paths: Paths) -> Self {
        JsonStore { paths }
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(value) {
        let _ = std::fs::write(path, text);
    }
}

fn read_json<T: for<'a> Deserialize<'a> + Default>(path: &Path) -> T {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

impl Store for JsonStore {
    fn load_settings(&self) -> Settings {
        read_json(&self.paths.settings_file())
    }
    fn save_settings(&self, settings: &Settings) {
        write_json(&self.paths.settings_file(), settings);
    }
    fn load_meta(&self) -> HashMap<String, GameMeta> {
        read_json(&self.paths.library_file())
    }
    fn save_meta(&self, meta: &HashMap<String, GameMeta>) {
        write_json(&self.paths.library_file(), meta);
    }
    fn load_controls(&self) -> ControlsConfig {
        vuretro_controls::load(&self.paths.controls_file())
    }
    fn save_controls(&self, controls: &ControlsConfig) -> Result<(), String> {
        vuretro_controls::save(&self.paths.controls_file(), controls)
    }
    fn load_macros(&self) -> MacroConfig {
        macros::load(&self.paths.macros_file())
    }
    fn save_macros(&self, config: &MacroConfig) -> Result<(), String> {
        macros::save(&self.paths.macros_file(), config)
    }
    fn load_game_settings(&self) -> GameSettings {
        vuretro_settings::load(&self.paths.game_settings_file())
    }
    fn save_game_settings(&self, settings: &GameSettings) -> Result<(), String> {
        vuretro_settings::save(&self.paths.game_settings_file(), settings)
    }
    fn read_cover(&self, rom: &Path) -> Option<Vec<u8>> {
        std::fs::read(self.paths.cover_file(rom)).ok()
    }
    fn cover_exists(&self, rom: &Path) -> bool {
        self.paths.cover_file(rom).exists()
    }
    fn remove_cover(&self, rom: &Path) {
        let _ = std::fs::remove_file(self.paths.cover_file(rom));
    }
    fn clear_cover_scratch(&self) {
        let _ = std::fs::remove_dir_all(self.paths.cover_scratch_dir());
    }
    fn ensure_demo_rom(&self) -> Result<PathBuf, String> {
        let rom = self.paths.demo_rom();
        std::fs::create_dir_all(self.paths.data_dir()).map_err(|e| e.to_string())?;
        std::fs::write(&rom, b"test").map_err(|e| e.to_string())?;
        Ok(rom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_and_meta_round_trip_and_missing_files_give_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonStore::new(Paths::new(dir.path()));
        assert_eq!(store.load_settings(), Settings::default());
        assert!(store.load_meta().is_empty());

        let s = Settings {
            rom_dir: Some("/roms".into()),
            disabled: vec!["psx".into()],
            cores: [("nes".to_string(), "mesen_libretro".to_string())].into(),
        };
        store.save_settings(&s);
        assert_eq!(store.load_settings(), s);

        let mut m = HashMap::new();
        m.insert(
            "/a".to_string(),
            GameMeta {
                last_played: 1,
                play_secs: 2,
                favorite: true,
            },
        );
        store.save_meta(&m);
        assert_eq!(store.load_meta(), m);
    }

    #[test]
    fn broken_json_falls_back_to_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        std::fs::write(paths.settings_file(), "{ not json").unwrap();
        assert_eq!(JsonStore::new(paths).load_settings(), Settings::default());
    }
}
