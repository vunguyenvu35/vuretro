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
use super::pc_recipes::{self, Recipe};
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
            if (*count).is_multiple_of(50) {
                log(&format!("  {count} files"));
            }
        }
    }
    Ok(())
}

/// Copy the files of a disc (an image, or a folder it was unpacked to) into `dest`: the folder
/// `root` of it, or the whole disc when that is empty.
pub(super) fn copy_root(
    disc: &Path,
    root: &str,
    dest: &Path,
    log: &dyn Fn(&str),
) -> Result<usize, String> {
    let mut count = 0;
    if disc.is_dir() {
        let from = if root.is_empty() {
            disc.to_path_buf()
        } else {
            super::pc_recipes::find_ci(disc, root)
                .ok_or(format!("there is no {root} on the disc"))?
        };
        copy_tree(&from, dest, &mut count, log)?;
        return Ok(count);
    }
    let iso = Iso::open(disc)?;
    for file in iso.files_under(root)? {
        let from = if root.is_empty() {
            file.clone()
        } else {
            format!("{root}/{file}")
        };
        let target = dest.join(&file);
        if let Some(dir) = target.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(&target, iso.read(&from)?)
            .map_err(|e| format!("{}: {e}", target.display()))?;
        count += 1;
        if count.is_multiple_of(50) {
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

/// The folder of a program that was installed on `C:` (`C:\\Games\\X\\X.EXE`), where the program is
/// started from: games look for their data beside them. `None` for a program on the CD drive.
fn program_folder(root: &Path, program: &str) -> Option<PathBuf> {
    let rel = program.get(..2).filter(|d| d.eq_ignore_ascii_case("C:"))?;
    let rel = program[rel.len()..].trim_start_matches(['\\', '/']);
    let (folder, _) = rel.rsplit_once(['\\', '/'])?;
    pc_recipes::find_ci(&root.join("prefix").join("drive_c"), folder).filter(|d| d.is_dir())
}

/// What a game is started with, besides Wine and its program.
struct Launch {
    /// The game draws in a window of its own (cnc-ddraw), not on a virtual desktop.
    framed: bool,
    /// The app makes the window as big as the screen allows once the game is open.
    zoom: bool,
    /// DXVK draws its Direct3D 10 and 11.
    dxvk: bool,
    /// More entries for `WINEDLLOVERRIDES`.
    overrides: Vec<String>,
}

impl Launch {
    /// How a game of this recipe is started, on a window with a frame or not.
    fn of(recipe: Option<&Recipe>, framed: bool) -> Launch {
        Launch {
            // a recipe that asks for something else than the frame has none, whatever was set up
            // before
            framed: framed && recipe.is_none_or(|r| r.window == "frame"),
            zoom: recipe.is_some_and(|r| r.window == "zoom"),
            dxvk: recipe.is_some_and(|r| r.renderer == "dxvk"),
            overrides: recipe.map(|r| r.overrides.clone()).unwrap_or_default(),
        }
    }
}

/// How to start the game: Wine, in the game's folder. `program` is a Windows path on the CD
/// drive. With `launch.framed` the game draws in a window of its own (cnc-ddraw); without, Wine
/// puts it on a virtual desktop of `size`, which has no title bar.
fn spec(wine: &Path, root: &Path, program: &str, size: &str, launch: &Launch) -> NativeSpec {
    let (args, mut overrides) = if launch.framed {
        (
            vec![program.to_string()],
            "mscoree,mshtml=;ddraw=n,b".to_string(),
        )
    } else if launch.zoom {
        // a window of its own, which the app zooms
        (vec![program.to_string()], "mscoree,mshtml=".to_string())
    } else {
        (
            // `cmd /c` takes the whole program, which may have a space in its name
            vec![
                "explorer".to_string(),
                format!("/desktop=Game,{size}"),
                "cmd".to_string(),
                "/c".to_string(),
                program.to_string(),
            ],
            "mscoree,mshtml=".to_string(),
        )
    };
    if launch.dxvk {
        overrides.push_str(";d3d11,d3d10core=n,b");
    }
    for o in &launch.overrides {
        overrides.push(';');
        overrides.push_str(o);
    }
    NativeSpec {
        exe: wine.to_path_buf(),
        args: args
            .into_iter()
            .map(|a| pc_recipes::on_cd_drive(&a))
            .collect(),
        cwd: Some(program_folder(root, program).unwrap_or_else(|| root.join("game"))),
        env: vec![
            (
                "WINEPREFIX".into(),
                root.join("prefix").to_string_lossy().into_owned(),
            ),
            ("WINEDLLOVERRIDES".into(), overrides),
            ("WINEDEBUG".into(), "-all".into()),
        ],
        // the server that Wine keeps for the prefix outlives the program
        stop: wine
            .parent()
            .map(|bin| bin.join("wineserver").to_string_lossy().into_owned())
            .map(|server| vec![server, "-k".into()]),
        zoom: launch.zoom,
    }
}

/// Fetch `url` (at most `max` bytes) and check its SHA-256: the whole file, or why not.
fn download_checked(url: &str, max: u64, sha256: &str) -> Result<Vec<u8>, String> {
    let mut resp = ureq::get(url)
        .call()
        .map_err(|e| format!("download failed: {e}"))?;
    let mut bytes = Vec::new();
    resp.body_mut()
        .as_reader()
        .take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("download failed: {e}"))?;
    if bytes.len() as u64 > max {
        return Err("the download is bigger than it should be".into());
    }
    let digest: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if digest != sha256 {
        return Err(format!("not the file expected (SHA-256 {digest})"));
    }
    Ok(bytes)
}

/// DXVK for macOS (github.com/Gcenx/DXVK-macOS, zlib licence): Direct3D 10 and 11 over Vulkan
/// (MoltenVK), as the `d3d10core.dll` and `d3d11.dll` that replace Wine's. Pinned to one release
/// and checked against its SHA-256.
const DXVK_URL: &str = "https://github.com/Gcenx/DXVK-macOS/releases/download/v1.10.3-20230507-repack/dxvk-macOS-async-v1.10.3-20230507-repack.tar.gz";
const DXVK_SHA256: &str = "acd1520ad105d8ef124a09c8e11a259a5dc8bdc565ad18e0e52693f9807b2477";
const DXVK_MAX_BYTES: u64 = 8 << 20;
/// The libraries of DXVK that are put in a game's Windows, and where they go from the folders
/// `x64` and `x32` of the release: 64-bit ones in `system32`, 32-bit ones in `syswow64`.
const DXVK_DLLS: [&str; 2] = ["d3d10core.dll", "d3d11.dll"];

/// The folder with the release of DXVK (`x32` and `x64` in it), fetched once.
fn dxvk_dir(paths: &Paths, log: &dyn Fn(&str)) -> Result<PathBuf, String> {
    let dir = paths.native_dir().join("wine").join("dxvk");
    if DXVK_DLLS.iter().all(|d| dir.join("x64").join(d).is_file()) {
        return Ok(dir);
    }
    log("Getting DXVK (Direct3D 11 for the game)");
    let bytes = download_checked(DXVK_URL, DXVK_MAX_BYTES, DXVK_SHA256)?;
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let archive = dir.join("dxvk.tar.gz");
    std::fs::write(&archive, bytes).map_err(|e| e.to_string())?;
    run(
        Command::new("tar")
            .arg("-xzf")
            .arg(&archive)
            .arg("--strip-components=1")
            .arg("-C")
            .arg(&dir),
        &|_| {},
    )?;
    let _ = std::fs::remove_file(&archive);
    if DXVK_DLLS.iter().all(|d| dir.join("x64").join(d).is_file()) {
        Ok(dir)
    } else {
        Err("DXVK was unpacked but its libraries are not where they should be".into())
    }
}

/// Put DXVK in the Windows of a game (`prefix`): the libraries replace those of Wine for the
/// game, with `WINEDLLOVERRIDES` saying so (see `spec`).
fn install_dxvk(prefix: &Path, dxvk: &Path) -> Result<(), String> {
    let windows = prefix.join("drive_c").join("windows");
    for (from, to) in [("x64", "system32"), ("x32", "syswow64")] {
        let target = windows.join(to);
        if !target.is_dir() {
            continue; // a Windows of one kind only
        }
        for dll in DXVK_DLLS {
            std::fs::copy(dxvk.join(from).join(dll), target.join(dll))
                .map_err(|e| format!("{dll}: {e}"))?;
        }
    }
    Ok(())
}

/// The `ddraw.dll` of cnc-ddraw, fetched once into `<data dir>/native/wine/cnc-ddraw`.
fn ddraw_dll(paths: &Paths, log: &dyn Fn(&str)) -> Result<PathBuf, String> {
    let dir = paths.native_dir().join("wine").join("cnc-ddraw");
    let dll = dir.join("ddraw.dll");
    if dll.is_file() {
        return Ok(dll);
    }
    log("Getting cnc-ddraw (a window for the game)");
    let bytes = download_checked(DDRAW_URL, DDRAW_MAX_BYTES, DDRAW_SHA256)?;
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
pub(super) fn add_window_frame(paths: &Paths, game: &Path, size: &str, log: &dyn Fn(&str)) -> bool {
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

/// `rel` (with `\` or `/`) inside `root`, matched without regard to letter case; the names as
/// they are on disk.
pub(super) fn resolve_ci(root: &Path, rel: &str) -> Option<Vec<String>> {
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
pub(super) fn find_program(game: &Path) -> Result<String, String> {
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

/// A file of the disc as it is there (an image, or a folder it was unpacked to): `root` is the
/// folder of the game, `rel` the path in it.
pub(super) fn fresh_file(disc: &Path, root: &str, rel: &str) -> Result<Vec<u8>, String> {
    let path = if root.is_empty() {
        rel.to_string()
    } else {
        format!("{root}/{rel}")
    };
    if disc.is_dir() {
        let at = super::pc_recipes::find_ci(disc, &path).ok_or(format!("{path} is not there"))?;
        return std::fs::read(at).map_err(|e| e.to_string());
    }
    Iso::open(disc)?.read(&path)
}

/// The patches of the program of a game, each best effort (the game works without them): its Dock
/// icon (the app's, or one drawn from its own) and, under Wine, the title of its window when the
/// recipe has one.
pub(super) fn patch_program(
    exe: &Path,
    recipe: &Recipe,
    icon: Option<&[u8]>,
    in_wine: bool,
    log: &dyn Fn(&str),
) {
    if let Err(e) = mac_icon_for(exe, icon) {
        log(&format!("The game keeps its old icon: {e}"));
    }
    let Some((from, to)) = recipe.title_bytes() else {
        return;
    };
    if !in_wine {
        return; // the title says what runs the game: nothing does on Windows
    }
    let titled = std::fs::read(exe)
        .map_err(|e| e.to_string())
        .and_then(|mut bytes| {
            crate::pe_icon::set_window_title(&mut bytes, &from, &to)?;
            std::fs::write(exe, bytes).map_err(|e| e.to_string())
        });
    if let Err(e) = titled {
        log(&format!("The window keeps its title: {e}"));
    }
}

/// Put the stand-ins of a recipe in the folder of the game's files.
pub(super) fn write_stubs(recipe: &Recipe, game: &Path) -> Result<(), String> {
    for stub in &recipe.stubs {
        let bytes = match stub.kind.as_str() {
            "aweman32" => aweman32_stub(),
            other => return Err(format!("no stand-in called {other}")),
        };
        std::fs::write(game.join(&stub.file), bytes).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(super) fn mac_icon_for(exe: &Path, icon: Option<&[u8]>) -> Result<(), String> {
    let mut bytes = std::fs::read(exe).map_err(|e| e.to_string())?;
    match icon {
        Some(rgba) => crate::pe_icon::set_icon(&mut bytes, rgba)?,
        None => crate::pe_icon::mac_style_icon(&mut bytes)?,
    }
    // written beside it and moved over it, so that a failure leaves the program as it was
    let part = exe.with_extension("part");
    std::fs::write(&part, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&part, exe).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        e.to_string()
    })
}

/// Whether a Wine is on this computer (the one this app fetched, or another).
pub fn ready(paths: &Paths) -> bool {
    // Windows runs these games by itself
    cfg!(windows) || locate_wine(paths).is_some()
}

/// Get Wine when the computer has none (nothing to do when it has one).
pub fn install_runtime(paths: &Paths, log: &dyn Fn(&str)) -> Result<(), String> {
    if cfg!(windows) {
        return Ok(()); // nothing to get
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
pub(super) fn game_id(disc: &Path) -> String {
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

/// Make the CD drive of a game: `R:` of its Wine, which is the files of the disc (or, for a recipe
/// that names the folder of the game on the disc, a folder that holds it under that name).
#[cfg(unix)]
fn setup_cd_drive(
    prefix: &Path,
    recipe: &Recipe,
    game: &Path,
    cdroot: &Path,
    id: &str,
) -> Result<(), String> {
    use std::os::unix::fs::symlink;
    let drive = prefix
        .join("dosdevices")
        .join(format!("{}:", pc_recipes::CD_DRIVE.to_ascii_lowercase()));
    let _ = std::fs::remove_file(&drive);
    let label = recipe.label.clone().unwrap_or_else(|| id.to_uppercase());
    match &recipe.drive_folder {
        Some(folder) => {
            // the game's folder shows up on the drive under the name the disc gives it
            let _ = std::fs::remove_dir_all(cdroot);
            std::fs::create_dir_all(cdroot).map_err(|e| e.to_string())?;
            symlink("../game", cdroot.join(folder)).map_err(|e| e.to_string())?;
            std::fs::write(cdroot.join(".windows-label"), &label).map_err(|e| e.to_string())?;
            symlink(cdroot, &drive).map_err(|e| e.to_string())
        }
        None => {
            std::fs::write(game.join(".windows-label"), &label).map_err(|e| e.to_string())?;
            symlink(game, &drive).map_err(|e| e.to_string())
        }
    }
}

#[cfg(not(unix))]
fn setup_cd_drive(_: &Path, _: &Recipe, _: &Path, _: &Path, _: &str) -> Result<(), String> {
    Ok(())
}

/// Whether a game comes as an archive (`.zip`, `.7z`, `.rar`) and not as a disc image.
pub(super) fn is_archive(disc: &Path) -> bool {
    crate::unpack::is_archive(disc)
}

/// The files of a game that comes as an archive: unpacked into the game's folder, once. An archive
/// that holds one folder with everything in it (the usual zip) has that folder's content as the
/// game's. Gives the folder of the game's files. Nothing is copied again after this: the game
/// runs from the files that were unpacked.
pub(super) fn unpack_into_game(
    archive: &Path,
    root: &Path,
    log: &dyn Fn(&str),
) -> Result<PathBuf, String> {
    let game = root.join("game");
    let marker = root.join("unpacked");
    if marker.is_file() && game.is_dir() {
        return Ok(game);
    }
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    log("Unpacking the game's files (a minute or two)");
    let part = root.join("game.unpacking");
    let _ = std::fs::remove_dir_all(&part);
    let _ = std::fs::remove_dir_all(&game);
    crate::unpack::extract(archive, &part)?;
    let entries: Vec<_> = std::fs::read_dir(&part)
        .map_err(|e| e.to_string())?
        .flatten()
        .collect();
    let inner = match entries.as_slice() {
        [one] if one.path().is_dir() => one.path(),
        _ => part.clone(),
    };
    std::fs::rename(&inner, &game).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_dir_all(&part);
    std::fs::write(&marker, "").map_err(|e| e.to_string())?;
    Ok(game)
}

/// The Windows of a game and its files: Wine's environment, the files of the disc, and the CD
/// drive. Done once (`environment` marks it): a game that cannot be started yet (its program is
/// not known) keeps it for its installer.
fn build_environment(
    env: &WineEnv,
    recipe: &Recipe,
    disc: &Path,
    root: &Path,
    log: &dyn Fn(&str),
) -> Result<(), String> {
    let archive = is_archive(disc);
    let (game, cdroot) = (root.join("game"), root.join("cdroot"));
    let marker = root.join("environment");
    if marker.is_file() && game.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    log("Making a Windows environment for the game (about a minute)");
    if !env.prefix.join("system.reg").is_file() {
        env.run(&["wineboot".into(), "--init".into()])?;
    }
    if !archive {
        log("Copying the game's files from the disc");
        if game.exists() {
            std::fs::remove_dir_all(&game).map_err(|e| e.to_string())?;
        }
        copy_root(disc, &pc_recipes::root_on(disc, recipe)?, &game, log)?;
    }
    log("Setting up the CD drive");
    setup_cd_drive(&env.prefix, recipe, &game, &cdroot, &game_id(disc))?;
    // the CD-ROM drive, for Wine
    env.run(&pc_recipes::drive_command())?;
    std::fs::write(&marker, "").map_err(|e| e.to_string())
}

/// Whether the CD drive of a game is what `setup_cd_drive` makes: the link of `R:` points where the
/// recipe says (the folder with the game's folder in it, or the game's files themselves).
fn cd_drive_is_right(prefix: &Path, recipe: &Recipe, game: &Path, cdroot: &Path) -> bool {
    let link = prefix
        .join("dosdevices")
        .join(format!("{}:", pc_recipes::CD_DRIVE.to_ascii_lowercase()));
    let expected = if recipe.drive_folder.is_some() {
        cdroot
    } else {
        game
    };
    std::fs::read_link(link).is_ok_and(|target| target == expected)
}

/// What a set-up game remembers, in its `ready` file, a line each: the program to start (a path on
/// the CD drive), the size of the screen it asks for, whether it has a window frame (`framed` or
/// `desktop`), then marks: `recipe:<id>` and `rev-<n>` for the recipe it was set up with.
struct Ready {
    program: String,
    size: String,
    mode: String,
    marks: Vec<String>,
}

impl Ready {
    fn read(file: &Path) -> Option<Ready> {
        let text = std::fs::read_to_string(file).ok()?;
        let mut lines = text.lines().map(String::from);
        Some(Ready {
            program: lines.next()?,
            size: lines.next()?,
            mode: lines.next().unwrap_or_default(),
            marks: lines.collect(),
        })
    }

    fn write(&self, file: &Path) -> Result<(), String> {
        let mut text = format!("{}\n{}\n{}\n", self.program, self.size, self.mode);
        for m in &self.marks {
            text.push_str(m);
            text.push('\n');
        }
        std::fs::write(file, text).map_err(|e| e.to_string())
    }

    fn framed(&self) -> bool {
        self.mode == "framed"
    }

    /// The recipe it was set up with. A game set up before recipes were data carries the mark
    /// `patched-3`: that is Road Rash at its revision 3.
    fn recipe(&self) -> Option<&'static Recipe> {
        self.marks
            .iter()
            .find_map(|m| m.strip_prefix("recipe:"))
            .map(pc_recipes::by_id)
            .or_else(|| {
                self.marks
                    .iter()
                    .any(|m| m == "patched-3")
                    .then(|| pc_recipes::by_id("road-rash"))
            })
    }

    fn revision(&self) -> u32 {
        self.marks
            .iter()
            .find_map(|m| m.strip_prefix("rev-").and_then(|n| n.parse().ok()))
            .or_else(|| self.marks.iter().any(|m| m == "patched-3").then_some(3))
            .unwrap_or(0)
    }

    fn set_recipe(&mut self, recipe: &Recipe) {
        self.marks
            .retain(|m| !m.starts_with("recipe:") && !m.starts_with("rev-") && m != "patched-3");
        self.marks.push(format!("recipe:{}", recipe.id));
        self.marks.push(format!("rev-{}", recipe.revision));
    }
}

/// A Wine and the environment of one game to run its programs in.
struct WineEnv {
    wine: PathBuf,
    prefix: PathBuf,
}

/// How long one command of Wine may take (the first start of a new Windows takes a few minutes
/// on a slow computer): a command that takes longer is hung, and is ended.
const WINE_COMMAND_LIMIT: std::time::Duration = std::time::Duration::from_secs(600);

/// Run `command` and wait for it for at most `limit`; what it says goes to `log_file`, and the end
/// of it is in the error when it fails. Not a pipe: the program Wine starts behind the command
/// (its server) would keep a pipe open for as long as it lives, and the wait would never end.
fn run_bounded(
    command: &mut Command,
    limit: std::time::Duration,
    log_file: &Path,
) -> Result<(), String> {
    let out = std::fs::File::create(log_file).map_err(|e| e.to_string())?;
    let mut child = command
        .stdin(std::process::Stdio::null())
        .stdout(out.try_clone().map_err(|e| e.to_string())?)
        .stderr(out)
        .spawn()
        .map_err(|e| format!("cannot start {:?}: {e}", command.get_program()))?;
    let started = std::time::Instant::now();
    let tail = || {
        let text = std::fs::read_to_string(log_file).unwrap_or_default();
        let lines: Vec<&str> = text.lines().rev().take(4).collect();
        lines.into_iter().rev().collect::<Vec<_>>().join("\n")
    };
    loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) if status.success() => return Ok(()),
            Some(status) => return Err(format!("{status}: {}", tail())),
            None if started.elapsed() > limit => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "it took more than {} seconds and was ended: {}",
                    limit.as_secs(),
                    tail()
                ));
            }
            None => std::thread::sleep(std::time::Duration::from_millis(100)),
        }
    }
}

impl WineEnv {
    fn run(&self, args: &[String]) -> Result<(), String> {
        self.run_in(args, None)
    }

    /// Run a command of Wine, from a folder when one is given.
    fn run_in(&self, args: &[String], cwd: Option<&Path>) -> Result<(), String> {
        let log = self.prefix.with_file_name("wine-command.log");
        let mut command = Command::new(&self.wine);
        if let Some(dir) = cwd {
            command.current_dir(dir);
        }
        let result = run_bounded(
            command
                .args(args)
                .env("WINEPREFIX", &self.prefix)
                .env("WINEDLLOVERRIDES", "mscoree,mshtml=")
                .env("WINEDEBUG", "-all"),
            WINE_COMMAND_LIMIT,
            &log,
        );
        if result.is_err() {
            // a command that hung leaves the server of Wine hung: end it, so the next starts clean
            if let Some(server) = self.wine.parent().map(|bin| bin.join("wineserver")) {
                let _ = Command::new(server)
                    .arg("-k")
                    .env("WINEPREFIX", &self.prefix)
                    .status();
            }
        }
        result.map_err(|e| {
            format!(
                "wine {}: {e}",
                args.first().map(String::as_str).unwrap_or("")
            )
        })
    }
}

/// What a recipe puts into a game's folder and the environment: the stand-ins, the patches of
/// the program and the entries of the registry. Gives the program to start.
fn apply_recipe(
    paths: &Paths,
    env: &WineEnv,
    recipe: &Recipe,
    game: &Path,
    icon: Option<&[u8]>,
    log: &dyn Fn(&str),
) -> Result<String, String> {
    write_stubs(recipe, game)?;
    if recipe.renderer == "dxvk" {
        install_dxvk(&env.prefix, &dxvk_dir(paths, log)?)?;
    }
    let program = match &recipe.program {
        Some(p) => p.clone(),
        None => find_program(game)?,
    };
    if let Some(exe) = recipe.program_in_game(game, &program) {
        patch_program(&exe, recipe, icon, true, log);
    }
    if !recipe.registry.is_empty() {
        // the folder the game says it is installed in is empty: its files are on the CD drive
        std::fs::create_dir_all(
            env.prefix.join("drive_c").join(
                recipe
                    .install_path()
                    .trim_start_matches("C:\\")
                    .trim_end_matches('\\'),
            ),
        )
        .map_err(|e| e.to_string())?;
        for command in pc_recipes::registry_commands(recipe, &recipe.install_path()) {
            env.run(&command)?;
        }
    }
    Ok(program)
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
    if cfg!(windows) {
        // no Wine: the disc is mounted and the game runs as it is
        return super::windows_pc::prepare(paths, disc, icon, log);
    }
    let wine = locate_wine(paths).ok_or("Wine is not on this computer")?;
    let root = paths.native_dir().join("pc").join(game_id(disc));
    let (game, prefix, cdroot) = (root.join("game"), root.join("prefix"), root.join("cdroot"));
    let done = root.join("ready");
    let env = WineEnv {
        wine: wine.clone(),
        prefix: prefix.clone(),
    };

    if let Some(mut ready) = Ready::read(&done) {
        if ready.mode.is_empty() {
            // set up before the frame existed: add it without doing the rest again
            let framed = add_window_frame(paths, &game, &ready.size, log);
            ready.mode = if framed { "framed" } else { "desktop" }.into();
            let _ = ready.write(&done);
        }
        // the recipe of the game: the one it was set up with, or the one its disc has
        let recipe = ready.recipe().or_else(|| {
            pc_recipes::for_disc(disc)
                .ok()
                .filter(|r| r.id != "generic")
        });
        // a game set up when the CD drive was `D:`, or whose `R:` is not what its recipe says,
        // gets the drive made again (see `pc_recipes::CD_DRIVE`)
        if game.is_dir()
            && !cd_drive_is_right(
                &prefix,
                recipe.unwrap_or_else(|| pc_recipes::by_id("generic")),
                &game,
                &cdroot,
            )
        {
            let done = setup_cd_drive(
                &prefix,
                recipe.unwrap_or_else(|| pc_recipes::by_id("generic")),
                &game,
                &cdroot,
                &game_id(disc),
            )
            .and_then(|_| env.run(&pc_recipes::drive_command()));
            if let Err(e) = done {
                log(&format!("Could not set up the CD drive: {e}"));
            }
        }
        // a game set up with an older revision of its recipe gets what changed: the program is
        // taken from the disc again, as the patches are written over it
        let launch = Launch::of(recipe, ready.framed());
        if let Some(recipe) = recipe.filter(|r| r.revision != ready.revision()) {
            if disc.exists() {
                match refresh(paths, &env, recipe, disc, &game, icon, &mut ready, log) {
                    Ok(()) => {
                        ready.set_recipe(recipe);
                        let _ = ready.write(&done);
                    }
                    Err(e) => log(&format!("The game keeps what it had: {e}")),
                }
            }
        }
        return Ok(spec(&wine, &root, &ready.program, &ready.size, &launch));
    }

    // a game that comes as an archive is unpacked first: its recipe is known from its files
    let recipe = if is_archive(disc) {
        pc_recipes::for_disc(&unpack_into_game(disc, &root, log)?)?
    } else {
        pc_recipes::for_disc(disc)?
    };
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;

    build_environment(&env, recipe, disc, &root, log)?;
    let program = apply_recipe(paths, &env, recipe, &game, icon, log)?;
    let mut marks = Vec::new();
    apply_setup_programs(&env, recipe, &game, &mut marks, log)?;
    apply_winetricks(&env, recipe, &mut marks, log)?;

    let framed = recipe.window == "frame" && add_window_frame(paths, &game, &recipe.screen, log);
    let mut ready = Ready {
        program,
        size: recipe.screen.clone(),
        mode: if framed { "framed" } else { "desktop" }.into(),
        marks,
    };
    if recipe.id != "generic" {
        ready.set_recipe(recipe);
    }
    ready.write(&done)?;
    Ok(spec(
        &wine,
        &root,
        &ready.program,
        &ready.size,
        &Launch::of(Some(recipe).filter(|r| r.id != "generic"), framed),
    ))
}

/// Run the programs of a recipe's `setup` that were not run yet (`marks` says which were:
/// `setup:<program>`), from the game's folder: such a program writes the settings that the game
/// reads. Each is marked when done.
fn apply_setup_programs(
    env: &WineEnv,
    recipe: &Recipe,
    game: &Path,
    marks: &mut Vec<String>,
    log: &dyn Fn(&str),
) -> Result<(), String> {
    for program in &recipe.setup {
        let mark = format!("setup:{program}");
        if marks.contains(&mark) {
            continue;
        }
        let exe = recipe
            .program_in_game(game, program)
            .ok_or_else(|| format!("{program} is not in the files of the game"))?;
        let rel = exe
            .strip_prefix(game)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('/', "\\");
        let on_drive = pc_recipes::on_cd_drive(&format!("D:\\{}{rel}", drive_prefix(recipe)));
        log(&format!("Running {program} (settings of the game)"));
        env.run_in(&[on_drive], Some(game))?;
        marks.push(mark);
    }
    Ok(())
}

/// The folder of the game on the CD drive with a backslash (`ROADRASH\\`), or nothing.
fn drive_prefix(recipe: &Recipe) -> String {
    recipe
        .drive_folder
        .as_ref()
        .map(|f| format!("{f}\\"))
        .unwrap_or_default()
}

/// The verbs of `winetricks` that a recipe lists and that were not run yet (`marks` says which
/// were: `winetricks:<verb>`).
fn pending_verbs<'a>(recipe: &'a Recipe, marks: &[String]) -> Vec<&'a str> {
    recipe
        .winetricks
        .iter()
        .map(String::as_str)
        .filter(|v| {
            !marks
                .iter()
                .any(|m| m.strip_prefix("winetricks:") == Some(v))
        })
        .collect()
}

