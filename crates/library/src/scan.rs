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

pub fn is_bios_file(name: &str) -> bool {
    SYSTEMS
        .iter()
        .flat_map(|s| s.bios.iter())
        .any(|b| b.file.eq_ignore_ascii_case(name))
}

fn alnum(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// A folder name denotes a system when it equals one of its aliases, or starts with
/// a long alias ignoring case and punctuation ("Neogeo Roms Complete", "Neo-Geo (MVS)").
fn system_for_dir(dir: &str) -> Option<&'static System> {
    let lower = dir.to_ascii_lowercase();
    if let Some(sys) = SYSTEMS.iter().find(|s| s.folders.contains(&lower.as_str())) {
        return Some(sys);
    }
    let flat = alnum(dir);
    SYSTEMS.iter().find(|s| {
        s.folders.iter().any(|f| {
            let f = alnum(f);
            f.len() >= 5 && flat.starts_with(&f)
        })
    })
}

pub fn detect_system(root: &Path, path: &Path) -> Option<&'static System> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    let rel = path.strip_prefix(root).unwrap_or(path);
    // The chosen folder's own name counts too (picking ".../NeoGeo" directly).
    let root_name = root.file_name().map(|n| n.to_string_lossy().into_owned());
    let dirs = root_name.into_iter().chain(
        rel.parent()
            .into_iter()
            .flat_map(|p| p.components())
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    for dir in dirs {
        if let Some(sys) = system_for_dir(&dir) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn id(root: &str, file: &str) -> Option<&'static str> {
        detect_system(Path::new(root), Path::new(file)).map(|s| s.id)
    }

    #[test]
    fn folder_names_are_matched_loosely() {
        let r = "/dl/NeoGeo/Neogeo Roms Complete_Retroiso";
        assert_eq!(id(r, &format!("{r}/aof.zip")), Some("neogeo"));
        assert_eq!(id("/roms", "/roms/Neo-Geo (MVS)/x.zip"), Some("neogeo"));
        assert_eq!(id("/roms", "/roms/PlayStation Games/x.cue"), Some("psx"));
        assert_eq!(id("/roms", "/roms/neogeo/x.zip"), Some("neogeo"));
        // the chosen folder's own name counts
        assert_eq!(id("/x/neogeo", "/x/neogeo/aof.zip"), Some("neogeo"));
        // short aliases need an exact match; archives elsewhere stay unknown
        assert_eq!(id("/roms", "/roms/Games/x.zip"), None);
        assert_eq!(id("/roms", "/roms/aes-collection/x.zip"), None);
    }
}
