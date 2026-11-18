use super::blocking;
use crate::state::Shell;
use tauri::State;
use vuretro_core::backup::{BackupManifest, BackupOptions, BackupSummary};

/// Write a backup zip to `dest` (slow with saves and pictures: off the UI thread).
#[tauri::command]
pub async fn create_backup(
    shell: State<'_, Shell>,
    dest: String,
    saves: bool,
    covers: bool,
) -> Result<BackupSummary, String> {
    let app = shell.app.clone();
    blocking(move || app.create_backup(&dest, BackupOptions { saves, covers })).await?
}

/// What a backup file holds, before the user agrees to restore it.
#[tauri::command]
pub async fn inspect_backup(
    shell: State<'_, Shell>,
    src: String,
) -> Result<BackupManifest, String> {
    let app = shell.app.clone();
    blocking(move || app.inspect_backup(&src)).await?
}

#[tauri::command]
pub async fn restore_backup(shell: State<'_, Shell>, src: String) -> Result<BackupSummary, String> {
    let app = shell.app.clone();
    blocking(move || app.restore_backup(&src)).await?
}

/// Save the library as a CSV spreadsheet; returns how many games were written.
#[tauri::command]
pub async fn export_library(shell: State<'_, Shell>, dest: String) -> Result<usize, String> {
    let app = shell.app.clone();
    blocking(move || app.export_library(&dest)).await?
}
