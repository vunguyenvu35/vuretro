//! Setting the original Windows version of Road Rash up to run under Wine on this computer.
//!
//! What the game needs, found by running it and reading what it asked for:
//! - a Wine (taken from the computer if it has one; a macOS build of Wine from github.com/Gcenx,
//!   pinned to one version and checked against its SHA-256, when it has none);
//! - a stand-in for `AWEMAN32.DLL`, the manager of the Sound Blaster AWE32, which talks to a
//!   16-bit driver that Wine on macOS cannot run: the stand-in answers every message with an
//!   error, as a computer with no such card would;
//! - a CD-ROM drive (`D:`) that holds the game's files in a `RoadRash` folder;
//! - the entries in the registry that the installer of the disc writes;
//! - a virtual desktop of 640x480, because the game changes the screen mode to 256 colours.

use super::native_installer::{find_tool, run, search_path};
use crate::iso::Iso;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use vuretro_core::paths::Paths;
use vuretro_core::ports::NativeSpec;

/// The Wine that is fetched when the computer has none: a build for macOS, of one version.
const WINE_URL: &str = "https://github.com/Gcenx/macOS_Wine_builds/releases/download/11.21/wine-staging-11.21-osx64.tar.xz";
const WINE_SHA256: &str = "d7bcd5abf4df27aae397a7898aef074fea4a2e0b51e0a55aa9659517bbf9d452";
const WINE_BYTES: u64 = 228_148_688;
/// cnc-ddraw (github.com/FunkyFr3sh/cnc-ddraw, MIT): a replacement for `ddraw.dll` that draws a
/// DirectDraw game in an ordinary window, with the title bar and buttons of the Mac. Fetched
/// when a game is set up, pinned to one version and checked against its SHA-256.
const DDRAW_URL: &str =
    "https://github.com/FunkyFr3sh/cnc-ddraw/releases/download/v7.1.0.0/cnc-ddraw.zip";
const DDRAW_SHA256: &str = "0b13ab89a64c9918189b1dadd449ef6ed3cb3b7b19cabd96d8adbd95505bb908";
const DDRAW_MAX_BYTES: u64 = 8 << 20;
/// Where the program of that Wine is, from the folder it is unpacked in.
const WINE_PROGRAM: &str = "Wine Staging.app/Contents/Resources/wine/bin/wine";

/// A stand-in for `AWEMAN32.DLL`: a minimal PE32 DLL whose only export, ordinal 2
/// `AWEManager` (stdcall, four DWORD arguments), returns the error 0x65 for every message. A
/// program that needs the AWE32 driver then goes on as if there were no such card.
pub fn aweman32_stub() -> Vec<u8> {
    const IMAGE_BASE: u32 = 0x1000_0000;
    const SECT_RVA: u32 = 0x1000;
    const FILE_ALIGN: usize = 0x200;
    let rva = |off: usize| SECT_RVA + off as u32;

    let mut section = vec![0u8; FILE_ALIGN];
    section[0x00..0x08].copy_from_slice(&[0xB8, 1, 0, 0, 0, 0xC2, 0x0C, 0x00]); // DllMain: eax = 1
    section[0x10..0x18].copy_from_slice(&[0xB8, 0x65, 0, 0, 0, 0xC2, 0x10, 0x00]); // eax = 0x65

    let (dll_name, fn_name) = (b"AWEMAN32.dll\0", b"AWEManager\0");
    let exp = 0x20usize;
    let (funcs, names, ords) = (exp + 40, exp + 44, exp + 48);
    let name_at = ords + 4;
    let fn_name_at = name_at + dll_name.len();
    let end = fn_name_at + fn_name.len();

    let mut dir = Vec::new();
    for v in [0u32, 0] {
        dir.extend_from_slice(&v.to_le_bytes());
    }
    for v in [0u16, 0] {
        dir.extend_from_slice(&v.to_le_bytes());
    }
    for v in [rva(name_at), 2, 1, 1, rva(funcs), rva(names), rva(ords)] {
        dir.extend_from_slice(&v.to_le_bytes());
    }
    section[exp..exp + 40].copy_from_slice(&dir);
    section[funcs..funcs + 4].copy_from_slice(&rva(0x10).to_le_bytes());
    section[names..names + 4].copy_from_slice(&rva(fn_name_at).to_le_bytes());
    section[ords..ords + 2].copy_from_slice(&0u16.to_le_bytes());
    section[name_at..name_at + dll_name.len()].copy_from_slice(dll_name);
    section[fn_name_at..fn_name_at + fn_name.len()].copy_from_slice(fn_name);

    let mut h: Vec<u8> = Vec::new();
    let mut dos = vec![0u8; 0x40];
    dos[0..2].copy_from_slice(b"MZ");
    dos[0x3C..0x40].copy_from_slice(&0x40u32.to_le_bytes());
    h.extend_from_slice(&dos);
    h.extend_from_slice(b"PE\0\0");
    // the file header: i386, one section, 224 bytes of optional header, a DLL
    for v in [0x14Cu16, 1] {
        h.extend_from_slice(&v.to_le_bytes());
    }
    for v in [0u32, 0, 0] {
        h.extend_from_slice(&v.to_le_bytes());
    }
    for v in [0xE0u16, 0x2102] {
        h.extend_from_slice(&v.to_le_bytes());
    }
    // the optional header
    h.extend_from_slice(&0x10Bu16.to_le_bytes());
    h.extend_from_slice(&[14, 0]);
    for v in [
        0x200u32,
        0,
        0,
        SECT_RVA,
        SECT_RVA,
        0x2000,
        IMAGE_BASE,
        0x1000,
        FILE_ALIGN as u32,
    ] {
        h.extend_from_slice(&v.to_le_bytes());
    }
    for v in [4u16, 0, 0, 0, 4, 0] {
        h.extend_from_slice(&v.to_le_bytes());
    }
    for v in [0u32, 0x2000, 0x200, 0] {
        h.extend_from_slice(&v.to_le_bytes());
    }
    for v in [2u16, 0] {
        h.extend_from_slice(&v.to_le_bytes());
    }
    for v in [0x10_0000u32, 0x1000, 0x10_0000, 0x1000, 0, 16] {
        h.extend_from_slice(&v.to_le_bytes());
    }
    // the data directories: the export table first
    let mut dirs = vec![0u8; 16 * 8];
    dirs[0..4].copy_from_slice(&rva(exp).to_le_bytes());
    dirs[4..8].copy_from_slice(&((end - exp) as u32).to_le_bytes());
    h.extend_from_slice(&dirs);
    // the section header
    h.extend_from_slice(b".text\0\0\0");
    for v in [0x200u32, SECT_RVA, 0x200, FILE_ALIGN as u32, 0, 0] {
        h.extend_from_slice(&v.to_le_bytes());
    }
    for v in [0u16, 0] {
        h.extend_from_slice(&v.to_le_bytes());
    }
    h.extend_from_slice(&0x6000_0020u32.to_le_bytes());
    h.resize(FILE_ALIGN, 0);
    h.extend_from_slice(&section);
    h
}

