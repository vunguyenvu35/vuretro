# BIOS files and cores: how to set them up and where they come from

Short version:

1. Open **Cores & BIOS**. A system card says **Ready**, **BIOS missing**, **Core missing** or **Off**.
2. **Cores** need nothing from you: the app downloads them (see [Cores](#cores-the-emulators)).
3. **BIOS files** you get from **your own console or arcade board** (see [Where to get a BIOS](#where-to-get-a-bios)).
   Many systems need none at all (see [Systems that need no BIOS](#systems-that-need-no-bios)).
4. Drop the files on **Cores & BIOS → Add BIOS files**. Vuretro checks each one and installs it under the right name.

## This folder

Put the BIOS files you own here to check them before you add them to the app. **Nothing in this
folder is committed** (`.gitignore` skips everything but this README): BIOS files are copyrighted,
and Vuretro does not provide them.

```bash
make check-bios DIR=docs/bios     # what is each file, and which systems would be Ready with them
```

## Cores (the emulators)

Cores are free and open. You never have to look for them: **the app downloads the core a game
needs when you press Play**, from the official libretro build server
(<https://buildbot.libretro.com/nightly/>), and **Cores & BIOS → Install** does the same by hand.

- `make cores` downloads the usual ones into `cores/bin/`.
- `make cores-offline` saves their zips in `cores/offline/` so a computer with no internet can
  install them (see `cores/offline/README.md`). Copy that folder next to the app.
- By hand: the same server has one zip per core for each OS, for example
  `https://buildbot.libretro.com/nightly/apple/osx/arm64/latest/mame2003_plus_libretro.dylib.zip`
  (`windows/x86_64`, `linux/x86_64`, `apple/osx/x86_64` for the others). Unzip it into
  `<data dir>/cores`.

## Systems that need no BIOS

A BIOS is the firmware of the original machine. Some cores bring their own replacement, so
the system works without one (a few games can be less accurate):

| System | Without a BIOS file |
|---|---|
| PlayStation | PCSX ReARMed uses a built-in BIOS. Real BIOS files are more compatible. |
| Game Boy Advance | mGBA has a built-in replacement. An open-source one also exists: [Cult-of-GBA/BIOS](https://github.com/Cult-of-GBA/BIOS) (read its page for the licence and limits). |
| Nintendo DS | melonDS has a built-in replacement for the BIOS (the firmware is optional). |
| Sega Saturn | Yabause has a built-in replacement. Beetle Saturn needs a real BIOS. |
| NES, Game Boy, Game Boy Color, Genesis, Master System, Game Gear, SNES, 32X, WonderSwan, Neo Geo Pocket, Virtual Boy | No BIOS. |

## Where to get a BIOS

**Vuretro cannot give you a BIOS, and it does not link to sites that host them**: those files are
copyrighted, and a download site is not a legal source. The legal source is **your own hardware**:
a BIOS is stored in a chip of the console or the board, and you may make a copy of the one you own.

How, in practice:

- **Consoles that run homebrew** (GBA with a flash cart, Nintendo DS, and others): a small homebrew
  "BIOS dumper" program run on the console writes the BIOS to a file.
- **Other consoles and arcade boards** (PlayStation, Saturn, Neo Geo, Lynx…): the BIOS is read from
  its chip with a reader or programmer, or taken from a dump tool for that console. The page of the
  core in the libretro documentation says which files are needed and how they are usually made.
- **Neo Geo**: `neogeo.zip` is the BIOS set of the MVS/AES hardware. Keep it **as a zip** (do not
  unpack it).

The authoritative lists of file names and MD5 checks are on the libretro documentation:

- BIOS Information Hub: <https://docs.libretro.com/library/bios/> (how to check a file, plus links to every core)
- Guide: <https://docs.libretro.com/guides/bios/>
- Per core: [FBNeo](https://docs.libretro.com/library/fbneo/) (Neo Geo),
  [PCSX ReARMed](https://docs.libretro.com/library/pcsx_rearmed/) (PlayStation),
  [mGBA](https://docs.libretro.com/library/mgba/) (GBA),
  [melonDS](https://docs.libretro.com/library/melonds/) (DS),
  [Yabause](https://docs.libretro.com/library/yabause/) and
  [Beetle Saturn](https://docs.libretro.com/library/beetle_saturn/) (Saturn),
  [FCEUmm](https://docs.libretro.com/library/fceumm/) (NES, Disk System),
  [Handy](https://docs.libretro.com/library/handy/) (Lynx),
  [ProSystem](https://docs.libretro.com/library/prosystem/) (Atari 7800),
  [MAME 2003-Plus](https://docs.libretro.com/library/mame2003_plus/) (arcade),
  [Beetle PCE Fast](https://docs.libretro.com/library/beetle_pce_fast/) (PC Engine).

## What Vuretro looks for

| System | File | Needed? | MD5 (Vuretro accepts this dump) |
|---|---|---|---|
| Neo Geo | `neogeo.zip` (MVS / AES BIOS set) | **Required** | any set that has the BIOS files |
| Arcade (MAME), PGM games | `pgm.zip` | Only for PGM games (Oriental Legend, Knights of Valour…) | any |
| PlayStation (NTSC-U) | `scph5501.bin` | Optional | `490f666e1afb15b7362b406ed1cea246` |
| PlayStation (NTSC-J) | `scph5500.bin` | Optional, for Japanese discs | `8dd7d5296a650fac7319bce665a6a53c` |
| PlayStation (PAL) | `scph5502.bin` | Optional | `32736f17079d0b2b7024407c39bd3050` |
| Game Boy Advance | `gba_bios.bin` | Optional | `a860e8c0b6d573d191e4ec7db1b1e4f6` |
| Nintendo DS | `bios7.bin`, `bios9.bin`, `firmware.bin` | Optional | any |
| Sega Saturn | `saturn_bios.bin` | Optional (Yabause) | any |
| Sega Saturn (Beetle) | `sega_101.bin` (JP) or `mpr-17933.bin` (US/EU) | One of them, for Beetle Saturn | `85ec9ca47d8f6807718151cbcca8b964` / `3240872c70984b6cbfda1586cab68dbe` |
| NES, Famicom Disk System | `disksys.rom` | Only for Disk System games | `ca30b50f880eb660a320674ed365ef7a` |
| PC Engine CD | `syscard3.pce` | Only for CD games (not supported yet) | `38179df8f4ac870017db21ebcbf53114` |
| Atari 7800 | `7800 BIOS (U).rom` | Optional | `0763f1ffb006ddbe32e52d497ee848ae` |
| Atari Lynx | `lynxboot.img` | Recommended, some games need it | `fcd403db69f54290b903b5f6be88b9d8` |

The list of the systems and their files is in `crates/core/data/systems.json`.

## Add them and check them

1. Drop the files on **Cores & BIOS** (or click the drop zone). A renamed file still works: Vuretro
   recognises it by its MD5 and installs it with the right name. A BIOS inside a folder of games you
   import is picked up too.
2. Or copy them yourself into the `system` folder (**File ▸ Show BIOS Folder** opens it):
   macOS `~/Library/Application Support/dev.vuvn.vuretro/system`,
   Windows `%APPDATA%\vuvn\vuretro\data\system`, Linux `~/.local/share/vuretro/system`.
3. The card of the system turns **Ready**. If it does not, check the file's MD5:

   ```bash
   md5 file.bin                      # macOS
   md5sum file.bin                   # Linux
   certutil -hashfile file.bin MD5   # Windows
   ```

   and compare it with the table above or the libretro page of the core. A different hash means a
   different version or a damaged dump: it can still work for some games, but Vuretro will not
   count it as the expected file.

Common problems:

- **mGBA ignores `gba_bios.bin`**: turn on the core option **Use BIOS file if found**.
- **Neo Geo: `sfix.sfix NOT FOUND` or "files missing"**: `neogeo.zip` was unpacked, renamed, or is a
  set for another version of the core. Use it as it is.
- **A BIOS is listed as Optional but the game still does not start**: read the end of
  `<data dir>/logs/runner.log` (the app shows it when a game fails to start).

## ROM sets and cores

A MAME or FBNeo ROM set fits **one version** of the core. If a game says files are missing, the set
was made for another version: use the core that matches the set (see **Core ▸ Runs on** in the
game's details), or a set made for the version of the core you use.
