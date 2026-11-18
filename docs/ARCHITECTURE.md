# vuretro architecture

## Overview

![vuretro architecture](images/architecture.svg)

The runner is a **separate process**: cores are third-party C code, so if one crashes you only lose the running game and the UI stays alive. The UI and runner talk through command-line arguments (later also JSON on stdout to report play time and save slots).

Try the runner now: `make dev` (see [DEVELOPMENT.md](DEVELOPMENT.md)).

## Layers

Dependencies point inward only; `crates/core/tests/architecture.rs` fails the build if that breaks.

```
src-tauri (Tauri shell: commands, menu, gamepad)      vuretro-runner (window, audio, core)
        │                                                   │
        ▼                                                   ▼
vuretro-library (adapters: disk, processes)   ──►  vuretro-core (domain, use cases, ports)
                                                   vuretro-controls / vuretro-settings (shared config files)
```

- **`vuretro-core`**: systems registry, ROM detection by path, `Settings`/`GameMeta`, the "why is this game blocked" rule, and `service::App` with every use case (overview, import, remove, favorites, launch, covers). It reaches the outside only through the traits in `ports.rs` (`Store`, `Catalog`, `Locator`, `Launcher`), so `crates/core/tests/service.rs` tests the use cases with in-memory fakes.
- **`vuretro-library::adapters`**: the real implementations of those ports (`JsonStore`, `FsCatalog`, `FsLocator`, `ProcessLauncher`).
- **`src-tauri`**: `main.rs` only wires adapters into `App`. `commands/` are thin Tauri commands, plus `menu.rs`, `gamepad.rs`, `platform.rs`.
- **`vuretro-runner`**: `main.rs` loads the core. `session.rs` runs the game loop (`Session`) against the traits in `ports.rs` (`Screen`, `Emulator`, `Controller`, `FrameSource`), so its tests use fakes. `screen.rs` (minifb window) and `real.rs` (core, gamepad, frames, `open()`) are the real implementations. `window.rs` holds the pure rules (actions, title, pacing, pause picture), `headless.rs` the windowless mode.
- **UI (`ui/src`)**: `lib/api.js` is the only door to the backend (`lib/mock.js` answers in a browser). `lib/library.js` holds the pure rules of the library screen (filter, sort, summaries; tested with `npm test`). `lib/store.svelte.js` is app state and actions, `lib/ui.svelte.js` the dialog, menu and toasts. Components only render and call these.

## The app

`src-tauri/` (crate `vuretro-app`, binary `vuretro`) exposes commands to the Svelte UI. A watcher thread (gilrs) keeps the connected-pad list and emits `pads-changed` and `pad-button` events for the Controls screen. Games start as `vuretro-runner` child processes: the app waits 1.5 s to catch immediate failures (missing BIOS, bad file) and shows the tail of `logs/runner.log`, then tracks play time until the process exits. It lives in its own Cargo workspace so the core workspace builds without system webview libraries.

## Crates

| Crate | Role |
|---|---|
| `libretro-sys` | FFI types from `libretro.h`, only what is used. Add constants as cores need them. |
| `vuretro-core` | Domain, use cases and ports. No I/O. |
| `vuretro-library` | Adapters for the ports, plus `scan` (walk a folder), `bios` (check and import), `roms` (import and remove). The `vuretro-scan` binary prints JSON. |
| `vuretro-controls` | Controls and macros config shared by runner and app, and the logic that assigns pads to players. |
| `vuretro-settings` | Game settings file shared by runner and app. |
| `vuretro-runner` | Binary that runs one game: `core.rs` loads symbols, `frontend.rs` holds the C callbacks, plus `audio.rs`, `input.rs`, `options.rs`, `session.rs`, `window.rs`. |
| `vuretro-app` (`src-tauri/`) | Tauri shell. Separate workspace. |
| `vuretro-test-core` | Fake core (cdylib) for tests and CI, with fake save RAM and one core option. `make dev` and `make smoke` use it. |

## How a game runs