/// The folder of the game in a folder of files: the folder itself, or the `ROADRASH` folder in it.
fn game_folder(folder: &Path) -> Option<PathBuf> {
    let has_exe = |d: &Path| {
        std::fs::read_dir(d).is_ok_and(|rd| {
            rd.flatten().any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .eq_ignore_ascii_case("ROADRASH.EXE")
            })
        })
    };
    if has_exe(folder) {
        return Some(folder.to_path_buf());
    }
    std::fs::read_dir(folder)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.is_dir()
                && p.file_name()
                    .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case("ROADRASH"))
                && has_exe(p)
        })
}

fn copy_tree(from: &Path, to: &Path, count: &mut usize, log: &dyn Fn(&str)) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for e in std::fs::read_dir(from)
        .map_err(|e| e.to_string())?
        .flatten()
    {
        let target = to.join(e.file_name());
        if e.path().is_dir() {
            copy_tree(&e.path(), &target, count, log)?;
        } else {
            std::fs::copy(e.path(), &target).map_err(|e| format!("{}: {e}", target.display()))?;
            *count += 1;
            if *count % 50 == 0 {
                log(&format!("  {count} files"));
            }
        }
    }
    Ok(())
}

/// Copy the game's files (the `ROADRASH` folder of the disc) from a disc image or a folder.
fn copy_game(disc: &Path, dest: &Path, log: &dyn Fn(&str)) -> Result<usize, String> {
    let mut count = 0;
    if disc.is_dir() {
        let root = game_folder(disc).ok_or(
            "that folder does not hold the game (no ROADRASH.EXE in it or in a ROADRASH folder)",
        )?;
        copy_tree(&root, dest, &mut count, log)?;
        return Ok(count);
    }
    let iso = Iso::open(disc)?;
    let prefix = if iso.find("ROADRASH.EXE").is_ok() {
        ""
    } else if iso.find("ROADRASH/ROADRASH.EXE").is_ok() {
        "ROADRASH"
    } else {
        return Err("this disc image does not hold Road Rash (no ROADRASH.EXE)".into());
    };
    for file in iso.files_under(prefix)? {
        let from = if prefix.is_empty() {
            file.clone()
        } else {
            format!("{prefix}/{file}")
        };
        let target = dest.join(&file);
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&target, iso.read(&from)?)
            .map_err(|e| format!("{}: {e}", target.display()))?;
        count += 1;
        if count % 50 == 0 {
            log(&format!("  {count} files"));
        }
    }
    Ok(count)
}

/// A Wine on this computer, if there is one: asked for in `VURETRO_WINE`, the one this app
/// fetched earlier, one on the search path, or CrossOver's.
fn locate_wine(paths: &Paths) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(p) = std::env::var_os("VURETRO_WINE") {
        candidates.push(PathBuf::from(p));
    }
    candidates.push(paths.native_dir().join("wine").join(WINE_PROGRAM));
    candidates.extend(["wine", "wine64"].iter().filter_map(|n| find_tool(n)));
    candidates.push(PathBuf::from(
        "/Applications/CrossOver.app/Contents/SharedSupport/CrossOver/bin/wine",
    ));
    candidates.into_iter().find(|p| p.is_file())
}

