# vuretro plan

Goal: a retro game frontend with a real UI, running on macOS, Windows and Linux.
Games run through libretro cores; the app handles the interface, library management and controls.

Try the current state: `make dev` (see [DEVELOPMENT.md](DEVELOPMENT.md)). Architecture: [ARCHITECTURE.md](ARCHITECTURE.md). Screen designs: [UI.md](UI.md).

## Roadmap

| Phase | Scope | Verified by | Status |
|---|---|---|---|
| 0 | Workspace, CI, fake core | `make ci` | ✅ |
| 1 | Runner plays games | `make dev`, `make play ROM=…` | in progress |
| 2 | Tauri UI (see [UI.md](UI.md)) | `make dev` | first version done |
| M1 | Releasable base: real-game checks on 3 OSes, in-app core installer, installers, updater | install from a package, play without a terminal | not started |
| M2 | Play experience: `wgpu`, shaders, in-game menu, slots with thumbnails, core options UI, rewind | real play | not started |
| M3 | More systems: data-driven registry, NES, GB/GBC, Genesis, SMS, PC Engine, multi-disc | `make cores` + ROMs | SNES + GBA done |
| M4 | Rich library: hash identification, cover art, collections, gamepad-driven UI | big ROM folder | not started |
| M5 | Advanced: RetroAchievements, hardware-render cores, netplay | per feature | not started |

