//! Known systems: which core runs them, which files they accept, which BIOS they need.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct BiosSpec {
    pub file: &'static str,
    /// Accepted MD5s (lowercase). Empty = only check presence (e.g. zip sets).
    pub md5: &'static [&'static str],
    pub required: bool,
    pub note: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct System {
    pub id: &'static str,
    pub name: &'static str,
    /// Core file stem; platform suffix (.dylib/.dll/.so) is added at runtime.
    pub core: &'static str,
    pub extensions: &'static [&'static str],
    /// Folder names under the ROM root that force this system.
    pub folders: &'static [&'static str],
    pub bios: &'static [BiosSpec],
    pub enabled: bool,
}

pub const SYSTEMS: &[System] = &[
    System {
        id: "neogeo",
        name: "Neo Geo",
        core: "fbneo_libretro",
        extensions: &["zip", "7z"],
        folders: &["neogeo", "neo-geo", "mvs", "aes"],
        bios: &[BiosSpec {
            file: "neogeo.zip",
            md5: &[],
            required: true,
            note: "MVS / AES BIOS set",
        }],
        enabled: true,
    },
    System {
        id: "psx",
        name: "PlayStation",
        core: "pcsx_rearmed_libretro",
        extensions: &["chd", "cue", "pbp", "m3u", "iso", "img"],
        folders: &["psx", "ps1", "playstation"],
        bios: &[
            BiosSpec {
                file: "scph5501.bin",
                md5: &["490f666e1afb15b7362b406ed1cea246"],
                required: true,
                note: "NTSC-U",
            },
            BiosSpec {
                file: "scph5500.bin",
                md5: &["8dd7d5296a650fac7319bce665a6a53c"],
                required: false,
                note: "NTSC-J",
            },
            BiosSpec {
                file: "scph5502.bin",
                md5: &["32736f17079d0b2b7024407c39bd3050"],
                required: false,
                note: "PAL",
            },
        ],
        enabled: true,
    },
    System {
        id: "snes",
        name: "Super Nintendo",
        core: "snes9x_libretro",
        extensions: &["sfc", "smc"],
        folders: &["snes", "sfc"],
        bios: &[],
        enabled: false,
    },
    System {
        id: "gba",
        name: "Game Boy Advance",
        core: "mgba_libretro",
        extensions: &["gba"],
        folders: &["gba"],
        bios: &[],
        enabled: false,
    },
];

pub fn by_id(id: &str) -> Option<&'static System> {
    SYSTEMS.iter().find(|s| s.id == id)
}

/// File name of a core for the current OS.
pub fn core_filename(core: &str) -> String {
    let ext = if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    };
    format!("{core}.{ext}")
}
