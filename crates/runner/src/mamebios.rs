//! The current MAME core looks for BIOS sets (`neogeo.zip`, `pgm.zip`…) next to the game or in
//! `<system dir>/mame/bios`, not in the system dir where Vuretro keeps BIOS files for every
//! core. Without them a Neo Geo game in the Arcade folder stops with `sfix.sfix NOT FOUND`.

use std::path::Path;

/// Make the `.zip` files of `system_dir` visible to MAME: a hard link (or a copy where links
/// are not possible) in `system_dir/mame/bios`. Files that are already there are left alone.
/// Returns how many were added.
pub fn expose(system_dir: &Path) -> usize {
    let target = system_dir.join("mame").join("bios");
    let Ok(entries) = std::fs::read_dir(system_dir) else {
        return 0;
    };
    let mut added = 0;
    for e in entries.flatten() {
        let path = e.path();
        let is_zip = path.is_file()
            && path
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("zip"));
        let Some(name) = path.file_name().filter(|_| is_zip) else {
            continue;
        };
        let dest = target.join(name);
        if dest.exists() || std::fs::create_dir_all(&target).is_err() {
            continue;
        }
        if std::fs::hard_link(&path, &dest).is_ok() || std::fs::copy(&path, &dest).is_ok() {
            log::info!("BIOS for MAME: {}", dest.display());
            added += 1;
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bios_zips_are_made_visible_once_and_other_files_are_left_out() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("neogeo.zip"), b"bios").unwrap();
        std::fs::write(dir.path().join("PGM.ZIP"), b"pgm").unwrap();
        std::fs::write(dir.path().join("saturn_bios.bin"), b"x").unwrap();
        assert_eq!(expose(dir.path()), 2);
        let bios = dir.path().join("mame/bios");
        assert_eq!(std::fs::read(bios.join("neogeo.zip")).unwrap(), b"bios");
        assert!(bios.join("PGM.ZIP").is_file());
        assert!(!bios.join("saturn_bios.bin").exists());
        // the second time nothing is added, and a file already there is not replaced
        std::fs::write(bios.join("neogeo.zip"), b"mine").unwrap();
        assert_eq!(expose(dir.path()), 0);
        assert_eq!(std::fs::read(bios.join("neogeo.zip")).unwrap(), b"mine");
    }

    #[test]
    fn a_missing_system_dir_adds_nothing() {
        assert_eq!(expose(Path::new("/no/such/dir")), 0);
    }
}
