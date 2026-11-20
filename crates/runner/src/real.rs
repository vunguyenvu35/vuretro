//! The real implementations of the session's ports.

use crate::screen::MiniFbScreen;
use crate::{appicon, audio, core, display, frontend, input};
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
use vuretro_controls::macros::Macro;
use vuretro_controls::ControlsConfig;
use vuretro_play::ports::{Controller, Emulator, Frame, FrameSource, Hotkeys, Screen, View};
use vuretro_play::session::{Config, Parts, Session};
use vuretro_play::window::clamp_scale;
use vuretro_settings::GameSettings;

impl Emulator for core::Core {
    fn run(&self) {
        core::Core::run(self)
    }
    fn reset(&self) {
        core::Core::reset(self)
    }
    fn save_state(&self) -> Option<Vec<u8>> {
        core::Core::save_state(self)
    }
    fn load_state(&self, data: &[u8]) -> bool {
        core::Core::load_state(self, data)
    }
    fn save_sram(&self, path: &Path) -> Result<usize> {
        core::Core::save_sram(self, path)
    }
    fn next_disc(&self) -> Option<String> {
        let cb = crate::frontend::with(|fe| fe.disk)?;
        Some(crate::disc::swap_to_next(&cb))
    }
}

impl Controller for input::Input {
    fn poll(&mut self, screen: &dyn Screen) {
        input::Input::poll(self, screen)
    }
    fn hotkeys(&mut self, _screen: &dyn Screen) -> Hotkeys {
        input::Input::hotkeys(self)
    }
}

/// Frames the core delivers through the libretro video callback.
pub struct CoreFrames;

impl FrameSource for CoreFrames {
    fn take_new(&mut self) -> Option<Frame> {
        frontend::with(|fe| {
            fe.frame_dirty.then(|| {
                fe.frame_dirty = false;
                match fe.gpu_frame {
                    // still on the graphics card: no pixels, only the size
                    Some((w, h)) => (Vec::new(), w, h),
                    None => (fe.frame.clone(), fe.width, fe.height),
                }
            })
        })
    }
    fn current(&self) -> Frame {
        frontend::with(|fe| {
            // a picture left on the graphics card is read back now, once
            if let Some((w, h)) = fe.gpu_frame.take() {
                if let Some((px, rw, rh)) = crate::hw::read_frame(w, h) {
                    fe.frame = px;
                    fe.width = rw;
                    fe.height = rh;
                }
            }
            (fe.frame.clone(), fe.width, fe.height)
        })
    }
}

/// Everything `main` has worked out before the window opens.
pub struct Params {
    pub core: core::Core,
    pub av: libretro_sys::retro_system_av_info,
    pub rom_name: String,
    pub save_dir: PathBuf,
    pub sram_path: PathBuf,
    pub shots_dir: PathBuf,
    pub settings: GameSettings,
    pub settings_path: PathBuf,
    pub overrides_path: PathBuf,
    pub system: String,
    pub controls: ControlsConfig,
    pub macros: Vec<Macro>,
    /// Window scale from the command line, overriding the saved setting.
    pub scale: Option<u8>,
    /// Where to save a preview image when the window closes.
    pub cover: Option<PathBuf>,
    pub audio: audio::AudioSink,
    /// Save slot to load at start.
    pub start_slot: Option<u8>,
}

