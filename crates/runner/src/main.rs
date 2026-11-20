//! vuretro-runner: play one game with one libretro core.
//!
//!   vuretro-runner --core cores/bin/fbneo_libretro.dylib --rom ~/roms/neogeo/mslug.zip
//!
//! Hotkeys: Esc quit · F5 save state · F7 load state · F6 next slot
//!          Tab (hold) fast-forward · F2 reset · F8 sound on/off
//!          F9 / F10 smaller / bigger window · P or F3 pause · F12 screenshot
//! Every one of these is also in the window's menu bar (macOS / Windows).

mod appicon;
mod audio;
mod autocoin;
mod core;
mod disc;
mod display;
mod files;
mod frontend;
mod headless;
mod input;
mod mamebios;
mod options;
mod padkeys;
mod real;
mod screen;

use anyhow::{Context, Result};
use clap::Parser;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Parser)]
#[command(
    name = "vuretro-runner",
    version,
    about = "Run a game with a libretro core"
)]
struct Args {
    /// Path to the libretro core (.dylib / .dll / .so)
    #[arg(long)]
    core: PathBuf,
    /// Path to the ROM / disc image
    #[arg(long, required_unless_present = "options_json")]
    rom: Option<PathBuf>,
    /// BIOS directory (default: <data dir>/system)
    #[arg(long)]
    system_dir: Option<PathBuf>,
    /// Save directory for SRAM and save states (default: <data dir>/saves)
    #[arg(long)]
    save_dir: Option<PathBuf>,
    /// Window scale 1..=8 (default: from the app's game settings)
    #[arg(long)]
    scale: Option<u8>,
    /// Game settings file (default: <data dir>/config/game.json)
    #[arg(long, value_name = "FILE")]
    settings: Option<PathBuf>,
    /// Run N frames without a window or audio, then exit (for CI and debugging)
    #[arg(long, value_name = "N")]
    headless_frames: Option<u32>,
    /// With --headless-frames: write the last frame to this .ppm file
    #[arg(long, value_name = "FILE")]
    screenshot: Option<PathBuf>,
    /// Save a preview image (PNG) of the game here: the best frame of a headless run,
    /// or the last frame when a window is closed
    #[arg(long, value_name = "FILE")]
    cover: Option<PathBuf>,
    /// Core options file, `key = value` per line (default: <data dir>/config/<core>.cfg)
    #[arg(long, value_name = "FILE")]
    options: Option<PathBuf>,
    /// Id of the system being played: picks its settings overrides and its own key bindings
    #[arg(long, value_name = "ID")]
    system: Option<String>,
    /// Controls file (default: <data dir>/config/controls.json)
    #[arg(long, value_name = "FILE")]
    controls: Option<PathBuf>,
    /// Print the core's options as a config template and exit
    #[arg(long)]
    list_options: bool,
    /// Load this save slot (1..=4) right after the game starts
    #[arg(long, value_name = "N")]
    load_slot: Option<u8>,
    /// Print the core's options as JSON (key, description, values) and exit; needs no ROM
    #[arg(long)]
    options_json: bool,
}

