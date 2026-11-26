//! Disc images that come inside an archive (`Game (Japan).7z` holding a `.cue` and a `.bin`):
//! look inside without unpacking, unpack, and mend a `.cue` that names its `.bin` differently
//! from the file that was packed (a very common mismatch in shared dumps).

use std::path::{Path, PathBuf};

/// Extensions of disc images (and the lists that go with them).
const DISC_EXTS: [&str; 9] = [
    "cue", "chd", "pbp", "iso", "img", "m3u", "ccd", "mds", "gdi",
];

fn ext_of(name: &str) -> String {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

pub fn is_archive(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("zip" | "7z" | "rar")
    )
}

/// A name inside an archive that stays under the folder it is unpacked into.
fn is_safe(name: &str) -> bool {
    use std::path::Component;
    Path::new(name)
        .components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
        && !name.contains('\\')
}

/// The names of the files in a `.rar`, without unpacking anything.
fn rar_names(archive: &Path) -> Result<Vec<String>, String> {
    let list = unrar::Archive::new(archive)
        .open_for_listing()
        .map_err(|e| format!("cannot read the rar: {e}"))?;
    let mut out = Vec::new();
    for entry in list {
        let entry = entry.map_err(|e| format!("cannot read the rar: {e}"))?;
        if !entry.is_directory() {
            out.push(entry.filename.to_string_lossy().into_owned());
        }
    }
    Ok(out)
}

/// Unpack a `.rar` into `dest`, refusing names that would leave it.
fn rar_extract(archive: &Path, dest: &Path) -> Result<(), String> {
    let mut open = unrar::Archive::new(archive)
        .open_for_processing()
        .map_err(|e| format!("cannot read the rar: {e}"))?;
    while let Some(header) = open
        .read_header()
        .map_err(|e| format!("cannot unpack it: {e}"))?
    {
        let name = header.entry().filename.to_string_lossy().into_owned();
        if !is_safe(&name) {
            return Err(format!("unsafe file name in the archive: {name}"));
        }
        open = if header.entry().is_file() {
            header
                .extract_with_base(dest)
                .map_err(|e| format!("cannot unpack {name}: {e}"))?
        } else {
            header
                .skip()
                .map_err(|e| format!("cannot unpack it: {e}"))?
        };
    }
    Ok(())
}

/// The names of the files in a `.zip`, `.7z` or `.rar`, without unpacking anything.
pub fn names(archive: &Path) -> Result<Vec<String>, String> {
    if ext_of(&archive.to_string_lossy()) == "rar" {
        return rar_names(archive);
    }
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
        zip.file_names()
            .map(|n| n.map(|n| n.into_owned()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("cannot read the zip: {e}"))
    }
}

/// Does the archive hold a disc image (and so has to be unpacked to be played)?
pub fn holds_disc(names: &[String]) -> bool {
    names
        .iter()
        .any(|n| DISC_EXTS.contains(&ext_of(n).as_str()))
}

