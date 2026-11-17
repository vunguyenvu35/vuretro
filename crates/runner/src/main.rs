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
mod core;
mod cover;
mod display;
mod frontend;
mod input;
mod options;

use anyhow::{Context, Result};
use clap::Parser;
use directories::ProjectDirs;
use minifb::{Key, KeyRepeat, Menu, Scale, Window, WindowOptions};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
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
    /// Controls file (default: <data dir>/config/controls.json)
    #[arg(long, value_name = "FILE")]
    controls: Option<PathBuf>,
    /// Print the core's options as a config template and exit
    #[arg(long)]
    list_options: bool,
}

const FAST_FORWARD_FRAMES: usize = 4;
const SLOTS: u8 = 4;
const NOTICE_FOR: std::time::Duration = std::time::Duration::from_secs(2);
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
    let mut user_options = std::fs::read_to_string(&options_path)
        .map(|t| options::parse_config(&t))
        .unwrap_or_default();
    if !user_options.is_empty() {
        log::info!(
            "core options: {} from {}",
            user_options.len(),
            options_path.display()
        );
    }

    // FBNeo's default Neo Geo mode boots the MVS BIOS, which shows several seconds of
    // garbled green memory-test screens (and needs coins). AES boots straight to the
    // logo and starts with Start; still overridable in the core's .cfg.
    if core_stem.starts_with("fbneo") {
        user_options
            .entry("fbneo-neogeo-mode".to_string())
            .or_insert_with(|| "AES_EUR".to_string());
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
        return run_headless(
            &core,
            n,
            args.screenshot.as_deref(),
            args.cover.as_deref(),
            &sram_path,
        );
    }

    let settings_path = args
        .settings
        .clone()
        .unwrap_or_else(|| dirs.data_dir().join("config").join("game.json"));
    let mut settings = vuretro_settings::load(&settings_path);
    let gain = Arc::new(AtomicU32::new(settings.gain_percent()));
    // Pace video to the screen when its refresh rate is close to the core's. The game
    // then runs a little faster or slower than the original (up to 5 %); audio is
    // resampled to follow, so it stays in sync.
    let core_fps = av.timing.fps.max(1.0);
    let fps = if settings.sync_display {
        display::refresh_rate()
            .and_then(|hz| display::match_rate(core_fps, hz))
            .unwrap_or(core_fps)
    } else {
        core_fps
    };
    if (fps - core_fps).abs() > 0.01 {
        log::info!("video paced to the display: {fps:.2} fps (core {core_fps:.2})");
    }
    let _audio = match audio::start(
        audio_buf,
        av.timing.sample_rate * fps / core_fps,
        gain.clone(),
    ) {
        Ok(a) => Some(a),
        Err(e) => {
            log::warn!("audio disabled: {e}");
            None
        }
    };

    let title = format!("vuretro — {rom_name}");
    let (base_w, base_h) = (
        av.geometry.base_width as usize,
        av.geometry.base_height as usize,
    );
    let mut factor = clamp_scale(args.scale.unwrap_or(settings.window_scale));
    let mut window = open_window(&title, base_w, base_h, factor)?;
    appicon::apply();
    // Frame pacing is done below against absolute deadlines: minifb's own limiter
    // sleeps a fixed amount per frame, so it drifts (57 fps instead of 59.18) and
    // the audio then underruns.
    let frame_time = std::time::Duration::from_secs_f64(1.0 / fps);
    let mut deadline = std::time::Instant::now();

    let controls_path = args
        .controls
        .clone()
        .unwrap_or_else(|| dirs.data_dir().join("config").join("controls.json"));
    let mut input = input::Input::new(vuretro_controls::load(&controls_path));
    let shots_dir = dirs.data_dir().join("screenshots");
    let mut slot: u8 = 1;
    let state_path = |slot: u8| save_dir.join(format!("{rom_name}.state{slot}"));
    let mut last_sram_save = std::time::Instant::now();
    let mut sound_on = settings.sound;
    // Pause: F3, or the window losing focus when that is switched on in the settings.
    let mut paused = false;
    let mut holding = false; // paused or in the background: emulation is stopped
    let mut last_frame: Option<(Vec<u32>, usize, usize)> = None;
    let mut dimmed: Option<Vec<u32>> = None;

    install_menus(&mut window);
    let mut notice: Option<(String, std::time::Instant)> = None;
    let mut shown_title = title.clone();
    let mut quit = false;

    while window.is_open() && !quit && !window.is_key_down(Key::Escape) {
        // Hotkeys and menu choices end up as the same list of actions.
        let mut acts: Vec<Act> = [
            (Key::F5, Act::SaveState),
            (Key::F7, Act::LoadState),
            (Key::F6, Act::NextSlot),
            (Key::F8, Act::Mute),
            (Key::F9, Act::SizeStep(-1)),
            (Key::F10, Act::SizeStep(1)),
            (Key::F2, Act::Reset),
            (Key::F3, Act::Pause),
            (Key::P, Act::Pause), // no Fn needed on a laptop keyboard
            (Key::F12, Act::Screenshot),
        ]
        .into_iter()
        .filter(|(k, _)| window.is_key_pressed(*k, KeyRepeat::No))
        .map(|(_, a)| a)
        .collect();
        while let Some(id) = window.is_menu_pressed() {
            acts.extend(act_from_id(id));
        }

        for act in acts {
            let said = match act {
                Act::SaveState => Some(match core.save_state() {
                    Some(data) => match std::fs::write(state_path(slot), data) {
                        Ok(_) => format!("saved to slot {slot}"),
                        Err(e) => format!("save failed: {e}"),
                    },
                    None => "this core has no save states".to_string(),
                }),
                Act::LoadState => Some(match std::fs::read(state_path(slot)) {
                    Ok(data) if core.load_state(&data) => format!("loaded slot {slot}"),
                    Ok(_) => format!("slot {slot} was rejected by the core"),
                    Err(_) => format!("slot {slot} is empty"),
                }),
                Act::NextSlot => {
                    slot = slot % SLOTS + 1;
                    Some(format!("slot {slot}"))
                }
                Act::Slot(n) => {
                    slot = n;
                    Some(format!("slot {slot}"))
                }
                Act::Reset => {
                    core.reset();
                    Some("reset".to_string())
                }
                Act::Pause => {
                    paused = !paused;
                    None
                }
                Act::Mute => {
                    sound_on = !sound_on;
                    if !holding {
                        gain.store(
                            gain_for(sound_on, settings.volume, false),
                            Ordering::Relaxed,
                        );
                    }
                    Some(format!("sound {}", if sound_on { "on" } else { "off" }))
                }
                Act::SizeStep(_) | Act::Size(_) => {
                    let wanted = match act {
                        Act::Size(n) => clamp_scale(n),
                        Act::SizeStep(d) => clamp_scale((factor as i32 + d).max(1) as u8),
                        _ => factor,
                    };
                    if wanted != factor {
                        let at = window.get_position();
                        // Open the new window before the old one closes so the app never has none.
                        window = open_window(&title, base_w, base_h, wanted)?;
                        window.set_position(at.0, at.1);
                        install_menus(&mut window);
                        shown_title.clear();
                        factor = wanted;
                        settings.window_scale = wanted as u8;
                        if let Err(e) = vuretro_settings::save(&settings_path, &settings) {
                            log::warn!("cannot save window size: {e}");
                        }
                    }
                    Some(format!("size {factor}×"))
                }
                Act::Screenshot => Some(match &last_frame {
                    Some((buf, w, h)) => {
                        let stamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        let path = shots_dir.join(format!("{rom_name}-{stamp}.png"));
                        match cover::write_png(&path, buf, *w, *h) {
                            Ok(_) => format!("screenshot saved ({})", shots_dir.display()),
                            Err(e) => format!("screenshot failed: {e}"),
                        }
                    }
                    None => "nothing to capture yet".to_string(),
                }),
                Act::Quit => {
                    quit = true;
                    None
                }
            };
            if let Some(msg) = said {
                log::info!("{msg}");
                notice = Some((msg, std::time::Instant::now()));
            }
        }

        let hold = paused || (settings.pause_on_blur && !window.is_active());
        if hold != holding {
            holding = hold;
            gain.store(
                gain_for(sound_on, settings.volume, holding),
                Ordering::Relaxed,
            );
            dimmed = last_frame
                .as_ref()
                .map(|(buf, w, h)| paused_overlay(buf, *w, *h));
            log::info!("{}", if holding { "paused" } else { "resumed" });
        }
        let wanted_title = window_title(&title, holding, &notice);
        if wanted_title != shown_title {
            window.set_title(&wanted_title);
            shown_title = wanted_title;
        }
        if holding {
            match (&dimmed, &last_frame) {
                (Some(buf), Some((_, w, h))) => window.update_with_buffer(buf, *w, *h)?,
                _ => window.update(),
            }
            pace(&mut deadline, frame_time);
            continue;
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
            Some((buf, w, h)) if w > 0 && h > 0 => {
                window.update_with_buffer(&buf, w, h)?;
                last_frame = Some((buf, w, h));
            }
            _ => window.update(),
        }
        pace(&mut deadline, frame_time);
    }
    if let Some(path) = &args.cover {
        let mut best = cover::Best::default();
        frontend::with(|fe| best.offer(&fe.frame, fe.width, fe.height));
        if let Err(e) = best.save(path) {
            log::warn!("cannot save cover: {e}");
        }
    }
    match core.save_sram(&sram_path) {
        Ok(0) => {}
        Ok(n) => log::info!("saved {n} bytes of save RAM"),
        Err(e) => log::error!("cannot save save RAM: {e}"),
    }
    Ok(())
}

