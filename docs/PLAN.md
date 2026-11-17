# vuretro plan

Goal: a retro game frontend with a real UI, running on macOS, Windows and Linux.
Games run through libretro cores; the app handles the interface, library management and controls.

Try the current state: `make dev` (see [DEVELOPMENT.md](DEVELOPMENT.md)). Architecture: [ARCHITECTURE.md](ARCHITECTURE.md). Screen designs: [UI.md](UI.md).

## Roadmap

| Phase | Scope | Verified by | Status |
|---|---|---|---|
| 0 | Workspace, CI, fake core | `make ci` | ✅ |
| 1 | Runner plays games | `make dev`, `make play ROM=…` | in progress |
| 2 | Tauri UI (3 screens, see [UI.md](UI.md)) | `make dev` | first version done |
| 3 | Play experience (wgpu, CRT shader) | real play | not started |
| 4 | More systems | `make cores` + ROMs | SNES + GBA done, Genesis pending |
| 5 | Release | installers for 3 OSes | not started |

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
- [ ] Real-game check on all 3 OSes: `make play ROM=…/mslug.zip` (FBNeo), `make play CORE=pcsx ROM=…/crash.chd` (PCSX ReARMed)
- [x] Core options (`GET_VARIABLE`) read from `<data dir>/config/<core>.cfg`; `--list-options` prints a template
- [x] Audio dynamic rate control (resampler speed follows buffer level); video stays timer-paced
- [x] Save SRAM (`.srm`) on exit and every 30 s, load on start

## Phase 2: Tauri UI

- [x] Tauri v2 + Svelte 5 in `ui/` and `src-tauri/` (own Cargo workspace so core CI needs no webview libs)
- [x] **Library** screen: system sidebar, "Continue playing" hero, cover grid, details panel, search, sort, favorites, welcome screen with demo
- [x] **Cores & BIOS** screen: switch systems on/off, BIOS status, drag-and-drop or browse BIOS files (MD5 check, renamed dumps accepted)
- [x] **Controls** screen: keyboard layout, hotkeys, gamepad notes (read-only)
- [x] Controls: device per player + swap, live gamepad list, per-player key and button rebinding, dead zone, stick-as-D-pad (`config/controls.json`, shared crate `vuretro-controls`)
- [ ] Controls: per-system and per-game overrides, button diagram, analog trigger and right-stick options
- [x] Tauri commands calling `vuretro-library` (scan, BIOS check/import) and spawning `vuretro-runner`, reporting early failures from `runner.log`
- [x] Metadata (play time, last played, favorites) in `library.json`; SQLite deferred until the library is big enough to need it
- [ ] Save-slot thumbnails and slot picker in the details panel (needs runner stdout JSON)
- [ ] Verify the window on Windows and Linux; CI job `app` is added but not yet seen green

## Phase 3: Play experience

- [ ] `wgpu` renderer replacing `minifb`: keep aspect ratio, integer scale, fullscreen
- [ ] CRT shader (scanlines, curvature), toggleable
- [ ] In-game menu (Esc): save/load, change slot, back to Library
- [ ] Per-game and per-system overrides on top of the global `controls.json`
- [ ] Thumbnails for save states

## Phase 4: More systems

- [x] SNES (snes9x) and GBA (mGBA) enabled: registry, extensions (incl. zip), optional GBA BIOS, `make play CORE=snes|gba`
- [ ] Genesis (Genesis Plus GX)
- [ ] Multi-disc PS1 via `.m3u` + disc swap (`SET_DISK_CONTROL_INTERFACE`)
- [ ] Cover art: user-supplied, or from a metadata source (ScreenScraper…)

## Phase 5: Release

- [ ] macOS: signed + notarized `.dmg` (universal arm64/x86_64)
- [ ] Windows: signed MSI/NSIS
- [ ] Linux: AppImage, Flatpak
- [ ] Auto-update (Tauri updater) via GitHub Releases
- [ ] Review each core's license before bundling

## Decisions made

| Item | Choice | Notes |
|---|---|---|
| Language | Rust | No GC, clean C FFI, cross-platform |
| UI | Tauri v2 | Rebuild the design in HTML/CSS, small bundle |
| Game execution | Separate `vuretro-runner` process | A core crash does not take down the UI |
| Window/video (Phase 1) | `minifb` | Pure Rust, no SDL; replaced by `wgpu` in Phase 3 |
| Audio | `cpal` | CoreAudio / WASAPI / ALSA |
| Gamepads | `gilrs` | Uses the SDL GameController DB for mapping |
| Neo Geo core | FBNeo | Non-commercial license → app must be free |
| PS1 core | PCSX ReARMed | Light, runs well on arm64; avoids DuckStation (license) |
