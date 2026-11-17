# vuretro

A cross-platform retro game frontend (macOS, Windows, Linux) that runs games through **libretro cores**.
It starts with Neo Geo (FBNeo) and PlayStation (PCSX ReARMed) and will grow to SNES, GBA and more.

> vuretro does **not** ship ROMs or BIOS files. You provide them from copies you own.

## Status

Phase 1 (runner) is in progress. See [docs/PLAN.md](docs/PLAN.md).

| Component | Status |
|---|---|
| `vuretro-runner`: core loading, video, audio, keyboard, gamepad, save states | ✅ working |
| `vuretro-library`: ROM scan, system detection, BIOS check (MD5) | ✅ tested |
| `vuretro-test-core`: fake core for testing without real ROMs | ✅ |
| Tauri UI (Library, Cores & BIOS, Controls) | ⏳ Phase 2 |

## Quick start

Requires the latest stable Rust (`rustup update`). Linux also needs:
`sudo apt install libasound2-dev libudev-dev libx11-dev libxkbcommon-dev`.

```bash
make dev                                         # fake core, no ROM needed
make cores                                       # download fbneo + pcsx_rearmed into cores/bin/
cp /path/to/neogeo.zip "<data dir>/system/"      # BIOS; the path is printed in the log on start
make play ROM=~/Games/roms/neogeo/mslug.zip      # Neo Geo
make play CORE=pcsx ROM=~/Games/roms/psx/crash.chd SCALE=4
make scan ROMS=~/Games/roms                      # scan ROMs + check BIOS
make help                                        # list all commands
```

`make dev` opens a window with color bars, a square you can move with the arrow keys, and a 440 Hz tone.
More on commands, debugging and CI: [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md).

Default `<data dir>`:
macOS `~/Library/Application Support/dev.vuvn.vuretro`,
Windows `%APPDATA%\vuvn\vuretro\data`,
Linux `~/.local/share/vuretro`.

## Default controls

![Default keyboard layout](docs/images/controls.svg)

| Key | RetroPad | Neo Geo |
|---|---|---|
| Arrow keys | D-pad | Joystick |
| `Z` / `X` / `A` / `S` | B / A / Y / X | A / B / C / D |
| `Enter` or `1` | Start | Start |
| `Right Shift` or `5` | Select | Coin |
| `Q` / `E` / `W` / `R` | L / R / L2 / R2 | |

Hotkeys: `Esc` quit · `F5` save state · `F7` load state · `F6` change slot · hold `Tab` to fast-forward · `F2` reset.
Gamepads (Xbox, DualSense, Switch Pro…) are detected automatically; the second pad is player 2.

## Layout

![vuretro architecture](docs/images/architecture.svg)

```
crates/
  libretro-sys/   FFI types from libretro.h (only what is used)
  runner/         vuretro-runner: window, game loop, audio, input
  library/        system registry, ROM scan, BIOS check
  test-core/      fake core: color bars + 440 Hz tone + controllable square
cores/            script that downloads cores from the libretro buildbot
docs/             plan, architecture, UI design, dev guide (+ images/)
Makefile          make dev, play, cores, scan, check, ci …
```

Docs: [DEVELOPMENT](docs/DEVELOPMENT.md) · [ARCHITECTURE](docs/ARCHITECTURE.md) · [UI](docs/UI.md) · [PLAN](docs/PLAN.md)

## License

vuretro source: GPL-3.0-or-later.
Cores have their own licenses (FBNeo: non-commercial, PCSX ReARMed: GPLv2), see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md#license).