/// Output level in percent: 0 when sound is off or the game is paused.
fn gain_for(sound_on: bool, volume: u8, holding: bool) -> u32 {
    if sound_on && !holding {
        volume as u32
    } else {
        0
    }
}

/// The paused picture: the last frame at half brightness with a pause symbol on it.
fn paused_overlay(buf: &[u32], w: usize, h: usize) -> Vec<u32> {
    let mut out: Vec<u32> = buf.iter().map(|p| (p >> 1) & 0x007F_7F7F).collect();
    let (bar_w, bar_h) = ((w / 18).max(2), (h / 4).max(4));
    let (x0, y0) = ((w - 3 * bar_w) / 2, (h - bar_h) / 2);
    for y in y0..y0 + bar_h {
        for x in (x0..x0 + bar_w).chain(x0 + 2 * bar_w..x0 + 3 * bar_w) {
            out[y * w + x] = 0x00FF_FFFF;
        }
    }
    out
}

/// Things the player can do while a game runs: by hotkey or from the menu bar.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Act {
    SaveState,
    LoadState,
    NextSlot,
    Slot(u8),
    Reset,
    Pause,
    Mute,
    SizeStep(i32),
    Size(u8),
    Screenshot,
    Quit,
}

const MENU_MOD: usize = if cfg!(target_os = "macos") {
    minifb::MENU_KEY_COMMAND
} else {
    minifb::MENU_KEY_CTRL
};

