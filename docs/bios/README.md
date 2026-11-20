# BIOS files and cores: where to get them

## This folder

Put the BIOS files you own here. **Nothing in this folder is committed** (`.gitignore` skips
everything but this README): BIOS files are copyrighted, and Vuretro does not provide them.

To use them, drop them on **Cores & BIOS → Add BIOS files** in the app. Vuretro checks the MD5 and
installs each one in the app's `system` folder under the right name (a renamed file still works).
You can also copy them to that folder yourself.

## Cores (the emulators)

Cores are free and open. You never have to look for them: **the app downloads the core a game
needs when you press Play**, from the official libretro build server
(<https://buildbot.libretro.com/nightly/>), and **Cores & BIOS → Install** does the same by hand.

- `make cores` downloads the usual ones into `cores/bin/`.
- `make cores-offline` saves their zips in `cores/offline/` so a computer with no internet can
  install them (see `cores/offline/README.md`). Copy that folder next to the app.
- By hand: the same server has one zip per core for each OS, for example
  `https://buildbot.libretro.com/nightly/apple/osx/arm64/latest/mame_libretro.dylib.zip`
  (`windows/x86_64`, `linux/x86_64`, `apple/osx/x86_64` for the others). Unzip it into
  `<data dir>/cores`.

## BIOS files

A BIOS is the firmware of the original machine. **Vuretro cannot give you one**, and this
repository does not hold any. The legal source is **your own hardware**: dump the BIOS of a
console or arcade board you own (the libretro documentation explains how for each system).

What each core expects, with the exact names and MD5 checks, is in the libretro documentation:
<https://docs.libretro.com/> (open the page of the core, for example *FBNeo*, *PCSX ReARMed*,
*Beetle Saturn*, *mGBA*, and read its **BIOS** section).

What Vuretro looks for:

| System | File | Needed? |
|---|---|---|
| Neo Geo | `neogeo.zip` (MVS / AES BIOS set) | **Required** |
| Arcade (MAME), PGM games | `pgm.zip` | Only for PGM games (Oriental Legend, Knights of Valour…) |
| PlayStation | `scph5501.bin` (NTSC-U), `scph5500.bin` (NTSC-J), `scph5502.bin` (PAL) | Optional (the core has a built-in BIOS, less compatible) |
| Game Boy Advance | `gba_bios.bin` | Optional (mGBA has a replacement) |
| Nintendo DS | `bios7.bin`, `bios9.bin`, `firmware.bin` | Optional (melonDS has a replacement) |
| Sega Saturn | `saturn_bios.bin`; `sega_101.bin` or `mpr-17933.bin` | Optional for Yabause; Beetle Saturn needs one |
| NES, Famicom Disk System | `disksys.rom` | Only for Disk System games |
| Atari Lynx / 7800 | `lynxboot.img` / `7800 BIOS (U).rom` | Recommended / optional |

The list of the systems and their files is in `crates/core/data/systems.json`.

## ROM sets and cores

A MAME or FBNeo ROM set fits **one version** of the core. If a game says files are missing, the set
was made for another version: use the core that matches the set (see **Core ▸ Runs on** in the
game's details), or a set made for the version of the core you use.