/// `winetricks` run without questions on the Windows of a game, with this Wine.
fn winetricks_command(tool: &Path, wine: &Path, prefix: &Path, verbs: &[&str]) -> Command {
    let mut path = wine
        .parent()
        .map(|d| d.as_os_str().to_os_string())
        .unwrap_or_default();
    path.push(":");
    path.push(search_path());
    let mut command = Command::new(tool);
    command
        .arg("-q")
        .args(verbs)
        .env("WINE", wine)
        .env("WINEPREFIX", prefix)
        .env("PATH", path);
    command
}

/// Run the verbs of `winetricks` of a recipe that were not run yet, with the `winetricks` that is on
/// this computer (it is not fetched: it runs a good deal of other programs). Each is marked when
/// done, so that it is not run twice.
fn apply_winetricks(
    env: &WineEnv,
    recipe: &Recipe,
    marks: &mut Vec<String>,
    log: &dyn Fn(&str),
) -> Result<(), String> {
    let verbs = pending_verbs(recipe, marks);
    if verbs.is_empty() {
        return Ok(());
    }
    let tool = find_tool("winetricks").ok_or(format!(
        "{} needs {} from winetricks: install it first (brew install winetricks)",
        recipe.name,
        verbs.join(", ")
    ))?;
    log(&format!("Installing {} (winetricks)", verbs.join(", ")));
    run(
        &mut winetricks_command(&tool, &env.wine, &env.prefix, &verbs),
        log,
    )?;
    let done: Vec<String> = verbs.iter().map(|v| format!("winetricks:{v}")).collect();
    marks.extend(done);
    Ok(())
}

