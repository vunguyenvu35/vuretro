//! vuretro library: the adapters that implement the `vuretro-core` ports on the real
//! machine (disk, processes), plus the file-level helpers they use.

pub mod adapters;
pub mod bios;
pub mod dat;
pub mod iso;
pub mod pe_icon;
pub mod roms;
pub mod scan;
pub mod unpack;

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
        let gba = vuretro_core::registry::by_id("gba").unwrap();
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

    /// Every BIOS file of every system, checked the same way: this is what makes sure a file
    /// added to the list works the day someone drops the real dump on the page.
    fn all_bios() -> Vec<(
        &'static vuretro_core::registry::System,
        &'static vuretro_core::registry::BiosSpec,
    )> {
        vuretro_core::registry::systems()
            .iter()
            .flat_map(|s| s.bios.iter().map(move |b| (s, b)))
            .collect()
    }

    #[test]
    fn the_bios_list_is_well_formed() {
        let all = all_bios();
        assert!(all.len() >= 10, "the list is not empty");
        for (sys, b) in &all {
            let who = format!("{} / {}", sys.id, b.file);
            assert!(
                !b.file.is_empty() && !b.file.contains(['/', '\\']),
                "{who}: a plain file name"
            );
            for m in b.md5 {
                assert!(
                    m.len() == 32
                        && m.chars()
                            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                    "{who}: MD5 {m} is 32 lower-case hex digits"
                );
            }
            // a file that is dropped as a game must be refused as one
            assert!(
                vuretro_core::detect::is_bios_file(b.file),
                "{who} is known as a BIOS file"
            );
        }
        // no two systems claim the same file with different checks (the drop zone could not
        // tell which one it is), and no MD5 belongs to two files
        for (i, (s1, b1)) in all.iter().enumerate() {
            for (s2, b2) in &all[i + 1..] {
                if b1.file.eq_ignore_ascii_case(b2.file) {
                    assert!(
                        s1.id == s2.id || b1.md5 == b2.md5,
                        "{} is listed for {} and {} with different checks",
                        b1.file,
                        s1.id,
                        s2.id
                    );
                }
                assert!(
                    !b1.md5.iter().any(|m| b2.md5.contains(m)),
                    "{} and {} share an MD5",
                    b1.file,
                    b2.file
                );
            }
        }
    }

    #[test]
    fn every_bios_file_goes_through_import_and_the_system_check() {
        for (sys, b) in all_bios() {
            let who = format!("{} / {}", sys.id, b.file);
            let (sysdir, src) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
            let file = src.path().join(b.file.to_ascii_uppercase());
            fs::write(&file, b"content that is no real dump").unwrap();

            // missing: the system says so, and only a required file blocks it
            let before = bios::check(sys, sysdir.path());
            let mine = before.iter().find(|s| s.file == b.file).unwrap();
            assert_eq!(mine.state, bios::BiosState::Missing, "{who}");
            assert_eq!(
                bios::ready(sys, sysdir.path()),
                !before.iter().any(|s| s.required),
                "{who}"
            );

            if b.md5.is_empty() {
                // a set checked by presence: found by name, whatever the capitals
                let done =
                    bios::import(&file, sysdir.path()).unwrap_or_else(|e| panic!("{who}: {e}"));
                assert_eq!((done.system, done.file), (sys.id, b.file), "{who}");
                assert!(
                    sysdir.path().join(b.file).is_file(),
                    "{who}: installed under its own name"
                );
                let after = bios::check(sys, sysdir.path());
                assert_eq!(
                    after.iter().find(|s| s.file == b.file).unwrap().state,
                    bios::BiosState::Ok,
                    "{who}"
                );
            } else {
                // a file with an MD5: the right name with the wrong content is refused, nothing
                // is copied, and the check calls it a bad dump if it is in the folder anyway
                let err = bios::import(&file, sysdir.path()).unwrap_err();
                assert!(err.contains("MD5"), "{who}: {err}");
                assert!(
                    !sysdir.path().join(b.file).exists(),
                    "{who}: nothing copied"
                );
                fs::write(sysdir.path().join(b.file), b"content that is no real dump").unwrap();
                let st = bios::check(sys, sysdir.path());
                assert!(
                    matches!(
                        st.iter().find(|s| s.file == b.file).unwrap().state,
                        bios::BiosState::BadHash { .. }
                    ),
                    "{who}"
                );
                // a required one that is a bad dump does not make the system ready
                assert_eq!(
                    bios::ready(sys, sysdir.path()),
                    !st.iter().any(|s| s.required),
                    "{who}"
                );
            }
        }
    }

    #[test]
    fn identify_falls_back_to_the_name_and_does_not_trust_wrong_content() {
        for (sys, b) in all_bios().into_iter().filter(|(_, b)| !b.md5.is_empty()) {
            // content that matches no listed MD5 falls back to the name, and is not verified
            let by_name = bios::identify(b"other bytes", b.file).unwrap();
            assert_eq!(by_name.spec.file, b.file, "{}", sys.id);
            assert!(
                !by_name.verified,
                "{}: right name, wrong content is not verified",
                b.file
            );
        }
        assert!(bios::identify(b"x", "holiday.jpg").is_err());
    }

    #[test]
    fn bios_states() {
        let dir = tempfile::tempdir().unwrap();
        let psx = vuretro_core::registry::by_id("psx").unwrap();
        fs::write(dir.path().join("scph5501.bin"), b"not a real bios").unwrap();
        let st = bios::check(psx, dir.path());
        assert!(matches!(st[0].state, bios::BiosState::BadHash { .. }));
        assert_eq!(st[1].state, bios::BiosState::Missing);
        // PlayStation's BIOS files are optional: the core has a built-in one
        assert!(bios::ready(psx, dir.path()));

        let neo = vuretro_core::registry::by_id("neogeo").unwrap();
        assert!(!bios::ready(neo, dir.path()));
        fs::write(dir.path().join("neogeo.zip"), b"zip").unwrap();
        assert!(bios::ready(neo, dir.path()));
    }
}