Phases 0 to 2 are the foundation. M1 to M5 are ordered by what gets vuretro to real users first (see [Gaps](#gaps-compared-with-other-frontends)).

## Phase 0: Setup ✅

- [x] Cargo workspace, 3-OS CI matrix (fmt, clippy, test, smoke test)
- [x] Minimal libretro FFI (`crates/libretro-sys`)
- [x] Per-OS core download script (`cores/fetch-cores.sh`)
- [x] Fake core to test without ROMs (`crates/test-core`)

## Phase 1: Runner plays games (in progress)

- [x] Dynamic core loading (`libloading`), all callbacks registered
- [x] Video: XRGB8888 / RGB565 / 0RGB1555 → window (`minifb`)
- [x] Audio: `cpal` + linear resampling, bounded buffer latency
- [x] Input: keyboard + gamepad (`gilrs`), left stick as D-pad, 2 players
- [x] 4 save-state slots (F5/F7/F6), reset, fast-forward
- [x] Headless mode + screenshot for CI
- [x] Real cores load, run and save-state on macOS (fbneo, pcsx_rearmed, snes9x, mgba fetched by `make cores`; verified with synthetic ROMs, not commercial games)
- [ ] Real-game check on all 3 OSes: `make play ROM=…/mslug.zip` (FBNeo), `make play CORE=pcsx ROM=…/crash.chd` (PCSX ReARMed) (tracked in M1)
- [x] Core options (`GET_VARIABLE`) read from `<data dir>/config/<core>.cfg`; `--list-options` prints a template
- [x] Audio dynamic rate control (resampler speed follows buffer level); video stays timer-paced
- [x] Save SRAM (`.srm`) on exit and every 30 s, load on start

## Phase 2: Tauri UI

- [x] Tauri v2 + Svelte 5 in `ui/` and `src-tauri/` (own Cargo workspace so core CI needs no webview libs)
- [x] **Library** screen: system sidebar, "Continue playing" hero, cover grid, details panel, search, sort, favorites, welcome screen with demo
- [x] **Cores & BIOS** screen: switch systems on/off, BIOS status, drag-and-drop or browse BIOS files (MD5 check, renamed dumps accepted)
- [x] **Controls** screen: keyboard layout, hotkeys, gamepad notes (read-only)
- [x] Controls: device per player + swap, live gamepad list, per-player key and button rebinding, dead zone, stick-as-D-pad (`config/controls.json`, shared crate `vuretro-controls`)
- [ ] Controls: per-system and per-game overrides, button diagram, analog trigger and right-stick options (moved to M2)
- [x] Tauri commands calling `vuretro-library` (scan, BIOS check/import) and spawning `vuretro-runner`, reporting early failures from `runner.log`
- [x] Metadata (play time, last played, favorites) in `library.json`; SQLite deferred until the library is big enough to need it
- [ ] Save-slot thumbnails and slot picker in the details panel (needs runner stdout JSON; moved to M2)
- [ ] Verify the window on Windows and Linux; CI job `app` is added but not yet seen green (moved to M1)

## Gaps compared with other frontends

Compared with RetroArch, OpenEmu, ES-DE/Pegasus and Playnite/LaunchBox, what vuretro lacks today:

| Area | Missing | Milestone |
|---|---|---|
| Release | Real-game checks on all 3 OSes; CI `app` job not yet green; no installers or updater; cores are fetched with `make cores`, not by the app | M1 |
| Play experience | `wgpu` renderer (aspect, integer scale, fullscreen), shaders/CRT, in-game menu, rewind, cheats, save-slot picker with thumbnails, core options UI (only `.cfg` files now) | M2 |
| Systems | Only 4 (Neo Geo, PS1, SNES, GBA); no NES, GB/GBC, Genesis, SMS, PC Engine; no choice of core per system; no PS1 disc swap | M3 |
| Library | No cover art source or scraper, no hash identification (No-Intro), no collections or tags, no multi-disc grouping | M4 |
| Input | No per-system/per-game overrides, rumble, rebinding of in-game hotkeys, gamepad hotkeys, lightgun/mouse | M2 |
| Living-room use | UI cannot be driven by a gamepad ("big picture") | M4 |
| Advanced | RetroAchievements, netplay, hardware-render cores (N64, PSP, Saturn), run-ahead | M5 |

## M1: Releasable base

Goal: someone installs vuretro from a package, opens it, gets cores, adds a ROM and plays, with no terminal.

- [ ] Real-game check on macOS, Windows and Linux (carried over from Phase 1)
- [ ] Verify the app window on Windows and Linux; CI job `app` green (carried over from Phase 2)
- [ ] **In-app core installer**: download from the libretro buildbot, verify a checksum, store in `<data dir>/cores`. New `CoreInstaller` port in `vuretro-core`, adapter in `vuretro-library`; Cores & BIOS screen gets an Install button and progress
- [ ] macOS: signed + notarized `.dmg` (universal arm64/x86_64)
- [ ] Windows: signed MSI/NSIS
- [ ] Linux: AppImage, Flatpak
- [ ] Auto-update (Tauri updater) via GitHub Releases
- [ ] Review each core's license before bundling; prefer downloading cores at install time over shipping them (FBNeo and snes9x are non-commercial)

## M2: Play experience

Goal: fullscreen, shaders, slots with thumbnails and core options without leaving the app.

- [ ] `wgpu` renderer replacing `minifb`: keep aspect ratio, integer scale, fullscreen. The `Screen` trait in `crates/runner/src/ports.rs` stays, so `Session` and its tests do not change
- [ ] CRT shader (scanlines, curvature), toggleable
- [ ] Runner reports to the app as JSON on stdout (play time, save slots); `GameProcess` in `vuretro-core` gets an event stream
- [ ] In-game menu (Esc): save/load, change slot, back to Library
- [ ] Save-slot thumbnails and slot picker in the details panel (carried over from Phase 2)
- [ ] **Core options screen** in the app, built from the core's `SET_VARIABLES`, replacing hand-edited `.cfg` files
- [ ] Per-game and per-system overrides on top of the global `controls.json` and `game.json` (carried over from Phase 2)
- [ ] Controls: button diagram, analog trigger and right-stick options (carried over from Phase 2); rumble; rebindable in-game hotkeys; gamepad hotkey chord
- [ ] Rewind, then cheats

## M3: More systems

Goal: adding a system is a data change plus a test, not a code change.

- [x] SNES (snes9x) and GBA (mGBA) enabled: registry, extensions (incl. zip), optional GBA BIOS, `make play CORE=snes|gba`
- [ ] Move `registry` from Rust constants to data (TOML/JSON): core, extensions, folder aliases, BIOS
- [ ] More than one core per system, selectable per system
- [ ] Genesis (Genesis Plus GX), NES, GB/GBC, Master System, PC Engine
- [ ] Multi-disc PS1 via `.m3u` + disc swap (`SET_DISK_CONTROL_INTERFACE`)

## M4: Rich library

Goal: a large ROM folder gets proper names, cover art and can be browsed from the couch.

- [ ] Identify games by hash (CRC/MD5 against No-Intro DATs) for correct names and duplicate detection
- [ ] Cover art: user-supplied, or from a metadata source (ScreenScraper, libretro-thumbnails) through a new `MetadataSource` port; cache in `covers/`. Check each service's terms and API key rules
- [ ] Collections and tags; group multi-disc games; richer filters
- [ ] Gamepad-driven navigation of the whole UI

## M5: Advanced

Only when there is demand; each item is large.

- [ ] RetroAchievements (`rcheevos`, MIT; needs per-system ROM hashing)
- [ ] Hardware-render cores (OpenGL/Vulkan) for N64 and PSP: the runner only handles software frames today
- [ ] Netplay, run-ahead, larger shader library (slang)

## Ordering and risks

1. **M1 first**: without installers nothing else reaches users.
2. **M2 and M3 in parallel**: they touch different crates; M3 is cheap and visible.
3. **M4 after M3**: metadata is worth more once there are more systems.
4. **M5 only on demand**, hardware rendering above all.

Architecture rule for all of it: rules and use cases go in `vuretro-core` behind ports, adapters in `vuretro-library`, glue in `src-tauri`, and each new use case gets fake-based tests (see [ARCHITECTURE.md](ARCHITECTURE.md#layers)). Size estimates in this plan are rough.

## Decisions made

| Item | Choice | Notes |
|---|---|---|
| Language | Rust | No GC, clean C FFI, cross-platform |
| UI | Tauri v2 | Rebuild the design in HTML/CSS, small bundle |
| Game execution | Separate `vuretro-runner` process | A core crash does not take down the UI |
| Window/video (Phase 1) | `minifb` | Pure Rust, no SDL; replaced by `wgpu` in M2 |
| Audio | `cpal` | CoreAudio / WASAPI / ALSA |
| Gamepads | `gilrs` | Uses the SDL GameController DB for mapping |
| Neo Geo core | FBNeo | Non-commercial license → app must be free |
| PS1 core | PCSX ReARMed | Light, runs well on arm64; avoids DuckStation (license) |