/// Fetch the pinned Wine into `<data dir>/native/wine`, checking its SHA-256.
fn fetch_wine(paths: &Paths, log: &dyn Fn(&str)) -> Result<PathBuf, String> {
    if !cfg!(target_os = "macos") {
        return Err("Install Wine with your package manager (`wine` was not found)".into());
    }
    let dir = paths.native_dir().join("wine");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let archive = dir.join("wine.tar.xz");
    log(&format!(
        "Downloading Wine ({} MB) from github.com/Gcenx",
        WINE_BYTES / 1_000_000
    ));
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(std::time::Duration::from_secs(20)))
        .timeout_recv_response(Some(std::time::Duration::from_secs(60)))
        .timeout_global(Some(std::time::Duration::from_secs(3600)))
        .build()
        .into();
    let mut resp = agent
        .get(WINE_URL)
        .call()
        .map_err(|e| format!("download failed: {e}"))?;
    let mut reader = resp.body_mut().as_reader();
    let mut file = std::fs::File::create(&archive).map_err(|e| e.to_string())?;
    let (mut hash, mut got, mut shown) = (Sha256::new(), 0u64, 0u64);
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("download failed: {e}"))?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        got += n as u64;
        if got > WINE_BYTES + (8 << 20) {
            let _ = std::fs::remove_file(&archive);
            return Err("the download is bigger than it should be".into());
        }
        if got * 10 / WINE_BYTES > shown {
            shown = got * 10 / WINE_BYTES;
            log(&format!("  {}%", shown * 10));
        }
    }
    drop(file);
    let digest: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if digest != WINE_SHA256 {
        let _ = std::fs::remove_file(&archive);
        return Err(format!(
            "the file that was downloaded is not the one expected (SHA-256 {digest})"
        ));
    }
    log("Checked: it is the file expected. Unpacking");
    run(
        Command::new("tar")
            .arg("-xf")
            .arg(&archive)
            .arg("-C")
            .arg(&dir),
        log,
    )?;
    let _ = std::fs::remove_file(&archive);
    let _ = Command::new("xattr")
        .args(["-dr", "com.apple.quarantine"])
        .arg(dir.join("Wine Staging.app"))
        .env("PATH", search_path())
        .status();
    let wine = dir.join(WINE_PROGRAM);
    if wine.is_file() {
        Ok(wine)
    } else {
        Err("Wine was unpacked but its program is not where it should be".into())
    }
}

/// How to start the game: Wine, in the game's folder. `program` is a Windows path on the CD
/// drive. With `framed` the game draws in a window of its own (cnc-ddraw); without, Wine puts it
/// on a virtual desktop of `size`, which has no title bar.
fn spec(wine: &Path, root: &Path, program: &str, size: &str, framed: bool) -> NativeSpec {
    let (args, overrides) = if framed {
        (vec![program.to_string()], "mscoree,mshtml=;ddraw=n,b")
    } else {
        (
            vec![
                "explorer".to_string(),
                format!("/desktop=Game,{size}"),
                program.to_string(),
            ],
            "mscoree,mshtml=",
        )
    };
    NativeSpec {
        exe: wine.to_path_buf(),
        args,
        cwd: Some(root.join("game")),
        env: vec![
            (
                "WINEPREFIX".into(),
                root.join("prefix").to_string_lossy().into_owned(),
            ),
            ("WINEDLLOVERRIDES".into(), overrides.into()),
            ("WINEDEBUG".into(), "-all".into()),
        ],
        // the server that Wine keeps for the prefix outlives the program
        stop: wine
            .parent()
            .map(|bin| bin.join("wineserver").to_string_lossy().into_owned())
            .map(|server| vec![server, "-k".into()]),
    }
}

