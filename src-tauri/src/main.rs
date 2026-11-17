//! vuretro desktop app. The UI (Svelte, `ui/`) talks to these commands; games run in
//! a separate `vuretro-runner` process so a crashing core never takes the UI down.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Manager, State};
use vuretro_controls::{ControlsConfig, PadInfo};
use vuretro_library::bios::{self, BiosStatus};
use vuretro_library::registry::{self, SYSTEMS};
use vuretro_library::roms;
use vuretro_library::scan;

#[derive(Default, Serialize, Deserialize, Clone)]
struct Settings {
    rom_dir: Option<String>,
    /// System ids the user switched off.
    #[serde(default)]
    disabled: Vec<String>,
}

#[derive(Default, Serialize, Deserialize, Clone)]
struct GameMeta {
    last_played: u64,
    play_secs: u64,
    favorite: bool,
}

struct AppState {
    data_dir: PathBuf,
    pads: Arc<Mutex<Vec<PadInfo>>>,
    settings: Mutex<Settings>,
    meta: Mutex<HashMap<String, GameMeta>>,
}

impl AppState {
    fn system_dir(&self) -> PathBuf {
        self.data_dir.join("system")
    }
    /// ROMs the user imported into the app live here.
    fn roms_dir(&self) -> PathBuf {
        self.data_dir.join("roms")
    }
    fn save_settings(&self) {
        write_json(
            &self.data_dir.join("settings.json"),
            &*self.settings.lock().unwrap(),
        );
    }
    fn save_meta(&self) {
        write_json(
            &self.data_dir.join("library.json"),
            &*self.meta.lock().unwrap(),
        );
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string_pretty(value) {
        let _ = std::fs::write(path, text);
    }
}

fn read_json<T: for<'a> Deserialize<'a> + Default>(path: &Path) -> T {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------- locating the runner and cores ----------

/// Directories to search for sibling files: the executable's folder and, when
/// running from a source checkout, every parent of it and of the working dir.
fn search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let starts = [
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf)),
        std::env::current_dir().ok(),
    ];
    for start in starts.into_iter().flatten() {
        for dir in start.ancestors() {
            if !roots.contains(&dir.to_path_buf()) {
                roots.push(dir.to_path_buf());
            }
        }
    }
    roots
}

fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn newest(paths: impl Iterator<Item = PathBuf>) -> Option<PathBuf> {
    paths
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .max_by_key(|(t, _)| *t)
        .map(|(_, p)| p)
}

fn find_runner() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("VURETRO_RUNNER") {
        return Some(PathBuf::from(p));
    }
    let name = exe_name("vuretro-runner");
    let candidates = search_roots().into_iter().flat_map(|r| {
        [
            r.join(&name),
            r.join("target/release").join(&name),
            r.join("target/debug").join(&name),
        ]
    });
    newest(candidates.filter(|p| p.is_file()))
}

fn find_core(state: &AppState, stem: &str) -> Option<PathBuf> {
    let file = registry::core_filename(stem);
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(d) = std::env::var("VURETRO_CORES_DIR") {
        dirs.push(d.into());
    }
    dirs.push(state.data_dir.join("cores"));
    dirs.extend(
        search_roots()
            .into_iter()
            .flat_map(|r| [r.join("cores"), r.join("cores/bin")]),
    );
    dirs.into_iter()
        .map(|d| d.join(&file))
        .find(|p| p.is_file())
}

fn find_test_core() -> Option<PathBuf> {
    let name = registry::core_filename("vuretro_test_core");
    let name = if cfg!(windows) {
        name
    } else {
        format!("lib{name}")
    };
    newest(
        search_roots()
            .into_iter()
            .flat_map(|r| {
                [
                    r.join("target/debug").join(&name),
                    r.join("target/release").join(&name),
                ]
            })
            .filter(|p| p.is_file()),
    )
}

// ---------- overview ----------

#[derive(Serialize)]
struct SystemView {
    id: &'static str,
    name: &'static str,
    core: &'static str,
    extensions: &'static [&'static str],
    /// Switched on by the user (Cores & BIOS screen).
    enabled: bool,
    core_installed: bool,
    bios: Vec<BiosStatus>,
    bios_ready: bool,
    game_count: usize,
}

#[derive(Serialize)]
struct GameView {
    system: String,
    title: String,
    path: String,
    size: u64,
    last_played: u64,
    play_secs: u64,
    favorite: bool,
    /// Reason the Play button is locked, if any.
    blocked: Option<String>,
    /// Imported into the app's library (can be removed from it), not just linked.
    imported: bool,
}

