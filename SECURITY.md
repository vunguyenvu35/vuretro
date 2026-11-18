# Security policy

## Reporting a vulnerability

Please **do not open a public issue** for a security problem.
Report it privately through GitHub: open the repository's **Security** tab and choose
**Report a vulnerability** (private vulnerability reporting).
Include what you found, how to reproduce it, and which version or commit you tested.

You can expect an answer within about a week. Fixes go to `main`; vuretro is pre-1.0, so
only the latest `main` is supported.

## What is in scope

- The desktop app (`src-tauri/`, `ui/`) and the runner (`crates/runner/`).
- Handling of files the user gives us: importing ROMs and BIOS files, `.cue`/`.m3u`
  companion files, drag and drop, and removing games from the library (deletion must
  never reach outside the library folder).
- Reading and writing the app's data folder (settings, controls, macros, saves, covers).
- The Tauri command surface and its capabilities (`src-tauri/capabilities/`).

## Things to know

- **Cores are native code.** The runner loads libretro cores (`.dylib` / `.dll` / `.so`)
  and they run with the user's full permissions. Only install cores from sources you
  trust (`make cores` downloads them from the official libretro buildbot). A malicious
  core or a bug in a core is outside vuretro's control, but the runner is a separate
  process so a crashing core cannot take the app down.
- **ROMs and BIOS files are not bundled** and vuretro does not download them.
- The environment variables `VURETRO_RUNNER` and `VURETRO_CORES_DIR` choose which
  executable and cores the app starts. Treat them like `PATH`.
- vuretro makes no network requests of its own; the app opens links only when you pick
  Help items from the menu.

## Out of scope

Vulnerabilities in third-party cores, in the operating system, or in dependencies that
are already fixed upstream (please update the dependency instead).
