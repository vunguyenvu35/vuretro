//! A small `.app` bundle to start the game runner from. The runner is a bare executable, and
//! macOS gives a bare executable the generic "exec" icon in the Dock for the moment before the
//! program can set its own icon. A process started from inside a bundle gets the bundle's icon
//! from the first instant, so the runner is copied into one (in the data folder) and started
//! from there. Only macOS uses it; the pieces are plain file work, so they are tested anywhere.

use std::io;
use std::path::{Path, PathBuf};

/// The app icon (256 px), the same one the runner sets from inside.
pub const ICON_PNG: &[u8] = include_bytes!("../../../../src-tauri/icons/128x128@2x.png");

/// Name of the executable inside the bundle.
const EXE: &str = "vuretro-runner";

/// An `.icns` file holding one 256 px PNG image (the `ic08` entry).
pub fn icns_from_png(png: &[u8]) -> Vec<u8> {
    let entry_len = (png.len() + 8) as u32;
    let mut out = Vec::with_capacity(png.len() + 16);
    out.extend_from_slice(b"icns");
    out.extend_from_slice(&(entry_len + 8).to_be_bytes());
    out.extend_from_slice(b"ic08");
    out.extend_from_slice(&entry_len.to_be_bytes());
    out.extend_from_slice(png);
    out
}

/// The `Info.plist` that makes the folder an app with Vuretro's name and icon.
pub fn info_plist() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key><string>{EXE}</string>
  <key>CFBundleIdentifier</key><string>dev.vuvn.vuretro.runner</string>
  <key>CFBundleName</key><string>Vuretro</string>
  <key>CFBundleDisplayName</key><string>Vuretro</string>
  <key>CFBundleIconFile</key><string>icon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
"#
    )
}

/// True when `path` already lives inside an `.app` bundle (an installed Vuretro keeps the
/// runner next to the app, so it already gets the app's icon).
pub fn inside_bundle(path: &Path) -> bool {
    path.components().any(|c| {
        c.as_os_str()
            .to_str()
            .is_some_and(|s| s.to_ascii_lowercase().ends_with(".app"))
    })
}

/// Write a file only when its content differs, so an unchanged bundle is left alone.
fn write_if_changed(path: &Path, content: &[u8]) -> io::Result<()> {
    if std::fs::read(path).is_ok_and(|old| old == content) {
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, content)
}

/// Whether the copy in the bundle must be made again: it is missing, differs in size, or the
/// runner was built after it was copied.
fn stale(src: &Path, dst: &Path) -> bool {
    let (Ok(s), Ok(d)) = (std::fs::metadata(src), std::fs::metadata(dst)) else {
        return true;
    };
    s.len() != d.len()
        || match (s.modified(), d.modified()) {
            (Ok(sm), Ok(dm)) => sm > dm,
            _ => true,
        }
}

/// Make (or refresh) the bundle at `app_dir` around a copy of `runner` and return the path
/// of the executable inside it. The copy is made under a temporary name and renamed, so a
/// running game and an interrupted copy never meet.
pub fn ensure(app_dir: &Path, runner: &Path, icon_png: &[u8]) -> io::Result<PathBuf> {
    let contents = app_dir.join("Contents");
    write_if_changed(&contents.join("Info.plist"), info_plist().as_bytes())?;
    write_if_changed(
        &contents.join("Resources").join("icon.icns"),
        &icns_from_png(icon_png),
    )?;
    let exe = contents.join("MacOS").join(EXE);
    if stale(runner, &exe) {
        let dir = contents.join("MacOS");
        std::fs::create_dir_all(&dir)?;
        let tmp = dir.join(format!(".{EXE}.{}.part", std::process::id()));
        std::fs::copy(runner, &tmp)?;
        std::fs::rename(&tmp, &exe)?;
    }
    Ok(exe)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_file_wraps_the_png_in_an_icns_entry() {
        let icns = icns_from_png(&[1, 2, 3]);
        assert_eq!(&icns[0..4], b"icns");
        assert_eq!(
            u32::from_be_bytes(icns[4..8].try_into().unwrap()) as usize,
            icns.len()
        );
        assert_eq!(&icns[8..12], b"ic08");
        assert_eq!(u32::from_be_bytes(icns[12..16].try_into().unwrap()), 3 + 8);
        assert_eq!(&icns[16..], &[1, 2, 3]);
        // the real icon is a PNG
        assert_eq!(&ICON_PNG[1..4], b"PNG");
    }

    #[test]
    fn the_plist_names_the_executable_and_the_icon() {
        let p = info_plist();
        assert!(p.contains("<string>vuretro-runner</string>"));
        assert!(p.contains("<key>CFBundleIconFile</key><string>icon</string>"));
        assert!(p.contains("<string>Vuretro</string>"));
    }

    #[test]
    fn a_runner_inside_an_app_is_left_where_it_is() {
        assert!(inside_bundle(Path::new(
            "/Applications/Vuretro.app/Contents/MacOS/vuretro-runner"
        )));
        assert!(!inside_bundle(Path::new(
            "/Users/x/vuretro/target/debug/vuretro-runner"
        )));
    }

    #[test]
    fn the_bundle_is_made_with_a_copy_of_the_runner_and_kept_up_to_date() {
        let dir = tempfile::tempdir().unwrap();
        let runner = dir.path().join("runner");
        std::fs::write(&runner, b"first build").unwrap();
        let app = dir.path().join("data/runner/Vuretro.app");

        let exe = ensure(&app, &runner, b"\x89PNG icon").unwrap();
        assert_eq!(exe, app.join("Contents/MacOS/vuretro-runner"));
        assert_eq!(std::fs::read(&exe).unwrap(), b"first build");
        assert!(app.join("Contents/Info.plist").is_file());
        assert!(app.join("Contents/Resources/icon.icns").is_file());

        // nothing changed: the copy is not made again
        let before = std::fs::metadata(&exe).unwrap().modified().unwrap();
        ensure(&app, &runner, b"\x89PNG icon").unwrap();
        assert_eq!(std::fs::metadata(&exe).unwrap().modified().unwrap(), before);

        // a new build (different size) replaces the copy; no temporary file is left
        std::fs::write(&runner, b"second, longer build").unwrap();
        ensure(&app, &runner, b"\x89PNG icon").unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"second, longer build");
        let left: Vec<_> = std::fs::read_dir(app.join("Contents/MacOS"))
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(left, vec!["vuretro-runner"]);
    }

    #[test]
    fn a_missing_runner_is_an_error_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        let app = dir.path().join("Vuretro.app");
        assert!(ensure(&app, &dir.path().join("nope"), b"x").is_err());
    }
}
