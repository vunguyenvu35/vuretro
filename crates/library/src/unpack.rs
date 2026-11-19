//! Disc images that come inside an archive (`Game (Japan).7z` holding a `.cue` and a `.bin`):
//! look inside without unpacking, unpack, and mend a `.cue` that names its `.bin` differently
//! from the file that was packed (a very common mismatch in shared dumps).

use std::path::{Path, PathBuf};

/// Extensions of disc images (and the lists that go with them).
const DISC_EXTS: [&str; 9] = ["cue", "chd", "pbp", "iso", "img", "m3u", "ccd", "mds", "gdi"];

fn ext_of(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

pub fn is_archive(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref(),
        Some("zip" | "7z")
    )
}

/// The names of the files in a `.zip` or `.7z`, without unpacking anything.
pub fn names(archive: &Path) -> Result<Vec<String>, String> {
    let file = std::fs::File::open(archive).map_err(|e| format!("cannot open it: {e}"))?;
    if ext_of(&archive.to_string_lossy()) == "7z" {
        let reader = sevenz_rust2::ArchiveReader::new(file, sevenz_rust2::Password::empty())
            .map_err(|e| format!("cannot read the 7z archive: {e}"))?;
        Ok(reader
            .archive()
            .files
            .iter()
            .filter(|f| !f.is_directory())
            .map(|f| f.name().to_string())
            .collect())
    } else {
        let zip = zip::ZipArchive::new(file).map_err(|e| format!("cannot read the zip: {e}"))?;
        Ok(zip.file_names().map(str::to_string).collect())
    }
}

/// Does the archive hold a disc image (and so has to be unpacked to be played)?
pub fn holds_disc(names: &[String]) -> bool {
    names.iter().any(|n| DISC_EXTS.contains(&ext_of(n).as_str()))
}

/// Unpack everything into `dest`. Names that would leave `dest` are refused.
pub fn extract(archive: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    if ext_of(&archive.to_string_lossy()) == "7z" {
        return sevenz_rust2::decompress_file(archive, dest)
            .map_err(|e| format!("cannot unpack it: {e}"));
    }
    let file = std::fs::File::open(archive).map_err(|e| format!("cannot open it: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("cannot read the zip: {e}"))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("cannot unpack it: {e}"))?;
        let Some(rel) = entry.enclosed_name() else {
            return Err(format!("unsafe file name in the archive: {}", entry.name()));
        };
        let out = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut f = std::fs::File::create(&out).map_err(|e| format!("cannot write: {e}"))?;
        std::io::copy(&mut entry, &mut f).map_err(|e| format!("cannot unpack it: {e}"))?;
    }
    Ok(())
}

/// Every file under `dir`, in a stable order.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = walkdir::WalkDir::new(dir)
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .collect();
    out.sort();
    out
}

/// The `FILE "name"` entries of a cue sheet, in order.
fn cue_files(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|l| l.trim().strip_prefix("FILE"))
        .filter_map(|r| r.split('"').nth(1))
        .map(str::to_string)
        .collect()
}

/// Make each `.cue` under `dir` point at files that exist. When the names it uses are not
/// there, and the files nothing refers to are as many as the missing ones, they are the
/// same files under other names: the sheet is rewritten to use them, in order.
pub fn mend_cues(dir: &Path) {
    let all = files_under(dir);
    let cues: Vec<&PathBuf> = all.iter().filter(|p| ext_of(&p.to_string_lossy()) == "cue").collect();
    let name_of = |p: &Path| p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let mut referenced: Vec<String> = Vec::new();
    for c in &cues {
        if let Ok(t) = std::fs::read_to_string(c) {
            referenced.extend(cue_files(&t));
        }
    }
    let loose: Vec<String> = all
        .iter()
        .filter(|p| !["cue", "m3u", "html", "txt", "nfo"].contains(&ext_of(&p.to_string_lossy()).as_str()))
        .map(|p| name_of(p))
        .filter(|n| !referenced.iter().any(|r| r.eq_ignore_ascii_case(n)))
        .collect();
    for c in cues {
        let Ok(text) = std::fs::read_to_string(c) else { continue };
        let folder = c.parent().unwrap_or(dir);
        let missing: Vec<String> = cue_files(&text)
            .into_iter()
            .filter(|n| !folder.join(n).is_file())
            .collect();
        if missing.is_empty() || missing.len() != loose.len() {
            continue;
        }
        let mut fixed = text.clone();
        for (old, new) in missing.iter().zip(&loose) {
            fixed = fixed.replace(&format!("\"{old}\""), &format!("\"{new}\""));
        }
        let _ = std::fs::write(c, fixed);
    }
}

