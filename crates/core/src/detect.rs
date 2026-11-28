//! Decide which system a ROM file belongs to from its path. Pure path logic, no disk access.
//! Folder name wins (roms/neogeo/x.zip); otherwise a unique extension decides.

use crate::registry::{systems, System};
use std::path::Path;

const ARCHIVE_EXTS: &[&str] = &["zip", "7z"];

/// True for file names that are BIOS files of a known system (not games).
pub fn is_bios_file(name: &str) -> bool {
    systems().iter().flat_map(|s| s.bios.iter()).any(|b| {
        b.file
            .rsplit('/')
            .next()
            .unwrap_or(b.file)
            .eq_ignore_ascii_case(name)
    })
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
    if let Some(sys) = systems()
        .iter()
        .find(|s| s.folders.contains(&lower.as_str()))
    {
        return Some(sys);
    }
    let flat = alnum(dir);
    systems().iter().find(|s| {
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
    let mut matches = systems()
        .iter()
        .filter(|s| !s.folder_only && s.extensions.contains(&ext.as_str()));
    match (matches.next(), matches.next()) {
        (Some(sys), None) => Some(sys),
        _ => None, // unknown or shared by several systems
    }
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

    #[test]
    fn unique_extension_decides_without_a_folder() {
        assert_eq!(id("/roms", "/roms/crash.chd"), Some("psx"));
        assert_eq!(id("/roms", "/roms/metroid.gba"), Some("gba"));
        assert_eq!(id("/roms", "/roms/notes.txt"), None);
    }

    #[test]
    fn new_systems_are_detected() {
        assert_eq!(id("/roms", "/roms/mario.nes"), Some("nes"));
        assert_eq!(id("/roms", "/roms/tetris.gb"), Some("gb"));
        assert_eq!(id("/roms", "/roms/zelda.gbc"), Some("gbc"));
        assert_eq!(id("/roms", "/roms/sonic.md"), Some("genesis"));
        assert_eq!(id("/roms", "/roms/alex.sms"), Some("sms"));
        assert_eq!(id("/roms", "/roms/sonic.gg"), Some("gg"));
        assert_eq!(id("/roms", "/roms/r-type.pce"), Some("pce"));
        // zip needs a folder; the folder decides between systems sharing a core
        assert_eq!(id("/roms", "/roms/Mega Drive/sonic.zip"), Some("genesis"));
        assert_eq!(id("/roms", "/roms/mastersystem/alex.zip"), Some("sms"));
        assert_eq!(id("/roms", "/roms/Games/sonic.zip"), None);
    }

    #[test]
    fn wave_two_systems_are_detected() {
        for (file, sys) in [
            ("a.a26", "atari2600"),
            ("a.a78", "atari7800"),
            ("a.lnx", "lynx"),
            ("a.ngp", "ngp"),
            ("a.ngc", "ngp"),
            ("a.ws", "wonderswan"),
            ("a.wsc", "wonderswan"),
            ("a.vb", "virtualboy"),
            ("a.32x", "sega32x"),
            ("a.nds", "nds"),
        ] {
            assert_eq!(id("/roms", &format!("/roms/{file}")), Some(sys), "{file}");
        }
        // zip sets need a folder, and "arcade" does not clash with the console folders
        assert_eq!(id("/roms", "/roms/mame/pacman.zip"), Some("arcade"));
        assert_eq!(id("/roms", "/roms/neogeo/pacman.zip"), Some("neogeo"));
    }

    #[test]
    fn saturn_discs_need_their_folder_and_do_not_steal_playstation_files() {
        assert_eq!(id("/roms", "/roms/Saturn/nights.cue"), Some("saturn"));
        assert_eq!(id("/roms", "/roms/sega-saturn/nights.chd"), Some("saturn"));
        // outside a Saturn folder the shared types still mean PlayStation
        assert_eq!(id("/roms", "/roms/crash.chd"), Some("psx"));
        assert_eq!(id("/roms", "/roms/crash.cue"), Some("psx"));
        assert_eq!(id("/roms", "/roms/psx/crash.cue"), Some("psx"));
    }

    #[test]
    fn computers_and_the_newer_consoles_are_found_by_folder_and_by_file_type() {
        let r = "/roms";
        assert_eq!(id(r, "/roms/c64/a.d64"), Some("c64"));
        assert_eq!(id(r, "/roms/Commodore 64/a.zip"), Some("c64"));
        assert_eq!(id(r, "/roms/amiga/a.adf"), Some("amiga"));
        assert_eq!(id(r, "/roms/msx/a.rom"), Some("msx"));
        assert_eq!(id(r, "/roms/jaguar/a.j64"), Some("jaguar"));
        // a file type only one system has needs no folder; a disc image does
        assert_eq!(id(r, "/roms/games/a.d64"), Some("c64"));
        assert_eq!(id(r, "/roms/games/a.gdi"), None);
        assert_eq!(id(r, "/roms/dreamcast/a.gdi"), Some("dreamcast"));
        assert_eq!(id(r, "/roms/dreamcast/a.chd"), Some("dreamcast"));
        assert_eq!(id(r, "/roms/psx/a.chd"), Some("psx"));
    }

    #[test]
    fn bios_names_are_recognised() {
        assert!(is_bios_file("NeoGeo.zip"));
        assert!(
            is_bios_file("PGM.zip"),
            "the PolyGame Master BIOS is not a game"
        );
        assert!(!is_bios_file("mslug.zip"));
    }
}
