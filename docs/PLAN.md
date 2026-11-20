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
| M1 | Releasable base: real-game checks on 3 OSes, in-app core installer, installers, updater | install from a package, play without a terminal | in progress: in-app core installer, unsigned macOS dmg and release workflow done; 3-OS checks, signing, Flatpak, updater open |
| M2 | Play experience: `wgpu`, shaders, in-game menu, slots with thumbnails, core options UI, rewind | real play | in progress: core options, slots with thumbnails, CRT scanlines, rewind, cheats done; wgpu, rumble, per-game overrides open |
| M3 | More systems: data-driven registry, NES, GB/GBC, Genesis, SMS, PC Engine, multi-disc | `make cores` + ROMs | data registry, core choice, wave 1 and 2 registered, disc swap; only SNES + GBA play-tested |
| M4 | Rich library: hash identification, cover art, collections, gamepad-driven UI | big ROM folder | in progress: DAT identification, collections, opt-in cover art, gamepad navigation done; multi-disc grouping, user art open |
| M5 | Advanced: RetroAchievements, hardware-render cores, netplay | per feature | not started |

Phases 0 to 2 are the foundation. M1 to M5 are ordered by what gets vuretro to real users first (see [Gaps](#gaps-compared-with-other-frontends)).

**Last updated 2026-11-18.** Built since the plan was written: data registry and 20 systems, per-system core choice, in-app core installer, core options screen, save slots with thumbnails, CRT scanlines, rewind, cheats, disc swap, DAT identification, collections, cover art (libretro-thumbnails, own images, ScreenScraper), gamepad navigation of the UI, release workflow and a local unsigned macOS `.dmg`. Almost all of it is verified by tests with fakes only; nothing has been played with real games except SNES and GBA with synthetic ROMs. Still open: real-game checks on 3 OS, signing, updater, Flatpak, `wgpu` renderer, per-game control overrides, hotkey rebinding, multi-disc grouping, Sega CD, DOS, and all of M5.

## Phase 0: Setup ✅

- [x] Cargo workspace, 3-OS CI matrix (fmt, clippy, test, smoke test)
- [x] Minimal libretro FFI (`crates/libretro-sys`)
- [x] Per-OS core download script (`cores/fetch-cores.sh`)
- [x] Fake core to test without ROMs (`crates/test-core`)

## Phase 1: Runner plays games (in progress)

- [x] Dynamic core loading (`libloading`), all callbacks registered
- [x] Video: XRGB8888 / RGB565 / 0RGB1555 → window (`minifb`)
- [x] Audio: `cpal`, lock-free queue, band-limited (Kaiser sinc) resampling, bounded buffer latency
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
- [x] Save-slot thumbnails and slot picker in the details panel (done in M2)
- [ ] Verify the window on Windows and Linux (moved to M1). CI first run on 2026-11-18: Windows `build` and `app` green, Linux `build` green; Linux `app` failed for a missing `libudev-dev` (fixed; all 6 jobs green on eab2742), `macos-14` is not a supported runner (switched to `macos-latest`). CI only compiles and runs tests, nobody has opened the window yet

## Gaps compared with other frontends

Compared with RetroArch, OpenEmu, ES-DE/Pegasus and Playnite/LaunchBox, what vuretro lacks today:

| Area | Missing | Milestone |
|---|---|---|
| Release | Real-game checks on all 3 OSes; CI `app` job not yet green; no installers or updater | M1 |
| Play experience | `wgpu` renderer (aspect, integer scale, fullscreen), shaders/CRT, in-game menu, rewind, cheats, save-slot picker with thumbnails, core options UI (only `.cfg` files now) | M2 |
| Systems | Only 4 (Neo Geo, PS1, SNES, GBA); no NES, GB/GBC, Genesis, SMS, PC Engine (wave 1), no Atari, handhelds, Sega CD, NDS, MAME, DOS (wave 2); no choice of core per system; no PS1 disc swap | M3 |
| Library | No cover art source or scraper, no hash identification (No-Intro), no collections or tags, no multi-disc grouping | M4 |
| Input | No per-system/per-game overrides, rumble, rebinding of in-game hotkeys, gamepad hotkeys, lightgun/mouse | M2 |
| Living-room use | UI cannot be driven by a gamepad ("big picture") | M4 |
| Advanced | RetroAchievements, netplay, hardware-render cores (N64, PSP, Saturn), run-ahead | M5 |

## M1: Releasable base

Goal: someone installs vuretro from a package, opens it, gets cores, adds a ROM and plays, with no terminal.

- [ ] Real-game check on macOS, Windows and Linux (carried over from Phase 1)
- [ ] Verify the app window on Windows and Linux; CI job `app` green on all 3 OS (all 6 CI jobs green on commit eab2742: compile and tests only, windows never opened)
- [x] **In-app core installer**: `CoreInstaller` port, `BuildbotInstaller` adapter (HTTPS download into `<data dir>/cores`, archive and native-library checks; the buildbot publishes no checksums), `install_core` command with `core-progress` events, Install button with progress on Cores & BIOS. Verified live with `cargo test -p vuretro-library -- --ignored`; the Install button itself is not yet clicked through on a clean machine
- [ ] macOS: signed + notarized universal `.dmg`. Done so far: `make app-build` produces an unsigned arm64 `.dmg` (6 MB) with `vuretro-runner` bundled inside (checked on this Mac); `release.yml` builds universal and signs when the Apple secrets exist (not run yet). First tagged run (v0.1.0): Windows and Linux packages built; macOS failed because the per-architecture sidecars were missing (fixed in `prepare-sidecar.sh`)
- [ ] Windows: signed MSI/NSIS (`release.yml` builds it; not run yet, signing needs a certificate)
- [ ] Linux: AppImage (`release.yml` builds it; not run yet), Flatpak (no manifest yet)
- [ ] Auto-update (Tauri updater) via GitHub Releases
- [ ] Review each core's license before bundling; prefer downloading cores at install time over shipping them (FBNeo and snes9x are non-commercial)

## M2: Play experience

Goal: fullscreen, shaders, slots with thumbnails and core options without leaving the app.

- [ ] `wgpu` renderer replacing `minifb`: keep aspect ratio, integer scale, fullscreen. The `Screen` trait in `crates/runner/src/ports.rs` stays, so `Session` and its tests do not change
- [x] CRT scanlines, toggleable (F4, Size menu, Settings): CPU version in `crt.rs` (pure, tested). Curvature and a GPU shader come with `wgpu`
- [x] Runner-to-app reporting: decided against stdout JSON; the runner writes files the app reads (save state + `.png` thumbnail per slot, `--load-slot N` to start from one). Play time is already tracked by the app from the process lifetime
- [ ] In-game menu (Esc): save/load, change slot, back to Library
- [x] Save-slot thumbnails and slot picker in the details panel (`Store::list_slots`, `SaveSlots.svelte`, `launch_game(path, slot)`; tested with fakes, not yet seen in the window)
- [x] **Core options screen** in the app: runner `--options-json` (no ROM needed), `Launcher::list_core_options`, `Store::{load,save}_core_options`, `App::{core_options,set_core_options}`, dialog on Cores & BIOS (shared `.cfg` format in `vuretro-settings`). Not yet checked in the window with a real core
- [x] Per-system overrides of window size, CRT and rewind (`vuretro-settings::scoped`: general → system → game layers, only differences stored, hotkey changes saved to the owning layer) and per-system key bindings (`config/controls/<system>.json`); Settings and Controls screens have an "Applies to" box. The per-game layer has its own editor ("Settings for this game" in the details panel, same three settings, games told apart by file name). Per-game key bindings are done too (`config/controls/games/<game>.json`, resolved game → system → general by the runner and by `App::controls_for_game`; button on the details panel and an "Applies to" entry on Controls). Hiding buttons a system does not have is still open. Tested with fakes and unit tests, not yet seen in the window
- [ ] Controls: button diagram, analog trigger and right-stick options (carried over from Phase 2); rebindable in-game hotkeys
- [x] Gamepad hotkey chord: hold Select plus another button (quit, save, load, slot, fast-forward, rewind, pause, screenshot, sound; `padkeys.rs`, pure and tested). Pad presses of a chord are hidden from the game. Keys that are runner hotkeys (Backspace, F1 to F12) can no longer be bound as buttons
- [x] Speed: fast-forward at 2×/3×/4×/8× (Settings, per system), held with Tab or latched with F1, slow motion at half speed (Shift+F1); the audio thread plays at the game speed and is muted by default while fast (`speed.rs`, pure and tested; "Sound while fast-forwarding" switch). Tested with fakes, not heard in a real game
- [x] Carry on where I left off: the runner saves `<game>.stateauto` on exit and loads it at start when the setting is on; a chosen slot wins over it
- [x] Picture: full-screen borderless window (macOS and Windows; `display::screen_size`, Alt+Enter), whole-number scaling (`scale.rs`) and an LCD pixel grid (`lcd.rs`, Shift+F4), all on the CPU. Full screen on macOS is not seen in a real window yet (menu bar and Dock coverage unchecked); Linux has none until a display size can be read there
- [x] Auto coin for arcade systems (`coin_op` flag in `systems.json`; setting `auto_coin`, off by default because Start should be Start; per system and game): a new press of Start puts a coin in (Select for 3 polls), waits 15, then holds Start for 6 (`autocoin.rs`, pure and tested). Checked headless against the real FBNeo with Neo Bomberman and Metal Slug: without it Start only shows INSERT COIN, with it the game starts. Pressed during a screen that is not ready for it, the coin is spent and Start is lost
- [x] Dock icon on macOS: the runner is started from a small app bundle in the data folder (`runner/Vuretro.app`, made by `adapters/bundle.rs` around a copy of the runner, refreshed when the runner changes), so the Dock shows the Vuretro icon from the first moment. Checked by screenshots of the Dock at 0.12 s steps: a bare runner shows the generic "exec" icon for a moment, the bundled one never does. An installed app whose runner is already inside the bundle is left alone
- [x] Moves and combos in fighting-game notation (`vuretro-controls::moves`, pure and tested): `236P`, `j.D, cl.C > 236P`, `A+B`, named motions, `cr.` `j.` prefixes, `,` `>` `xN` `[ ]`, facing left; turned into the steps of a macro sequence (`Macro.notation`, recompiled when the macro is saved) and played by one keyboard key or pad button (checked with the runner's input code and a fake keyboard). The Macros screen has **+ Move**, the live check of the notation and a notation guide. Button names follow the Neo Geo (A B C D) on a Neo Geo game; the mapping to RetroPad buttons was read from the core (`RUST_LOG=debug`). A built library of games and characters was tried and removed: it only had a template with placeholder rows. Timings are approximations (3 frames per direction or button, 8 for a link) and not tried in a real game
- [x] Moves that pick their side (`Macro.auto_facing`): when the trigger goes down, holding left plays the notation mirrored and holding right as written (`Engine::hint_facing`, tested with the engine and the runner's input code)
- [x] Macro profiles per game (`MacroConfig.profiles` and `active`, `Profile`): the game's shared macros, the profile in use and the global macros play together (`for_game`); the Macros screen has a profile bar, the game's details a "Profile in use" selector and a button into the screen. Tested; the guide is `docs/MACROS.md`
- [x] Diagonal keys (`up_left` … `down_right`, keyboard only): one key presses two directions, for keyboards without a numpad; bound on the Controls screen and understood by Record
- [x] Rumble: `GET_RUMBLE_INTERFACE` answered, requests passed to the gamepad through gilrs force feedback (`rumble.rs`, tested; the gamepad call itself is not). gilrs has no force feedback on macOS
- [x] Rewind: bounded state stack (`rewind.rs`, 64 MB), hold Backspace, Settings switch (off by default). Tested with fakes, not felt in a real game
- [x] Cheats: per-game text file (`config/cheats/<rom>.cht`, `vuretro-settings::cheats`), applied by the runner through `retro_cheat_set` when the core has it; editor in the details panel. Untested with a real core (the fake core has no cheat support)

## M3: More systems

Goal: adding a system is a data change plus a test, not a code change.

- [x] SNES (snes9x) and GBA (mGBA) enabled: registry, extensions (incl. zip), optional GBA BIOS, `make play CORE=snes|gba`
Order of work: registry as data first, then systems one by one (each is a data entry, a `make cores` entry and a test).

- [x] Move `registry` from Rust constants to data: `crates/core/data/systems.json`, parsed and validated by `registry::parse_systems` (duplicate ids, empty extensions rejected)
- [x] More than one core per system, selectable per system: `Settings.cores`, `App::set_system_core`, drop-down on Cores & BIOS (tested with fakes)
- [x] Systems wave 1 registered and detected (cores exist on the buildbot; not yet played with real games, tracked in M1) (cheap, software cores, no BIOS or optional BIOS):

| System | Default core | Alternative | License note |
|---|---|---|---|
| NES / Famicom | `fceumm` | `mesen`, `nestopia` | GPL |
| Game Boy / GBC | `gambatte` | `mgba` (already used for GBA) | GPL |
| Genesis / Mega Drive, Master System, Game Gear | `genesis_plus_gx` | `picodrive` | non-commercial, same rule as FBNeo |
| PC Engine / TurboGrafx-16 | `mednafen_pce_fast` | `mednafen_pce` | GPL; CD games need `syscard3.pce` BIOS |

- [x] Saturn registered (`yabause` default, Beetle Saturn as alternative; BIOS optional for Yabause) with a new `folder_only` flag so its disc types (`.cue`, `.chd`) never claim PlayStation files; the Arcade system also lists `mame_libretro` and `mame2003_libretro` as alternatives. Core names checked on the buildbot, nothing played; Saturn may be slow on the software cores
- [x] Systems wave 2 registered and detected, except Sega CD and DOS (Sega CD needs the disc work below; DOS needs keyboard and mouse input in the runner). Cores exist on the buildbot; no real games tried yet:
- [ ] Arcade (MAME 2003-Plus) found crashing at load on macOS because the runner did not offer `GET_LOG_INTERFACE`; fixed, the core now loads and reports missing ROMs normally. Still not played with a real MAME 2003+ romset

| System | Default core | Notes |
|---|---|---|
| Atari 2600 | `stella` | |
| Atari 7800, Lynx | `prosystem`, `handy` | Lynx needs a BIOS |
| Neo Geo Pocket, WonderSwan, Virtual Boy | `mednafen_ngp`, `mednafen_wswan`, `mednafen_vb` | |
| Sega CD, 32X | `genesis_plus_gx`, `picodrive` | Sega CD needs BIOS and disc images (`.chd`/`.cue`) |
| Nintendo DS | `melonds` or `desmume` | two screens: `Screen` and the window layout must support it; BIOS optional |
| Arcade (MAME) | `mame2003_plus` | romset versions must match the core |
| DOS | `dosbox_pure` | folder or `.zip`, keyboard and mouse input |

- [x] Disc swap: the runner handles `SET_DISK_CONTROL_INTERFACE` (`disc.rs`, tested with a fake core) and F11 / Game menu changes disc; `.m3u` playlists are already accepted. Not yet tried with a real multi-disc game. Grouping discs of one game in the library is still open
- [ ] Each system must have: a registry entry, a core download entry, a documented BIOS list, a synthetic-ROM or fake-core test, and a line in [USAGE.md](USAGE.md)

## M4: Rich library

Goal: a large ROM folder gets proper names, cover art and can be browsed from the couch.

- [x] Identify games by hash against DAT files the user adds (`vuretro-library::dat`: XML parser, CRC32 + size, iNES/SMC/Lynx/7800 headers ignored, single-file zips): official titles, duplicate flag. DATs are not bundled (the user adds them on Cores & BIOS). Not yet tried with a real No-Intro DAT
- [x] Cover art from libretro-thumbnails (`ArtSource` port, `ThumbnailsArt` adapter): opt-in switch on Cores & BIOS because it sends game names to a third party; works for games that have an official name from a DAT; box art wins over the screenshot preview. User-supplied cover images (PNG/JPEG, "Set cover image…" in the details panel) are done; ScreenScraper (`ScreenScraperArt`, fallback after libretro-thumbnails, needs the user's account plus developer credentials in `VURETRO_SS_DEVID`/`VURETRO_SS_DEVPASSWORD`) is written and tested with fakes only: the project is not registered there, the system ids were written from memory and nothing was run against the live API. Tested with fakes; not yet run against the live server
- [x] Previews made lazily: only the games on screen (IntersectionObserver, plus the hero and the details panel) are read and made; `App::request_covers` replaces the waiting list with each new request, so scrolling drops what was passed and the worker makes the new ones in order (tested with fakes). A game that already has art (chosen, downloaded or an earlier preview) is not touched, and closing a game no longer replaces its picture with the last frame (`--cover` is no longer passed on launch). Tested with fakes, not watched in the window
- [x] Collections: tags per game (details panel), sidebar list and filter. Still open: group multi-disc games, richer filters
- [x] ROM patches: IPS, UPS and BPS (`vuretro-core::patch`, pure; CRC32 checks for UPS and BPS; "Apply patch…" in a game's right-click menu) write a patched copy `<game> [<patch>].<ext>` into the library and never touch the original. Tested with patches built in the tests, not with files from the wild; IPS32 and games inside archives are not supported
- [x] Gamepad-driven navigation of the whole UI: D-pad or left stick moves focus (`gamepadNav.js`, spatial pick is tested), A presses, B sends Escape; paused while a button is being recorded on Controls. Not yet tried with a real pad in the window

## M5: Advanced

Only when there is demand; each item is large.

- [ ] RetroAchievements (`rcheevos`, MIT; needs per-system ROM hashing)
- [x] Guard rails for 3D systems (done ahead of the renderer): N64, PSP, GameCube, PS2 and 3DS are registered as `hardware` systems that are off until the user turns them on; `assess()` compares memory and cores with each system's minimum and says good, marginal or poor (the GPU is not measured and the verdict says so); the Cores screen shows this computer and asks before turning one on; the runner times every frame and shows "(slow: about N fps)" in the title when it cannot keep up (all systems). Their games stay locked until the renderer exists (`HARDWARE_RENDERER` in `service.rs`). Tested with fakes; real hardware numbers (the minimums) are my estimates
- [ ] Hardware-render cores (OpenGL/Vulkan) for N64 (`mupen64plus_next`), PSP (`ppsspp`), Dreamcast (`flycast`), Saturn: the runner only handles software frames today. Needs `SET_HW_RENDER` and a GL/Vulkan context shared with `wgpu`, so do it after M2
- [x] Backup, restore and CSV export of the library (`vuretro-core::backup`, `ZipBackup` adapter, Settings section); restore refuses entries outside the data folder or outside the allowed kinds and makes a safety copy first. Tested with fakes and a hostile-zip test, not yet used on real data
- [ ] Netplay, run-ahead, larger shader library (slang)

## Done when (acceptance per milestone)

| Milestone | Done when |
|---|---|
| M1 | A clean machine of each OS installs a package, installs a core from the app, adds a ROM and plays; CI builds the installers; an update from version N to N+1 works |
| M2 | A game runs fullscreen with correct aspect and a CRT shader; Esc opens a menu; slots show thumbnails; core options are edited in the app; rewind works on at least SNES |
| M3 | Wave 1 systems play a real game each; adding a wave 2 system touches only data and tests |
| M4 | A folder of 1000+ ROMs shows correct names and covers for most files, and the whole UI works with a gamepad only |
| M5 | Per feature: one real game works (an achievement unlocks; an N64 game renders; two machines play netplay) |

## Ordering and risks

1. **M1 first**: without installers nothing else reaches users.
2. **M2 and M3 in parallel**: they touch different crates; M3 is cheap and visible.
3. **M4 after M3**: metadata is worth more once there are more systems.
4. **M5 only on demand**, hardware rendering above all.

Risks:

| Risk | Impact | Mitigation |
|---|---|---|
| Core licenses (FBNeo, snes9x, Genesis Plus GX are non-commercial) | App must stay free; cannot bundle in a paid build | Download cores at install time; keep a license table per core in the registry data |
| Code signing certificates cost money and need setup time | M1 installers blocked | Start with unsigned builds and document the Gatekeeper/SmartScreen steps; add signing when certificates exist |
| BIOS files cannot be shipped | Users hit "BIOS missing" | Keep the BIOS check and import flow; give exact file names and MD5 in the UI |
| Cover-art services have terms and API keys | M4 metadata blocked | Support user-supplied art first; one `MetadataSource` adapter per service |
| Hardware rendering shares a GL/Vulkan context with the renderer | M5 rework | Design the `wgpu` renderer in M2 with a hardware-frame path in mind |
| Real-game behaviour differs from synthetic ROMs | Late bugs | Do the 3-OS real-game check at the start of M1 and keep a short manual checklist in [DEVELOPMENT.md](DEVELOPMENT.md) |

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