![Game flow](images/game-flow.svg)

1. `Core::load`: `dlopen` the core, check `retro_api_version`, register 6 callbacks, call `retro_init`.
2. `load_game`: if the core has `need_fullpath`, pass only the path; otherwise read the file into RAM. A missing BIOS is the usual failure here.
3. `retro_get_system_av_info` → frame size, fps, sample rate.
4. Each iteration: read input → `retro_run` (fast-forward runs it 4 times) → the core calls `video_cb` (pixels converted to `0RGB` u32) and `audio_batch_cb` (pushed into the ring buffer) → draw the window.
5. The `cpal` audio thread pulls samples from the ring buffer and linearly resamples from the core rate to the device rate. **Dynamic rate control** nudges the resampling speed by at most ±0.5 % so the buffer level converges to ~60 ms: video is paced by a timer, audio follows it without crackles or growing latency. The buffer is capped at ~250 ms; on overflow the oldest samples are dropped.
6. Save RAM: after `load_game` the runner copies `<rom>.srm` into `retro_get_memory_data(SAVE_RAM)`; it writes it back every 30 s and on exit.

## Core options

The core announces options with `SET_VARIABLES` (`"Description; a|b|c"`, first value = default). The runner answers `GET_VARIABLE` from `<data dir>/config/<core>.cfg` (`key = value` lines). `--list-options` prints every option as a ready-to-edit template. Newer cores that only use `SET_CORE_OPTIONS_V2` still work through the `SET_VARIABLES` fallback (verified with snes9x and mGBA).

## Callbacks and global state

libretro callbacks carry no user-data pointer, so frontend state lives in `OnceLock<Mutex<Frontend>>`. Rule: **never hold the lock while calling into the core**, because the core calls back and would lock the mutex again.

## Supported environment calls

`GET_CAN_DUPE`, `SET_PIXEL_FORMAT`, `GET_SYSTEM_DIRECTORY`, `GET_CORE_ASSETS_DIRECTORY`, `GET_SAVE_DIRECTORY`, `GET_VARIABLE` (values from the options file, otherwise the core default), `GET_VARIABLE_UPDATE`, `SET_VARIABLES` (recorded for `--list-options`), `SET_INPUT_DESCRIPTORS`, `SET_PERFORMANCE_LEVEL`. Other commands return `false` and are logged at `trace` level (`RUST_LOG=trace make dev`).

## System detection during ROM scan

![System detection](images/rom-detection.svg)

1. A file inside a system-named folder (`roms/neogeo/…`, `roms/psx/…`) → that system.
2. No folder: use the file extension if it belongs to only one system (`.chd` → PS1).
3. Archives (`.zip`, `.7z`) must be in a system folder, since they can hold games for any system.
4. BIOS files (`neogeo.zip`, `scph*.bin`) are excluded from the game list.

Try it: `make scan ROMS=~/Games/roms`.

## Data directory

Resolved with the `directories` crate (`ProjectDirs::from("dev", "vuvn", "vuretro")`):

| OS | Data dir |
|---|---|
| macOS | `~/Library/Application Support/dev.vuvn.vuretro` |
| Windows | `%APPDATA%\vuvn\vuretro\data` |
| Linux | `~/.local/share/vuretro` |

```
<data dir>/
├─ system/   BIOS (neogeo.zip, scph5501.bin, …)
├─ config/   per-core option files (<core>.cfg)
└─ saves/    <rom>.srm, <rom>.state1 … .state4
```

## License

- vuretro source: MIT. Cores are separate libraries loaded at run time, each under its own license; if you ship a core together with vuretro, that core's license applies to it.
- **FBNeo**: non-commercial license → vuretro must be free if it bundles FBNeo.
- **PCSX ReARMed**: GPLv2. **snes9x**: non-commercial. **mGBA**: MPL-2.0.
- DuckStation is not used: its current license does not allow free redistribution.
- ROMs and BIOS: provided by the user; the app neither bundles them nor explains where to download them.