/// The `ddraw.dll` of cnc-ddraw, fetched once into `<data dir>/native/wine/cnc-ddraw`.
fn ddraw_dll(paths: &Paths, log: &dyn Fn(&str)) -> Result<PathBuf, String> {
    let dir = paths.native_dir().join("wine").join("cnc-ddraw");
    let dll = dir.join("ddraw.dll");
    if dll.is_file() {
        return Ok(dll);
    }
    log("Getting cnc-ddraw (a window for the game)");
    let mut resp = ureq::get(DDRAW_URL)
        .call()
        .map_err(|e| format!("download failed: {e}"))?;
    let mut bytes = Vec::new();
    resp.body_mut()
        .as_reader()
        .take(DDRAW_MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("download failed: {e}"))?;
    if bytes.len() as u64 > DDRAW_MAX_BYTES {
        return Err("the download is bigger than it should be".into());
    }
    let digest: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if digest != DDRAW_SHA256 {
        return Err(format!("not the file expected (SHA-256 {digest})"));
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut file = zip.by_name("ddraw.dll").map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    file.read_to_end(&mut out).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::fs::write(&dll, out).map_err(|e| e.to_string())?;
    Ok(dll)
}

/// The settings of cnc-ddraw for a game that asks for a screen of `size` (`640x480`): a window
/// one and a half times as big, with its frame.
fn ddraw_ini(size: &str) -> String {
    let (w, h) = size.split_once('x').unwrap_or(("640", "480"));
    let scaled = |n: &str| n.parse::<u32>().unwrap_or(640) * 3 / 2;
    format!(
        "[ddraw]\nwindowed=true\nborder=true\nfullscreen=false\nmaintas=true\nrenderer=gdi\n; the mouse is not kept inside the window\ndevmode=true\nwidth={}\nheight={}\nposX=-32000\nposY=-32000\n",
        scaled(w),
        scaled(h)
    )
}

/// Put cnc-ddraw in the game's folder so it opens in a window with a frame. `false` (and a line
/// in the log) when it cannot be had: the game then runs on a virtual desktop.
fn add_window_frame(paths: &Paths, game: &Path, size: &str, log: &dyn Fn(&str)) -> bool {
    let done = ddraw_dll(paths, log)
        .and_then(|dll| std::fs::copy(dll, game.join("ddraw.dll")).map_err(|e| e.to_string()))
        .and_then(|_| {
            std::fs::write(game.join("ddraw.ini"), ddraw_ini(size)).map_err(|e| e.to_string())
        });
    if let Err(e) = &done {
        log(&format!("No window frame this time: {e}"));
    }
    done.is_ok()
}

/// What is known about the disc: Road Rash needs a few things done for it (found by running
/// it); any other disc is tried as it is.
#[derive(Debug, PartialEq)]
enum Recipe {
    RoadRash,
    Generic,
}

fn recipe_for(disc: &Path) -> Result<Recipe, String> {
    let road_rash = if disc.is_dir() {
        game_folder(disc).is_some()
    } else {
        let iso = Iso::open(disc)?;
        iso.find("ROADRASH.EXE").is_ok() || iso.find("ROADRASH/ROADRASH.EXE").is_ok()
    };
    Ok(if road_rash {
        Recipe::RoadRash
    } else {
        Recipe::Generic
    })
}

/// The whole disc (an image or a folder) copied into `dest`.
fn copy_disc(disc: &Path, dest: &Path, log: &dyn Fn(&str)) -> Result<usize, String> {
    let mut count = 0;
    if disc.is_dir() {
        copy_tree(disc, dest, &mut count, log)?;
        return Ok(count);
    }
    let iso = Iso::open(disc)?;
    for file in iso.files_under("")? {
        let target = dest.join(&file);
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&target, iso.read(&file)?)
            .map_err(|e| format!("{}: {e}", target.display()))?;
        count += 1;
        if count % 50 == 0 {
            log(&format!("  {count} files"));
        }
    }
    Ok(count)
}

/// `rel` (with `\` or `/`) inside `root`, matched without regard to letter case; the names as
/// they are on disk.
fn resolve_ci(root: &Path, rel: &str) -> Option<Vec<String>> {
    let mut dir = root.to_path_buf();
    let mut names = Vec::new();
    for part in rel
        .split(['\\', '/'])
        .filter(|p| !p.is_empty() && *p != ".")
    {
        let found = std::fs::read_dir(&dir)
            .ok()?
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .find(|n| n.eq_ignore_ascii_case(part))?;
        dir.push(&found);
        names.push(found);
    }
    (!names.is_empty() && dir.is_file()).then_some(names)
}

/// The program of a disc: the one its `autorun.inf` names, else the only program in its first
/// folder level. A Windows path on the CD drive.
fn find_program(game: &Path) -> Result<String, String> {
    let windows = |names: Vec<String>| format!("D:\\{}", names.join("\\"));
    if let Some(autorun) = resolve_ci(game, "autorun.inf") {
        let text = std::fs::read(game.join(autorun.join("/")))
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            if !key.trim().eq_ignore_ascii_case("open") {
                continue;
            }
            let value = value.trim();
            let end = value.to_ascii_lowercase().find(".exe").map(|i| i + 4);
            if let Some(names) = end.and_then(|e| resolve_ci(game, &value[..e])) {
                return Ok(windows(names));
            }
        }
    }
    let mut programs: Vec<String> = std::fs::read_dir(game)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.to_ascii_lowercase().ends_with(".exe"))
        .collect();
    programs.sort();
    match programs.as_slice() {
        [one] => Ok(windows(vec![one.clone()])),
        [] => Err("there is no program on this disc to start".into()),
        many => Err(format!(
            "cannot tell which program to start on this disc ({})",
            many.join(", ")
        )),
    }
}

