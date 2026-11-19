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

/// What a move written in notation would play; an error says what is wrong with it.
#[tauri::command]
pub fn compile_move(
    shell: State<'_, Shell>,
    notation: String,
    mirror: bool,
) -> Result<vuretro_core::model::CompiledMove, String> {
    shell.app.compile_move(&notation, mirror)
}

/// Choose which macro profile of a game plays (none: only the shared macros).
#[tauri::command]
pub fn set_macro_profile(
    shell: State<'_, Shell>,
    game: String,
    profile: Option<String>,
) -> Result<MacroConfig, String> {
    shell.app.set_active_macro_profile(&game, profile)
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

#[derive(Serialize)]
pub struct ScopedControlsView {
    config: ControlsConfig,
    custom: bool,
    pads: Vec<PadInfo>,
}

fn scoped_view(shell: &Shell, v: vuretro_core::model::ScopedControls) -> ScopedControlsView {
    ScopedControlsView {
        config: v.config,
        custom: v.custom,
        pads: shell.pads.lock().unwrap().clone(),
    }
}

/// Key bindings for everything (`system` empty) or one system.
#[tauri::command]
pub fn get_controls_for(
    shell: State<'_, Shell>,
    system: Option<String>,
) -> Result<ScopedControlsView, String> {
    let v = shell
        .app
        .controls_for(system.as_deref().filter(|s| !s.is_empty()))?;
    Ok(scoped_view(&shell, v))
}

#[tauri::command]
pub fn set_controls_for(
    shell: State<'_, Shell>,
    system: String,
    config: ControlsConfig,
) -> Result<ScopedControlsView, String> {
    let v = shell.app.set_controls_for(&system, config)?;
    Ok(scoped_view(&shell, v))
}

#[tauri::command]
pub fn reset_controls_for(
    shell: State<'_, Shell>,
    system: String,
) -> Result<ScopedControlsView, String> {
    let v = shell.app.reset_controls_for(&system)?;
    Ok(scoped_view(&shell, v))
}

#[tauri::command]
pub fn get_scoped_settings(
    shell: State<'_, Shell>,
    system: Option<String>,
) -> Result<vuretro_core::model::ScopedSettings, String> {
    shell
        .app
        .scoped_settings(system.as_deref().filter(|s| !s.is_empty()))
}

#[tauri::command]
pub fn set_system_override(
    shell: State<'_, Shell>,
    system: String,
    overrides: vuretro_settings::scoped::Override,
) -> Result<vuretro_core::model::ScopedSettings, String> {
    shell.app.set_system_override(&system, overrides)
}

/// Settings of one game, on top of its system's.
#[tauri::command]
pub fn get_game_override(
    shell: State<'_, Shell>,
    path: String,
) -> vuretro_core::model::ScopedSettings {
    shell.app.game_override(&path)
}

#[tauri::command]
pub fn set_game_override(
    shell: State<'_, Shell>,
    path: String,
    overrides: vuretro_settings::scoped::Override,
) -> Result<vuretro_core::model::ScopedSettings, String> {
    shell.app.set_game_override(&path, overrides)
}

/// Key bindings of one game (its own, else its system's, else the general ones).
#[tauri::command]
pub fn get_controls_for_game(shell: State<'_, Shell>, path: String) -> ScopedControlsView {
    let v = shell.app.controls_for_game(&path);
    scoped_view(&shell, v)
}

#[tauri::command]
pub fn set_controls_for_game(
    shell: State<'_, Shell>,
    path: String,
    config: ControlsConfig,
) -> Result<ScopedControlsView, String> {
    let v = shell.app.set_controls_for_game(&path, config)?;
    Ok(scoped_view(&shell, v))
}

#[tauri::command]
pub fn reset_controls_for_game(
    shell: State<'_, Shell>,
    path: String,
) -> Result<ScopedControlsView, String> {
    let v = shell.app.reset_controls_for_game(&path)?;
    Ok(scoped_view(&shell, v))
}
