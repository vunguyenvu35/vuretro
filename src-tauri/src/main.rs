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
use vuretro_library::adapters::{
    BuildbotInstaller, DevLogin, FsCatalog, FsLocator, JsonStore, ProcessLauncher, RoadRashInstaller,
    ScreenScraperArt, SysMachine, ThumbnailsArt, ZipBackup,
};

fn app_paths() -> Paths {
    vuretro_library::adapters::system_paths()
        .unwrap_or_else(|| Paths::new(PathBuf::from("vuretro-data")))
}

fn build_app() -> App {
    let paths = app_paths();
    let app = App::new(
        paths.clone(),
        Box::new(JsonStore::new(paths.clone())),
        Box::new(FsCatalog::new(paths.clone())),
        Box::new(FsLocator::new(paths.clone())),
        Box::new(ProcessLauncher::new(paths.clone())),
    )
    .with_installer(Box::new(BuildbotInstaller::new(paths.clone())))
    .with_native_installer(Box::new(RoadRashInstaller::new(paths.clone())))
    .with_archive(Box::new(ZipBackup::new(paths)))
    .with_machine(Box::new(SysMachine))
    .with_art_source(Box::new(ThumbnailsArt::new()));
    // ScreenScraper needs developer credentials issued to the project; without them it is off.
    match DevLogin::from_env() {
        Some(dev) => app.with_art_source(Box::new(ScreenScraperArt::new(dev))),
        None => app,
    }
}

fn main() {
    let pad_names_file = app_paths().config_dir().join("pad_names.json");
    let shell = Shell {
        app: Arc::new(build_app()),
        pads: Arc::new(Mutex::new(Vec::new())),
        dongle_waiting: Arc::default(),
        pad_names: Arc::new(Mutex::new(vuretro_controls::padnames::load(
            &pad_names_file,
        ))),
        pad_names_file,
    };
    let pads = shell.pads.clone();
    let pad_names = shell.pad_names.clone();
    let dongle_waiting = shell.dongle_waiting.clone();

    use commands::{backup::*, config::*, cores::*, covers::*, launch::*, library::*, update::*};
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // the shortcut that stops the open game (see `commands::launch::STOP_KEYS`)
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        use tauri::Manager;
                        app.state::<Shell>().app.stop_game();
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(shell)
        .menu(menu::build)
        .on_menu_event(|app, event| menu::handle(app, event.id().as_ref()))
        .setup(move |app| {
            gamepad::spawn(
                app.handle().clone(),
                pads.clone(),
                pad_names.clone(),
                dongle_waiting.clone(),
            );
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_overview,
            list_native_games,
            found_native_games,
            native_tools_missing,
            install_native_tools,
            install_road_rash,
            native_covers,
            programs_in_folder,
            add_native_game,
            remove_native_game,
            launch_native,
            set_rom_dir,
            set_system_enabled,
            set_system_core,
            install_core,
            get_core_options,
            get_save_slots,
            import_dats,
            set_fetch_art,
            set_custom_art,
            regenerate_previews,
            set_screenscraper,
            clear_art,
            fetch_art,
            identify_games,
            set_tags,
            rename_game,
            rename_pad,
            clear_recent,
            set_game_core,
            reset_mame_inputs,
            get_cheats,
            set_cheats,
            get_controls_for,
            set_controls_for,
            reset_controls_for,
            get_scoped_settings,
            set_system_override,
            get_game_override,
            open_url,
            check_update,
            install_update,
            set_game_override,
            get_controls_for_game,
            create_backup,
            inspect_backup,
            restore_backup,
            export_library,
            set_controls_for_game,
            reset_controls_for_game,
            get_slot_thumbnail,
            set_core_options,
            toggle_favorite,
            import_bios,
            import_roms,
            apply_patch,
            remove_rom,
            reveal_path,
            remove_roms,
            launch_game,
            stop_game,
            launch_demo,
            get_controls,
            set_controls,
            default_controls,
            get_game_settings,
            get_macros,
            set_macro_profile,
            compile_move,
            set_macros,
            get_covers,
            generate_covers,
            set_game_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vuretro");
}