/// The arguments of `wine reg add` for the entries that the installer of the disc writes.
fn registry_commands(recipe: &Recipe) -> Vec<Vec<String>> {
    let add = |key: &str, rest: &[&str], view32: bool| {
        let mut c: Vec<String> = vec!["reg".into(), "add".into(), key.into()];
        c.extend(rest.iter().map(|s| s.to_string()));
        c.push("/f".into());
        if view32 {
            // the game is a 32-bit program: it reads the 32-bit view of the registry
            c.push("/reg:32".into());
        }
        c
    };
    // the CD-ROM drive
    let mut commands = vec![add(
        "HKLM\\Software\\Wine\\Drives",
        &["/v", "d:", "/d", "cdrom"],
        false,
    )];
    if *recipe != Recipe::RoadRash {
        return commands;
    }
    for root in ["HKCU", "HKLM"] {
        let key = format!("{root}\\Software\\Electronic Arts\\RoadRash 95");
        commands.push(add(
            &key,
            &["/v", "Path", "/t", "REG_SZ", "/d", "C:\\RoadRash\\"],
            true,
        ));
        for (name, value) in [
            ("HiRes", "1"),
            ("HiResBikers", "1"),
            ("HiResCars", "1"),
            ("Inset", "0"),
            // no video at the start: its window has no title bar and the game cannot be closed
            // while it plays
            ("Intro", "0"),
        ] {
            commands.push(add(
                &key,
                &["/v", name, "/t", "REG_DWORD", "/d", value],
                true,
            ));
        }
    }
    commands
}

/// The program of Road Rash as it is on the disc (an image, or a folder it was unpacked to).
fn fresh_road_rash_exe(disc: &Path) -> Result<Vec<u8>, String> {
    if disc.is_dir() {
        let folder = game_folder(disc).ok_or("the disc folder does not hold the game")?;
        let names = resolve_ci(&folder, "ROADRASH.EXE").ok_or("ROADRASH.EXE is not there")?;
        return std::fs::read(folder.join(names.join("/"))).map_err(|e| e.to_string());
    }
    let iso = Iso::open(disc)?;
    let path = ["ROADRASH/ROADRASH.EXE", "ROADRASH.EXE"]
        .into_iter()
        .find(|p| iso.find(p).is_ok())
        .ok_or("ROADRASH.EXE is not on the disc")?;
    iso.read(path)
}

/// Give the program in `exe` an icon that looks like a Mac icon (see [`crate::pe_icon`]).
fn mac_icon_for(exe: &Path, icon: Option<&[u8]>) -> Result<(), String> {
    let mut bytes = std::fs::read(exe).map_err(|e| e.to_string())?;
    match icon {
        Some(rgba) => crate::pe_icon::set_icon(&mut bytes, rgba)?,
        None => crate::pe_icon::mac_style_icon(&mut bytes)?,
    }
    std::fs::write(exe, bytes).map_err(|e| e.to_string())
}

/// Whether a Wine is on this computer (the one this app fetched, or another).
pub fn ready(paths: &Paths) -> bool {
    locate_wine(paths).is_some()
}

/// Get Wine when the computer has none (nothing to do when it has one).
pub fn install_runtime(paths: &Paths, log: &dyn Fn(&str)) -> Result<(), String> {
    if cfg!(windows) {
        return Err("Windows runs these games by itself".into());
    }
    if cfg!(target_os = "macos")
        && std::env::consts::ARCH == "aarch64"
        && !Command::new("arch")
            .args(["-x86_64", "/usr/bin/true"])
            .status()
            .is_ok_and(|s| s.success())
    {
        return Err("Rosetta 2 is needed to run Wine on this Mac: run `softwareupdate --install-rosetta` in a terminal".into());
    }
    if let Some(w) = locate_wine(paths) {
        log(&format!("Using {}", w.display()));
        return Ok(());
    }
    fetch_wine(paths, log).map(|_| ())
}

/// A folder name for a disc image: its file name without the extension, in safe letters.
fn game_id(disc: &Path) -> String {
    let stem = disc
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    let id = stem
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if id.is_empty() {
        "game".into()
    } else {
        id
    }
}