/// Menu item ids are plain numbers: map them to actions and back.
fn act_id(a: Act) -> usize {
    match a {
        Act::SaveState => 1,
        Act::LoadState => 2,
        Act::NextSlot => 3,
        Act::Slot(n) => 10 + n as usize,
        Act::Reset => 4,
        Act::Pause => 5,
        Act::Mute => 6,
        Act::SizeStep(-1) => 7,
        Act::SizeStep(_) => 8,
        Act::Size(n) => 20 + n as usize,
        Act::Screenshot => 9,
        Act::Quit => 30,
    }
}

fn act_from_id(id: usize) -> Option<Act> {
    let all = [
        Act::SaveState,
        Act::LoadState,
        Act::NextSlot,
        Act::Reset,
        Act::Pause,
        Act::Mute,
        Act::SizeStep(-1),
        Act::SizeStep(1),
        Act::Screenshot,
        Act::Quit,
    ];
    all.into_iter()
        .chain((1..=SLOTS).map(Act::Slot))
        .chain((1..=4).map(Act::Size))
        .find(|a| act_id(*a) == id)
}

/// Add the game menus to the window (macOS and Windows; elsewhere minifb has none and
/// the hotkeys do the job). Call again for a new window.
fn install_menus(window: &mut Window) {
    let mut add = |name: &str, build: &dyn Fn(&mut Menu)| match Menu::new(name) {
        Ok(mut m) => {
            build(&mut m);
            window.add_menu(&m);
        }
        Err(e) => log::debug!("no {name} menu: {e}"),
    };
    let item = |m: &mut Menu, label: &str, act: Act, key: Option<Key>| {
        let mut it = m.add_item(label, act_id(act));
        match key {
            Some(k) => it.shortcut(k, MENU_MOD).build(),
            None => it.build(),
        };
    };
    add("Game", &|m| {
        item(m, "Pause / Resume (P or F3)", Act::Pause, Some(Key::P));
        item(m, "Reset (F2)", Act::Reset, Some(Key::R));
        m.add_separator();
        m.add_item("Fast Forward: hold Tab", 0)
            .enabled(false)
            .build();
        m.add_separator();
        item(m, "Screenshot (F12)", Act::Screenshot, Some(Key::T));
        item(m, "Sound On / Off (F8)", Act::Mute, Some(Key::M));
        m.add_separator();
        item(m, "Close Game (Esc)", Act::Quit, None);
    });
    add("State", &|m| {
        item(m, "Save State (F5)", Act::SaveState, Some(Key::S));
        item(m, "Load State (F7)", Act::LoadState, Some(Key::L));
        m.add_separator();
        item(m, "Next Slot (F6)", Act::NextSlot, None);
        for n in 1..=SLOTS {
            item(m, &format!("Use Slot {n}"), Act::Slot(n), None);
        }
    });
    add("Size", &|m| {
        item(m, "Smaller (F9)", Act::SizeStep(-1), Some(Key::Minus));
        item(m, "Bigger (F10)", Act::SizeStep(1), Some(Key::Equal));
        m.add_separator();
        for n in 1..=4u8 {
            item(m, &format!("{n}×"), Act::Size(n), None);
        }
    });
}

