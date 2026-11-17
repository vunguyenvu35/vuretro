//! Walk a ROM folder and detect the system of each file.
//! Folder name wins (roms/neogeo/x.zip); otherwise a unique extension decides.

use crate::registry::{System, SYSTEMS};
use serde::Serialize;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GameEntry {
    pub system: String,
    pub title: String,
    pub path: PathBuf,
    pub size: u64,
}

const ARCHIVE_EXTS: &[&str] = &["zip", "7z"];

fn is_bios_file(name: &str) -> bool {
    SYSTEMS
        .iter()
        .flat_map(|s| s.bios.iter())
        .any(|b| b.file.eq_ignore_ascii_case(name))
}

pub fn detect_system(root: &Path, path: &Path) -> Option<&'static System> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let rel = path.strip_prefix(root).unwrap_or(path);
    for comp in rel.parent().into_iter().flat_map(|p| p.components()) {
        let dir = comp.as_os_str().to_string_lossy().to_ascii_lowercase();
        if let Some(sys) = SYSTEMS.iter().find(|s| s.folders.contains(&dir.as_str())) {
            return sys.extensions.contains(&ext.as_str()).then_some(sys);
        }
    }
    // Archives can hold any system, so they need a system folder.
    if ARCHIVE_EXTS.contains(&ext.as_str()) {
        return None;
    }
    let mut matches = SYSTEMS
        .iter()
        .filter(|s| s.extensions.contains(&ext.as_str()));
    match (matches.next(), matches.next()) {
        (Some(sys), None) => Some(sys),
        _ => None, // unknown or shared by several systems
    }
}

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
