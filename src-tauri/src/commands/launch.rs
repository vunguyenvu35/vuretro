use super::blocking;
use crate::state::Shell;
use tauri::{AppHandle, Emitter, State};
use vuretro_core::service::Running;

/// Wait for the game on its own thread, then tell the page it ended.
fn watch(app: &AppHandle, shell: &Shell, running: Option<Running>) {
    let Some(running) = running else { return };
    let (app, core) = (app.clone(), shell.app.clone());
    std::thread::spawn(move || {
        core.wait_for_exit(running);
        let _ = app.emit("game-exited", ());
    });
}

#[tauri::command]
pub async fn launch_game(
    app: AppHandle,
    shell: State<'_, Shell>,
    path: String,
) -> Result<(), String> {
    let core = shell.app.clone();
    let running = blocking(move || core.launch_game(path)).await??;
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
