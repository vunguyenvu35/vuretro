use super::blocking;
use crate::state::Shell;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Clone, Serialize)]
struct Progress<'a> {
    core: &'a str,
    percent: u8,
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
