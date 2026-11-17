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
    fn detects_snes_and_gba() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("snes")).unwrap();
        fs::create_dir_all(root.join("gba")).unwrap();
        fs::write(root.join("snes/zelda.sfc"), b"x").unwrap();
        fs::write(root.join("gba/zip.zip"), b"x").unwrap();
        fs::write(root.join("metroid.gba"), b"x").unwrap();
        fs::write(root.join("mario.smc"), b"x").unwrap();
        fs::write(root.join("gba_bios.bin"), b"bios").unwrap();

        let games = scan::scan(root);
        let ids: Vec<_> = games
            .iter()
            .map(|g| (g.system.as_str(), g.title.as_str()))
            .collect();
        assert_eq!(
            ids,
            vec![
                ("gba", "metroid"),
                ("gba", "zip"),
                ("snes", "mario"),
                ("snes", "zelda")
            ]
        );
    }

    #[test]
    fn gba_bios_is_optional() {
        let dir = tempfile::tempdir().unwrap();
        let gba = registry::by_id("gba").unwrap();
        assert!(bios::ready(gba, dir.path()));
        assert_eq!(
            bios::check(gba, dir.path())[0].state,
            bios::BiosState::Missing
        );
    }

    #[test]
    fn imports_bios_by_md5_or_name() {
        let sys = tempfile::tempdir().unwrap();
        let src = tempfile::tempdir().unwrap();

        // presence-only set matched by name
        fs::write(src.path().join("NeoGeo.zip"), b"zip").unwrap();
        let r = bios::import(&src.path().join("NeoGeo.zip"), sys.path()).unwrap();
        assert_eq!((r.system, r.file), ("neogeo", "neogeo.zip"));
        assert!(sys.path().join("neogeo.zip").exists());

        // right name, wrong content: rejected, nothing copied
        fs::write(src.path().join("scph5501.bin"), b"fake").unwrap();
        let err = bios::import(&src.path().join("scph5501.bin"), sys.path()).unwrap_err();
        assert!(err.contains("MD5"), "{err}");
        assert!(!sys.path().join("scph5501.bin").exists());

        // unknown file
        fs::write(src.path().join("readme.txt"), b"x").unwrap();
        assert!(bios::import(&src.path().join("readme.txt"), sys.path()).is_err());
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
