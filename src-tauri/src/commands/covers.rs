use crate::state::Shell;
use serde::Deserialize;
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, State};

/// The preview images that exist, as `data:` URLs the page can show directly.
#[tauri::command]
pub fn get_covers(shell: State<'_, Shell>, paths: Vec<String>) -> HashMap<String, String> {
    use base64::Engine;
    shell
        .app
        .covers(paths)
        .into_iter()
        .map(|(path, bytes)| {
            let mime = vuretro_core::model::image_mime(&bytes).unwrap_or("image/png");
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            (path, format!("data:{mime};base64,{b64}"))
        })
        .collect()
}

#[derive(Deserialize)]
pub struct CoverRequest {
    path: String,
    system: String,
}

/// Make preview images of the games asked for, in the background (a new request replaces
/// the one still waiting, so only what is on screen is made). Emits `covers-updated` after every image and
/// `covers-done` at the end.
#[tauri::command]
pub fn generate_covers(app: AppHandle, shell: State<'_, Shell>, games: Vec<CoverRequest>) {
    let games = games.into_iter().map(|g| (g.path, g.system)).collect();
    if !shell.app.request_covers(games) {
        return;
    }
    let core = shell.app.clone();
    std::thread::spawn(move || {
        core.run_covers(|key| {
            let _ = app.emit("covers-updated", key);
        });
        let _ = app.emit("covers-done", ());
    });
}

/// Switch cover-art downloads on or off (they send game names to the libretro-thumbnails
/// server, so they are off until the user asks).
#[tauri::command]
pub fn set_fetch_art(shell: State<'_, Shell>, enabled: bool) {
    shell.app.set_fetch_art(enabled);
}

/// Download box art for named games in the background; emits `covers-updated` per game.
#[tauri::command]
pub fn fetch_art(app: AppHandle, shell: State<'_, Shell>) {
    let core = shell.app.clone();
    std::thread::spawn(move || {
        core.fetch_art(|key| {
            let _ = app.emit("covers-updated", key);
        });
    });
}

/// Use a picture the user chose as a game's cover.
#[tauri::command]
pub fn set_custom_art(shell: State<'_, Shell>, path: String, image: String) -> Result<(), String> {
    shell.app.set_custom_art(&path, &image)
}

/// Drop downloaded or chosen cover art.
#[tauri::command]
pub fn clear_art(shell: State<'_, Shell>, path: String) {
    shell.app.clear_art(&path);
}

/// Save (or, with an empty user name, forget) the user's ScreenScraper login.
#[tauri::command]
pub fn set_screenscraper(shell: State<'_, Shell>, user: String, password: String) {
    shell.app.set_screenscraper(user, password);
}

/// Delete every screenshot preview so they are made again; returns how many were removed.
#[tauri::command]
pub fn regenerate_previews(shell: State<'_, Shell>) -> usize {
    shell.app.regenerate_previews()
}
