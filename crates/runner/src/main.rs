//! vuretro-runner: play one game with one libretro core.
//!
//!   vuretro-runner --core cores/bin/fbneo_libretro.dylib --rom ~/roms/neogeo/mslug.zip
//!
//! Hotkeys: Esc quit · F5 save state · F7 load state · F6 next slot
//!          Tab (hold) fast-forward · F2 reset

mod audio;
mod core;
mod frontend;
mod input;
mod options;

use anyhow::{Context, Result};
use clap::Parser;
use directories::ProjectDirs;
use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};
use std::collections::VecDeque;
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
    #[arg(long)]
    rom: PathBuf,
    /// BIOS directory (default: <data dir>/system)
    #[arg(long)]
    system_dir: Option<PathBuf>,
    /// Save directory for SRAM and save states (default: <data dir>/saves)
    #[arg(long)]
    save_dir: Option<PathBuf>,
    /// Integer window scale: 1, 2, 4
    #[arg(long, default_value_t = 2)]
    scale: u8,
    /// Run N frames without a window or audio, then exit (for CI and debugging)
    #[arg(long, value_name = "N")]
    headless_frames: Option<u32>,
    /// With --headless-frames: write the last frame to this .ppm file
    #[arg(long, value_name = "FILE")]
    screenshot: Option<PathBuf>,
    /// Core options file, `key = value` per line (default: <data dir>/config/<core>.cfg)
    #[arg(long, value_name = "FILE")]
    options: Option<PathBuf>,
    /// Print the core's options as a config template and exit
    #[arg(long)]
    list_options: bool,
}

const FAST_FORWARD_FRAMES: usize = 4;
const SLOTS: u8 = 4;
const SRAM_AUTOSAVE: std::time::Duration = std::time::Duration::from_secs(30);

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();

    let dirs = ProjectDirs::from("dev", "vuvn", "vuretro").context("no home directory")?;
    let system_dir = args
        .system_dir
        .unwrap_or_else(|| dirs.data_dir().join("system"));
    let save_dir = args
        .save_dir
        .unwrap_or_else(|| dirs.data_dir().join("saves"));
    std::fs::create_dir_all(&system_dir)?;
    std::fs::create_dir_all(&save_dir)?;
    std::fs::create_dir_all(dirs.data_dir().join("config"))?;
    log::info!("system dir: {}", system_dir.display());

    let core_stem = args
        .core
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let options_path = args.options.clone().unwrap_or_else(|| {
        dirs.data_dir()
            .join("config")
            .join(format!("{core_stem}.cfg"))
    });
    let user_options = std::fs::read_to_string(&options_path)
        .map(|t| options::parse_config(&t))
        .unwrap_or_default();
    if !user_options.is_empty() {
        log::info!(
            "core options: {} from {}",
            user_options.len(),
            options_path.display()
        );
    }

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
    let av = core.load_game(&args.rom)?;
    if args.list_options {
        let defs = frontend::with(|fe| fe.definitions.clone());
        print!("{}", options::template(&defs));
        return Ok(());
    }
    let rom_name = args
        .rom
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
        return run_headless(&core, n, args.screenshot.as_deref(), &sram_path);
    }

    let _audio = match audio::start(audio_buf, av.timing.sample_rate) {
        Ok(a) => Some(a),
        Err(e) => {
            log::warn!("audio disabled: {e}");
            None
        }
    };

    let scale = match args.scale {
        1 => Scale::X1,
        4 => Scale::X4,
        _ => Scale::X2,
    };
    let mut window = Window::new(
        &format!("vuretro — {rom_name}"),
        av.geometry.base_width as usize,
        av.geometry.base_height as usize,
        WindowOptions {
            resize: true,
            scale,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(av.timing.fps.round().max(1.0) as usize);

    let mut input = input::Input::new();
    let mut slot: u8 = 1;
    let state_path = |slot: u8| save_dir.join(format!("{rom_name}.state{slot}"));
    let mut last_sram_save = std::time::Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.is_key_pressed(Key::F5, KeyRepeat::No) {
            match core.save_state() {
                Some(data) => match std::fs::write(state_path(slot), data) {
                    Ok(_) => log::info!("saved state to slot {slot}"),
                    Err(e) => log::error!("save state failed: {e}"),
                },
                None => log::warn!("core does not support save states"),
            }
        }
        if window.is_key_pressed(Key::F7, KeyRepeat::No) {
            match std::fs::read(state_path(slot)) {
                Ok(data) if core.load_state(&data) => log::info!("loaded slot {slot}"),
                Ok(_) => log::error!("core rejected state in slot {slot}"),
                Err(_) => log::warn!("slot {slot} is empty"),
            }
        }
        if window.is_key_pressed(Key::F6, KeyRepeat::No) {
            slot = slot % SLOTS + 1;
            log::info!("slot {slot}");
        }
        if window.is_key_pressed(Key::F2, KeyRepeat::No) {
            core.reset();
        }

        if last_sram_save.elapsed() >= SRAM_AUTOSAVE {
            last_sram_save = std::time::Instant::now();
            if let Err(e) = core.save_sram(&sram_path) {
                log::warn!("autosave failed: {e}");
            }
        }

        input.poll(&window);
        let runs = if window.is_key_down(Key::Tab) {
            FAST_FORWARD_FRAMES
        } else {
            1
        };
        for _ in 0..runs {
            core.run();
        }

        let frame = frontend::with(|fe| {
            fe.frame_dirty.then(|| {
                fe.frame_dirty = false;
                (fe.frame.clone(), fe.width, fe.height)
            })
        });
        match frame {
            Some((buf, w, h)) if w > 0 && h > 0 => window.update_with_buffer(&buf, w, h)?,
            _ => window.update(),
        }
    }
    match core.save_sram(&sram_path) {
        Ok(0) => {}
        Ok(n) => log::info!("saved {n} bytes of save RAM"),
        Err(e) => log::error!("cannot save save RAM: {e}"),
    }
    Ok(())
}

fn run_headless(
    core: &core::Core,
    frames: u32,
    screenshot: Option<&std::path::Path>,
    sram_path: &std::path::Path,
) -> Result<()> {
    let start = std::time::Instant::now();
    for _ in 0..frames {
        core.run();
    }
    let elapsed = start.elapsed().as_secs_f64();
    log::info!(
        "ran {frames} frames in {elapsed:.2}s ({:.0} fps)",
        frames as f64 / elapsed.max(1e-9)
    );

    let (buf, w, h) = frontend::with(|fe| (fe.frame.clone(), fe.width, fe.height));
    anyhow::ensure!(w > 0 && h > 0, "core produced no video frame");
    if let Some(path) = screenshot {
        let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
        ppm.extend(
            buf.iter()
                .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8]),
        );
        std::fs::write(path, ppm)?;
        log::info!("screenshot: {}", path.display());
    }
    if let Some(state) = core.save_state() {
        anyhow::ensure!(core.load_state(&state), "save state round-trip failed");
        log::info!("save state round-trip ok ({} bytes)", state.len());
    }
    let n = core.save_sram(sram_path)?;
    if n > 0 {
        log::info!("saved {n} bytes of save RAM to {}", sram_path.display());
    }
    Ok(())
}
