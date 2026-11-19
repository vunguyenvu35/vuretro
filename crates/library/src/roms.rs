//! Importing ROMs into the app's own library folder (`<library>/<system folder>/file`)
//! and removing them again. Importing copies; the original files are never touched.

use crate::{scan, unpack};
use std::path::{Path, PathBuf};
use vuretro_core::detect::{detect_system, is_bios_file};
pub use vuretro_core::model::ImportedRom as Imported;
use vuretro_core::registry::by_id as system_by_id;

/// Files a multi-file image needs next to it: the `FILE "x.bin"` entries of a `.cue`,
/// the lines of an `.m3u`.
fn companions(src: &Path) -> Result<Vec<PathBuf>, String> {
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext != "cue" && ext != "m3u" {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(src).map_err(|e| format!("cannot read it: {e}"))?;
    let dir = src.parent().unwrap_or(Path::new("."));
    let mut out = Vec::new();
    for line in text.lines().map(str::trim) {
        let name = if ext == "m3u" {
            (!line.is_empty() && !line.starts_with('#')).then(|| line.to_string())
        } else {
            line.strip_prefix("FILE")
                .and_then(|r| r.split('"').nth(1))
                .map(str::to_string)
        };
        if let Some(name) = name {
            let p = dir.join(&name);
            if !p.is_file() {
                return Err(format!("it needs {name}, which is not next to it"));
            }
            out.push(p);
        }
    }
    Ok(out)
}

/// Put `src` into `dir`: a copy, or (for files unpacked from an archive, which are ours to
/// spend) a move, which does not write a big disc image twice.
fn copy_into(src: &Path, dir: &Path, moving: bool) -> Result<(PathBuf, bool), String> {
    let name = src.file_name().ok_or("not a file")?;
    let dest = dir.join(name);
    let size = std::fs::metadata(src).map_err(|e| e.to_string())?.len();
    if std::fs::metadata(&dest).is_ok_and(|m| m.len() == size) {
        return Ok((dest, true));
    }
    if moving && std::fs::rename(src, &dest).is_ok() {
        return Ok((dest, false));
    }
    // Copy under a temporary name so an interrupted import never leaves a broken ROM.
    let tmp = dir.join(format!(".{}.part", name.to_string_lossy()));
    std::fs::copy(src, &tmp).map_err(|e| format!("cannot copy: {e}"))?;
    std::fs::rename(&tmp, &dest).map_err(|e| format!("cannot finish copy: {e}"))?;
    Ok((dest, false))
}

/// Copy one ROM into `library`. `hint` is a system id chosen by the user; it settles
/// archives (`.zip`) whose system cannot be told from the file.
pub fn import_file(src: &Path, library: &Path, hint: Option<&str>) -> Result<Imported, String> {
    // A disc that comes in an archive is unpacked first: no one has to do it by hand.
    if unpack::is_archive(src) {
        if let Ok(list) = unpack::names(src) {
            if unpack::holds_disc(&list) {
                return import_disc_archive(src, library, hint);
            }
        }
    }
    import_inner(src, library, hint, false)
}

/// Unpack an archive that holds a disc image beside the library, mend the `.cue`, import what
/// is in it, and clean up. The first game imported is the answer.
fn import_disc_archive(src: &Path, library: &Path, hint: Option<&str>) -> Result<Imported, String> {
    let tmp = library.join(format!(".unpack-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let result = (|| {
        unpack::extract(src, &tmp)?;
        unpack::mend_cues(&tmp);
        let mut first: Option<Imported> = None;
        let mut error = None;
        for entry in unpack::entry_points(&tmp) {
            match import_inner(&entry, library, hint, true) {
                Ok(i) => {
                    first.get_or_insert(i);
                }
                Err(e) => error = error.or(Some(e)),
            }
        }
        first.ok_or_else(|| error.unwrap_or_else(|| "no disc image found in the archive".into()))
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

fn import_inner(
    src: &Path,
    library: &Path,
    hint: Option<&str>,
    moving: bool,
) -> Result<Imported, String> {
    let name = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("not a file")?;
    if !src.is_file() {
        return Err("not a file".into());
    }
    if is_bios_file(name) {
        return Err("this is a BIOS file, add it in Cores & BIOS".into());
    }
    if vuretro_core::patch::is_patch_name(name) {
        return Err(
            "this is a patch, not a game: right-click a game and choose Apply patch…".into(),
        );
    }
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let sys = match hint.and_then(system_by_id) {
        Some(s) if s.extensions.contains(&ext.as_str()) => s,
        Some(s) => return Err(format!("not a {} file (.{ext})", s.name)),
        None => detect_system(src.parent().unwrap_or(Path::new("")), src).ok_or(
            "cannot tell which system it is for: select the system in the sidebar, then import again",
        )?,
    };
    let extra = companions(src)?;
    let dir = library.join(sys.folders[0]);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for p in &extra {
        copy_into(p, &dir, moving)?;
    }
    let (dest, already_there) = copy_into(src, &dir, moving)?;
    Ok(Imported {
        system: sys.id,
        dest,
        already_there,
    })
}

/// Cartridge-sized files only: a patch is applied in memory, and discs are not patched.
const MAX_PATCHABLE: u64 = 64 * 1024 * 1024;
const NOT_PATCHABLE: [&str; 9] = ["zip", "7z", "rar", "chd", "cue", "m3u", "iso", "img", "pbp"];

/// Apply an IPS, UPS or BPS `patch` to `rom` and put the patched copy into `library`, named
/// `<game> [<patch>].<ext>` next to the other games of that system. The original is never
/// changed. Running it again with the same files finds the copy already there.
pub fn apply_patch(rom: &Path, patch: &Path, library: &Path) -> Result<Imported, String> {
    let name = rom
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("not a file")?;
    let ext = rom
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    if NOT_PATCHABLE.contains(&ext.to_ascii_lowercase().as_str()) {
        return Err(format!(
            "cannot patch .{ext} files: patch the game file inside it first"
        ));
    }
    let sys = detect_system(rom.parent().unwrap_or(Path::new("")), rom)
        .ok_or("cannot tell which system this game is for")?;
    let size = std::fs::metadata(rom)
        .map_err(|e| format!("cannot read {name}: {e}"))?
        .len();
    if size > MAX_PATCHABLE {
        return Err("this file is too large to patch".into());
    }
    let rom_bytes = std::fs::read(rom).map_err(|e| format!("cannot read {name}: {e}"))?;
    let patch_bytes = std::fs::read(patch).map_err(|e| format!("cannot read the patch: {e}"))?;
    let patched = vuretro_core::patch::apply(&rom_bytes, &patch_bytes)?;

    let stem = rom.file_stem().and_then(|s| s.to_str()).unwrap_or("game");
    let patch_stem = patch
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("patched");
    let dir = library.join(sys.folders[0]);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dest = dir.join(format!("{stem} [{patch_stem}].{ext}"));
    if std::fs::read(&dest).is_ok_and(|existing| existing == patched) {
        return Ok(Imported {
            system: sys.id,
            dest,
            already_there: true,
        });
    }
    // under a temporary name, so an interrupted write never leaves a broken ROM
    let tmp = dir.join(format!(".{stem}.patch.part"));
    std::fs::write(&tmp, &patched).map_err(|e| format!("cannot write the patched game: {e}"))?;
    std::fs::rename(&tmp, &dest).map_err(|e| format!("cannot finish the patched game: {e}"))?;
    Ok(Imported {
        system: sys.id,
        dest,
        already_there: false,
    })
}

/// Import every recognised ROM found in `folder` (subfolders included).
pub fn import_folder(folder: &Path, library: &Path) -> Vec<(PathBuf, Result<Imported, String>)> {
    scan::scan(folder)
        .into_iter()
        .map(|g| {
            let r = import_file(&g.path, library, Some(&g.system));
            (g.path, r)
        })
        .collect()
}

/// Delete an imported ROM (and the files a `.cue`/`.m3u` points to). Only files inside
/// `library` can be removed.
pub fn remove(library: &Path, path: &Path) -> Result<(), String> {
    let lib = library.canonicalize().map_err(|e| e.to_string())?;
    let file = path.canonicalize().map_err(|e| e.to_string())?;
    if !file.starts_with(&lib) || file == lib {
        return Err("only ROMs imported into the library can be removed".into());
    }
    for p in companions(&file).unwrap_or_default() {
        let _ = std::fs::remove_file(p);
    }
    std::fs::remove_file(&file).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn imports_by_extension_hint_and_folder_name() {
        let src = tempfile::tempdir().unwrap();
        let lib = tempfile::tempdir().unwrap();
        fs::write(src.path().join("game.sfc"), b"snes").unwrap();
        fs::write(src.path().join("mslug.zip"), b"zip").unwrap();
        fs::write(src.path().join("neogeo.zip"), b"bios").unwrap();

        let r = import_file(&src.path().join("game.sfc"), lib.path(), None).unwrap();
        assert_eq!(r.system, "snes");
        assert!(lib.path().join("snes/game.sfc").is_file());
        assert!(src.path().join("game.sfc").is_file()); // original untouched

        // an archive needs a system
        assert!(import_file(&src.path().join("mslug.zip"), lib.path(), None).is_err());
        let r = import_file(&src.path().join("mslug.zip"), lib.path(), Some("neogeo")).unwrap();
        assert!(lib.path().join("neogeo/mslug.zip").is_file() && !r.already_there);
        // twice: no second copy
        let again = import_file(&src.path().join("mslug.zip"), lib.path(), Some("neogeo")).unwrap();
        assert!(again.already_there);
        // wrong system for the extension, and BIOS files are refused
        assert!(import_file(&src.path().join("game.sfc"), lib.path(), Some("gba")).is_err());
        assert!(import_file(&src.path().join("neogeo.zip"), lib.path(), Some("neogeo")).is_err());
    }

    #[test]
    fn a_disc_in_an_archive_is_unpacked_and_its_cue_mended_on_import() {
        use std::io::Write;
        let tmp = tempfile::tempdir().unwrap();
        let lib = tempfile::tempdir().unwrap();
        // the .cue names a file that is not the one that was packed
        let zip = tmp.path().join("Game (Japan).zip");
        let mut w = zip::ZipWriter::new(fs::File::create(&zip).unwrap());
        for (name, data) in [
            ("Game (Japan).cue", "FILE \"Game [SLPS-1].bin\" BINARY\n  TRACK 01 MODE2/2352\n"),
            ("Game (Japan).bin", "disc data"),
            ("readme.html", "x"),
        ] {
            w.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
            w.write_all(data.as_bytes()).unwrap();
        }
        w.finish().unwrap();

        let done = import_file(&zip, lib.path(), None).unwrap();
        assert_eq!(done.system, "psx");
        assert!(done.dest.ends_with("psx/Game (Japan).cue"), "{:?}", done.dest);
        assert!(lib.path().join("psx/Game (Japan).bin").is_file());
        let cue = fs::read_to_string(&done.dest).unwrap();
        assert!(cue.contains("\"Game (Japan).bin\""), "{cue}");
        assert!(!lib.path().join("psx/readme.html").exists(), "only the disc is imported");
        // nothing is left behind, and doing it again finds the game already there
        let left: Vec<_> = fs::read_dir(lib.path()).unwrap().flatten().map(|e| e.file_name()).collect();
        assert_eq!(left.len(), 1, "{left:?}");
        assert!(import_file(&zip, lib.path(), None).unwrap().already_there);
    }

    #[test]
    fn an_archive_of_a_rom_set_is_still_imported_as_it_is() {
        use std::io::Write;
        let tmp = tempfile::tempdir().unwrap();
        let lib = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("neogeo");
        fs::create_dir_all(&dir).unwrap();
        let zip = dir.join("mslug.zip");
        let mut w = zip::ZipWriter::new(fs::File::create(&zip).unwrap());
        w.start_file("201-p1.p1", zip::write::SimpleFileOptions::default()).unwrap();
        w.write_all(b"rom").unwrap();
        w.finish().unwrap();
        let done = import_file(&zip, lib.path(), None).unwrap();
        assert!(done.dest.ends_with("neogeo/mslug.zip"));
        assert!(done.dest.is_file());
    }

    #[test]
    fn folder_import_and_cue_companions_and_remove() {
        let src = tempfile::tempdir().unwrap();
        let lib = tempfile::tempdir().unwrap();
        let roms = src.path().join("Neo Geo Roms");
        fs::create_dir_all(&roms).unwrap();
        fs::write(roms.join("aof.zip"), b"x").unwrap();
        fs::write(roms.join("neogeo.zip"), b"bios").unwrap();
        fs::write(roms.join("notes.txt"), b"x").unwrap();
        let res = import_folder(&roms, lib.path());
        assert_eq!(res.len(), 1);
        assert!(res[0].1.is_ok());
        assert!(lib.path().join("neogeo/aof.zip").is_file());

        let disc = src.path().join("disc");
        fs::create_dir_all(&disc).unwrap();
        fs::write(
            disc.join("g.cue"),
            "FILE \"g.bin\" BINARY\n  TRACK 01 MODE2/2352\n",
        )
        .unwrap();
        // missing track file -> refused, nothing copied
        assert!(import_file(&disc.join("g.cue"), lib.path(), None).is_err());
        assert!(!lib.path().join("psx/g.cue").exists());
        fs::write(disc.join("g.bin"), b"bin").unwrap();
        import_file(&disc.join("g.cue"), lib.path(), None).unwrap();
        assert!(lib.path().join("psx/g.bin").is_file());

        remove(lib.path(), &lib.path().join("psx/g.cue")).unwrap();
        assert!(!lib.path().join("psx/g.cue").exists() && !lib.path().join("psx/g.bin").exists());
        // cannot delete things outside the library
        assert!(remove(lib.path(), &disc.join("g.bin")).is_err());
        assert!(disc.join("g.bin").is_file());
    }

    fn ips_patch(offset: usize, bytes: &[u8]) -> Vec<u8> {
        let mut p = b"PATCH".to_vec();
        p.extend_from_slice(&(offset as u32).to_be_bytes()[1..]);
        p.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        p.extend_from_slice(bytes);
        p.extend_from_slice(b"EOF");
        p
    }

    #[test]
    fn a_patched_copy_goes_into_the_library_and_the_original_is_untouched() {
        let src = tempfile::tempdir().unwrap();
        let lib = tempfile::tempdir().unwrap();
        let rom = src.path().join("metroid.gba");
        fs::write(&rom, vec![1u8; 16]).unwrap();
        let patch = src.path().join("Easy Mode.ips");
        fs::write(&patch, ips_patch(4, &[9, 9])).unwrap();

        let r = apply_patch(&rom, &patch, lib.path()).unwrap();
        assert_eq!(r.system, "gba");
        assert!(!r.already_there);
        assert_eq!(r.dest, lib.path().join("gba/metroid [Easy Mode].gba"));
        let out = fs::read(&r.dest).unwrap();
        assert_eq!(&out[3..7], &[1, 9, 9, 1]);
        assert_eq!(
            fs::read(&rom).unwrap(),
            vec![1u8; 16],
            "the original stays as it was"
        );

        // the same files again: nothing is rewritten
        assert!(apply_patch(&rom, &patch, lib.path()).unwrap().already_there);
        // no temporary file is left behind
        let names: Vec<_> = fs::read_dir(lib.path().join("gba"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, vec!["metroid [Easy Mode].gba"]);
    }

    #[test]
    fn patching_refuses_archives_discs_unknown_systems_and_broken_patches() {
        let src = tempfile::tempdir().unwrap();
        let lib = tempfile::tempdir().unwrap();
        let patch = src.path().join("p.ips");
        fs::write(&patch, ips_patch(0, &[1])).unwrap();

        for name in ["game.zip", "game.chd", "game.cue"] {
            let f = src.path().join(name);
            fs::write(&f, b"x").unwrap();
            let e = apply_patch(&f, &patch, lib.path()).unwrap_err();
            assert!(e.contains("cannot patch"), "{name}: {e}");
        }
        let unknown = src.path().join("notes.txt");
        fs::write(&unknown, b"x").unwrap();
        assert!(apply_patch(&unknown, &patch, lib.path())
            .unwrap_err()
            .contains("which system"));

        let rom = src.path().join("zelda.sfc");
        fs::write(&rom, b"0123456789").unwrap();
        let not_a_patch = src.path().join("readme.ips");
        fs::write(&not_a_patch, b"hello").unwrap();
        assert!(apply_patch(&rom, &not_a_patch, lib.path())
            .unwrap_err()
            .contains("not a patch"));
        assert!(
            !lib.path().join("snes").exists()
                || fs::read_dir(lib.path().join("snes"))
                    .unwrap()
                    .next()
                    .is_none()
        );
    }

    #[test]
    fn a_patch_file_is_not_imported_as_a_game_and_the_message_says_what_to_do() {
        let src = tempfile::tempdir().unwrap();
        let lib = tempfile::tempdir().unwrap();
        let patch = src.path().join("Translation.bps");
        fs::write(&patch, b"BPS1").unwrap();
        let e = import_file(&patch, lib.path(), Some("gba")).unwrap_err();
        assert!(e.contains("Apply patch"), "{e}");
    }
}
