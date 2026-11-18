use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use vuretro_core::backup::{restore_target, BackupManifest, BackupSummary, FORMAT};
use vuretro_core::paths::Paths;
use vuretro_core::ports::Archive;

const MANIFEST: &str = "manifest.json";
/// A backup holds settings, saves and pictures: nothing in it is bigger than this, so a
/// zip claiming more is not one of ours.
const MAX_ENTRY: u64 = 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 200_000;

/// Backups as zip files: the manifest, then the data folder's files under `data/`.
pub struct ZipBackup {
    data_dir: PathBuf,
}

impl ZipBackup {
    pub fn new(paths: Paths) -> Self {
        ZipBackup {
            data_dir: paths.data_dir().to_path_buf(),
        }
    }
}

fn err(what: &str, e: impl std::fmt::Display) -> String {
    format!("{what}: {e}")
}

impl Archive for ZipBackup {
    fn create_backup(
        &self,
        dest: &Path,
        includes: &[&str],
        app_version: &str,
        created: u64,
    ) -> Result<BackupSummary, String> {
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir).map_err(|e| err("cannot create the folder", e))?;
        }
        let file = File::create(dest).map_err(|e| err("cannot create the backup file", e))?;
        let result = (|| -> Result<(usize, u64), String> {
            let mut zip = zip::ZipWriter::new(file);
            let opts = zip::write::SimpleFileOptions::default();
            let manifest = BackupManifest {
                format: FORMAT,
                app_version: app_version.to_string(),
                created,
                includes: includes.iter().map(|s| s.to_string()).collect(),
            };
            zip.start_file(MANIFEST, opts).map_err(|e| e.to_string())?;
            zip.write_all(
                serde_json::to_string_pretty(&manifest)
                    .map_err(|e| e.to_string())?
                    .as_bytes(),
            )
            .map_err(|e| e.to_string())?;

            let (mut files, mut bytes) = (0usize, 0u64);
            for inc in includes {
                for entry in walkdir::WalkDir::new(self.data_dir.join(inc))
                    .follow_links(false)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|e| e.file_type().is_file())
                {
                    let path = entry.path();
                    if path == dest {
                        continue; // never put the backup into itself
                    }
                    let rel = path
                        .strip_prefix(&self.data_dir)
                        .map_err(|e| e.to_string())?;
                    let name = format!(
                        "data/{}",
                        rel.components()
                            .map(|c| c.as_os_str().to_string_lossy())
                            .collect::<Vec<_>>()
                            .join("/")
                    );
                    zip.start_file(name, opts).map_err(|e| e.to_string())?;
                    let mut src = File::open(path).map_err(|e| err("cannot read a file", e))?;
                    bytes += std::io::copy(&mut src, &mut zip).map_err(|e| e.to_string())?;
                    files += 1;
                }
            }
            zip.finish().map_err(|e| e.to_string())?;
            Ok((files, bytes))
        })();
        match result {
            Ok((files, bytes)) => Ok(BackupSummary {
                path: dest.to_string_lossy().into_owned(),
                files,
                bytes,
            }),
            Err(e) => {
                let _ = std::fs::remove_file(dest); // no half-written backup left behind
                Err(e)
            }
        }
    }

    fn inspect_backup(&self, src: &Path) -> Result<BackupManifest, String> {
        let file = File::open(src).map_err(|e| err("cannot open the file", e))?;
        let mut zip = zip::ZipArchive::new(file)
            .map_err(|_| "this is not a Vuretro backup (not a zip file)".to_string())?;
        let mut entry = zip
            .by_name(MANIFEST)
            .map_err(|_| "this zip is not a Vuretro backup (no manifest)".to_string())?;
        let mut text = String::new();
        entry
            .by_ref()
            .take(64 * 1024)
            .read_to_string(&mut text)
            .map_err(|e| err("cannot read the manifest", e))?;
        serde_json::from_str(&text).map_err(|_| "the backup manifest is damaged".to_string())
    }

    fn restore_backup(&self, src: &Path) -> Result<BackupSummary, String> {
        let file = File::open(src).map_err(|e| err("cannot open the file", e))?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| err("cannot read the backup", e))?;
        if zip.len() > MAX_ENTRIES {
            return Err("this backup has too many files".into());
        }
        let (mut files, mut bytes) = (0usize, 0u64);
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
            if !entry.is_file() {
                continue;
            }
            // anything that is not a safe, allowed path inside the data folder is skipped
            let Some(rel) = restore_target(entry.name()) else {
                continue;
            };
            if entry.size() > MAX_ENTRY {
                return Err(format!("{} is too large for a backup", entry.name()));
            }
            let target = self.data_dir.join(rel);
            if let Some(dir) = target.parent() {
                std::fs::create_dir_all(dir).map_err(|e| err("cannot create a folder", e))?;
            }
            let mut out = File::create(&target)
                .map_err(|e| err(&format!("cannot write {}", target.display()), e))?;
            bytes += std::io::copy(&mut entry.by_ref().take(MAX_ENTRY), &mut out)
                .map_err(|e| err("cannot restore a file", e))?;
            files += 1;
        }
        Ok(BackupSummary {
            path: src.to_string_lossy().into_owned(),
            files,
            bytes,
        })
    }

    fn write_text(&self, dest: &Path, text: &str) -> Result<(), String> {
        if let Some(dir) = dest.parent() {
            std::fs::create_dir_all(dir).map_err(|e| err("cannot create the folder", e))?;
        }
        std::fs::write(dest, text).map_err(|e| err("cannot write the file", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(dir: &Path) {
        for (p, body) in [
            ("settings.json", "{}"),
            ("library.json", "{\"a\":1}"),
            ("config/controls.json", "c"),
            ("config/controls/games/Zelda.json", "z"),
            ("saves/zelda.srm", "sram"),
            ("covers/zelda.png", "pic"),
            ("roms/snes/zelda.sfc", "ROM"),
            ("system/neogeo.zip", "BIOS"),
            ("cores/x.dylib", "core"),
        ] {
            let f = dir.join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, body).unwrap();
        }
    }

    fn names(zip: &Path) -> Vec<String> {
        let mut z = zip::ZipArchive::new(File::open(zip).unwrap()).unwrap();
        let mut v: Vec<String> = (0..z.len())
            .map(|i| z.by_index(i).unwrap().name().to_string())
            .collect();
        v.sort();
        v
    }

    #[test]
    fn a_backup_holds_what_was_chosen_and_never_roms_bios_or_cores() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("data");
        data(&src);
        let b = ZipBackup::new(Paths::new(&src));
        let dest = dir.path().join("out/b.zip");
        let s = b
            .create_backup(
                &dest,
                &["settings.json", "library.json", "config", "saves"],
                "9.9",
                123,
            )
            .unwrap();
        assert_eq!(s.files, 5);
        assert_eq!(
            names(&dest),
            [
                "data/config/controls.json",
                "data/config/controls/games/Zelda.json",
                "data/library.json",
                "data/saves/zelda.srm",
                "data/settings.json",
                "manifest.json",
            ]
        );
        let m = b.inspect_backup(&dest).unwrap();
        assert_eq!(
            (m.format, m.app_version.as_str(), m.created),
            (FORMAT, "9.9", 123)
        );
    }

    #[test]
    fn restoring_into_another_data_folder_brings_the_files_back() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a");
        data(&src);
        let dest = dir.path().join("b.zip");
        ZipBackup::new(Paths::new(&src))
            .create_backup(
                &dest,
                &["settings.json", "config", "saves", "covers"],
                "1",
                1,
            )
            .unwrap();

        let fresh = dir.path().join("fresh");
        let s = ZipBackup::new(Paths::new(&fresh))
            .restore_backup(&dest)
            .unwrap();
        assert_eq!(s.files, 5);
        assert_eq!(
            std::fs::read_to_string(fresh.join("saves/zelda.srm")).unwrap(),
            "sram"
        );
        assert_eq!(
            std::fs::read_to_string(fresh.join("config/controls/games/Zelda.json")).unwrap(),
            "z"
        );
        assert!(!fresh.join("roms").exists());
    }

    #[test]
    fn a_hostile_zip_cannot_write_outside_the_data_folder_or_plant_roms() {
        let dir = tempfile::tempdir().unwrap();
        let evil = dir.path().join("evil.zip");
        {
            let mut z = zip::ZipWriter::new(File::create(&evil).unwrap());
            let o = zip::write::SimpleFileOptions::default();
            for name in [
                "manifest.json",
                "data/settings.json",
                "data/../escaped.txt",
                "../escaped2.txt",
                "/abs.txt",
                "data/roms/planted.zip",
                "data/config/../../escaped3.txt",
            ] {
                z.start_file(name, o).unwrap();
                z.write_all(b"x").unwrap();
            }
            z.finish().unwrap();
        }
        let fresh = dir.path().join("fresh");
        let s = ZipBackup::new(Paths::new(&fresh))
            .restore_backup(&evil)
            .unwrap();
        assert_eq!(s.files, 1, "only data/settings.json is safe");
        assert!(fresh.join("settings.json").exists());
        for p in [
            "escaped.txt",
            "escaped2.txt",
            "abs.txt",
            "escaped3.txt",
            "roms",
        ] {
            assert!(
                !dir.path().join(p).exists() && !fresh.join(p).exists(),
                "{p}"
            );
        }
    }

    #[test]
    fn files_that_are_not_backups_are_refused_politely() {
        let dir = tempfile::tempdir().unwrap();
        let b = ZipBackup::new(Paths::new(dir.path().join("d")));
        let txt = dir.path().join("a.txt");
        std::fs::write(&txt, "hello").unwrap();
        assert!(b
            .inspect_backup(&txt)
            .unwrap_err()
            .contains("not a Vuretro backup"));
        let plain = dir.path().join("plain.zip");
        {
            let mut z = zip::ZipWriter::new(File::create(&plain).unwrap());
            z.start_file("x.txt", zip::write::SimpleFileOptions::default())
                .unwrap();
            z.finish().unwrap();
        }
        assert!(b
            .inspect_backup(&plain)
            .unwrap_err()
            .contains("no manifest"));
        assert!(b.inspect_backup(&dir.path().join("missing.zip")).is_err());
    }

    #[test]
    fn a_failed_backup_leaves_no_half_written_file() {
        let dir = tempfile::tempdir().unwrap();
        let b = ZipBackup::new(Paths::new(dir.path().join("d")));
        // a directory where the file should go makes the create fail up front
        let dest = dir.path().join("blocked");
        std::fs::create_dir_all(&dest).unwrap();
        assert!(b.create_backup(&dest, &["settings.json"], "1", 1).is_err());
        assert!(dest.is_dir());
    }
}