/// Title bar text: the game, its state and a short-lived message about what just happened.
fn window_title(
    base: &str,
    holding: bool,
    notice: &Option<(String, std::time::Instant)>,
) -> String {
    let mut t = base.to_string();
    if holding {
        t.push_str(" (paused, press P to resume)");
    }
    if let Some((msg, at)) = notice {
        if at.elapsed() < NOTICE_FOR {
            t.push_str(" · ");
            t.push_str(msg);
        }
    }
    t
}

fn clamp_scale(n: u8) -> usize {
    n.clamp(vuretro_settings::MIN_SCALE, vuretro_settings::MAX_SCALE) as usize
}

/// minifb's own scaling only does 1, 2, 4, 8 and 16; for other sizes the window is
/// made that many times bigger and the frame is stretched to fit it.
fn open_window(title: &str, base_w: usize, base_h: usize, factor: usize) -> Result<Window> {
    let (scale, stretch) = match factor {
        1 => (Scale::X1, 1),
        2 => (Scale::X2, 1),
        4 => (Scale::X4, 1),
        8 => (Scale::X8, 1),
        n => (Scale::X1, n),
    };
    Ok(Window::new(
        title,
        base_w * stretch,
        base_h * stretch,
        WindowOptions {
            resize: true,
            scale,
            ..WindowOptions::default()
        },
    )?)
}

/// Sleep until the next frame deadline (sleep most of it, spin the rest for accuracy).
/// If we fell far behind (window dragged, machine suspended) resync instead of
/// racing to catch up.
fn pace(deadline: &mut std::time::Instant, frame_time: std::time::Duration) {
    use std::time::{Duration, Instant};
    *deadline += frame_time;
    let now = Instant::now();
    if *deadline > now {
        let wait = *deadline - now;
        if wait > Duration::from_millis(2) {
            std::thread::sleep(wait - Duration::from_millis(1));
        }
        while Instant::now() < *deadline {
            std::hint::spin_loop();
        }
    } else if now - *deadline > frame_time * 4 {
        *deadline = now;
    }
}

fn run_headless(
    core: &core::Core,
    frames: u32,
    screenshot: Option<&std::path::Path>,
    cover: Option<&std::path::Path>,
    sram_path: &std::path::Path,
) -> Result<()> {
    let start = std::time::Instant::now();
    let mut best = cover::Best::default();
    for i in 0..frames {
        core.run();
        // Boot screens are dull: look for the most colourful frame in the later part.
        if cover.is_some() && i >= frames / 4 && i % 30 == 0 {
            frontend::with(|fe| best.offer(&fe.frame, fe.width, fe.height));
        }
    }
    if let Some(path) = cover {
        if best.save(path)? {
            log::info!("cover: {}", path.display());
        } else {
            log::warn!("cover: no colourful frame found");
        }
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

#[cfg(test)]
mod pause_tests {
    use super::*;

    #[test]
    fn gain_is_zero_when_muted_or_paused() {
        assert_eq!(gain_for(true, 80, false), 80);
        assert_eq!(gain_for(false, 80, false), 0);
        assert_eq!(gain_for(true, 80, true), 0);
    }

    #[test]
    fn paused_picture_is_dimmed_with_a_symbol() {
        let buf = vec![0x00FF_FFFFu32; 64 * 48];
        let out = paused_overlay(&buf, 64, 48);
        assert_eq!(out.len(), buf.len());
        assert_eq!(out[0], 0x007F_7F7F); // half brightness
        assert!(out.contains(&0x00FF_FFFF)); // the bars
    }
}