/// The names under which the installer of a disc is looked for, in this order.
const INSTALLER_NAMES: [&str; 6] = [
    "SETUP.EXE",
    "INSTALL.EXE",
    "SETUP/SETUP.EXE",
    "INSTALL/INSTALL.EXE",
    "SETUP/INSTALL.EXE",
    "INSTALL/SETUP.EXE",
];

/// The installer of a disc (a path on the CD drive): the one the recipe names, else the one that
/// `autorun.inf` names if it looks like an installer, else a program with a usual name.
fn find_installer(game: &Path, recipe: &Recipe) -> Option<String> {
    let on_drive = |names: Vec<String>| format!("D:\\{}", names.join("\\"));
    let in_game = |rel: &str| match &recipe.drive_folder {
        // the files are under the folder of the recipe on the drive
        Some(folder) => resolve_ci(game, rel).map(|n| {
            let mut v = vec![folder.clone()];
            v.extend(n);
            v
        }),
        None => resolve_ci(game, rel),
    };
    if let Some(rel) = &recipe.installer {
        let rel = rel.trim_start_matches("D:").trim_start_matches(['\\', '/']);
        return in_game(rel).map(on_drive);
    }
    if let Some(autorun) = resolve_ci(game, "autorun.inf") {
        let text = std::fs::read(game.join(autorun.join("/")))
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let lower = value.to_ascii_lowercase();
            if key.trim().eq_ignore_ascii_case("open")
                && (lower.contains("setup") || lower.contains("install"))
            {
                if let Some(end) = lower.find(".exe").map(|i| i + 4) {
                    if let Some(found) = in_game(&value.trim()[..end]) {
                        return Some(on_drive(found));
                    }
                }
            }
        }
    }
    INSTALLER_NAMES
        .iter()
        .find_map(|name| in_game(name))
        .map(on_drive)
}

