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

/// Make a patched copy of a game (IPS, UPS or BPS patch) in the library.
#[tauri::command]
pub async fn apply_patch(
    shell: State<'_, Shell>,
    rom: String,
    patch: String,
) -> Result<Outcome, String> {
    let app = shell.app.clone();
    blocking(move || app.apply_patch(rom, patch)).await
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

/// Add DAT files so games get their official names.
#[tauri::command]
pub fn import_dats(shell: State<'_, Shell>, paths: Vec<String>) -> Vec<Outcome> {
    shell.app.import_dats(paths)
}

/// Look games up in the DAT files (reads files, so off the UI thread). Returns how many
/// games got a name.
#[tauri::command]
pub async fn identify_games(shell: State<'_, Shell>) -> Result<usize, String> {
    let core_app = shell.app.clone();
    super::blocking(move || core_app.identify_games()).await
}

/// Give a game a name of the user's own (`None` or an empty name: its original one again).
/// Returns the name now kept.
#[tauri::command]
pub fn rename_game(
    shell: State<'_, Shell>,
    path: String,
    name: Option<String>,
) -> Result<Option<String>, String> {
    shell.app.rename_game(path, name)
}

/// Empty the Recent list; returns how many games were on it.
#[tauri::command]
pub fn clear_recent(shell: State<'_, Shell>) -> usize {
    shell.app.clear_recent()
}

/// Replace a game's collections; returns the cleaned list.
#[tauri::command]
pub fn set_tags(shell: State<'_, Shell>, path: String, tags: Vec<String>) -> Vec<String> {
    shell.app.set_tags(path, tags)
}

/// Run one game on another core (`None`: its system's).
#[tauri::command]
pub fn set_game_core(
    shell: State<'_, Shell>,
    path: String,
    core: Option<String>,
) -> Result<(), String> {
    shell.app.set_game_core(path, core)
}

/// Drop the key remaps MAME saved for one game (it keeps them in `saves/<core>/cfg/<rom>.cfg`
/// and they win over the app's buttons). The coin counters and sound settings in the file stay,
/// and the old file is kept next to it as `.bak-before-reset`. Returns how many files changed.
#[tauri::command]
pub fn reset_mame_inputs(shell: State<'_, Shell>, path: String) -> Result<usize, String> {
    let stem = std::path::Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("this file has no name")?
        .to_string();
    let saves = shell.app.paths().saves_dir();
    let Ok(cores) = std::fs::read_dir(&saves) else {
        return Ok(0);
    };
    let mut changed = 0;
    for core in cores.flatten() {
        let cfg = core.path().join("cfg").join(format!("{stem}.cfg"));
        let Ok(text) = std::fs::read_to_string(&cfg) else {
            continue;
        };
        let Some(clean) = vuretro_core::mamecfg::without_input(&text) else {
            continue;
        };
        let mut backup = cfg.clone().into_os_string();
        backup.push(".bak-before-reset");
        std::fs::copy(&cfg, &backup).map_err(|e| format!("cannot back up {}: {e}", cfg.display()))?;
        std::fs::write(&cfg, clean).map_err(|e| format!("cannot write {}: {e}", cfg.display()))?;
        changed += 1;
    }
    Ok(changed)
}

/// A game's cheat codes as text.
#[tauri::command]
pub fn get_cheats(shell: State<'_, Shell>, path: String) -> String {
    shell.app.cheats(&path)
}

#[tauri::command]
pub fn set_cheats(shell: State<'_, Shell>, path: String, text: String) -> Result<(), String> {
    shell.app.set_cheats(&path, &text)
}

/// Open one of the project's own links (GitHub, Ko-fi) in the browser. Anything else is
/// refused, so a page can never use this to open an arbitrary address.
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    if !vuretro_core::links::url_allowed(&url) {
        return Err("this link is not allowed".into());
    }
    platform::open_external(&url);
    Ok(())
}