/// Open the window, start the audio and wire up the real core and controls.
pub fn open(p: Params) -> Result<Session> {
    let av = p.av;
    let gain = Arc::new(AtomicU32::new(p.settings.gain_percent()));
    let speed = Arc::new(AtomicU32::new(100));
    // Pace video to the screen when its refresh rate is close to the core's. The game
    // then runs a little faster or slower than the original (up to 5 %); audio is
    // resampled to follow, so it stays in sync.
    let core_fps = av.timing.fps.max(1.0);
    let fps = if p.settings.sync_display {
        display::refresh_rate()
            .and_then(|hz| display::match_rate(core_fps, hz))
            .unwrap_or(core_fps)
    } else {
        core_fps
    };
    if (fps - core_fps).abs() > 0.01 {
        log::info!("video paced to the display: {fps:.2} fps (core {core_fps:.2})");
    }
    let audio = match audio::start(
        p.audio,
        av.timing.sample_rate * fps / core_fps,
        gain.clone(),
        speed.clone(),
    ) {
        Ok(a) => Some(a),
        Err(e) => {
            log::warn!("audio disabled: {e}");
            None
        }
    };

    let (base_w, base_h) = (
        av.geometry.base_width as usize,
        av.geometry.base_height as usize,
    );
    let factor = clamp_scale(p.scale.unwrap_or(p.settings.window_scale));
    let title = format!("Vuretro — {}", p.rom_name);
    let view = View {
        fullscreen: p.settings.fullscreen,
        integer: p.settings.integer_scale,
    };
    // the icon first, so the Dock tile is made with it; once more after the window exists,
    // because the first window changes how the process presents itself
    appicon::apply(&p.system);
    let mut window = "opengl";
    let screen: Box<dyn Screen> = if use_gl_window() {
        let hw = crate::frontend::with(|fe| fe.hw).map(|request| crate::gl_screen::HwSetup {
            request,
            max: (
                (av.geometry.max_width as usize).max(base_w),
                (av.geometry.max_height as usize).max(base_h),
            ),
        });
        let wants_gpu = hw.is_some();
        match crate::gl_screen::GlScreen::open(&title, base_w, base_h, factor, view, hw) {
            Ok(screen) => Box::new(screen),
            // a computer with no usable OpenGL still plays the games that do not need it
            Err(e) if !wants_gpu => {
                log::warn!("no OpenGL window ({e:#}); using the basic window");
                window = "basic";
                Box::new(MiniFbScreen::open(&title, base_w, base_h, factor, view)?)
            }
            Err(e) => return Err(e.context("this game needs an OpenGL window")),
        }
    } else {
        window = "basic";
        Box::new(MiniFbScreen::open(&title, base_w, base_h, factor, view)?)
    };
    appicon::apply(&p.system);
    // the game is loaded and its window is up: the app can stop waiting for a failure
    crate::events::emit(&vuretro_core::protocol::Event::Ready {
        core: p.core.info.name.clone(),
        core_version: p.core.info.version.clone(),
        width: base_w as u32,
        height: base_h as u32,
        fps: core_fps,
        window: window.to_string(),
        hardware_render: frontend::with(|fe| fe.hw.is_some()),
    });

    let coin_op = vuretro_core::registry::by_id(&p.system).is_some_and(|s| s.coin_op);
    if coin_op && p.settings.auto_coin {
        log::info!("auto coin: a coin goes in whenever Start is pressed");
    }
    if !p.macros.is_empty() {
        log::info!("{} macro button(s) for this game", p.macros.len());
    }
    let parts = Parts {
        screen,
        emulator: Box::new(p.core),
        controller: Box::new(
            input::Input::new(p.controls, p.macros, fps)
                .with_rumble(p.settings.rumble)
                .with_auto_coin(coin_op && p.settings.auto_coin),
        ),
        frames: Box::new(CoreFrames),
        files: Box::new(crate::files::DiskFiles),
        events: Box::new(crate::events::RunnerEvents),
        gain,
        speed,
        keep_alive: Box::new(audio),
    };
    Ok(Session::new(
        parts,
        Config {
            rom_name: p.rom_name,
            save_dir: p.save_dir,
            sram_path: p.sram_path,
            shots_dir: p.shots_dir,
            settings: p.settings,
            settings_path: p.settings_path,
            overrides_path: p.overrides_path,
            system: p.system,
            cover: p.cover,
            base_w,
            base_h,
            factor,
            fps,
            start_slot: p.start_slot,
        },
    ))
}

/// Which window to use: the OpenGL one (winit) unless `VURETRO_WINDOW=minifb` asks for the
/// basic one. The basic window cannot run 3D cores.
pub fn use_gl_window() -> bool {
    !std::env::var("VURETRO_WINDOW").is_ok_and(|v| v.eq_ignore_ascii_case("minifb"))
}
