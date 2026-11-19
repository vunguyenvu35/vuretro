# Changelog

## Unreleased

- **Core options for FBNeo and similar cores:** the options screen now loads one of your games of that system to read the options (FBNeo announces most of them only after a game is loaded, so the screen was empty), and shows Vuretro's own defaults. For Neo Geo that is the mode picked from your `neogeo.zip` (AES, or the Universe BIOS); switch **Neo-Geo mode** for the arcade way.

## 0.1.4

New since 0.1.3: backup and restore, a better choice of preview pictures, and a crash fix for the arcade core. Still an early release with unsigned packages.

| System | File |
|---|---|
| macOS (Apple Silicon and Intel) | `Vuretro_0.1.4_universal.dmg` |
| Windows | `Vuretro_0.1.4_x64-setup.exe` or `Vuretro_0.1.4_x64_en-US.msi` |
| Linux | `Vuretro_0.1.4_amd64.AppImage`, `.deb` or `.rpm` |

- **Backup and restore** (Settings): one zip with settings, favorites, collections, controls, cheats and optionally saves and cover pictures; restoring first saves a copy of the current state. **Export** the game list as CSV.
- Import: BIOS files found in an imported folder, or picked by mistake as a game, are now installed as BIOS.
- **Previews:** a game that did not start properly (corrupted picture, flat stripes, FBNeo's "romset is missing files for this version" error page, static) no longer gets that picture as its preview; it gets none, so the title shows instead. **Make all previews again…** (right-click in the library) redoes the existing ones.
- Cores that assume the frontend offers a log function no longer crash the runner (MAME 2003-Plus did).
- Fixes: list header overlapping game rows, readability of secondary text, text shown wrongly on Controls and the Install button.

## 0.1.3

Same features as 0.1.2 (see below); the app is now named **Vuretro** (capital V) in its window title, menu, Dock and installer file names. The 0.1.2 packages keep the old lower-case names.

| System | File |
|---|---|
| macOS (Apple Silicon and Intel) | `Vuretro_0.1.3_universal.dmg` |
| Windows | `Vuretro_0.1.3_x64-setup.exe` or `Vuretro_0.1.3_x64_en-US.msi` |
| Linux | `Vuretro_0.1.3_amd64.AppImage`, `.deb` or `.rpm` (package names may stay lower-case) |

Still an early release with unsigned packages; see the notes for 0.1.2 for what is and is not tried yet.

## 0.1.2

A big step from the first builds: many more systems, a core installer inside the app, and per-system and per-game settings. **This is an early release.** Only SNES and Game Boy Advance have been played, and only with test ROMs; the other systems are set up but not yet tried with real games. Packages are **not signed**, so macOS Gatekeeper and Windows SmartScreen will warn when you open them.

### Install

| System | File |
|---|---|
| macOS (Apple Silicon and Intel) | `vuretro_0.1.2_universal.dmg` |
| Windows | `vuretro_0.1.2_x64-setup.exe` or `vuretro_0.1.2_x64_en-US.msi` |
| Linux | `vuretro_0.1.2_amd64.AppImage`, `.deb` or `.rpm` |

On first start open **Cores & BIOS**, press **Install** on the systems you want, add the BIOS files they need, then **Import ROMs**. No terminal needed. Full guide: [docs/USAGE.md](docs/USAGE.md).

### New

**Systems**
- 21 systems are set up (was 4): NES, Game Boy, Game Boy Color, Genesis, Master System, Game Gear, PC Engine, Atari 2600/7800/Lynx, Neo Geo Pocket, WonderSwan, Virtual Boy, Sega 32X, Nintendo DS, Sega Saturn and arcade (MAME), besides Neo Geo, PlayStation, SNES and GBA.
- Systems can have more than one core; pick one per system on Cores & BIOS.
- Multi-disc games: press **F11** to change disc.
- N64, PSP, GameCube, PS2 and 3DS are listed behind a switch, **off by default**. The card shows whether your computer (memory and processor cores) is likely to cope, and asks before turning one on. **These games cannot be played yet**: the 3D renderer is not in this version.

**Playing**
- Save slots with a picture each; start a game from any slot in its details.
- Rewind (hold **Backspace**; switch it on in Settings).
- CRT scanlines (**F4**).
- Cheat codes per game, and a core options screen.
- The window title shows **(slow: about N fps)** when your computer cannot keep up with a game.
- Settings (window size, CRT, rewind) and key bindings can be set **per system and per game**, from the **Applies to** box on Settings and Controls.

**Library**
- Official game names and duplicate detection from No-Intro or Redump DAT files you add.
- Collections, shown in the sidebar.
- Cover art: your own images, or downloaded from libretro-thumbnails (off until you switch it on, because it sends game names to that server). ScreenScraper is supported if you have an account, but needs developer credentials the project does not have yet.
- The whole app can be driven with a gamepad.
- The sidebar can be resized by dragging its edge; slimmer scrollbars.

### Not yet
- 3D systems (see above), Sega CD, DOS and Amiga.
- Fullscreen and shaders (the window is still a simple one).
- Auto-update, signed installers, Flatpak.
- Rumble and rebinding the in-game hotkeys.

### Known limits
- Arcade games need a ROM set that matches the core's version, or they will not start.
- Saturn may be slow on the software cores.
- Light-gun games have no gun input.

Report problems with the text of `runner.log` from the vuretro data folder.

## 0.1.0 – 0.1.1

First builds, tagged while the release workflow was being set up. 0.1.1 is the first one with a macOS package.
