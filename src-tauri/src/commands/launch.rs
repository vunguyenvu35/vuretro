use super::blocking;
use crate::state::Shell;
use tauri::{AppHandle, Emitter, State};
use vuretro_core::model::NativeGame;
use vuretro_core::service::Running;

/// Watch the game on its own thread, passing on what it reports, then tell the page it ended.
/// The keys that stop the open game from anywhere (a Windows game has no window to close, and
/// its window hides the one of the app). Taken only while a game is open.
pub const STOP_KEYS: &str = "CommandOrControl+Alt+Q";

fn watch(app: &AppHandle, shell: &Shell, running: Option<Running>) {
    let Some(running) = running else { return };
    let (app, core) = (app.clone(), shell.app.clone());
    // if another program holds the keys, the Stop button is still there
    let shortcut = tauri_plugin_global_shortcut::GlobalShortcutExt::global_shortcut(&app)
        .register(STOP_KEYS)
        .is_ok();
    std::thread::spawn(move || {
        // what the game reports while it plays (save slots used, screenshots, play time) goes
        // to the page as it happens
        core.watch(running, |event| {
            let _ = app.emit("game-event", event);
        });
        if shortcut {
            let _ = tauri_plugin_global_shortcut::GlobalShortcutExt::global_shortcut(&app)
                .unregister(STOP_KEYS);
        }
        let _ = app.emit("game-exited", ());
    });
}

#[tauri::command]
pub async fn launch_game(
    app: AppHandle,
    shell: State<'_, Shell>,
    path: String,
    slot: Option<u8>,
) -> Result<(), String> {
    let core = shell.app.clone();
    let events = app.clone();
    let running = blocking(move || {
        // The core is downloaded first when the game needs one that is not here yet
        // (from the offline set when there is one), so Play just works.
        if let Some(stem) = core.core_to_install(&path) {
            let _ = events.emit("core-downloading", stem.as_str());
            core.install_core(&stem, |percent| {
                let _ = events.emit(
                    "core-progress",
                    super::cores::Progress {
                        core: &stem,
                        percent,
                    },
                );
            })
            .map_err(|e| format!("could not get the {stem} core: {e}"))?;
        }
        // A Windows game is set up from its disc the first time (minutes): the page is told
        // what is going on, and Play below finds it ready.
        let _ = events.emit("pc-preparing", "Starting");
        let prepared = core.prepare_pc(&path, |line| {
            let _ = events.emit("pc-preparing", line.trim());
        });
        // an earlier press of Play is setting this game up: its banner stays
        if !matches!(&prepared, Err(e) if e == vuretro_core::service::PC_BUSY) {
            let _ = events.emit("pc-prepared", ());
        }
        prepared?;
        core.launch_game(path, slot)
    })
    .await??;
    let started = running.is_some();
    let zoom = running.as_ref().is_some_and(|r| r.zoom());
    watch(&app, &shell, running);
    if started {
        // a Wine game's window opens behind the app unless it is handed the front
        std::thread::spawn(crate::front::raise_wine_game);
    }
    if zoom {
        // a game that takes the whole display for its window is zoomed to what the screen holds
        std::thread::spawn(crate::front::zoom_wine_window);
    }
    Ok(())
}

/// Run the installer of a Windows game's disc. The user follows it in its window; the program it
/// leaves becomes the one that Play starts. Gives the name of that program.
#[tauri::command]
pub async fn run_pc_installer(
    app: AppHandle,
    shell: State<'_, Shell>,
    path: String,
) -> Result<String, String> {
    let core = shell.app.clone();
    let events = app.clone();
    blocking(move || {
        let _ = events.emit("pc-preparing", "Starting");
        let done = core.run_pc_installer(&path, |line| {
            let _ = events.emit("pc-preparing", line.trim());
        });
        if !matches!(&done, Err(e) if e == vuretro_core::service::PC_BUSY) {
            let _ = events.emit("pc-prepared", ());
        }
        done
    })
    .await?
}

/// End the game that is open (a Windows game has no window of its own to close).
#[tauri::command]
pub fn stop_game(shell: State<'_, Shell>) -> bool {
    shell.app.stop_game()
}

#[tauri::command]
pub async fn launch_demo(app: AppHandle, shell: State<'_, Shell>) -> Result<(), String> {
    let core = shell.app.clone();
    let running = blocking(move || core.launch_demo()).await??;
    watch(&app, &shell, running);
    Ok(())
}

#[tauri::command]
pub fn list_native_games(shell: State<'_, Shell>) -> Vec<NativeGame> {
    shell.app.native_games()
}

#[tauri::command]
pub fn add_native_game(
    shell: State<'_, Shell>,
    name: String,
    exe: String,
    args: Vec<String>,
    cover: Option<String>,
) -> Result<NativeGame, String> {
    shell.app.add_native_game_with(&name, &exe, args, cover)
}

#[tauri::command]
pub fn remove_native_game(shell: State<'_, Shell>, id: String) {
    shell.app.remove_native_game(&id);
}

#[tauri::command]
pub async fn launch_native(
    app: AppHandle,
    shell: State<'_, Shell>,
    id: String,
) -> Result<(), String> {
    let core = shell.app.clone();
    let running = blocking(move || core.launch_native(&id)).await??;
    watch(&app, &shell, running);
    Ok(())
}

#[tauri::command]
pub fn found_native_games(shell: State<'_, Shell>) -> Vec<NativeGame> {
    shell.app.found_native_games()
}

#[tauri::command]
pub fn programs_in_folder(shell: State<'_, Shell>, dir: String) -> Result<Vec<NativeGame>, String> {
    shell.app.programs_in_folder(&dir)
}

/// The tools that setting up Road Rash needs and this computer lacks.
#[tauri::command]
pub fn native_tools_missing(shell: State<'_, Shell>) -> Result<Vec<String>, String> {
    shell.app.native_tools_missing()
}

/// Install the missing tools; every line the installer prints goes to the page as
/// `native-install-log`.
#[tauri::command]
pub async fn install_native_tools(app: AppHandle, shell: State<'_, Shell>) -> Result<(), String> {
    let core = shell.app.clone();
    blocking(move || {
        core.install_native_tools(|line| {
            let _ = app.emit("native-install-log", line);
        })
    })
    .await?
}

/// Build Road Rash from its source, import the disc the user chose and add the game to the
/// library; the lines of the build go to the page as `native-install-log`.
#[tauri::command]
pub async fn install_road_rash(
    app: AppHandle,
    shell: State<'_, Shell>,
    disc: String,
) -> Result<NativeGame, String> {
    let core = shell.app.clone();
    blocking(move || {
        core.install_road_rash(&disc, |line| {
            let _ = app.emit("native-install-log", line);
        })
    })
    .await?
}

/// The pictures of the native games that have one, as `data:` URLs by id.
#[tauri::command]
pub fn native_covers(shell: State<'_, Shell>) -> std::collections::HashMap<String, String> {
    use base64::Engine;
    shell
        .app
        .native_games()
        .into_iter()
        .filter_map(|g| {
            let bytes = shell.app.native_cover(&g.id)?;
            let mime = vuretro_core::model::image_mime(&bytes)?;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            Some((g.id, format!("data:{mime};base64,{b64}")))
        })
        .collect()
}
