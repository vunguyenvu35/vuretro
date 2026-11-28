use super::blocking;
use crate::state::Shell;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Clone, Serialize)]
pub struct Progress<'a> {
    pub core: &'a str,
    pub percent: u8,
}

/// Download a core into the app's cores folder. Emits `core-progress` while it runs;
/// the returned error (if any) is the reason it failed.
#[tauri::command]
pub async fn install_core(
    app: AppHandle,
    shell: State<'_, Shell>,
    core: String,
) -> Result<(), String> {
    let core_app = shell.app.clone();
    blocking(move || {
        core_app.install_core(&core, |percent| {
            let _ = app.emit(
                "core-progress",
                Progress {
                    core: &core,
                    percent,
                },
            );
        })
    })
    .await?
}

/// Switch the background download of missing cores (when the app starts) on or off.
#[tauri::command]
pub fn set_auto_install_cores(shell: State<'_, Shell>, enabled: bool) {
    shell.app.set_auto_install_cores(enabled);
}

/// The cores of the systems that are on and not installed yet (`wine` for Windows games).
#[tauri::command]
pub fn missing_cores(shell: State<'_, Shell>, include_off: bool) -> Vec<String> {
    shell.app.missing_cores(include_off)
}

/// The installed cores that have a newer build on the server. Asks the server, so it runs off the
/// UI thread; the error is why it could not (no internet).
#[tauri::command]
pub async fn core_updates(shell: State<'_, Shell>) -> Result<Vec<String>, String> {
    let core_app = shell.app.clone();
    blocking(move || core_app.core_updates()).await?
}

/// The options a core offers, with the user's saved choices. Starts the runner, so it
/// runs off the UI thread.
#[tauri::command]
pub async fn get_core_options(
    shell: State<'_, Shell>,
    core: String,
) -> Result<Vec<vuretro_core::model::CoreOptionView>, String> {
    let core_app = shell.app.clone();
    blocking(move || core_app.core_options(&core)).await?
}

/// Save the options that differ from the core's defaults.
#[tauri::command]
pub fn set_core_options(
    shell: State<'_, Shell>,
    core: String,
    overrides: std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
    shell.app.set_core_options(&core, overrides)
}