/// Unpack everything into `dest`. Names that would leave `dest` are refused.
pub fn extract(archive: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    if ext_of(&archive.to_string_lossy()) == "rar" {
        return rar_extract(archive, dest);
    }
    if ext_of(&archive.to_string_lossy()) == "7z" {
        return sevenz_rust2::decompress_file(archive, dest)
            .map_err(|e| format!("cannot unpack it: {e}"));
    }
    let file = std::fs::File::open(archive).map_err(|e| format!("cannot open it: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("cannot read the zip: {e}"))?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| format!("cannot unpack it: {e}"))?;
        let Some(rel) = entry.enclosed_name() else {
            let name = entry.name().map(|n| n.into_owned()).unwrap_or_default();
            return Err(format!("unsafe file name in the archive: {name}"));
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
pub fn files_under(dir: &Path) -> Vec<PathBuf> {
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
    let cues: Vec<&PathBuf> = all
        .iter()
        .filter(|p| ext_of(&p.to_string_lossy()) == "cue")
        .collect();
    let name_of = |p: &Path| {
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let mut referenced: Vec<String> = Vec::new();
    for c in &cues {
        if let Ok(t) = std::fs::read_to_string(c) {
            referenced.extend(cue_files(&t));
        }
    }
    let loose: Vec<String> = all
        .iter()
        .filter(|p| {
            !["cue", "m3u", "html", "txt", "nfo"].contains(&ext_of(&p.to_string_lossy()).as_str())
        })
        .map(|p| name_of(p))
        .filter(|n| !referenced.iter().any(|r| r.eq_ignore_ascii_case(n)))
        .collect();
    for c in cues {
        let Ok(text) = std::fs::read_to_string(c) else {
            continue;
        };
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
    for set in [
        &["m3u"][..],
        &["cue"],
        &["chd", "pbp", "iso", "img", "ccd", "mds", "gdi"],
    ] {
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
            w.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            w.write_all(data.as_bytes()).unwrap();
        }
        w.finish().unwrap();
    }

    #[test]
    fn an_archive_with_a_disc_is_told_from_one_with_a_rom_set() {
        let dir = tempfile::tempdir().unwrap();
        let disc = dir.path().join("g.zip");
        zip_with(
            &disc,
            &[("g.cue", "x"), ("g.bin", "y"), ("readme.html", "z")],
        );
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
        let wrong =
            "FILE \"Game [SLPS-1].bin\" BINARY\n  TRACK 01 MODE2/2352\n    INDEX 01 00:00:00\n";
        std::fs::write(dir.path().join("Game.cue"), wrong).unwrap();
        std::fs::write(dir.path().join("Game.bin"), "d").unwrap();
        std::fs::write(dir.path().join("readme.html"), "r").unwrap();
        mend_cues(dir.path());
        let fixed = std::fs::read_to_string(dir.path().join("Game.cue")).unwrap();
        assert!(fixed.contains("FILE \"Game.bin\""), "{fixed}");
        // already right: untouched
        mend_cues(dir.path());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("Game.cue")).unwrap(),
            fixed
        );
        // two files and one name: cannot tell which, so it is left for the import to refuse
        std::fs::write(dir.path().join("Other.bin"), "d").unwrap();
        std::fs::write(dir.path().join("Game.cue"), wrong).unwrap();
        mend_cues(dir.path());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("Game.cue")).unwrap(),
            wrong
        );
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

    /// Written by Python's zipfile, not by the zip crate this program uses.
    const PYTHON_ZIP: &[u8] = &[
        80, 75, 3, 4, 20, 0, 0, 0, 8, 0, 0, 0, 33, 80, 227, 81, 61, 141, 10, 0, 0, 0, 23, 0, 0, 0,
        5, 0, 0, 0, 97, 46, 98, 105, 110, 203, 72, 205, 201, 201, 87, 200, 64, 39, 1, 80, 75, 3, 4,
        20, 0, 0, 0, 8, 0, 0, 0, 33, 80, 105, 17, 31, 182, 8, 0, 0, 0, 6, 0, 0, 0, 9, 0, 0, 0, 100,
        105, 114, 47, 98, 46, 116, 120, 116, 43, 78, 77, 206, 207, 75, 1, 0, 80, 75, 1, 2, 20, 3,
        20, 0, 0, 0, 8, 0, 0, 0, 33, 80, 227, 81, 61, 141, 10, 0, 0, 0, 23, 0, 0, 0, 5, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 128, 1, 0, 0, 0, 0, 97, 46, 98, 105, 110, 80, 75, 1, 2, 20, 3, 20, 0,
        0, 0, 8, 0, 0, 0, 33, 80, 105, 17, 31, 182, 8, 0, 0, 0, 6, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 128, 1, 45, 0, 0, 0, 100, 105, 114, 47, 98, 46, 116, 120, 116, 80, 75, 5, 6, 0,
        0, 0, 0, 2, 0, 2, 0, 106, 0, 0, 0, 92, 0, 0, 0, 0, 0,
    ];

    /// Written by Python's zipfile, not by the zip crate this program uses.
    const PYTHON_ZIP_WITH_AN_ESCAPING_NAME: &[u8] = &[
        80, 75, 3, 4, 20, 0, 0, 0, 8, 0, 0, 0, 33, 80, 146, 84, 169, 190, 6, 0, 0, 0, 4, 0, 0, 0,
        8, 0, 0, 0, 103, 111, 111, 100, 46, 116, 120, 116, 75, 203, 204, 75, 5, 0, 80, 75, 3, 4,
        20, 0, 0, 0, 8, 0, 0, 0, 33, 80, 91, 12, 248, 146, 9, 0, 0, 0, 7, 0, 0, 0, 11, 0, 0, 0, 46,
        46, 47, 101, 118, 105, 108, 46, 116, 120, 116, 75, 45, 78, 78, 44, 72, 77, 1, 0, 80, 75, 1,
        2, 20, 3, 20, 0, 0, 0, 8, 0, 0, 0, 33, 80, 146, 84, 169, 190, 6, 0, 0, 0, 4, 0, 0, 0, 8, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 1, 0, 0, 0, 0, 103, 111, 111, 100, 46, 116, 120, 116,
        80, 75, 1, 2, 20, 3, 20, 0, 0, 0, 8, 0, 0, 0, 33, 80, 91, 12, 248, 146, 9, 0, 0, 0, 7, 0,
        0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 1, 44, 0, 0, 0, 46, 46, 47, 101, 118, 105,
        108, 46, 116, 120, 116, 80, 75, 5, 6, 0, 0, 0, 0, 2, 0, 2, 0, 111, 0, 0, 0, 94, 0, 0, 0, 0,
        0,
    ];

    #[test]
    fn a_zip_written_by_another_program_is_listed_and_unpacked() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("python.zip");
        std::fs::write(&zip, PYTHON_ZIP).unwrap();
        let mut names = names(&zip).unwrap();
        names.sort();
        assert_eq!(names, ["a.bin", "dir/b.txt"]);
        let out = dir.path().join("out");
        extract(&zip, &out).unwrap();
        assert_eq!(
            std::fs::read(out.join("a.bin")).unwrap(),
            b"hello hello hello hello"
        );
        assert_eq!(std::fs::read(out.join("dir/b.txt")).unwrap(), b"second");
    }

    #[test]
    fn a_name_that_leaves_the_folder_is_refused_and_nothing_escapes() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("evil.zip");
        std::fs::write(&zip, PYTHON_ZIP_WITH_AN_ESCAPING_NAME).unwrap();
        let out = dir.path().join("inner").join("out");
        let err = extract(&zip, &out).unwrap_err();
        assert!(err.contains("unsafe file name"), "{err}");
        assert!(err.contains("evil.txt"), "{err}");
        // not beside the folder, nor anywhere above it
        assert!(!out.parent().unwrap().join("evil.txt").exists());
        assert!(!dir.path().join("evil.txt").exists());
    }

    #[test]
    fn a_rar_is_an_archive_and_a_broken_one_says_so() {
        assert!(is_archive(Path::new("g.rar")) && is_archive(Path::new("G.RAR")));
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.rar");
        std::fs::write(&bad, "this is not a rar").unwrap();
        assert!(names(&bad).is_err());
        assert!(extract(&bad, &dir.path().join("out")).is_err());
    }

    #[test]
    fn names_that_leave_the_folder_are_unsafe() {
        assert!(is_safe("game.cue") && is_safe("disc/game.cue"));
        assert!(!is_safe("../evil.bin") && !is_safe("/etc/passwd") && !is_safe("a/../../b"));
    }
}
