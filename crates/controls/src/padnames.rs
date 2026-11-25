//! Names the user gives to gamepads (`<data dir>/config/pad_names.json`): "8BitDo Pro 2" for
//! a pad the system calls "Nintendo Switch Pro Controller". Keyed by [`PadInfo::id`], so a name
//! belongs to the model: two identical pads share one (they cannot be told apart anyway).

use crate::PadInfo;
use std::collections::BTreeMap;
use std::path::Path;

/// Pad id -> the name the user chose.
pub type PadNames = BTreeMap<String, String>;

/// The longest name kept.
pub const MAX_LEN: usize = 40;

/// A name as it will be stored: trimmed, on one line, at most [`MAX_LEN`] characters.
/// `None` for an empty one, which means "use the system's name".
pub fn clean(name: &str) -> Option<String> {
    let one_line = name.split_whitespace().collect::<Vec<_>>().join(" ");
    let name: String = one_line.chars().take(MAX_LEN).collect();
    let name = name.trim().to_string();
    (!name.is_empty()).then_some(name)
}

/// Set (or, for an empty name, clear) the name of a pad.
pub fn set(names: &mut PadNames, id: &str, name: &str) {
    match clean(name) {
        Some(n) => {
            names.insert(id.to_string(), n);
        }
        None => {
            names.remove(id);
        }
    }
}

/// The pads with the user's names in place of the system's.
pub fn apply(pads: &[PadInfo], names: &PadNames) -> Vec<PadInfo> {
    pads.iter()
        .map(|p| PadInfo {
            id: p.id.clone(),
            name: names.get(&p.id).cloned().unwrap_or_else(|| p.name.clone()),
        })
        .collect()
}

pub fn load(path: &Path) -> PadNames {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<PadNames>(&t).ok())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(id, n)| clean(&n).map(|n| (id, n)))
        .collect()
}

pub fn save(path: &Path, names: &PadNames) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(names).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pad(id: &str, name: &str) -> PadInfo {
        PadInfo {
            id: id.into(),
            name: name.into(),
        }
    }

    #[test]
    fn a_name_is_trimmed_flattened_and_cut() {
        assert_eq!(clean("  8BitDo \n Pro 2 "), Some("8BitDo Pro 2".into()));
        assert_eq!(clean("   "), None);
        assert_eq!(clean(&"x".repeat(100)).unwrap().chars().count(), MAX_LEN);
        assert_eq!(clean("ổ đĩa tay cầm").unwrap(), "ổ đĩa tay cầm");
    }

    #[test]
    fn naming_a_pad_replaces_its_name_and_an_empty_one_clears_it() {
        let pads = [pad("a", "Pro Controller"), pad("b", "Xbox")];
        let mut names = PadNames::new();
        set(&mut names, "a", "8BitDo Pro 2");
        let shown = apply(&pads, &names);
        assert_eq!(shown[0].name, "8BitDo Pro 2");
        assert_eq!(shown[0].id, "a", "the id never changes");
        assert_eq!(shown[1].name, "Xbox");
        set(&mut names, "a", "  ");
        assert_eq!(apply(&pads, &names)[0].name, "Pro Controller");
        assert!(names.is_empty());
    }

    #[test]
    fn names_survive_a_round_trip_and_a_broken_file_gives_none() {
        let dir = std::env::temp_dir().join(format!("vuretro-padnames-{}", std::process::id()));
        let path = dir.join("config").join("pad_names.json");
        let mut names = PadNames::new();
        set(&mut names, "a", "8BitDo");
        save(&path, &names).unwrap();
        assert_eq!(load(&path), names);
        std::fs::write(&path, "{ not json").unwrap();
        assert!(load(&path).is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }
}