#[derive(Serialize)]
struct Overview {
    data_dir: String,
    system_dir: String,
    rom_dir: Option<String>,
    systems: Vec<SystemView>,
    games: Vec<GameView>,
    runner_found: bool,
    demo_available: bool,
}

#[tauri::command]
async fn get_overview(state: State<'_, AppState>) -> Result<Overview, String> {
    let settings = state.settings.lock().unwrap().clone();
    let meta = state.meta.lock().unwrap().clone();
    let system_dir = state.system_dir();

    // The app's own library plus, if the user still links one, that folder read in place.
    let roms_dir = state.roms_dir();
    let linked = settings.rom_dir.clone().map(PathBuf::from);
    let scan_dir = roms_dir.clone();
    let scanned = tauri::async_runtime::spawn_blocking(move || {
        let mut games = scan::scan(&scan_dir);
        if let Some(dir) = linked {
            for g in scan::scan(&dir) {
                if !games.iter().any(|x| x.path == g.path) {
                    games.push(g);
                }
            }
        }
        games
    })
    .await
    .map_err(|e| e.to_string())?;

    let mut systems = Vec::new();
    for s in SYSTEMS.iter().filter(|s| s.enabled) {
        let bios = bios::check(s, &system_dir);
        systems.push(SystemView {
            id: s.id,
            name: s.name,
            core: s.core,
            extensions: s.extensions,
            enabled: !settings.disabled.iter().any(|d| d == s.id),
            core_installed: find_core(&state, s.core).is_some(),
            bios_ready: bios::ready(s, &system_dir),
            bios,
            game_count: scanned.iter().filter(|g| g.system == s.id).count(),
        });
    }

    let games = scanned
        .into_iter()
        .filter_map(|g| {
            let sys = systems.iter().find(|s| s.id == g.system)?;
            let blocked = if !sys.enabled {
                Some(format!("{} is switched off", sys.name))
            } else if !sys.core_installed {
                Some(format!("core {} not installed (run: make cores)", sys.core))
            } else if !sys.bios_ready {
                Some("BIOS missing".to_string())
            } else {
                None
            };
            let imported = g.path.starts_with(&roms_dir);
            let key = g.path.to_string_lossy().into_owned();
            let m = meta.get(&key).cloned().unwrap_or_default();
            Some(GameView {
                system: g.system,
                title: g.title,
                path: key,
                size: g.size,
                last_played: m.last_played,
                play_secs: m.play_secs,
                favorite: m.favorite,
                blocked,
                imported,
            })
        })
        .collect();

    Ok(Overview {
        data_dir: state.data_dir.to_string_lossy().into_owned(),
        system_dir: system_dir.to_string_lossy().into_owned(),
        rom_dir: settings.rom_dir,
        systems,
        games,
        runner_found: find_runner().is_some(),
        demo_available: find_test_core().is_some(),
    })
}

// ---------- importing ROMs ----------

