//! Importing ROMs into the app's own library folder (`<library>/<system folder>/file`)
//! and removing them again. Importing copies; the original files are never touched.

use crate::registry::{System, SYSTEMS};
use crate::scan;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct Imported {
    pub system: &'static str,
    pub dest: PathBuf,
    /// The same file (same size) was already in the library; nothing was copied.
    pub already_there: bool,
}

fn system_by_id(id: &str) -> Option<&'static System> {
    SYSTEMS.iter().find(|s| s.id == id)
}

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

fn copy_into(src: &Path, dir: &Path) -> Result<(PathBuf, bool), String> {
    let name = src.file_name().ok_or("not a file")?;
    let dest = dir.join(name);
    let size = std::fs::metadata(src).map_err(|e| e.to_string())?.len();
    if std::fs::metadata(&dest).is_ok_and(|m| m.len() == size) {
        return Ok((dest, true));
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
    let name = src
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("not a file")?;
    if !src.is_file() {
        return Err("not a file".into());
    }
    if scan::is_bios_file(name) {
        return Err("this is a BIOS file, add it in Cores & BIOS".into());
    }
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let sys = match hint.and_then(system_by_id) {
        Some(s) if s.extensions.contains(&ext.as_str()) => s,
        Some(s) => return Err(format!("not a {} file (.{ext})", s.name)),
        None => scan::detect_system(src.parent().unwrap_or(Path::new("")), src).ok_or(
            "cannot tell which system it is for: select the system in the sidebar, then import again",
        )?,
    };
    let extra = companions(src)?;
    let dir = library.join(sys.folders[0]);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    for p in &extra {
        copy_into(p, &dir)?;
    }
    let (dest, already_there) = copy_into(src, &dir)?;
    Ok(Imported {
        system: sys.id,
        dest,
        already_there,
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
}
