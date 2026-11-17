//! Walk a ROM folder and list the games in it (system detection lives in `vuretro_core::detect`).

use std::path::Path;
use vuretro_core::detect::{detect_system, is_bios_file};
pub use vuretro_core::model::GameEntry;
use walkdir::WalkDir;

pub fn scan(root: &Path) -> Vec<GameEntry> {
    let mut games: Vec<GameEntry> = WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| {
            let path = e.path();
            let name = path.file_name()?.to_str()?;
            if name.starts_with('.') || is_bios_file(name) {
                return None;
            }
            let sys = detect_system(root, path)?;
            Some(GameEntry {
                system: sys.id.to_string(),
                title: path.file_stem()?.to_string_lossy().into_owned(),
                path: path.to_path_buf(),
                size: e.metadata().map(|m| m.len()).unwrap_or(0),
            })
        })
        .collect();
    games.sort_by(|a, b| (&a.system, &a.title).cmp(&(&b.system, &b.title)));
    games
}