/// Copy ROM files, or every ROM found inside the folders given, into the app's library.
/// `system` is the system chosen in the sidebar; it settles archives (.zip) whose system
/// cannot be told from the file.
#[tauri::command]
async fn import_roms(
    state: State<'_, AppState>,
    paths: Vec<String>,
    system: Option<String>,
) -> Result<Vec<ImportOutcome>, String> {
    let library = state.roms_dir();
    tauri::async_runtime::spawn_blocking(move || {
        let mut out = Vec::new();
        let describe = |r: Result<roms::Imported, String>, source: String| match r {
            Ok(i) if i.already_there => ImportOutcome {
                source,
                ok: true,
                message: "already in the library".into(),
            },
            Ok(i) => ImportOutcome {
                source,
                ok: true,
                message: format!("added to {}", i.system),
            },
            Err(message) => ImportOutcome {
                source,
                ok: false,
                message,
            },
        };
        for p in paths {
            let path = PathBuf::from(&p);
            if path.is_dir() {
                let results = roms::import_folder(&path, &library);
                if results.is_empty() {
                    out.push(ImportOutcome {
                        source: p,
                        ok: false,
                        message: "no games it recognizes in that folder".into(),
                    });
                }
                for (src, r) in results {
                    out.push(describe(r, src.to_string_lossy().into_owned()));
                }
            } else {
                out.push(describe(
                    roms::import_file(&path, &library, system.as_deref()),
                    p,
                ));
            }
        }
        out
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_rom(state: State<'_, AppState>, path: String) -> Result<(), String> {
    roms::remove(&state.roms_dir(), Path::new(&path))?;
    state.meta.lock().unwrap().remove(&path);
    state.save_meta();
    Ok(())
}

// ---------- settings ----------

#[tauri::command]
fn set_rom_dir(state: State<'_, AppState>, path: Option<String>) {
    state.settings.lock().unwrap().rom_dir = path;
    state.save_settings();
}

#[tauri::command]
fn set_system_enabled(state: State<'_, AppState>, id: String, enabled: bool) {
    {
        let mut s = state.settings.lock().unwrap();
        s.disabled.retain(|d| *d != id);
        if !enabled {
            s.disabled.push(id);
        }
    }
    state.save_settings();
}

#[tauri::command]
fn toggle_favorite(state: State<'_, AppState>, path: String) -> bool {
    let fav = {
        let mut meta = state.meta.lock().unwrap();
        let m = meta.entry(path).or_default();
        m.favorite = !m.favorite;
        m.favorite
    };
    state.save_meta();
    fav
}

#[derive(Serialize)]
struct ImportOutcome {
    source: String,
    ok: bool,
    message: String,
}

#[tauri::command]
fn import_bios(state: State<'_, AppState>, paths: Vec<String>) -> Vec<ImportOutcome> {
    let dir = state.system_dir();
    paths
        .into_iter()
        .map(|p| match bios::import(Path::new(&p), &dir) {
            Ok(r) => ImportOutcome {
                source: p,
                ok: true,
                message: format!("Installed {}", r.file),
            },
            Err(message) => ImportOutcome {
                source: p,
                ok: false,
                message,
            },
        })
        .collect()
}

// ---------- launching ----------

fn spawn_runner(
    app: &tauri::AppHandle,
    state: &AppState,
    core: &Path,
    rom: &Path,
    track: Option<String>,
) -> Result<(), String> {
    let runner = find_runner().ok_or("vuretro-runner not found. Run `make build` first.")?;
    let log_path = state.data_dir.join("logs/runner.log");
    std::fs::create_dir_all(log_path.parent().unwrap()).map_err(|e| e.to_string())?;
    let log = std::fs::File::create(&log_path).map_err(|e| e.to_string())?;

    let mut child = Command::new(runner)
        .arg("--core")
        .arg(core)
        .arg("--rom")
        .arg(rom)
        .arg("--system-dir")
        .arg(state.system_dir())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(log)
        .spawn()
        .map_err(|e| format!("cannot start runner: {e}"))?;

    // Failing fast (missing BIOS, bad file) should be reported, not silently ignored.
    let started = Instant::now();
    while started.elapsed() < Duration::from_millis(1500) {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            if status.success() {
                return Ok(());
            }
            let log = std::fs::read_to_string(&log_path).unwrap_or_default();
            let tail: Vec<&str> = log.lines().rev().take(4).collect();
            let msg = tail.into_iter().rev().collect::<Vec<_>>().join("\n");
            return Err(if msg.is_empty() {
                format!("runner exited with {status}")
            } else {
                msg
            });
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    if let Some(key) = &track {
        let mut meta = state.meta.lock().unwrap();
        meta.entry(key.clone()).or_default().last_played = now();
    }
    state.save_meta();

    let app = app.clone();
    std::thread::spawn(move || {
        let _ = child.wait();
        if let Some(key) = track {
            let state = app.state::<AppState>();
            state.meta.lock().unwrap().entry(key).or_default().play_secs +=
                started.elapsed().as_secs();
            state.save_meta();
        }
        let _ = app.emit("game-exited", ());
    });
    Ok(())
}

#[tauri::command]
async fn launch_game(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    let rom_dir = state
        .settings
        .lock()
        .unwrap()
        .rom_dir
        .clone()
        .unwrap_or_default();
    let rom = PathBuf::from(&path);
    let sys =
        scan::detect_system(Path::new(&rom_dir), &rom).ok_or("unknown system for this file")?;
    if !bios::ready(sys, &state.system_dir()) {
        return Err("BIOS missing: add it in Cores & BIOS".into());
    }
    let core = find_core(&state, sys.core)
        .ok_or_else(|| format!("core {} not installed (run: make cores)", sys.core))?;
    spawn_runner(&app, &state, &core, &rom, Some(path))
}

/// Run the fake test core so the app can be tried without any ROM.
#[tauri::command]
async fn launch_demo(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let core = find_test_core().ok_or("test core not built. Run `make build` first.")?;
    let rom = state.data_dir.join("demo.test");
    std::fs::create_dir_all(&state.data_dir).map_err(|e| e.to_string())?;
    std::fs::write(&rom, b"test").map_err(|e| e.to_string())?;
    spawn_runner(&app, &state, &core, &rom, None)
}

// ---------- controls ----------

impl AppState {
    fn controls_path(&self) -> PathBuf {
        self.data_dir.join("config").join("controls.json")
    }
}

// ---------- game settings ----------

impl AppState {
    fn game_settings_path(&self) -> PathBuf {
        self.data_dir.join("config").join("game.json")
    }
}

#[tauri::command]
fn get_game_settings(state: State<'_, AppState>) -> vuretro_settings::GameSettings {
    vuretro_settings::load(&state.game_settings_path())
}

#[tauri::command]
fn set_game_settings(
    state: State<'_, AppState>,
    settings: vuretro_settings::GameSettings,
) -> Result<vuretro_settings::GameSettings, String> {
    let settings = settings.sanitized();
    vuretro_settings::save(&state.game_settings_path(), &settings)?;
    Ok(settings)
}

#[derive(Serialize)]
struct ControlsView {
    config: ControlsConfig,
    pads: Vec<PadInfo>,
}

#[tauri::command]
fn get_controls(state: State<'_, AppState>) -> ControlsView {
    ControlsView {
        config: vuretro_controls::load(&state.controls_path()),
        pads: state.pads.lock().unwrap().clone(),
    }
}

#[tauri::command]
fn set_controls(
    state: State<'_, AppState>,
    config: ControlsConfig,
) -> Result<ControlsConfig, String> {
    let config = config.sanitized();
    vuretro_controls::save(&state.controls_path(), &config)?;
    Ok(config)
}

#[tauri::command]
fn default_controls() -> ControlsConfig {
    ControlsConfig::default()
}

#[derive(Serialize, Clone)]
struct PadButton {
    id: String,
    name: String,
    button: String,
}

fn uuid_hex(u: [u8; 16]) -> String {
    u.iter().map(|b| format!("{b:02x}")).collect()
}

/// Watches gamepads on its own thread (gilrs must live where it was created), keeps the
/// shared pad list current and tells the UI about connects and button presses so it can
/// identify pads and capture bindings.
fn spawn_gamepad_watcher(app: tauri::AppHandle, pads: Arc<Mutex<Vec<PadInfo>>>) {
    std::thread::spawn(move || {
        let Ok(mut g) = gilrs::Gilrs::new() else {
            return;
        };
        let snapshot = |g: &gilrs::Gilrs| -> Vec<PadInfo> {
            g.gamepads()
                .filter(|(_, p)| p.is_connected())
                .map(|(_, p)| PadInfo {
                    id: uuid_hex(p.uuid()),
                    name: p.name().to_string(),
                })
                .collect()
        };
        *pads.lock().unwrap() = snapshot(&g);
        loop {
            while let Some(ev) = g.next_event() {
                match ev.event {
                    gilrs::EventType::Connected | gilrs::EventType::Disconnected => {
                        let list = snapshot(&g);
                        *pads.lock().unwrap() = list.clone();
                        let _ = app.emit("pads-changed", list);
                    }
                    gilrs::EventType::ButtonPressed(btn, _) => {
                        let pad = g.gamepad(ev.id);
                        let _ = app.emit(
                            "pad-button",
                            PadButton {
                                id: uuid_hex(pad.uuid()),
                                name: pad.name().to_string(),
                                button: format!("{btn:?}"),
                            },
                        );
                    }
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(8));
        }
    });
}

fn main() {
    let data_dir = ProjectDirs::from("dev", "vuvn", "vuretro")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("vuretro-data"));
    let state = AppState {
        settings: Mutex::new(read_json(&data_dir.join("settings.json"))),
        meta: Mutex::new(read_json(&data_dir.join("library.json"))),
        data_dir,
        pads: Arc::new(Mutex::new(Vec::new())),
    };
    let pads = state.pads.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .setup(move |app| {
            spawn_gamepad_watcher(app.handle().clone(), pads.clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_overview,
            set_rom_dir,
            set_system_enabled,
            toggle_favorite,
            import_bios,
            import_roms,
            remove_rom,
            launch_game,
            launch_demo,
            get_controls,
            set_controls,
            default_controls,
            get_game_settings,
            set_game_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vuretro");
}