/// The files to import from an unpacked archive: an `.m3u` when there is one (a multi-disc
/// game), else every `.cue`, else the disc images that need nothing beside them.
pub fn entry_points(dir: &Path) -> Vec<PathBuf> {
    let all = files_under(dir);
    for set in [&["m3u"][..], &["cue"], &["chd", "pbp", "iso", "img", "ccd", "mds", "gdi"]] {
        let found: Vec<PathBuf> = all
            .iter()
            .filter(|p| set.contains(&ext_of(&p.to_string_lossy()).as_str()))
            .cloned()
            .collect();
        if !found.is_empty() {
            return found;
        }
    }
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_with(path: &Path, files: &[(&str, &str)]) {
        let mut w = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, data) in files {
            w.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(data.as_bytes()).unwrap();
        }
        w.finish().unwrap();
    }

    #[test]
    fn an_archive_with_a_disc_is_told_from_one_with_a_rom_set() {
        let dir = tempfile::tempdir().unwrap();
        let disc = dir.path().join("g.zip");
        zip_with(&disc, &[("g.cue", "x"), ("g.bin", "y"), ("readme.html", "z")]);
        assert!(holds_disc(&names(&disc).unwrap()));
        let rom = dir.path().join("mslug.zip");
        zip_with(&rom, &[("201-p1.p1", "x"), ("201-c1.c1", "y")]);
        assert!(!holds_disc(&names(&rom).unwrap()));
        assert!(names(&dir.path().join("none.zip")).is_err());
    }

    #[test]
    fn a_7z_is_listed_and_unpacked_too() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("g.cue"), "FILE \"g.bin\" BINARY\n").unwrap();
        std::fs::write(src.join("g.bin"), "data").unwrap();
        let archive = dir.path().join("g.7z");
        sevenz_rust2::compress_to_path(&src, &archive).unwrap();
        let list = names(&archive).unwrap();
        assert!(holds_disc(&list), "{list:?}");
        let out = dir.path().join("out");
        extract(&archive, &out).unwrap();
        assert_eq!(std::fs::read_to_string(out.join("g.bin")).unwrap(), "data");
    }

    #[test]
    fn a_cue_that_names_another_file_is_mended_and_a_right_one_is_left() {
        let dir = tempfile::tempdir().unwrap();
        let wrong = "FILE \"Game [SLPS-1].bin\" BINARY\n  TRACK 01 MODE2/2352\n    INDEX 01 00:00:00\n";
        std::fs::write(dir.path().join("Game.cue"), wrong).unwrap();
        std::fs::write(dir.path().join("Game.bin"), "d").unwrap();
        std::fs::write(dir.path().join("readme.html"), "r").unwrap();
        mend_cues(dir.path());
        let fixed = std::fs::read_to_string(dir.path().join("Game.cue")).unwrap();
        assert!(fixed.contains("FILE \"Game.bin\""), "{fixed}");
        // already right: untouched
        mend_cues(dir.path());
        assert_eq!(std::fs::read_to_string(dir.path().join("Game.cue")).unwrap(), fixed);
        // two files and one name: cannot tell which, so it is left for the import to refuse
        std::fs::write(dir.path().join("Other.bin"), "d").unwrap();
        std::fs::write(dir.path().join("Game.cue"), wrong).unwrap();
        mend_cues(dir.path());
        assert_eq!(std::fs::read_to_string(dir.path().join("Game.cue")).unwrap(), wrong);
    }

    #[test]
    fn entry_points_prefer_the_playlist_then_the_cues() {
        let dir = tempfile::tempdir().unwrap();
        for f in ["a.cue", "a.bin", "b.cue", "b.bin"] {
            std::fs::write(dir.path().join(f), "x").unwrap();
        }
        assert_eq!(entry_points(dir.path()).len(), 2);
        std::fs::write(dir.path().join("all.m3u"), "a.cue\nb.cue\n").unwrap();
        let one = entry_points(dir.path());
        assert_eq!(one.len(), 1);
        assert!(one[0].ends_with("all.m3u"));
        assert!(entry_points(&dir.path().join("nothing")).is_empty());
    }
}
