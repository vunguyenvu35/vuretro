use super::blocking;
use crate::state::Shell;
use tauri::{AppHandle, Emitter, State};
use vuretro_core::service::Running;

/// Watch the game on its own thread, passing on what it reports, then tell the page it ended.
fn watch(app: &AppHandle, shell: &Shell, running: Option<Running>) {
    let Some(running) = running else { return };
    let (app, core) = (app.clone(), shell.app.clone());
    std::thread::spawn(move || {
        // what the game reports while it plays (save slots used, screenshots, play time) goes
        // to the page as it happens
        core.watch(running, |event| {
            let _ = app.emit("game-event", event);
        });
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
        core.launch_game(path, slot)
    })
    .await??;
    watch(&app, &shell, running);
    Ok(())
}

#[tauri::command]
pub async fn launch_demo(app: AppHandle, shell: State<'_, Shell>) -> Result<(), String> {
    let core = shell.app.clone();
    let running = blocking(move || core.launch_demo()).await??;
    watch(&app, &shell, running);
    Ok(())
}