/// The programs under a folder with their sizes (to see which ones an installer added).
fn programs_under(dir: &Path) -> std::collections::BTreeMap<PathBuf, u64> {
    let mut found = std::collections::BTreeMap::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let path = e.path();
            let Ok(kind) = e.file_type() else {
                continue;
            };
            if kind.is_dir() {
                // Windows' own folder holds the programs of Windows
                if !(d == dir && e.file_name().eq_ignore_ascii_case("windows")) {
                    todo.push(path);
                }
            } else if path
                .extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("exe"))
            {
                found.insert(path, e.metadata().map(|m| m.len()).unwrap_or(0));
            }
        }
    }
    found
}

/// Which program an installer left for the game: of the programs that were not there before,
/// those that are not helpers (uninstallers, installers, redistributables, settings) are
/// kept; the one whose name is like the game's, else the biggest.
fn choose_installed(
    before: &std::collections::BTreeMap<PathBuf, u64>,
    after: &std::collections::BTreeMap<PathBuf, u64>,
    game: &str,
) -> Result<PathBuf, String> {
    const HELPERS: [&str; 11] = [
        "unins", "uninst", "setup", "install", "update", "patch", "config", "readme", "dxsetup",
        "vcredist", "redist",
    ];
    let new: Vec<(&PathBuf, u64)> = after
        .iter()
        .filter(|(p, _)| !before.contains_key(*p))
        .map(|(p, n)| (p, *n))
        .collect();
    if new.is_empty() {
        return Err("the installer did not leave any program: was it finished?".into());
    }
    let name = |p: &PathBuf| {
        p.file_stem()
            .map(|n| n.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default()
    };
    let mut kept: Vec<(&PathBuf, u64)> = new
        .iter()
        .copied()
        .filter(|(p, _)| !HELPERS.iter().any(|h| name(p).contains(h)))
        .collect();
    if kept.is_empty() {
        kept = new;
    }
    let flat = |s: &str| {
        s.chars()
            .filter(char::is_ascii_alphanumeric)
            .collect::<String>()
            .to_ascii_lowercase()
    };
    let game = flat(game);
    let like: Vec<(&PathBuf, u64)> = kept
        .iter()
        .copied()
        .filter(|(p, _)| {
            let n = flat(&name(p));
            !n.is_empty() && !game.is_empty() && (game.contains(&n) || n.contains(&game))
        })
        .collect();
    let pool = if like.is_empty() { kept } else { like };
    pool.into_iter()
        .max_by_key(|(_, size)| *size)
        .map(|(p, _)| p.clone())
        .ok_or_else(|| "no program was found".to_string())
}

/// The Windows path (`C:\...`) of a file in the `C:` drive of a game.
fn windows_path(drive_c: &Path, file: &Path) -> Result<String, String> {
    let rel = file.strip_prefix(drive_c).map_err(|e| e.to_string())?;
    Ok(format!(
        "C:\\{}",
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("\\")
    ))
}

/// Run the installer of the disc of a game under Wine (the user follows it in its window), and
/// make the program that it leaves the one that Play starts. Gives the name of that program.
pub fn run_installer(
    paths: &Paths,
    disc: &Path,
    icon: Option<&[u8]>,
    log: &dyn Fn(&str),
) -> Result<String, String> {
    if cfg!(windows) {
        return Err("On Windows run the installer from the mounted disc yourself".into());
    }
    let wine = locate_wine(paths).ok_or("Wine is not on this computer")?;
    let root = paths.native_dir().join("pc").join(game_id(disc));
    let recipe = if is_archive(disc) {
        pc_recipes::for_disc(&unpack_into_game(disc, &root, log)?)?
    } else {
        pc_recipes::for_disc(disc)?
    };
    let (game, prefix) = (root.join("game"), root.join("prefix"));
    let env = WineEnv {
        wine: wine.clone(),
        prefix: prefix.clone(),
    };
    build_environment(&env, recipe, disc, &root, log)?;
    let installer = find_installer(&game, recipe).ok_or("this disc has no installer to run")?;
    let drive_c = prefix.join("drive_c");
    let before = programs_under(&drive_c);

    log("Running the installer of the disc: follow it in the window that opens");
    let status = Command::new(&wine)
        .args([
            "explorer".to_string(),
            "/desktop=Installer,800x600".to_string(),
            pc_recipes::on_cd_drive(&installer),
        ])
        .current_dir(&game)
        .env("WINEPREFIX", &prefix)
        .env("WINEDLLOVERRIDES", "mscoree,mshtml=")
        .env("WINEDEBUG", "-all")
        .status()
        .map_err(|e| format!("cannot start Wine: {e}"))?;
    log(&format!("The installer ended ({status})"));
    // an installer may start the program that does the work and end: wait for all of it
    if let Some(server) = wine.parent().map(|bin| bin.join("wineserver")) {
        let _ = Command::new(server)
            .arg("-w")
            .env("WINEPREFIX", &prefix)
            .status();
    }

    let installed = choose_installed(&before, &programs_under(&drive_c), &game_id(disc))?;
    let program = windows_path(&drive_c, &installed)?;
    log(&format!("The game is {program}"));
    if let Some(folder) = installed.parent() {
        // the window frame is the library that sits beside the program
        add_window_frame(paths, folder, "800x600", log);
    }
    patch_program(&installed, recipe, icon, true, log);
    let mut ready = Ready {
        program,
        size: "800x600".into(),
        mode: "framed".into(),
        marks: vec!["installed".into()],
    };
    if recipe.id != "generic" {
        ready.set_recipe(recipe);
    }
    ready.write(&root.join("ready"))?;
    Ok(installed
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default())
}

/// Bring a game set up with an older revision of its recipe up to date: the stand-ins, the program
/// from the disc with the patches, and the registry.
#[allow(clippy::too_many_arguments)]
fn refresh(
    paths: &Paths,
    env: &WineEnv,
    recipe: &Recipe,
    disc: &Path,
    game: &Path,
    icon: Option<&[u8]>,
    ready: &mut Ready,
    log: &dyn Fn(&str),
) -> Result<(), String> {
    // the program is taken from the disc again (a game that came as an archive has no disc to take
    // it from, and keeps the one it was unpacked with)
    let exe = recipe
        .program_in_game(game, &ready.program)
        .filter(|_| !is_archive(disc));
    if let Some(exe) = exe {
        let rel = exe
            .strip_prefix(game)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = fresh_file(disc, &pc_recipes::root_on(disc, recipe)?, &rel)?;
        std::fs::write(&exe, bytes).map_err(|e| e.to_string())?;
    }
    apply_recipe(paths, env, recipe, game, icon, log)?;
    apply_setup_programs(env, recipe, game, &mut ready.marks, log)?;
    apply_winetricks(env, recipe, &mut ready.marks, log)
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

    fn launch(framed: bool, dxvk: bool) -> Launch {
        Launch {
            framed,
            zoom: false,
            dxvk,
            overrides: Vec::new(),
        }
    }

    #[test]
    fn the_game_starts_wine_in_its_own_folder_framed_or_on_a_virtual_desktop() {
        let root = Path::new("/data/pc/road-rash");
        let wine = Path::new("/opt/wine/bin/wine");
        let desktop = spec(wine, root, "D:\\GAME.EXE", "800x600", &launch(false, false));
        assert_eq!(desktop.exe, wine);
        assert_eq!(
            desktop.args,
            [
                "explorer",
                "/desktop=Game,800x600",
                "cmd",
                "/c",
                "R:\\GAME.EXE"
            ]
        );
        assert_eq!(desktop.cwd.as_deref(), Some(root.join("game").as_path()));
        assert!(desktop
            .env
            .iter()
            .any(|(k, v)| k == "WINEPREFIX" && Path::new(v) == root.join("prefix").as_path()));
        let framed = spec(wine, root, "D:\\GAME.EXE", "800x600", &launch(true, false));
        assert_eq!(framed.args, ["R:\\GAME.EXE"]);
        assert!(framed
            .env
            .iter()
            .any(|(k, v)| k == "WINEDLLOVERRIDES" && v.contains("ddraw=n,b")));
        // the command that ends what Wine left: its server, from the folder of Wine
        let stop = framed.stop.as_deref().unwrap();
        assert_eq!(
            Path::new(&stop[0]),
            wine.parent().unwrap().join("wineserver")
        );
        assert_eq!(stop[1], "-k");
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

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("vuretro-wine-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn the_files_of_a_game_are_copied_from_its_folder_or_from_the_whole_disc() {
        let disc = temp("disc");
        std::fs::create_dir_all(disc.join("RoadRash/DATA")).unwrap();
        std::fs::write(disc.join("RoadRash/ROADRASH.EXE"), "exe").unwrap();
        std::fs::write(disc.join("RoadRash/DATA/X.BIN"), "x").unwrap();
        std::fs::create_dir_all(disc.join("SETUP")).unwrap();
        std::fs::write(disc.join("SETUP/OTHER.DLL"), "no").unwrap();
        let dest = temp("dest");
        assert_eq!(
            copy_root(&disc, "ROADRASH", &dest.join("g"), &|_| {}).unwrap(),
            2
        );
        assert!(dest.join("g/DATA/X.BIN").is_file());
        assert!(!dest.join("g/SETUP").exists(), "only the game's own folder");
        // the whole disc
        let dest2 = temp("dest2");
        assert_eq!(copy_root(&disc, "", &dest2.join("g"), &|_| {}).unwrap(), 3);
        assert!(dest2.join("g/SETUP/OTHER.DLL").is_file());
        // a folder that is not there
        assert!(copy_root(&disc, "NOPE", &dest2.join("h"), &|_| {}).is_err());
        for d in [disc, dest, dest2] {
            std::fs::remove_dir_all(d).unwrap();
        }
    }

    #[test]
    fn a_file_that_is_not_a_disc_image_is_refused() {
        let dir = temp("notiso");
        let file = dir.join("x.iso");
        std::fs::write(&file, vec![0u8; 40 * 2048]).unwrap();
        assert!(copy_root(&file, "", &dir.join("g"), &|_| {}).is_err());
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
        assert_eq!(copy_root(&disc, "", &dest.join("g"), &|_| {}).unwrap(), 2);
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
        let paths = Paths::new(std::env::var("VURETRO_DATA").unwrap());
        let iso = PathBuf::from(std::env::var("VURETRO_ROADRASH_ISO").unwrap());
        prepare(&paths, &iso, Some(&icon), &|l| println!("{l}")).unwrap();
        let ready = std::fs::read_to_string(paths.native_dir().join("pc/roadrash/ready")).unwrap();
        assert!(ready.ends_with("rev-3\n"), "{ready}");
    }

    #[test]
    fn the_icon_of_a_games_program_is_replaced_in_the_copy_and_a_bad_one_is_left_alone() {
        let game = temp("any-icon");
        std::fs::create_dir_all(game.join("Bin")).unwrap();
        std::fs::write(game.join("Bin/Play.EXE"), crate::pe_icon::tests::program()).unwrap();
        let lines = std::cell::RefCell::new(Vec::new());
        let log = |l: &str| lines.borrow_mut().push(l.to_string());
        let generic = Recipe::generic();
        let before = std::fs::read(game.join("Bin/Play.EXE")).unwrap();
        // the path on the CD drive is matched without regard to letter case
        let exe = generic.program_in_game(&game, "D:\\bin\\play.exe").unwrap();
        patch_program(&exe, &generic, None, true, &log);
        let after = std::fs::read(game.join("Bin/Play.EXE")).unwrap();
        assert!(
            after.len() > before.len(),
            "a section with the icon was added"
        );
        assert!(lines.borrow().is_empty());
        assert!(!game.join("Bin/Play.part").exists(), "no leftover");
        // a file that is not a program is left as it was, and the log says so
        std::fs::write(game.join("Bad.EXE"), b"not a program").unwrap();
        patch_program(&game.join("Bad.EXE"), &generic, None, true, &log);
        assert_eq!(
            std::fs::read(game.join("Bad.EXE")).unwrap(),
            b"not a program"
        );
        assert_eq!(lines.borrow().len(), 1);
        std::fs::remove_dir_all(game).unwrap();
    }

    #[test]
    fn a_ready_file_is_read_and_written_and_an_old_one_is_road_rash_at_revision_3() {
        let dir = temp("ready");
        let file = dir.join("ready");
        // as written before recipes were data: the program, the screen, the mode, one mark
        std::fs::write(
            &file,
            "D:\\RoadRash\\ROADRASH.EXE\n640x480\nframed\npatched-3\n",
        )
        .unwrap();
        let r = Ready::read(&file).unwrap();
        assert!(r.framed());
        assert_eq!(r.recipe().unwrap().id, "road-rash");
        assert_eq!(r.revision(), 3);
        // set up again with the recipe now: its own marks replace the old one
        let mut r = r;
        r.set_recipe(pc_recipes::by_id("road-rash"));
        assert_eq!(r.marks, ["recipe:road-rash", "rev-3"]);
        r.write(&file).unwrap();
        let again = Ready::read(&file).unwrap();
        assert_eq!(again.revision(), 3);
        assert_eq!(again.program, "D:\\RoadRash\\ROADRASH.EXE");
        // a game that no recipe knows has no recipe and revision 0
        std::fs::write(&file, "D:\\GAME.EXE\n800x600\ndesktop\n").unwrap();
        let g = Ready::read(&file).unwrap();
        assert!(g.recipe().is_none());
        assert_eq!(g.revision(), 0);
        // a file with fewer than two lines is no state at all
        std::fs::write(&file, "D:\\GAME.EXE\n").unwrap();
        assert!(Ready::read(&file).is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Prepares (or finds again) the game in a data folder (`VURETRO_DATA`) from its disc image,
    /// and says how to start it: for trying a game that was set up earlier.
    #[test]
    #[ignore]
    fn prepares_the_game_in_a_data_folder() {
        let paths = Paths::new(std::env::var("VURETRO_DATA").unwrap());
        let iso = PathBuf::from(std::env::var("VURETRO_ROADRASH_ISO").unwrap());
        let spec = prepare(&paths, &iso, None, &|l| println!("{l}")).unwrap();
        println!("SPEC {spec:?}");
    }

    #[test]
    fn a_path_on_the_cd_drive_is_changed_to_the_drive_that_is_really_used() {
        assert_eq!(
            pc_recipes::on_cd_drive("D:\\RoadRash\\X.EXE"),
            "R:\\RoadRash\\X.EXE"
        );
        assert_eq!(pc_recipes::on_cd_drive("d:\\x.exe"), "R:\\x.exe");
        assert_eq!(pc_recipes::on_cd_drive("C:\\x.exe"), "C:\\x.exe");
        assert_eq!(pc_recipes::on_cd_drive("explorer"), "explorer");
        assert_eq!(
            pc_recipes::on_cd_drive("/desktop=Game,800x600"),
            "/desktop=Game,800x600"
        );
        assert_eq!(pc_recipes::drive_command()[4], "r:");
    }

    #[test]
    fn dxvk_is_a_override_of_the_direct3d_libraries_only_when_the_game_asks_for_it() {
        let wine = Path::new("/opt/wine/bin/wine");
        let root = Path::new("/data/pc/g");
        let overrides = |s: &NativeSpec| {
            s.env
                .iter()
                .find(|(k, _)| k == "WINEDLLOVERRIDES")
                .unwrap()
                .1
                .clone()
        };
        let plain = spec(wine, root, "R:\\G.EXE", "800x600", &launch(true, false));
        assert!(!overrides(&plain).contains("d3d11"));
        let dxvk = spec(wine, root, "R:\\G.EXE", "800x600", &launch(true, true));
        assert_eq!(
            overrides(&dxvk),
            "mscoree,mshtml=;ddraw=n,b;d3d11,d3d10core=n,b"
        );
        let on_desktop = spec(wine, root, "R:\\G.EXE", "800x600", &launch(false, true));
        assert_eq!(
            overrides(&on_desktop),
            "mscoree,mshtml=;d3d11,d3d10core=n,b"
        );
    }

    #[test]
    fn dxvk_is_copied_where_each_kind_of_windows_looks_for_its_libraries() {
        let dxvk = temp("dxvk-release");
        for arch in ["x32", "x64"] {
            std::fs::create_dir_all(dxvk.join(arch)).unwrap();
            for dll in DXVK_DLLS {
                std::fs::write(dxvk.join(arch).join(dll), arch).unwrap();
            }
        }
        let prefix = temp("dxvk-prefix");
        let windows = prefix.join("drive_c/windows");
        std::fs::create_dir_all(windows.join("system32")).unwrap();
        std::fs::create_dir_all(windows.join("syswow64")).unwrap();
        install_dxvk(&prefix, &dxvk).unwrap();
        assert_eq!(
            std::fs::read(windows.join("system32/d3d11.dll")).unwrap(),
            b"x64"
        );
        assert_eq!(
            std::fs::read(windows.join("syswow64/d3d10core.dll")).unwrap(),
            b"x32"
        );
        // a Windows with no syswow64 gets the 64-bit ones only
        let small = temp("dxvk-small");
        std::fs::create_dir_all(small.join("drive_c/windows/system32")).unwrap();
        install_dxvk(&small, &dxvk).unwrap();
        assert!(small.join("drive_c/windows/system32/d3d11.dll").is_file());
        for d in [dxvk, prefix, small] {
            std::fs::remove_dir_all(d).unwrap();
        }
    }

    /// Needs the network: DXVK is fetched, checked against its SHA-256 and unpacked.
    #[test]
    #[ignore]
    fn fetches_the_real_dxvk() {
        let dir = temp("dxvk-real");
        let paths = Paths::new(dir.to_str().unwrap());
        let found = dxvk_dir(&paths, &|l| println!("{l}")).unwrap();
        for arch in ["x32", "x64"] {
            for dll in DXVK_DLLS {
                assert!(found.join(arch).join(dll).metadata().unwrap().len() > 100_000);
            }
        }
        // the second time nothing is fetched
        assert_eq!(
            dxvk_dir(&paths, &|_| panic!("fetched again")).unwrap(),
            found
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_overrides_of_a_recipe_are_added_to_those_of_wine() {
        let recipe = pc_recipes::parse(
            r#"[{"id":"a","name":"A","detect":["X"],"overrides":["quartz=n,b","msvcrt=n"],"renderer":"dxvk"}]"#,
        )
        .unwrap();
        let l = Launch::of(recipe.first(), true);
        let s = spec(
            Path::new("/w/bin/wine"),
            Path::new("/d"),
            "R:\\G.EXE",
            "640x480",
            &l,
        );
        let o = &s
            .env
            .iter()
            .find(|(k, _)| k == "WINEDLLOVERRIDES")
            .unwrap()
            .1;
        assert_eq!(
            o,
            "mscoree,mshtml=;ddraw=n,b;d3d11,d3d10core=n,b;quartz=n,b;msvcrt=n"
        );
        // a game that no recipe knows has none
        let none = Launch::of(None, false);
        assert!(!none.dxvk && none.overrides.is_empty());
    }

    #[test]
    fn winetricks_runs_only_the_verbs_that_were_not_run_and_without_questions() {
        let recipe = pc_recipes::parse(
            r#"[{"id":"a","name":"A","detect":["X"],"winetricks":["vcrun2015","corefonts"]}]"#,
        )
        .unwrap();
        let marks = vec!["winetricks:corefonts".to_string(), "rev-1".to_string()];
        assert_eq!(pending_verbs(&recipe[0], &marks), ["vcrun2015"]);
        assert!(pending_verbs(
            &recipe[0],
            &["winetricks:vcrun2015".into(), "winetricks:corefonts".into()]
        )
        .is_empty());
        let c = winetricks_command(
            Path::new("/usr/local/bin/winetricks"),
            Path::new("/w/bin/wine"),
            Path::new("/data/prefix"),
            &["vcrun2015"],
        );
        let args: Vec<_> = c
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args, ["-q", "vcrun2015"]);
        let env = |k: &str| {
            c.get_envs()
                .find(|(n, _)| *n == k)
                .and_then(|(_, v)| v)
                .map(|v| v.to_string_lossy().into_owned())
        };
        assert_eq!(env("WINE").as_deref(), Some("/w/bin/wine"));
        assert_eq!(env("WINEPREFIX").as_deref(), Some("/data/prefix"));
        assert!(
            env("PATH").unwrap().starts_with("/w/bin:"),
            "Wine's own programs first"
        );
    }

    #[test]
    fn the_installer_of_a_disc_is_found_by_the_recipe_by_autorun_or_by_its_name() {
        let game = temp("installer");
        std::fs::create_dir_all(game.join("Install")).unwrap();
        std::fs::write(game.join("Install/Setup.exe"), "x").unwrap();
        std::fs::write(game.join("GAME.EXE"), "x").unwrap();
        let generic = Recipe::generic();
        // a usual name, in any letter case
        assert_eq!(
            find_installer(&game, &generic).unwrap(),
            "D:\\Install\\Setup.exe"
        );
        // autorun names an installer: it wins over the usual names
        std::fs::write(game.join("SETUP.EXE"), "x").unwrap();
        std::fs::write(
            game.join("autorun.inf"),
            "[autorun]\nopen=install\\setup.exe /a\n",
        )
        .unwrap();
        assert_eq!(
            find_installer(&game, &generic).unwrap(),
            "D:\\Install\\Setup.exe"
        );
        // autorun that names the game itself is not an installer
        std::fs::write(game.join("autorun.inf"), "[autorun]\nopen=game.exe\n").unwrap();
        assert_eq!(find_installer(&game, &generic).unwrap(), "D:\\SETUP.EXE");
        // a recipe names its own, and its folder on the drive comes first
        let recipe = pc_recipes::parse(
            r#"[{"id":"a","name":"A","detect":["X"],"drive_folder":"GAMEDIR","installer":"D:\\install\\setup.exe"}]"#,
        )
        .unwrap();
        assert_eq!(
            find_installer(&game, &recipe[0]).unwrap(),
            "D:\\GAMEDIR\\Install\\Setup.exe"
        );
        // a disc with none
        let none = temp("noinstaller");
        std::fs::write(none.join("GAME.EXE"), "x").unwrap();
        assert!(find_installer(&none, &generic).is_none());
        for d in [game, none] {
            std::fs::remove_dir_all(d).unwrap();
        }
    }

    fn programs(list: &[(&str, u64)]) -> std::collections::BTreeMap<PathBuf, u64> {
        list.iter().map(|(p, n)| (PathBuf::from(p), *n)).collect()
    }

    #[test]
    fn of_the_programs_an_installer_adds_the_game_is_chosen_not_its_helpers() {
        let before = programs(&[("/c/Old/old.exe", 5)]);
        let after = programs(&[
            ("/c/Old/old.exe", 5),
            ("/c/Games/Cool Game/unins000.exe", 900),
            ("/c/Games/Cool Game/COOLGAME.EXE", 300),
            ("/c/Games/Cool Game/editor.exe", 700),
            ("/c/Games/Cool Game/config.exe", 800),
        ]);
        // the one with the name of the game, though a bigger one is there
        let chosen = choose_installed(&before, &after, "cool-game-1999").unwrap();
        assert_eq!(chosen, PathBuf::from("/c/Games/Cool Game/COOLGAME.EXE"));
        // no name in common: the biggest that is not a helper
        let chosen = choose_installed(&before, &after, "zzz").unwrap();
        assert_eq!(chosen, PathBuf::from("/c/Games/Cool Game/editor.exe"));
        // only helpers were added: one of them rather than nothing
        let only = programs(&[("/c/Old/old.exe", 5), ("/c/G/unins000.exe", 9)]);
        assert!(choose_installed(&before, &only, "g").is_ok());
        // nothing was added: the installer was not finished
        assert!(choose_installed(&before, &before, "g")
            .unwrap_err()
            .contains("finished"));
    }

    #[test]
    fn the_programs_of_windows_itself_are_not_what_an_installer_added() {
        let c = temp("drive-c");
        std::fs::create_dir_all(c.join("windows/system32")).unwrap();
        std::fs::create_dir_all(c.join("Games/X")).unwrap();
        std::fs::write(c.join("windows/system32/notepad.exe"), "x").unwrap();
        std::fs::write(c.join("Games/X/x.EXE"), "x").unwrap();
        std::fs::write(c.join("Games/X/readme.txt"), "x").unwrap();
        let found = programs_under(&c);
        assert_eq!(found.len(), 1);
        let (file, _) = found.iter().next().unwrap();
        assert_eq!(windows_path(&c, file).unwrap(), "C:\\Games\\X\\x.EXE");
        std::fs::remove_dir_all(c).unwrap();
    }

    #[test]
    fn a_program_installed_on_c_is_started_from_its_own_folder() {
        let root = temp("started-from");
        std::fs::create_dir_all(root.join("prefix/drive_c/Games/X")).unwrap();
        std::fs::create_dir_all(root.join("game")).unwrap();
        let wine = Path::new("/w/bin/wine");
        let l = Launch::of(None, true);
        let installed = spec(wine, &root, "C:\\games\\x\\x.exe", "800x600", &l);
        assert_eq!(installed.cwd.unwrap(), root.join("prefix/drive_c/Games/X"));
        // one on the CD drive, or one whose folder is not there, starts in the game's files
        let on_cd = spec(wine, &root, "D:\\X.EXE", "800x600", &l);
        assert_eq!(on_cd.cwd.unwrap(), root.join("game"));
        let missing = spec(wine, &root, "C:\\Nope\\x.exe", "800x600", &l);
        assert_eq!(missing.cwd.unwrap(), root.join("game"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_command_that_hangs_is_ended_and_one_that_fails_says_why() {
        let dir = temp("bounded");
        let log = dir.join("out.log");
        let limit = std::time::Duration::from_millis(300);
        // done in time
        assert!(run_bounded(
            Command::new("/bin/sh").args(["-c", "echo hi"]),
            limit * 10,
            &log
        )
        .is_ok());
        // a failure carries what the command said last
        let err = run_bounded(
            Command::new("/bin/sh").args(["-c", "echo one; echo two >&2; exit 3"]),
            limit * 10,
            &log,
        )
        .unwrap_err();
        assert!(err.contains("two") && err.contains("3"), "{err}");
        // one that hangs is ended after the limit, even when something it started keeps going
        let started = std::time::Instant::now();
        let err = run_bounded(
            Command::new("/bin/sh").args(["-c", "sleep 30 & wait"]),
            limit,
            &log,
        )
        .unwrap_err();
        assert!(err.contains("was ended"), "{err}");
        assert!(started.elapsed().as_secs() < 5);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn the_cd_drive_of_a_game_is_right_only_when_it_points_where_its_recipe_says() {
        let root = temp("cd-right");
        let (game, cdroot) = (root.join("game"), root.join("cdroot"));
        std::fs::create_dir_all(&game).unwrap();
        std::fs::create_dir_all(root.join("prefix/dosdevices")).unwrap();
        let prefix = root.join("prefix");
        let road_rash = pc_recipes::by_id("road-rash");
        let generic = pc_recipes::by_id("generic");
        // no drive at all
        assert!(!cd_drive_is_right(&prefix, road_rash, &game, &cdroot));
        // the drive of a game of unknown recipe (the files themselves) is wrong for Road Rash,
        // whose files are under a folder of the drive
        setup_cd_drive(&prefix, generic, &game, &cdroot, "x").unwrap();
        assert!(cd_drive_is_right(&prefix, generic, &game, &cdroot));
        assert!(!cd_drive_is_right(&prefix, road_rash, &game, &cdroot));
        // made again for Road Rash, it is right for it and wrong for the other
        setup_cd_drive(&prefix, road_rash, &game, &cdroot, "x").unwrap();
        assert!(cd_drive_is_right(&prefix, road_rash, &game, &cdroot));
        assert!(!cd_drive_is_right(&prefix, generic, &game, &cdroot));
        assert!(
            cdroot.join("ROADRASH").exists()
                || std::fs::symlink_metadata(cdroot.join("ROADRASH")).is_ok()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    /// A zip with the given files (names and contents).
    fn make_zip(path: &Path, files: &[(&str, &str)]) {
        let mut w = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, body) in files {
            w.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut w, body.as_bytes()).unwrap();
        }
        w.finish().unwrap();
    }

    #[test]
    fn a_zip_with_one_folder_is_unpacked_as_the_game_and_only_once() {
        let dir = temp("zip-game");
        let zip = dir.join("Mega Man X8.zip");
        make_zip(
            &zip,
            &[
                ("megaman-x8/Megaman X8.exe", "exe"),
                ("megaman-x8/SetupReg.exe", "setup"),
                ("megaman-x8/ogg/a.ogg", "sound"),
            ],
        );
        assert!(is_archive(&zip));
        let root = dir.join("pc/mega");
        let game = unpack_into_game(&zip, &root, &|_| {}).unwrap();
        // the folder inside the zip is the game's folder: its files are at the top
        assert_eq!(game, root.join("game"));
        assert_eq!(std::fs::read(game.join("Megaman X8.exe")).unwrap(), b"exe");
        assert!(game.join("ogg/a.ogg").is_file());
        assert!(!root.join("game.unpacking").exists(), "no leftover");
        // the recipe is known from the files
        assert_eq!(pc_recipes::for_disc(&game).unwrap().id, "megaman-x8");
        // the second time nothing is unpacked: a file that was changed stays changed
        std::fs::write(game.join("Megaman X8.exe"), "changed").unwrap();
        unpack_into_game(&zip, &root, &|l| panic!("unpacked again: {l}")).unwrap();
        assert_eq!(
            std::fs::read(game.join("Megaman X8.exe")).unwrap(),
            b"changed"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_archive_with_files_at_the_top_is_unpacked_as_it_is_and_a_bad_one_is_refused() {
        let dir = temp("zip-flat");
        let zip = dir.join("flat.zip");
        make_zip(&zip, &[("GAME.EXE", "a"), ("DATA/x.bin", "b")]);
        let game = unpack_into_game(&zip, &dir.join("a"), &|_| {}).unwrap();
        assert!(game.join("GAME.EXE").is_file() && game.join("DATA/x.bin").is_file());
        // not an archive at all: refused, and nothing is left that says it was unpacked
        let bad = dir.join("bad.zip");
        std::fs::write(&bad, "not a zip").unwrap();
        assert!(unpack_into_game(&bad, &dir.join("b"), &|_| {}).is_err());
        assert!(!dir.join("b/unpacked").exists());
        // a name that would leave the folder is refused
        let evil = dir.join("evil.zip");
        make_zip(&evil, &[("../evil.txt", "x")]);
        assert!(unpack_into_game(&evil, &dir.join("c"), &|_| {}).is_err());
        assert!(!dir.join("evil.txt").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_7z_is_unpacked_like_a_zip() {
        let dir = temp("sevenz-game");
        let src = dir.join("src/game");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("GAME.EXE"), "x").unwrap();
        let archive = dir.join("game.7z");
        sevenz_rust2::compress_to_path(dir.join("src"), &archive).unwrap();
        assert!(is_archive(&archive));
        let game = unpack_into_game(&archive, &dir.join("pc"), &|_| {}).unwrap();
        assert!(
            game.join("GAME.EXE").is_file(),
            "{:?}",
            std::fs::read_dir(&game).map(|d| d.count())
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_programs_a_recipe_runs_first_are_on_the_cd_drive_where_the_game_is() {
        let mega = pc_recipes::by_id("megaman-x8");
        assert_eq!(mega.setup, ["SetupReg.exe"]);
        assert_eq!(drive_prefix(mega), "");
        assert_eq!(drive_prefix(pc_recipes::by_id("road-rash")), "ROADRASH\\");
        // the game asks for a window that the app zooms, so it has no frame even when it was set up
        // with one, and no virtual desktop either
        let l = Launch::of(Some(mega), true);
        assert!(!l.framed && l.zoom);
        assert!(Launch::of(Some(pc_recipes::by_id("road-rash")), true).framed);
        let s = spec(
            Path::new("/w/bin/wine"),
            Path::new("/d"),
            mega.program.as_deref().unwrap(),
            &mega.screen,
            &l,
        );
        assert_eq!(s.args, ["R:\\Megaman X8.exe"]);
        assert!(s.zoom, "the app is told to zoom its window");
        let other = spec(
            Path::new("/w/bin/wine"),
            Path::new("/d"),
            "R:\\G.EXE",
            "800x600",
            &Launch::of(None, true),
        );
        assert!(!other.zoom);
    }
}
