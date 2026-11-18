use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use vuretro_controls::{macros, macros::MacroConfig, ControlsConfig};
use vuretro_core::model::{GameMeta, Settings, SlotInfo, SAVE_SLOTS};
use vuretro_core::paths::Paths;
use vuretro_core::ports::Store;
use vuretro_settings::scoped::Overrides;
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
    fn load_overrides(&self) -> Overrides {
        vuretro_settings::scoped::load_overrides(&self.paths.overrides_file())
    }
    fn save_overrides(&self, overrides: &Overrides) -> Result<(), String> {
        vuretro_settings::scoped::save_overrides(&self.paths.overrides_file(), overrides)
    }
    fn load_system_controls(&self, system: &str) -> Option<ControlsConfig> {
        let path = self.paths.system_controls_file(system);
        path.is_file().then(|| vuretro_controls::load(&path))
    }
    fn save_system_controls(&self, system: &str, controls: &ControlsConfig) -> Result<(), String> {
        vuretro_controls::save(&self.paths.system_controls_file(system), controls)
    }
    fn remove_system_controls(&self, system: &str) -> Result<(), String> {
        match std::fs::remove_file(self.paths.system_controls_file(system)) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    }
    fn load_game_settings(&self) -> GameSettings {
        vuretro_settings::load(&self.paths.game_settings_file())
    }
    fn save_game_settings(&self, settings: &GameSettings) -> Result<(), String> {
        vuretro_settings::save(&self.paths.game_settings_file(), settings)
    }
    fn read_cover(&self, rom: &Path) -> Option<Vec<u8>> {
        std::fs::read(self.paths.art_file(rom))
            .or_else(|_| std::fs::read(self.paths.cover_file(rom)))
            .ok()
    }
    fn cover_exists(&self, rom: &Path) -> bool {
        self.paths.art_file(rom).exists() || self.paths.cover_file(rom).exists()
    }
    fn remove_cover(&self, rom: &Path) {
        let _ = std::fs::remove_file(self.paths.cover_file(rom));
        let _ = std::fs::remove_file(self.paths.art_file(rom));
    }
    fn save_art(&self, rom: &Path, png: &[u8]) -> Result<(), String> {
        let path = self.paths.art_file(rom);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, png).map_err(|e| format!("cannot write {}: {e}", path.display()))
    }
    fn art_exists(&self, rom: &Path) -> bool {
        self.paths.art_file(rom).exists()
    }
    fn import_art(&self, rom: &Path, src: &Path) -> Result<(), String> {
        let bytes =
            std::fs::read(src).map_err(|e| format!("cannot read {}: {e}", src.display()))?;
        if bytes.len() > vuretro_core::model::MAX_ART_BYTES {
            return Err("image is larger than 8 MB".into());
        }
        if vuretro_core::model::image_mime(&bytes).is_none() {
            return Err("not a PNG or JPEG image".into());
        }
        self.save_art(rom, &bytes)
    }
    fn remove_art(&self, rom: &Path) {
        let _ = std::fs::remove_file(self.paths.art_file(rom));
    }
    fn load_cheats(&self, rom: &Path) -> String {
        std::fs::read_to_string(self.paths.cheats_file(rom)).unwrap_or_default()
    }
    fn save_cheats(&self, rom: &Path, text: &str) -> Result<(), String> {
        let path = self.paths.cheats_file(rom);
        if text.trim().is_empty() {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                    Err(format!("cannot remove {}: {e}", path.display()))
                }
                _ => Ok(()),
            };
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
    }
    fn load_core_options(&self, stem: &str) -> BTreeMap<String, String> {
        std::fs::read_to_string(self.paths.core_options_file(stem))
            .map(|t| vuretro_settings::core_options::parse_config(&t))
            .unwrap_or_default()
    }

    fn save_core_options(
        &self,
        stem: &str,
        options: &BTreeMap<String, String>,
    ) -> Result<(), String> {
        let path = self.paths.core_options_file(stem);
        if options.is_empty() {
            return match std::fs::remove_file(&path) {
                Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                    Err(format!("cannot reset {}: {e}", path.display()))
                }
                _ => Ok(()),
            };
        }
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(
            &path,
            vuretro_settings::core_options::render_config(options),
        )
        .map_err(|e| format!("cannot write {}: {e}", path.display()))
    }

    fn list_slots(&self, rom: &Path) -> Vec<SlotInfo> {
        (1..=SAVE_SLOTS)
            .filter_map(|slot| {
                let modified = std::fs::metadata(self.paths.state_file(rom, slot))
                    .ok()?
                    .modified()
                    .ok()?;
                Some(SlotInfo {
                    slot,
                    saved_at: modified
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    has_thumb: self.paths.state_thumb(rom, slot).is_file(),
                })
            })
            .collect()
    }

    fn read_slot_thumb(&self, rom: &Path, slot: u8) -> Option<Vec<u8>> {
        if !(1..=SAVE_SLOTS).contains(&slot) {
            return None;
        }
        std::fs::read(self.paths.state_thumb(rom, slot)).ok()
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
    fn chosen_images_are_validated_and_win_over_the_screenshot() {
        let dir = tempfile::tempdir().unwrap();
        let store = JsonStore::new(Paths::new(dir.path().join("data")));
        let rom = Path::new("/roms/a.sfc");
        let good = dir.path().join("pic.jpg");
        std::fs::write(&good, [0xFF, 0xD8, 0xFF, 0xE0, 1, 2]).unwrap();
        let bad = dir.path().join("pic.png");
        std::fs::write(&bad, b"not an image").unwrap();

        assert!(store
            .import_art(rom, &bad)
            .unwrap_err()
            .contains("not a PNG or JPEG"));
        assert!(!store.art_exists(rom));
        store.import_art(rom, &good).unwrap();
        assert!(store.art_exists(rom));
        assert_eq!(store.read_cover(rom).unwrap()[..3], [0xFF, 0xD8, 0xFF]);
        store.remove_art(rom);
        assert!(store.read_cover(rom).is_none());
    }

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
            fetch_art: true,
            hardware_on: vec!["n64".into()],
            screenscraper: Some(vuretro_core::model::ScreenScraperLogin {
                user: "me".into(),
                password: "pw".into(),
            }),
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
                name: Some("Official".into()),
                tags: vec!["Favorites 2".into()],
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
