//! vuretro desktop app. The UI (Svelte, `ui/`) talks to the commands in `commands/`; games
//! run in a separate `vuretro-runner` process so a crashing core never takes the UI down.
//!
//! This file only wires things together: adapters from `vuretro-library` are plugged into
//! the use cases of `vuretro-core`, and those are handed to Tauri.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod gamepad;
mod menu;
mod platform;
mod state;

use state::Shell;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use vuretro_core::paths::Paths;
use vuretro_core::service::App;
use vuretro_library::adapters::{FsCatalog, FsLocator, JsonStore, ProcessLauncher};

fn build_app() -> App {
    let paths =
        Paths::system_default().unwrap_or_else(|| Paths::new(PathBuf::from("vuretro-data")));
    App::new(
        paths.clone(),
        Box::new(JsonStore::new(paths.clone())),
        Box::new(FsCatalog::new(paths.clone())),
        Box::new(FsLocator::new(paths.clone())),
        Box::new(ProcessLauncher::new(paths)),
    )
}

fn main() {
    let shell = Shell {
        app: Arc::new(build_app()),
        pads: Arc::new(Mutex::new(Vec::new())),
    };
    let pads = shell.pads.clone();

    use commands::{config::*, covers::*, launch::*, library::*};
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(shell)
        .menu(menu::build)
        .on_menu_event(|app, event| menu::handle(app, event.id().as_ref()))
        .setup(move |app| {
            gamepad::spawn(app.handle().clone(), pads.clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_overview,
            set_rom_dir,
            set_system_enabled,
            set_system_core,
            toggle_favorite,
            import_bios,
            import_roms,
            remove_rom,
            reveal_path,
            remove_roms,
            launch_game,
            launch_demo,
            get_controls,
            set_controls,
            default_controls,
            get_game_settings,
            get_macros,
            set_macros,
            get_covers,
            generate_covers,
            set_game_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vuretro");
}
