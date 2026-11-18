use super::blocking;
use crate::platform;
use crate::state::Shell;
use tauri::State;
use vuretro_core::model::{Outcome, Overview};

#[tauri::command]
pub async fn get_overview(shell: State<'_, Shell>) -> Result<Overview, String> {
    let app = shell.app.clone();
    blocking(move || app.overview()).await
}

#[tauri::command]
pub async fn import_roms(
    shell: State<'_, Shell>,
    paths: Vec<String>,
    system: Option<String>,
) -> Result<Vec<Outcome>, String> {
    let app = shell.app.clone();
    blocking(move || app.import_roms(paths, system)).await
}

#[tauri::command]
pub fn import_bios(shell: State<'_, Shell>, paths: Vec<String>) -> Vec<Outcome> {
    shell.app.import_bios(paths)
}

#[tauri::command]
pub fn remove_rom(shell: State<'_, Shell>, path: String) -> Result<(), String> {
    shell.app.remove_rom(&path)
}

#[tauri::command]
pub fn remove_roms(shell: State<'_, Shell>, paths: Vec<String>) -> Vec<Outcome> {
    shell.app.remove_roms(paths)
}

#[tauri::command]
pub fn set_rom_dir(shell: State<'_, Shell>, path: Option<String>) {
    shell.app.set_rom_dir(path);
}

#[tauri::command]
pub fn set_system_enabled(shell: State<'_, Shell>, id: String, enabled: bool) {
    shell.app.set_system_enabled(id, enabled);
}

#[tauri::command]
pub fn set_system_core(shell: State<'_, Shell>, id: String, core: String) -> Result<(), String> {
    shell.app.set_system_core(&id, &core)
}

#[tauri::command]
pub fn toggle_favorite(shell: State<'_, Shell>, path: String) -> bool {
    shell.app.toggle_favorite(path)
}

/// Show a file in Finder / Explorer / the file manager.
#[tauri::command]
pub fn reveal_path(path: String) {
    platform::reveal(&path);
}

/// Save slots of a game that hold a save.
#[tauri::command]
pub fn get_save_slots(shell: State<'_, Shell>, path: String) -> Vec<vuretro_core::model::SlotInfo> {
    shell.app.save_slots(&path)
}

/// A slot's picture as a `data:` URL.
#[tauri::command]
pub fn get_slot_thumbnail(shell: State<'_, Shell>, path: String, slot: u8) -> Option<String> {
    use base64::Engine;
    shell.app.slot_thumbnail(&path, slot).map(|bytes| {
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        format!("data:image/png;base64,{b64}")
    })
}