/// User options for a core, plus the frontend's own defaults for specific cores.
fn core_options(
    path: &std::path::Path,
    core_stem: &str,
    system_dir: &std::path::Path,
) -> HashMap<String, String> {
    let mut user_options = std::fs::read_to_string(path)
        .map(|t| options::parse_config(&t))
        .unwrap_or_default();
    if !user_options.is_empty() {
        log::info!(
            "core options: {} from {}",
            user_options.len(),
            path.display()
        );
    }
    // the frontend's own defaults (see `options::frontend_defaults`); the .cfg still wins
    for (key, value) in options::frontend_defaults(core_stem, system_dir) {
        user_options.entry(key).or_insert(value);
    }
    user_options
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();

    // A game window shows up in the Dock as soon as the process first talks to the window
    // system, which is long before the window opens (the core is loaded first). Setting the
    // icon now means the Dock never shows the generic "exec" icon. Headless runs (previews,
    // CI) have no window and must not appear in the Dock at all.
    if args.headless_frames.is_none() && !args.list_options && !args.options_json {
        appicon::apply(args.system.as_deref().unwrap_or(""));
    }

    let paths = vuretro_library::adapters::system_paths().context("no home directory")?;
    let system_dir = args.system_dir.unwrap_or_else(|| paths.system_dir());
    let save_dir = args.save_dir.unwrap_or_else(|| paths.saves_dir());
    std::fs::create_dir_all(&system_dir)?;
    std::fs::create_dir_all(&save_dir)?;
    std::fs::create_dir_all(paths.config_dir())?;
    log::info!("system dir: {}", system_dir.display());

    let core_stem = args
        .core
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if core_stem == "mame_libretro" {
        mamebios::expose(&system_dir);
    }
    let options_path = args
        .options
        .unwrap_or_else(|| paths.core_options_file(&core_stem));
    let user_options = core_options(&options_path, &core_stem, &system_dir);

    let audio_buf = Arc::new(Mutex::new(VecDeque::<i16>::new()));
    frontend::init(
        &system_dir.to_string_lossy(),
        &save_dir.to_string_lossy(),
        audio_buf.clone(),
        user_options,
    );

    let mut core = core::Core::load(&args.core)?;
    log::info!(
        "core: {} {} [{}]",
        core.info.name,
        core.info.version,
        core.info.extensions
    );
    if args.options_json {
        // Some cores (FBNeo) announce most of their options only once a game is loaded: with
        // a ROM given, load it first. A game that fails to load still leaves the options
        // announced so far.
        if let Some(rom) = &args.rom {
            if let Err(e) = core.load_game(rom) {
                log::warn!("could not load {} to read its options: {e}", rom.display());
            }
        }
        let defs = frontend::with(|fe| fe.definitions.clone());
        println!(
            "{}",
            options::to_json(&defs, &options::frontend_defaults(&core_stem, &system_dir))
        );
        return Ok(());
    }
    let rom = args.rom.clone().context("--rom is required")?;
    let mut av = core.load_game(&rom)?;
    if core_stem.starts_with("mame") || av.timing.sample_rate < 1.0 || av.geometry.base_width == 0 {
        // MAME reports its largest picture (512x256) until it has run a frame, which would
        // stretch a 384x224 game: run one frame, then ask again.
        core.run();
        av = core.av_info();
        audio_buf.lock().unwrap().clear();
    }
    if args.list_options {
        let defs = frontend::with(|fe| fe.definitions.clone());
        print!("{}", options::template(&defs));
        return Ok(());
    }
    let cheat_file = paths.cheats_file(&rom);
    if let Ok(text) = std::fs::read_to_string(&cheat_file) {
        let codes = vuretro_settings::cheats::active(&text);
        if !codes.is_empty() {
            match core.set_cheats(&codes) {
                Some(n) => log::info!("cheats: {n} active from {}", cheat_file.display()),
                None => log::warn!("this core has no cheat support"),
            }
        }
    }
    let rom_name = rom
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let sram_path = save_dir.join(format!("{rom_name}.srm"));
    match core.load_sram(&sram_path) {
        Ok(0) => {}
        Ok(n) => log::info!("loaded {n} bytes of save RAM from {}", sram_path.display()),
        Err(e) => log::warn!("cannot load save RAM: {e}"),
    }
    log::info!(
        "game: {}x{} @ {:.2} fps, audio {:.0} Hz",
        av.geometry.base_width,
        av.geometry.base_height,
        av.timing.fps,
        av.timing.sample_rate
    );

    if let Some(n) = args.headless_frames {
        return headless::run_headless(
            &core,
            n,
            args.screenshot.as_deref(),
            args.cover.as_deref(),
            &sram_path,
        );
    }

    let system = args.system.clone().unwrap_or_default();
    let settings_path = args.settings.unwrap_or_else(|| paths.game_settings_file());
    let overrides_path = paths.overrides_file();
    // general settings, then this system's choices, then this game's
    let settings = vuretro_settings::scoped::load_overrides(&overrides_path).resolve(
        &vuretro_settings::load(&settings_path),
        vuretro_settings::scoped::Scope {
            system: &system,
            game: &rom_name,
        },
    );
    // key bindings: the game's own, else its system's, else the general ones
    let controls_path = args.controls.unwrap_or_else(|| {
        let game = paths.game_controls_file(&rom);
        let sys = paths.system_controls_file(&system);
        if game.is_file() {
            game
        } else if !system.is_empty() && sys.is_file() {
            sys
        } else {
            paths.controls_file()
        }
    });
    let game_macros = vuretro_controls::macros::load(&paths.macros_file())
        .for_game(&vuretro_controls::macros::game_key(&rom_name), &system);

    real::open(real::Params {
        core,
        av,
        save_dir,
        sram_path,
        shots_dir: paths.screenshots_dir(),
        settings,
        settings_path,
        overrides_path,
        system,
        controls: vuretro_controls::load(&controls_path),
        macros: game_macros,
        scale: args.scale,
        cover: args.cover,
        audio_buf,
        rom_name,
        start_slot: args.load_slot,
    })?
    .run()
}
