//! vuretro library: system registry, ROM scanning and BIOS checks.

pub mod bios;
pub mod registry;
pub mod scan;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn folder_decides_ambiguous_zip() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("neogeo")).unwrap();
        fs::write(root.join("neogeo/mslug.zip"), b"x").unwrap();
        fs::write(root.join("neogeo/neogeo.zip"), b"bios").unwrap();
        fs::write(root.join("loose.zip"), b"x").unwrap();
        fs::write(root.join("crash.chd"), b"x").unwrap();

        let games = scan::scan(root);
        let ids: Vec<_> = games
            .iter()
            .map(|g| (g.system.as_str(), g.title.as_str()))
            .collect();
        assert_eq!(ids, vec![("neogeo", "mslug"), ("psx", "crash")]);
    }

    #[test]
    fn bios_states() {
        let dir = tempfile::tempdir().unwrap();
        let psx = registry::by_id("psx").unwrap();
        fs::write(dir.path().join("scph5501.bin"), b"not a real bios").unwrap();
        let st = bios::check(psx, dir.path());
        assert!(matches!(st[0].state, bios::BiosState::BadHash { .. }));
        assert_eq!(st[1].state, bios::BiosState::Missing);
        assert!(!bios::ready(psx, dir.path()));

        let neo = registry::by_id("neogeo").unwrap();
        assert!(!bios::ready(neo, dir.path()));
        fs::write(dir.path().join("neogeo.zip"), b"zip").unwrap();
        assert!(bios::ready(neo, dir.path()));
    }
}