/// Make the game on `disc` ready to start under Wine, and say how to start it. The first time
/// this builds an environment and copies the game's files (a minute or two); later it only
/// finds them again.
pub fn prepare(
    paths: &Paths,
    disc: &Path,
    icon: Option<&[u8]>,
    log: &dyn Fn(&str),
) -> Result<NativeSpec, String> {
    let wine = locate_wine(paths).ok_or("Wine is not on this computer")?;
    let root = paths.native_dir().join("pc").join(game_id(disc));
    let done = root.join("ready");
    // `ready` holds the program to start, the size of the screen it asks for and whether it
    // has a window frame
    let ready_lines: Vec<String> = std::fs::read_to_string(&done)
        .map(|t| t.lines().map(String::from).collect())
        .unwrap_or_default();
    if let [program, size, rest @ ..] = ready_lines.as_slice() {
        let mut framed = rest.first().is_some_and(|m| m == "framed");
        if rest.is_empty() {
            // set up before the frame existed: add it without doing the rest again
            framed = add_window_frame(paths, &root.join("game"), size, log);
            let mode = if framed { "framed" } else { "desktop" };
            let _ = std::fs::write(&done, format!("{program}\n{size}\n{mode}\n"));
        }
        // a game set up with the icon of the program (or none) gets the app's icon with the
        // badge of the system: the program is taken from the disc again, as the icon is
        // written over its own
        let is_road_rash = program.to_ascii_uppercase().ends_with("ROADRASH.EXE");
        if let (Some(icon), true) = (icon, is_road_rash) {
            if !rest.iter().any(|f| f == "appicon") {
                let exe = root.join("game").join("ROADRASH.EXE");
                let done_icon = fresh_road_rash_exe(disc)
                    .and_then(|bytes| std::fs::write(&exe, bytes).map_err(|e| e.to_string()))
                    .and_then(|_| mac_icon_for(&exe, Some(icon)));
                match done_icon {
                    Ok(()) => {
                        let mode = if framed { "framed" } else { "desktop" };
                        let _ =
                            std::fs::write(&done, format!("{program}\n{size}\n{mode}\nappicon\n"));
                    }
                    Err(e) => log(&format!("The game keeps its old icon: {e}")),
                }
            }
        }
        return Ok(spec(&wine, &root, program, size, framed));
    }
    let recipe = recipe_for(disc)?;
    let (game, prefix, cdroot) = (root.join("game"), root.join("prefix"), root.join("cdroot"));
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let env = [
        ("WINEPREFIX", prefix.to_string_lossy().into_owned()),
        ("WINEDLLOVERRIDES", "mscoree,mshtml=".to_string()),
        ("WINEDEBUG", "-all".to_string()),
    ];
    let wine_run = |args: &[String]| {
        run(
            Command::new(&wine)
                .args(args)
                .envs(env.iter().map(|(k, v)| (*k, v))),
            &|_| {},
        )
    };

    log("Making a Windows environment for the game (about a minute)");
    if !prefix.join("system.reg").is_file() {
        wine_run(&["wineboot".into(), "--init".into()])?;
    }

    log("Copying the game's files from the disc");
    if game.exists() {
        std::fs::remove_dir_all(&game).map_err(|e| e.to_string())?;
    }
    let (program, size) = match recipe {
        Recipe::RoadRash => {
            copy_game(disc, &game, log)?;
            std::fs::write(game.join("AWEMAN32.DLL"), aweman32_stub())
                .map_err(|e| e.to_string())?;
            // its Dock icon: the program's own, drawn as a Mac icon (best effort)
            if let Err(e) = mac_icon_for(&game.join("ROADRASH.EXE"), icon) {
                log(&format!("The game keeps its old icon: {e}"));
            }
            std::fs::create_dir_all(prefix.join("drive_c").join("RoadRash"))
                .map_err(|e| e.to_string())?;
            ("D:\\RoadRash\\ROADRASH.EXE".to_string(), "640x480")
        }
        Recipe::Generic => {
            copy_disc(disc, &game, log)?;
            (find_program(&game)?, "800x600")
        }
    };

    log("Setting up the CD drive");
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let drive = prefix.join("dosdevices").join("d:");
        let _ = std::fs::remove_file(&drive);
        match recipe {
            Recipe::RoadRash => {
                // the disc's game folder shows up as D:\RoadRash
                let _ = std::fs::remove_dir_all(&cdroot);
                std::fs::create_dir_all(&cdroot).map_err(|e| e.to_string())?;
                symlink("../game", cdroot.join("ROADRASH")).map_err(|e| e.to_string())?;
                std::fs::write(cdroot.join(".windows-label"), "ROADRASH")
                    .map_err(|e| e.to_string())?;
                symlink(&cdroot, &drive).map_err(|e| e.to_string())?;
            }
            Recipe::Generic => {
                std::fs::write(game.join(".windows-label"), game_id(disc).to_uppercase())
                    .map_err(|e| e.to_string())?;
                symlink(&game, &drive).map_err(|e| e.to_string())?;
            }
        }
    }
    for command in registry_commands(&recipe) {
        wine_run(&command)?;
    }
    let framed = add_window_frame(paths, &game, size, log);
    let mode = if framed { "framed" } else { "desktop" };
    let flags = if recipe == Recipe::RoadRash && icon.is_some() {
        "appicon\n"
    } else {
        ""
    };
    std::fs::write(&done, format!("{program}\n{size}\n{mode}\n{flags}"))
        .map_err(|e| e.to_string())?;
    Ok(spec(&wine, &root, &program, size, framed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stand_in_dll_is_the_one_that_was_shown_to_work() {
        // the bytes of the file that made the game start under Wine: its MD5
        let dll = aweman32_stub();
        assert_eq!(
            format!("{:x}", md5::compute(&dll)),
            "303f3a630b1c5c05004b24a78e5807ed"
        );
        assert_eq!(&dll[..2], b"MZ");
        assert_eq!(&dll[0x40..0x44], b"PE\0\0");
        assert!(dll.windows(10).any(|w| w == b"AWEManager"));
        // the code of the one export: mov eax, 0x65 ; ret 0x10
        assert_eq!(
            &dll[0x200 + 0x10..0x200 + 0x18],
            &[0xB8, 0x65, 0, 0, 0, 0xC2, 0x10, 0]
        );
        assert_eq!(dll.len(), 0x400);
    }

    #[test]
    fn the_game_starts_wine_in_its_own_folder_framed_or_on_a_virtual_desktop() {
        let root = Path::new("/data/pc/road-rash");
        let wine = Path::new("/opt/wine/bin/wine");
        let desktop = spec(wine, root, "D:\\GAME.EXE", "800x600", false);
        assert_eq!(desktop.exe, wine);
        assert_eq!(
            desktop.args,
            ["explorer", "/desktop=Game,800x600", "D:\\GAME.EXE"]
        );
        assert_eq!(desktop.cwd.as_deref(), Some(root.join("game").as_path()));
        assert!(desktop
            .env
            .iter()
            .any(|(k, v)| k == "WINEPREFIX" && v == "/data/pc/road-rash/prefix"));
        let framed = spec(wine, root, "D:\\GAME.EXE", "800x600", true);
        assert_eq!(framed.args, ["D:\\GAME.EXE"]);
        assert!(framed
            .env
            .iter()
            .any(|(k, v)| k == "WINEDLLOVERRIDES" && v.contains("ddraw=n,b")));
        assert_eq!(
            framed.stop.as_deref(),
            Some(&["/opt/wine/bin/wineserver".to_string(), "-k".to_string()][..])
        );
    }

    #[test]
    fn the_window_of_a_game_is_one_and_a_half_times_its_screen() {
        let ini = ddraw_ini("640x480");
        assert!(ini.contains("width=960") && ini.contains("height=720"));
        assert!(ini.contains("border=true") && ini.contains("windowed=true"));
        assert!(
            ini.contains("devmode=true"),
            "the mouse can leave the window"
        );
    }

    #[test]
    fn a_game_gets_a_folder_name_from_its_disc_image() {
        assert_eq!(
            game_id(Path::new("/x/Road Rash (1996).iso")),
            "road-rash-1996"
        );
        assert_eq!(game_id(Path::new("/x/().iso")), "game");
    }

    #[test]
    fn the_registry_entries_cover_the_drive_and_the_settings_the_game_reads() {
        let commands = registry_commands(&Recipe::RoadRash);
        let text: Vec<String> = commands.iter().map(|c| c.join(" ")).collect();
        assert!(text
            .iter()
            .any(|c| c.contains("Wine\\Drives") && c.contains("d: ") && c.contains("cdrom")));
        for root in ["HKCU", "HKLM"] {
            assert!(text.iter().any(|c| c.starts_with(&format!(
                "reg add {root}\\Software\\Electronic Arts\\RoadRash 95"
            )) && c.contains("Path")));
        }
        assert!(
            text.iter().all(|c| c.contains(" /f")),
            "never asks a question"
        );
        assert!(text
            .iter()
            .filter(|c| c.contains("RoadRash 95"))
            .all(|c| c.ends_with("/reg:32")));
        assert!(text
            .iter()
            .any(|c| c.contains("HiResBikers") && c.contains("REG_DWORD")));
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("vuretro-wine-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_games_files_are_copied_from_a_folder_or_from_its_roadrash_folder() {
        let disc = temp("disc");
        std::fs::create_dir_all(disc.join("RoadRash/DATA")).unwrap();
        std::fs::write(disc.join("RoadRash/ROADRASH.EXE"), "exe").unwrap();
        std::fs::write(disc.join("RoadRash/DATA/X.BIN"), "x").unwrap();
        std::fs::create_dir_all(disc.join("SETUP")).unwrap();
        std::fs::write(disc.join("SETUP/OTHER.DLL"), "no").unwrap();
        let dest = temp("dest");
        assert_eq!(copy_game(&disc, &dest.join("g"), &|_| {}).unwrap(), 2);
        assert!(dest.join("g/DATA/X.BIN").is_file());
        assert!(!dest.join("g/SETUP").exists(), "only the game's own folder");
        // the folder itself
        let direct = disc.join("RoadRash");
        let dest2 = temp("dest2");
        assert_eq!(copy_game(&direct, &dest2.join("g"), &|_| {}).unwrap(), 2);
        // a folder that is not the game
        let other = temp("other");
        assert!(copy_game(&other, &other.join("g"), &|_| {}).is_err());
        for d in [disc, dest, dest2, other] {
            std::fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn a_file_that_is_not_a_disc_image_is_refused() {
        let dir = temp("notiso");
        let file = dir.join("x.iso");
        std::fs::write(&file, vec![0u8; 40 * 2048]).unwrap();
        assert!(copy_game(&file, &dir.join("g"), &|_| {}).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Needs a real Wine (`VURETRO_WINE`) and the game's disc image (`VURETRO_ROADRASH_ISO`):
    /// `cargo test -p vuretro-library prepares_the_real_game -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn prepares_the_real_game() {
        let iso = PathBuf::from(std::env::var("VURETRO_ROADRASH_ISO").unwrap());
        let dir = temp("real");
        let paths = Paths::new(dir.to_str().unwrap());
        assert!(ready(&paths));
        let spec = prepare(&paths, &iso, None, &|l| println!("{l}")).unwrap();
        println!("{spec:?}");
        assert!(spec.cwd.unwrap().join("ROADRASH.EXE").is_file());
        // the second time it is only found again
        let t = std::time::Instant::now();
        prepare(&paths, &iso, None, &|_| {}).unwrap();
        assert!(t.elapsed().as_secs() < 2);
        println!("kept in {}", dir.display());
    }

    #[test]
    fn a_disc_is_road_rash_only_when_it_holds_its_program_and_anything_else_is_tried_as_it_is() {
        let rr = temp("rr");
        std::fs::create_dir_all(rr.join("ROADRASH")).unwrap();
        std::fs::write(rr.join("ROADRASH/ROADRASH.EXE"), "x").unwrap();
        assert_eq!(recipe_for(&rr).unwrap(), Recipe::RoadRash);
        let other = temp("other-disc");
        std::fs::write(other.join("GAME.EXE"), "x").unwrap();
        assert_eq!(recipe_for(&other).unwrap(), Recipe::Generic);
        assert!(recipe_for(&other.join("nothing.iso")).is_err());
        // only Road Rash gets its registry entries
        assert_eq!(registry_commands(&Recipe::Generic).len(), 1);
        for d in [rr, other] {
            std::fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn the_program_of_a_disc_is_the_one_autorun_names_or_the_only_one() {
        let d = temp("prog");
        std::fs::create_dir_all(d.join("Bin")).unwrap();
        std::fs::write(d.join("Bin/Play.EXE"), "x").unwrap();
        std::fs::write(d.join("SETUP.EXE"), "x").unwrap();
        std::fs::write(d.join("OTHER.EXE"), "x").unwrap();
        // two programs and no autorun: not guessed
        let err = find_program(&d).unwrap_err();
        assert!(
            err.contains("OTHER.EXE") && err.contains("SETUP.EXE"),
            "{err}"
        );
        // autorun names one, in any letter case and with arguments
        std::fs::write(
            d.join("AUTORUN.INF"),
            "[autorun]\r\nOpen=bin\\play.exe /go\r\n",
        )
        .unwrap();
        assert_eq!(find_program(&d).unwrap(), "D:\\Bin\\Play.EXE");
        std::fs::remove_file(d.join("AUTORUN.INF")).unwrap();
        std::fs::remove_file(d.join("OTHER.EXE")).unwrap();
        assert_eq!(find_program(&d).unwrap(), "D:\\SETUP.EXE");
        std::fs::remove_dir_all(&d).unwrap();
        let empty = temp("noprog");
        assert!(find_program(&empty).is_err());
        std::fs::remove_dir_all(empty).unwrap();
    }

    #[test]
    fn a_whole_disc_folder_is_copied_for_any_game() {
        let disc = temp("whole");
        std::fs::create_dir_all(disc.join("DATA")).unwrap();
        std::fs::write(disc.join("GAME.EXE"), "x").unwrap();
        std::fs::write(disc.join("DATA/A.DAT"), "x").unwrap();
        let dest = temp("whole-dest");
        assert_eq!(copy_disc(&disc, &dest.join("g"), &|_| {}).unwrap(), 2);
        assert!(dest.join("g/DATA/A.DAT").is_file());
        for d in [disc, dest] {
            std::fs::remove_dir_all(d).unwrap();
        }
    }

    /// Needs the network and a Wine (`VURETRO_WINE`): a game set up before the window frame
    /// existed gets cnc-ddraw (checked against its SHA-256) without being set up again.
    #[test]
    #[ignore]
    fn a_game_set_up_earlier_gets_the_window_frame() {
        let dir = temp("upgrade");
        let paths = Paths::new(dir.to_str().unwrap());
        let root = paths.native_dir().join("pc").join("g");
        std::fs::create_dir_all(root.join("game")).unwrap();
        std::fs::write(root.join("ready"), "D:\\X.EXE\n640x480\n").unwrap();
        let spec = prepare(&paths, Path::new("/x/g.iso"), None, &|l| println!("{l}")).unwrap();
        assert_eq!(spec.args, ["D:\\X.EXE"]);
        assert!(root.join("game/ddraw.dll").metadata().unwrap().len() > 100_000);
        assert!(std::fs::read_to_string(root.join("game/ddraw.ini"))
            .unwrap()
            .contains("width=960"));
        assert!(std::fs::read_to_string(root.join("ready"))
            .unwrap()
            .ends_with("framed\n"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// A game set up before the app icon was used gets it (`VURETRO_ICON_RGBA`: 256x256 RGBA
    /// bytes; `VURETRO_DATA` and `VURETRO_ROADRASH_ISO` as for the other ignored tests).
    #[test]
    #[ignore]
    fn the_app_icon_reaches_the_program_of_a_game_set_up_earlier() {
        let icon = std::fs::read(std::env::var("VURETRO_ICON_RGBA").unwrap()).unwrap();
        let paths = Paths::new(&std::env::var("VURETRO_DATA").unwrap());
        let iso = PathBuf::from(std::env::var("VURETRO_ROADRASH_ISO").unwrap());
        prepare(&paths, &iso, Some(&icon), &|l| println!("{l}")).unwrap();
        let ready = std::fs::read_to_string(paths.native_dir().join("pc/roadrash/ready")).unwrap();
        assert!(ready.ends_with("appicon\n"), "{ready}");
    }
}
