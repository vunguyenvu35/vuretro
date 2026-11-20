# vuretro architecture

## Overview

![vuretro architecture](images/architecture.svg)

The runner is a **separate process**: cores are third-party C code, so if one crashes you only lose the running game and the UI stays alive. The app starts it with command-line arguments, and it answers with JSON lines on its standard output (see [Runner reports](#runner-reports)).

Try the runner now: `make dev` (see [DEVELOPMENT.md](DEVELOPMENT.md)).

## Runner reports

With `--events` the runner writes one JSON object per line to its standard output, which the app reads on a thread of its own so the runner never waits for it. The format is defined once, in `vuretro_core::protocol`, and used by both sides.

```text
{"vuretro":1,"event":"ready","core":"snes9x","core_version":"1.62","width":256,"height":224,"fps":60.09,"window":"opengl","hardware_render":false}
{"vuretro":1,"event":"state","action":"saved","slot":1}
{"vuretro":1,"event":"screenshot","path":"/…/game-1732100000.png"}
{"vuretro":1,"event":"progress","played_secs":30}
{"vuretro":1,"event":"error","message":"cannot read /…/game.rom: No such file or directory"}
{"vuretro":1,"event":"exit","played_secs":1832}
```

- `ready`: the game is loaded and its window is open. `progress` comes every 30 s, so a crash loses little. `exit` ends a game that was closed normally. `error` says why the runner could not go on.
- `played_secs` counts the time the game was running, not paused.
- Cores are C code and may print to the same output, so a reader skips every line that is not one of these, and a writer starts its line on a fresh one. An event or a version a reader does not know is skipped, so a newer runner does not break an older app.
- Without `--events` nothing is written (a person running the runner from a terminal does not want JSON between the log lines). The session reports through the `Events` port, so it stays free of any output code.

## Layers

Dependencies point inward only; `crates/core/tests/architecture.rs` fails the build if that breaks.

```
src-tauri (Tauri shell: commands, menu, gamepad)      vuretro-runner (window, audio, pads, core loading)
        │                                                   │
        ▼                                                   ▼
vuretro-library (adapters: disk, processes)           vuretro-play (game session, pure rules, ports)
        │                                                   │
        └──────────────►  vuretro-core (domain, use cases, ports)  ◄──┘
                          vuretro-controls / vuretro-settings (shared config files)
```

- **`vuretro-core`**: systems registry, ROM detection by path, `Settings`/`GameMeta`, the "why is this game blocked" rule, and `service::App` with every use case (overview, import, remove, favorites, launch, covers). It reaches the outside only through the traits in `ports.rs` (`Store`, `Catalog`, `Locator`, `Launcher`), so `crates/core/tests/service.rs` tests the use cases with in-memory fakes.
- **`vuretro-library::adapters`**: the real implementations of those ports (`JsonStore`, `FsCatalog`, `FsLocator`, `ProcessLauncher`).
- **`src-tauri`**: `main.rs` only wires adapters into `App`. `commands/` are thin Tauri commands, plus `menu.rs`, `gamepad.rs`, `platform.rs`.
- **`vuretro-play`**: `session.rs` runs the game loop (`Session`) against the traits in `ports.rs` (`Screen`, `Emulator`, `Controller`, `FrameSource`, `Files`), so its tests use fakes. `window.rs` holds the pure rules (actions, title, pacing, pause picture), next to `rewind`, `speed`, `perf`, `rumble`, the picture filters (`crt`, `lcd`, `scale`, `scale2x`), `cover` (which frame makes a good preview), `badge` (the system badge drawn on the Dock icon) and its own `Key` type. It has no window, audio, gamepad, FFI or image-encoding dependency (`play_is_pure` in `architecture.rs`).
- **`vuretro-runner`**: the adapters of `vuretro-play`. `main.rs` loads the core; `screen.rs` (minifb window), `real.rs` (core, gamepad, frames, `open()`), `audio.rs` (cpal), `input.rs` and `padkeys.rs` (keyboard and gilrs), `files.rs` (disk and PNG encoding), `appicon.rs` (Dock icon with the system badge) are the real implementations. `headless.rs` is the windowless mode.
- **UI (`ui/src`)**: `lib/api.js` is the only door to the backend (`lib/mock.js` answers in a browser). `lib/library.js` holds the pure rules of the library screen (filter, sort, summaries; tested with `npm test`). `lib/store.svelte.js` is app state and actions, `lib/ui.svelte.js` the dialog, menu and toasts. Components only render and call these.

## The app

`src-tauri/` (crate `vuretro-app`, binary `vuretro`) exposes commands to the Svelte UI. A watcher thread (gilrs) keeps the connected-pad list and emits `pads-changed` and `pad-button` events for the Controls screen. Games start as `vuretro-runner` child processes. The game counts as started when the runner reports `ready` (a runner that says nothing is given 1.5 s), a failure is explained by the runner's own `error` message (the tail of `logs/runner.log` is the fallback), and play time is what the runner counted, not how long the process lived. While the game plays, what it reports is passed to the page as `game-event` (the save slots refresh when one is saved). It lives in its own Cargo workspace so the core workspace builds without system webview libraries.

## Crates

| Crate | Role |
|---|---|
| `libretro-sys` | FFI types from `libretro.h`, only what is used. Add constants as cores need them. |
| `vuretro-core` | Domain, use cases and ports. No I/O. |
| `vuretro-library` | Adapters for the ports, plus `scan` (walk a folder), `bios` (check and import), `roms` (import and remove). The `vuretro-scan` binary prints JSON. |
| `vuretro-controls` | Controls and macros config shared by runner and app, and the logic that assigns pads to players. |
| `vuretro-settings` | Game settings file shared by runner and app. |
| `vuretro-play` | The game session and its pure rules, behind ports. No window, audio or gamepad code. |
| `vuretro-runner` | Binary that runs one game, the adapters of `vuretro-play`: `core.rs` loads symbols, `frontend.rs` holds the C callbacks, plus `screen.rs`, `audio.rs`, `input.rs`, `options.rs`, `files.rs`, `appicon.rs`. |
| `vuretro-app` (`src-tauri/`) | Tauri shell. Separate workspace. |
| `vuretro-test-core` | Fake core (cdylib) for tests and CI, with fake save RAM and one core option. `make dev` and `make smoke` use it. |

## How a game runs

![Game flow](images/game-flow.svg)

1. `Core::load`: `dlopen` the core, check `retro_api_version`, register 6 callbacks, call `retro_init`.
2. `load_game`: if the core has `need_fullpath`, pass only the path; otherwise read the file into RAM. A missing BIOS is the usual failure here.
3. `retro_get_system_av_info` → frame size, fps, sample rate.
4. Each iteration: read input → `retro_run` (fast-forward runs it 4 times) → the core calls `video_cb` (pixels converted to `0RGB` u32) and `audio_batch_cb` (pushed into the ring buffer) → draw the window.
5. The core's samples go into a lock-free queue (`rtrb`, one writer and one reader, so neither thread ever waits for the other). The `cpal` audio thread (`audio.rs`) reads them and resamples from the core rate to the device rate with a Kaiser-windowed sinc filter (`resample.rs`, about 76 to 90 dB below the tone, where the linear resampler it replaced was 14 to 23 dB below it at 10 to 15 kHz). **Dynamic rate control** nudges the resampling speed by at most ±0.5 % so the queue level converges to ~60 ms: video is paced by a timer, audio follows it without crackles or growing latency. Past ~250 ms the audio thread skips ahead to the target; a full queue drops the newest samples. A dry queue fades the last frame out instead of holding it, and the volume and mute follow a 5 ms ramp, so neither clicks.
6. Save RAM: after `load_game` the runner copies `<rom>.srm` into `retro_get_memory_data(SAVE_RAM)`; it writes it back every 30 s and on exit.

## Core options

The core announces options with `SET_VARIABLES` (`"Description; a|b|c"`, first value = default). The runner answers `GET_VARIABLE` from `<data dir>/config/<core>.cfg` (`key = value` lines). `--list-options` prints every option as a ready-to-edit template. Newer cores that only use `SET_CORE_OPTIONS_V2` still work through the `SET_VARIABLES` fallback (verified with snes9x and mGBA).

## Callbacks and global state

libretro callbacks carry no user-data pointer, so frontend state lives in `OnceLock<Mutex<Frontend>>`. Rule: **never hold the lock while calling into the core**, because the core calls back and would lock the mutex again.

## Supported environment calls

Answered with data: `GET_CAN_DUPE`, `GET_OVERSCAN`, `SET_PIXEL_FORMAT`, `GET_SYSTEM_DIRECTORY`, `GET_CORE_ASSETS_DIRECTORY`, `GET_SAVE_DIRECTORY`, `GET_VARIABLE` (values from the options file, otherwise the core default), `GET_VARIABLE_UPDATE`, `GET_LOG_INTERFACE`, `GET_RUMBLE_INTERFACE`, `GET_LANGUAGE` (English), `GET_USERNAME`, `GET_INPUT_DEVICE_CAPABILITIES` (joypad), `GET_INPUT_MAX_USERS`, `GET_AUDIO_VIDEO_ENABLE`, `GET_MESSAGE_INTERFACE_VERSION`.

Acted on: `SET_VARIABLES` (recorded for `--list-options`), `SET_VARIABLE` (a core changes one of its own options), `SET_ROTATION` (the picture is turned in `video_cb`, so vertical arcade games stand up), `SHUTDOWN` (the window closes), `SET_MESSAGE` and `SET_MESSAGE_EXT` (written to the log), `SET_DISK_CONTROL_INTERFACE`, `SET_INPUT_DESCRIPTORS`.

Acknowledged, nothing to do: `SET_GEOMETRY` (the size comes with every frame), `SET_PERFORMANCE_LEVEL`, `SET_SUPPORT_NO_GAME`, `SET_CONTROLLER_INFO`, `SET_SUBSYSTEM_INFO`, `SET_MEMORY_MAPS`, `SET_SERIALIZATION_QUIRKS`.

Hardware rendering (not with `VURETRO_WINDOW=minifb`): `SET_HW_RENDER` is accepted for `OPENGL_CORE` and `OPENGL` contexts when the run has the OpenGL window, and `GET_PREFERRED_HW_RENDER` answers `OPENGL_CORE`. See [Hardware rendering](#hardware-rendering). Refused with a warning in the log: other renderer types (Vulkan, GLES, Direct3D), `SET_HW_SHARED_CONTEXT`, and `SET_SYSTEM_AV_INFO` (the frame rate and sample rate are not changed while playing). Other commands return `false` and are logged at `trace` level (`RUST_LOG=trace make dev`).

The `EXPERIMENTAL` flag (0x10000) is part of the command number and is never stripped: the header has two commands numbered 44 (`SET_SERIALIZATION_QUIRKS`, and `SET_HW_SHARED_CONTEXT` with the flag).

## The game window

The game window is the OpenGL one (`gl_screen.rs`, winit + glutin) unless `VURETRO_WINDOW=minifb` asks for the basic minifb window, which cannot run 3D cores and draws the CRT and LCD looks on the CPU. If no OpenGL window can be made, a game that does not need one plays in the basic window with a warning in the log.

## Hardware rendering

3D cores (N64, PSP, GameCube, PS2, 3DS) draw with OpenGL instead of handing over pixels. `gl_screen.rs` opens a winit window with a glutin OpenGL context of the version the core asked for (`SET_HW_RENDER` comes during `retro_load_game`, before the window), then `hw.rs` makes a framebuffer the size of the core's largest picture and calls the core's `context_reset`. The core draws into it each `retro_run` and passes `RETRO_HW_FRAME_BUFFER_VALID` to the video callback; the picture stays on the graphics card. The video callback only records its size; the session gets a frame with no pixels (`Frame` with an empty buffer) and the window draws the core's framebuffer texture straight to the window (`Screen::present_gpu`), flipped in the shader if the core draws from the bottom, with the CRT or LCD look and whole-number scaling done there too. When the session needs the pixels (a screenshot, the pause picture, Scale2x, a window that cannot do this, a rotated picture) it asks `FrameSource::current`, which reads the picture back at that moment only. On close, and on a window change, `context_destroy` runs while the context is still current.

macOS gives only OpenGL 4.1 core (or a legacy 2.1), so a core that asks for a compatibility context gets a core one there.

## CRT and LCD looks on the GPU

A window can say it draws the looks itself (`Screen::gpu_look`). The OpenGL window does: the session then sends the picture as the game made it and tells the window which look to draw (`Screen::set_look`), and the fragment shader in `gl_screen.rs` darkens the last row (CRT, 150/256) or the last row and column (LCD, 190/256) of every cell of window pixels a picture pixel covers. With whole-number scaling the cell is the largest whole factor that fits the window; otherwise it is the factor the CPU looks used for a picture of that height. The minifb window, and Scale2x with a look, still use the CPU looks in `vuretro-play` (`crt.rs`, `lcd.rs`), which also keep the same levels.

Checks that need a window and a GPU: `vuretro-runner --window-selftest` (colours reach the screen, and the CRT and LCD looks darken the right rows and columns) and `vuretro-runner --hw-selftest <test core> <rom>` (a core draws with OpenGL, the picture reaches the window without being read back and is the right way up, and so is a later read-back).

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
