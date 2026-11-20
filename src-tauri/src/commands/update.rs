//! Updates of the app itself: ask GitHub whether a newer version is out, and install it.
//!
//! The release carries a `latest.json` and each package is signed; the updater refuses a package
//! whose signature does not match the public key in `tauri.conf.json`, so only a build made with
//! the project's private key can ever be installed.

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

#[derive(Serialize)]
pub struct UpdateInfo {
    pub version: String,
    pub current: String,
    pub notes: Option<String>,
}

#[derive(Serialize, Clone)]
struct Progress {
    done: u64,
    total: Option<u64>,
}

/// The newer version, or `None` when this one is the latest.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<Option<UpdateInfo>, String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    Ok(update.map(|u| UpdateInfo {
        version: u.version,
        current: u.current_version,
        notes: u.body,
    }))
}

/// Download and install the newer version (progress goes out as `update-progress`), then
/// restart into it.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or("already on the latest version")?;
    let mut done = 0u64;
    let sink = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                done += chunk as u64;
                let _ = sink.emit("update-progress", Progress { done, total });
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;
    app.restart()
}
