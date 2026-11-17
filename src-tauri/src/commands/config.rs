//! Controls, macros and game settings: small files the runner reads too.

use crate::state::Shell;
use serde::Serialize;
use tauri::State;
use vuretro_controls::macros::MacroConfig;
use vuretro_controls::{ControlsConfig, PadInfo};
use vuretro_settings::GameSettings;

#[derive(Serialize)]
pub struct ControlsView {
    config: ControlsConfig,
    pads: Vec<PadInfo>,
}

#[tauri::command]
pub fn get_controls(shell: State<'_, Shell>) -> ControlsView {
    ControlsView {
        config: shell.app.controls(),
        pads: shell.pads.lock().unwrap().clone(),
    }
}

#[tauri::command]
pub fn set_controls(
    shell: State<'_, Shell>,
    config: ControlsConfig,
) -> Result<ControlsConfig, String> {
    shell.app.set_controls(config)
}

#[tauri::command]
pub fn default_controls(shell: State<'_, Shell>) -> ControlsConfig {
    shell.app.default_controls()
}

#[tauri::command]
pub fn get_macros(shell: State<'_, Shell>) -> MacroConfig {
    shell.app.macros()
}

#[tauri::command]
pub fn set_macros(shell: State<'_, Shell>, config: MacroConfig) -> Result<MacroConfig, String> {
    shell.app.set_macros(config)
}

#[tauri::command]
pub fn get_game_settings(shell: State<'_, Shell>) -> GameSettings {
    shell.app.game_settings()
}

#[tauri::command]
pub fn set_game_settings(
    shell: State<'_, Shell>,
    settings: GameSettings,
) -> Result<GameSettings, String> {
    shell.app.set_game_settings(settings)
}
