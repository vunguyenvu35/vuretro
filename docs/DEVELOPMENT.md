# Development guide

All common commands live in the [Makefile](../Makefile). Run `make` to list them.

## Setup

| OS | Requirements |
|---|---|
| macOS | Stable Rust (`rustup update`), Xcode Command Line Tools |
| Windows | Stable Rust (MSVC toolchain), Git Bash or MSYS2 to run `make` and `fetch-cores.sh` |
| Linux | Stable Rust + `sudo apt install libasound2-dev libudev-dev libx11-dev libxkbcommon-dev` |

## Quick start (no ROM needed)

```bash
make dev
```

This builds `vuretro-test-core` and opens the runner with the fake core. You get a 2× window with color bars, a square you can move with the arrow keys, and a 440 Hz tone. Quit with `Esc`.

Change the window size: `make dev SCALE=4`.

## Play a real game

```bash
make cores                                   # download fbneo + pcsx_rearmed into cores/bin/
cp neogeo.zip "<data dir>/system/"           # your own BIOS; the path is printed in the log on start
make play ROM=~/Games/roms/neogeo/mslug.zip  # Neo Geo (default CORE=fbneo)
make play CORE=pcsx ROM=~/Games/roms/psx/crash.chd SCALE=4
```

`<data dir>`: macOS `~/Library/Application Support/dev.vuvn.vuretro`, Windows `%APPDATA%\vuvn\vuretro\data`, Linux `~/.local/share/vuretro`.

Scan a ROM folder (prints JSON with games and BIOS status):

```bash
make scan ROMS=~/Games/roms
```

## Command reference

| Command | What it does |
|---|---|
| `make dev` | Build and run the runner with the fake core |
| `make play ROM=… [CORE=fbneo\|pcsx] [SCALE=2]` | Run a real game (release build) |
| `make cores` | Download cores from the libretro buildbot |
| `make scan ROMS=…` | Scan ROMs and check BIOS files |
| `make build` / `make release` | Build the whole workspace |
| `make test` | Unit tests |
| `make fmt` | Format code |
| `make lint` | `fmt --check` + `clippy -D warnings` |
| `make check` | `lint` + `test` |
| `make screenshot` | Run 120 headless frames, write `target/shot.ppm` |
| `make ci` | `check` + `screenshot`, same as GitHub Actions |
| `make clean` | Remove `target/` |

## Debugging

- Verbose logs: `RUST_LOG=debug make dev`. To see environment calls the core makes that are not supported yet: `RUST_LOG=trace`.
- Core fails to load: check that the file extension matches your OS (`.dylib` / `.dll` / `.so`) and the architecture (arm64 vs x86_64).
- `load_game` error: almost always a missing BIOS. Run `make scan` to see which file is missing.
- No ROM and no display (CI/SSH): use `make screenshot`.

## CI

[`.github/workflows/ci.yml`](../.github/workflows/ci.yml) runs on macOS, Windows and Ubuntu: `fmt`, `clippy`, `test`, then a runner smoke test with the fake core. `make ci` repeats these steps locally.
