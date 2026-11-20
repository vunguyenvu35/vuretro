//! An interactive game session: the window, the frame loop and the player's actions.
//! Built from ports (see `ports.rs`), so it runs against fakes in tests; `real::open`
//! builds the real one. [`Session::run`] plays until the window closes.

use crate::perf::{self, PerfMonitor, Status};
use crate::ports::{Controller, Emulator, Frame, FrameSource, Screen, View};
use crate::rewind::Rewind;
use crate::speed::{self, Mode};
use crate::window::{
    self, clamp_scale, gain_for, pace, paused_overlay, quit_overlay, window_title, Act, SLOTS,
};
use crate::{cover, crt, lcd, scale, scale2x};
use anyhow::Result;
use minifb::Key;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use vuretro_settings::scoped::{load_overrides, save_overrides, Change, Scope};
use vuretro_settings::GameSettings;

/// Rewind keeps a state every this many frames, up to this much memory.
const REWIND_EVERY: u64 = 2;
const REWIND_BYTES: usize = 64 * 1024 * 1024;
const SRAM_AUTOSAVE: Duration = Duration::from_secs(30);

/// What a session needs, built either from the real machine (`real::open`) or from fakes.
pub struct Parts {
    pub screen: Box<dyn Screen>,
    pub emulator: Box<dyn Emulator>,
    pub controller: Box<dyn Controller>,
    pub frames: Box<dyn FrameSource>,
    /// Output level in percent, read by the audio thread.
    pub gain: Arc<AtomicU32>,
    /// Game speed in percent (100 = normal), read by the audio thread.
    pub speed: Arc<AtomicU32>,
    /// Held for as long as the session lives (the audio stream).
    pub keep_alive: Box<dyn std::any::Any>,
}

pub struct Config {
    pub rom_name: String,
    pub save_dir: PathBuf,
    pub sram_path: PathBuf,
    pub shots_dir: PathBuf,
    pub settings: GameSettings,
    pub settings_path: PathBuf,
    /// Per-system and per-game overrides of the settings.
    pub overrides_path: PathBuf,
    /// Id of the system being played (empty when unknown).
    pub system: String,
    /// Where to save a preview image when the window closes.
    pub cover: Option<PathBuf>,
    pub base_w: usize,
    pub base_h: usize,
    /// Window size as a multiple of the native one (already clamped).
    pub factor: usize,
    /// Frames per second the video is paced to.
    pub fps: f64,
    /// Load this save slot as soon as the game starts.
    pub start_slot: Option<u8>,
}

/// How loading a saved state went.
enum Loaded {
    Yes,
    Rejected,
    Missing,
}

pub struct Session {
    screen: Box<dyn Screen>,
    emulator: Box<dyn Emulator>,
    controller: Box<dyn Controller>,
    frames: Box<dyn FrameSource>,
    gain: Arc<AtomicU32>,
    speed_percent: Arc<AtomicU32>,
    _keep_alive: Box<dyn std::any::Any>,
    settings: GameSettings,
    settings_path: PathBuf,
    overrides_path: PathBuf,
    system: String,
    rom_name: String,
    save_dir: PathBuf,
    sram_path: PathBuf,
    shots_dir: PathBuf,
    cover: Option<PathBuf>,

    title: String,
    shown_title: String,
    base_w: usize,
    base_h: usize,
    factor: usize,
    view: View,

    frame_time: Duration,
    deadline: Instant,
    last_sram_save: Instant,

    rewind: Option<Rewind>,
    perf: PerfMonitor,
    /// Set while the game runs slower than it should.
    slow_fps: Option<u32>,
    frames_run: u64,
    crt: bool,
    lcd: bool,
    scale2x: bool,
    /// Fast-forward key (or pad chord) is down right now.
    ff_held: bool,
    /// Fast-forward was switched on and stays on.
    ff_latched: bool,
    slow_motion: bool,
    rewind_held: bool,
    mode: Mode,
    /// What the loop does at the current speed.
    plan: speed::Plan,
    slot: u8,
    start_slot: Option<u8>,
    sound_on: bool,
    /// Pause: F3, or the window losing focus when that is switched on in the settings.
    paused: bool,
    /// Esc was pressed: the game waits, with the question "quit?" on the picture.
    asking: bool,
    /// What the dimmed picture was made for (the pause or the question).
    dim_asking: bool,
    /// Paused or in the background: emulation is stopped.
    holding: bool,
    last_frame: Option<Frame>,
    dimmed: Option<Vec<u32>>,
    notice: Option<(String, Instant)>,
    quit: bool,
}

impl Session {
    pub fn new(parts: Parts, cfg: Config) -> Self {
        let title = format!("Vuretro — {}", cfg.rom_name);
        let crt = cfg.settings.crt;
        let lcd = cfg.settings.lcd;
        let scale2x = cfg.settings.scale2x;
        let rewind = cfg.settings.rewind.then(|| Rewind::new(REWIND_BYTES));
        let view = parts.screen.view();
        Session {
            screen: parts.screen,
            emulator: parts.emulator,
            controller: parts.controller,
            frames: parts.frames,
            gain: parts.gain,
            speed_percent: parts.speed,
            _keep_alive: parts.keep_alive,
            sound_on: cfg.settings.sound,
            settings: cfg.settings,
            settings_path: cfg.settings_path,
            overrides_path: cfg.overrides_path,
            system: cfg.system,
            rom_name: cfg.rom_name,
            save_dir: cfg.save_dir,
            sram_path: cfg.sram_path,
            shots_dir: cfg.shots_dir,
            cover: cfg.cover,
            shown_title: title.clone(),
            title,
            base_w: cfg.base_w,
            base_h: cfg.base_h,
            factor: cfg.factor,
            view,
            // Frame pacing is done against absolute deadlines: minifb's own limiter
            // sleeps a fixed amount per frame, so it drifts (57 fps instead of 59.18)
            // and the audio then underruns.
            frame_time: Duration::from_secs_f64(1.0 / cfg.fps),
            deadline: Instant::now(),
            last_sram_save: Instant::now(),
            rewind,
            frames_run: 0,
            perf: PerfMonitor::new(Duration::from_secs_f64(1.0 / cfg.fps), perf::WINDOW),
            slow_fps: None,
            crt,
            lcd,
            scale2x,
            ff_held: false,
            ff_latched: false,
            slow_motion: false,
            rewind_held: false,
            mode: Mode::Normal,
            plan: speed::plan(100),
            slot: 1,
            start_slot: cfg.start_slot.filter(|n| (1..=SLOTS).contains(n)),
            paused: false,
            holding: false,
            asking: false,
            dim_asking: false,
            last_frame: None,
            dimmed: None,
            notice: None,
            quit: false,
        }
    }

    /// Where the game is saved when the window closes, to carry on from there.
    fn auto_state_path(&self) -> PathBuf {
        self.save_dir.join(format!("{}.stateauto", self.rom_name))
    }

    /// Play until the window closes, then save.
    pub fn run(mut self) -> Result<()> {
        if let Some(n) = self.start_slot.take() {
            self.slot = n;
            let note = self.apply(Act::LoadState)?;
            self.notice = note.map(|t| (t, Instant::now()));
        } else if self.settings.resume && self.auto_state_path().is_file() {
            let path = self.auto_state_path();
            let note = match self.load_state_file(&path) {
                Loaded::Yes => "carried on from where you left off",
                _ => "could not carry on from the last game: starting over",
            };
            self.notice = Some((note.to_string(), Instant::now()));
        }
        while self.screen.is_open() && !self.quit {
            self.frame()?;
        }
        self.finish();
        Ok(())
    }

    /// One pass of the loop: handle input, then either show the paused picture or run the core.
    fn frame(&mut self) -> Result<()> {
        self.update_quit_question();
        let hotkeys = self.controller.hotkeys(&*self.screen);
        let mut acts = self.pending_actions();
        acts.extend(hotkeys.acts);
        if self.asking {
            // while the question is open only the menu's Quit counts
            acts.retain(|a| *a == Act::Quit);
        }
        for act in acts {
            if let Some(msg) = self.apply(act)? {
                log::info!("{msg}");
                self.notice = Some((msg, Instant::now()));
            }
        }
        self.ff_held = self.screen.key_down(Key::Tab) || hotkeys.fast_forward;
        self.rewind_held = self.screen.key_down(Key::Backspace) || hotkeys.rewind;
        self.update_speed();
        self.update_hold();
        self.update_title();

        if self.holding {
            match (&self.dimmed, &self.last_frame) {
                (Some(buf), Some((_, w, h))) => {
                    let (w, h) = (*w, *h);
                    let buf = buf.clone();
                    self.show(&buf, w, h)?
                }
                _ => self.screen.idle(),
            }
            pace(&mut self.deadline, self.frame_time);
        } else {
            self.autosave_sram();
            self.run_core()?;
            pace(&mut self.deadline, self.frame_time * self.plan.stretch);
        }
        Ok(())
    }

    /// Esc asks "quit the game?" (an accidental press must not lose the play): Esc again
    /// quits, Enter, Space or N go on playing. Closing the window or the menu's Quit are
    /// deliberate and quit at once.
    fn update_quit_question(&mut self) {
        if self.asking {
            if self.screen.key_pressed(Key::Escape) {
                self.quit = true;
            } else if [Key::Enter, Key::Space, Key::N]
                .iter()
                .any(|k| self.screen.key_pressed(*k))
            {
                self.asking = false;
            }
        } else if self.screen.key_pressed(Key::Escape) {
            self.asking = true;
        }
    }

    /// Hotkeys and menu choices end up as the same list of actions.
    fn pending_actions(&mut self) -> Vec<Act> {
        let shift = self.screen.key_down(Key::LeftShift) || self.screen.key_down(Key::RightShift);
        let alt = self.screen.key_down(Key::LeftAlt) || self.screen.key_down(Key::RightAlt);
        let mut acts: Vec<Act> = [
            (Key::F5, Act::SaveState),
            (Key::F7, Act::LoadState),
            (Key::F6, Act::NextSlot),
            (Key::F8, Act::Mute),
            (
                Key::F9,
                if shift {
                    Act::Scale2x
                } else {
                    Act::SizeStep(-1)
                },
            ),
            (Key::F10, Act::SizeStep(1)),
            (Key::F2, Act::Reset),
            (Key::F3, Act::Pause),
            (Key::P, Act::Pause), // no Fn needed on a laptop keyboard
            (Key::F12, Act::Screenshot),
            (Key::F4, if shift { Act::Lcd } else { Act::Crt }),
            (
                Key::F1,
                if shift {
                    Act::SlowMotion
                } else {
                    Act::FastForwardToggle
                },
            ),
            (Key::F11, Act::NextDisc),
        ]
        .into_iter()
        .filter(|(k, _)| self.screen.key_pressed(*k))
        .map(|(_, a)| a)
        .collect();
        if alt && self.screen.key_pressed(Key::Enter) {
            acts.push(Act::Fullscreen);
        }
        while let Some(id) = self.screen.menu_pressed() {
            acts.extend(window::act_from_id(id));
        }
        acts
    }

    fn state_path(&self, slot: u8) -> PathBuf {
        self.save_dir.join(format!("{}.state{slot}", self.rom_name))
    }

    /// The picture shown for the slot in the app's slot picker.
    fn save_thumbnail(&self) {
        let (buf, w, h) = self.frames.current();
        let png = self
            .save_dir
            .join(format!("{}.state{}.png", self.rom_name, self.slot));
        if let Err(e) = cover::write_png(&png, &buf, w, h) {
            log::warn!("cannot save slot thumbnail: {e}");
        }
    }

    /// Hand a saved state to the core.
    fn load_state_file(&mut self, path: &Path) -> Loaded {
        // older states belong to a timeline that no longer exists
        if let Some(r) = self.rewind.as_mut() {
            r.clear();
        }
        match std::fs::read(path) {
            Ok(data) if self.emulator.load_state(&data) => Loaded::Yes,
            Ok(_) => Loaded::Rejected,
            Err(_) => Loaded::Missing,
        }
    }

    /// Carry out one action; the returned text is shown in the title bar for a moment.
    fn apply(&mut self, act: Act) -> Result<Option<String>> {
        Ok(match act {
            Act::SaveState => Some(match self.emulator.save_state() {
                Some(data) => match std::fs::write(self.state_path(self.slot), data) {
                    Ok(_) => {
                        self.save_thumbnail();
                        format!("saved to slot {}", self.slot)
                    }
                    Err(e) => format!("save failed: {e}"),
                },
                None => "this core has no save states".to_string(),
            }),
            Act::LoadState => {
                let slot = self.slot;
                let path = self.state_path(slot);
                Some(match self.load_state_file(&path) {
                    Loaded::Yes => format!("loaded slot {slot}"),
                    Loaded::Rejected => format!("slot {slot} was rejected by the core"),
                    Loaded::Missing => format!("slot {slot} is empty"),
                })
            }
            Act::NextSlot => {
                self.slot = self.slot % SLOTS + 1;
                Some(format!("slot {}", self.slot))
            }
            Act::PrevSlot => {
                self.slot = (self.slot + SLOTS - 2) % SLOTS + 1;
                Some(format!("slot {}", self.slot))
            }
            Act::Slot(n) => {
                self.slot = n;
                Some(format!("slot {}", self.slot))
            }
            Act::Reset => {
                self.emulator.reset();
                Some("reset".to_string())
            }
            Act::Pause => {
                self.paused = !self.paused;
                None
            }
            Act::Mute => {
                self.sound_on = !self.sound_on;
                self.set_gain();
                Some(format!(
                    "sound {}",
                    if self.sound_on { "on" } else { "off" }
                ))
            }
            Act::SizeStep(_) | Act::Size(_) => {
                let wanted = match act {
                    Act::Size(n) => clamp_scale(n),
                    Act::SizeStep(d) => clamp_scale((self.factor as i32 + d).max(1) as u8),
                    _ => self.factor,
                };
                if wanted != self.factor || self.view.fullscreen {
                    // a new size means a window again, not the full screen
                    self.view.fullscreen = false;
                    self.settings.fullscreen = false;
                    self.persist(Change::Fullscreen(false));
                    self.resize(wanted)?;
                }
                Some(format!("size {}×", self.factor))
            }
            Act::Crt => {
                self.crt = !self.crt;
                self.settings.crt = self.crt;
                self.persist(Change::Crt(self.crt));
                if self.crt && self.lcd {
                    self.lcd = false;
                    self.settings.lcd = false;
                    self.persist(Change::Lcd(false));
                }
                Some(format!(
                    "CRT effect {}",
                    if self.crt { "on" } else { "off" }
                ))
            }
            Act::Lcd => {
                self.lcd = !self.lcd;
                self.settings.lcd = self.lcd;
                self.persist(Change::Lcd(self.lcd));
                if self.lcd && self.crt {
                    self.crt = false;
                    self.settings.crt = false;
                    self.persist(Change::Crt(false));
                }
                Some(format!(
                    "LCD effect {}",
                    if self.lcd { "on" } else { "off" }
                ))
            }
            Act::Scale2x => {
                self.scale2x = !self.scale2x;
                self.settings.scale2x = self.scale2x;
                self.persist(Change::Scale2x(self.scale2x));
                Some(format!(
                    "Scale2x smoothing {}",
                    if self.scale2x { "on" } else { "off" }
                ))
            }
            Act::FastForwardToggle => {
                self.ff_latched = !self.ff_latched;
                Some(if self.ff_latched {
                    format!("fast-forward {}× on", self.settings.fast_forward)
                } else {
                    "fast-forward off".to_string()
                })
            }
            Act::SlowMotion => {
                self.slow_motion = !self.slow_motion;
                Some(format!(
                    "slow motion {}",
                    if self.slow_motion { "on" } else { "off" }
                ))
            }
            Act::Speed(n) => {
                let n = vuretro_settings::snap_fast_forward(n);
                self.settings.fast_forward = n;
                self.persist(Change::FastForward(n));
                Some(format!("fast-forward speed {n}×"))
            }
            Act::Fullscreen => {
                if !self.screen.can_fullscreen() {
                    Some("full screen is not available on this system yet".to_string())
                } else {
                    self.view.fullscreen = !self.view.fullscreen;
                    self.settings.fullscreen = self.view.fullscreen;
                    self.persist(Change::Fullscreen(self.view.fullscreen));
                    self.resize(self.factor)?;
                    Some(format!(
                        "full screen {}",
                        if self.view.fullscreen { "on" } else { "off" }
                    ))
                }
            }
            Act::NextDisc => Some(
                self.emulator
                    .next_disc()
                    .unwrap_or_else(|| "this core cannot change discs".to_string()),
            ),
            Act::Screenshot => Some(self.screenshot()),
            Act::Quit => {
                self.quit = true;
                None
            }
        })
    }

    /// Output level: muted while paused, in slow motion, and (by choice) while fast-forwarding.
    fn set_gain(&self) {
        let silent = self.holding || speed::silent(self.mode, self.settings.ff_sound);
        self.gain.store(
            gain_for(self.sound_on, self.settings.volume, silent),
            Ordering::Relaxed,
        );
    }

    /// Work out the speed for this pass of the loop and tell the audio about it.
    fn update_speed(&mut self) {
        let mode = speed::mode(self.ff_held || self.ff_latched, self.slow_motion);
        let percent = speed::percent(mode, self.settings.fast_forward);
        self.plan = speed::plan(percent);
        self.speed_percent.store(percent, Ordering::Relaxed);
        if mode != self.mode {
            self.mode = mode;
            self.set_gain();
        }
    }

    fn resize(&mut self, wanted: usize) -> Result<()> {
        self.screen
            .reopen(&self.title, self.base_w, self.base_h, wanted, self.view)?;
        self.view = self.screen.view();
        self.shown_title.clear();
        self.factor = wanted;
        self.settings.window_scale = wanted as u8;
        self.persist(Change::WindowScale(wanted as u8));
        Ok(())
    }

    /// Remember a setting changed by hotkey, in the layer (general, system or game) that
    /// owns it, so it comes back the same way next time.
    fn persist(&self, change: Change) {
        let scope = Scope {
            system: &self.system,
            game: &self.rom_name,
        };
        let mut base = vuretro_settings::load(&self.settings_path);
        let mut overrides = load_overrides(&self.overrides_path);
        overrides.record_change(&mut base, scope, change);
        if let Err(e) = vuretro_settings::save(&self.settings_path, &base) {
            log::warn!("cannot save settings: {e}");
        }
        if let Err(e) = save_overrides(&self.overrides_path, &overrides) {
            log::warn!("cannot save settings: {e}");
        }
    }

    fn screenshot(&self) -> String {
        match &self.last_frame {
            Some((buf, w, h)) => {
                let stamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let path = self
                    .shots_dir
                    .join(format!("{}-{stamp}.png", self.rom_name));
                match cover::write_png(&path, buf, *w, *h) {
                    Ok(_) => format!("screenshot saved ({})", self.shots_dir.display()),
                    Err(e) => format!("screenshot failed: {e}"),
                }
            }
            None => "nothing to capture yet".to_string(),
        }
    }

    /// Pause on request, or when the window goes to the background if that is enabled.
    fn update_hold(&mut self) {
        let hold =
            self.paused || self.asking || (self.settings.pause_on_blur && !self.screen.is_active());
        if hold == self.holding && self.asking == self.dim_asking {
            return;
        }
        let changed = hold != self.holding;
        self.holding = hold;
        self.dim_asking = self.asking;
        self.set_gain();
        let asking = self.asking;
        self.dimmed = self.last_frame.as_ref().map(|(buf, w, h)| {
            if asking {
                quit_overlay(buf, *w, *h)
            } else {
                paused_overlay(buf, *w, *h)
            }
        });
        if changed {
            log::info!("{}", if hold { "paused" } else { "resumed" });
        }
    }

    fn update_title(&mut self) {
        let label = (self.mode != Mode::Normal)
            .then(|| speed::label(self.mode, self.settings.fast_forward));
        let wanted = window_title(
            &self.title,
            self.holding,
            self.slow_fps,
            label.as_deref(),
            &self.notice,
        );
        if wanted != self.shown_title {
            self.screen.set_title(&wanted);
            self.shown_title = wanted;
        }
    }

    fn autosave_sram(&mut self) {
        if self.last_sram_save.elapsed() >= SRAM_AUTOSAVE {
            self.last_sram_save = Instant::now();
            if let Err(e) = self.emulator.save_sram(&self.sram_path) {
                log::warn!("autosave failed: {e}");
            }
        }
    }

    /// Put a picture on the screen: the CRT or LCD look when one is on, enlarged by whole
    /// numbers when the view asks for it.
    fn show(&mut self, buf: &[u32], w: usize, h: usize) -> Result<()> {
        let (out, ow, oh) = self.compose(buf, w, h);
        self.screen.present(&out, ow, oh)
    }

    /// The picture as it is sent to the window.
    fn compose(&self, buf: &[u32], w: usize, h: usize) -> Frame {
        if self.view.integer {
            // The whole factor is always worked out for the picture as the game made it, so
            // turning Scale2x on never makes the picture smaller: it only changes what the
            // enlarged pixels look like.
            let (win_w, win_h) = self.screen.size();
            let k = scale::integer_factor(w, h, win_w, win_h);
            if self.scale2x && k >= 2 {
                let (big, bw, bh) = scale2x::apply(buf, w, h);
                if bw != w {
                    // The smoothed picture is twice the size; bring it to k times the original.
                    // An even k is a whole enlargement of it, an odd k picks the nearest pixel.
                    let (mut out, ow, oh) = if k.is_multiple_of(2) {
                        scale::enlarge(&big, bw, bh, k / 2)
                    } else {
                        scale::resample(&big, bw, bh, w * k, h * k)
                    };
                    // the look is drawn on the final picture, in cells of k x k
                    if self.crt {
                        crt::darken_rows(&mut out, ow, oh, k);
                    } else if self.lcd {
                        lcd::grid_in_place(&mut out, ow, oh, k);
                    }
                    return (out, ow, oh);
                }
            }
            return self.enlarge_with_look(buf, w, h, k);
        }
        // In a window that stretches the picture, Scale2x first, then the look
        if self.scale2x {
            let (big, bw, bh) = scale2x::apply(buf, w, h);
            if bw != w {
                return self.look_without_whole_numbers(&big, bw, bh);
            }
        }
        self.look_without_whole_numbers(buf, w, h)
    }

    /// The CRT or LCD look at a factor chosen by the whole-number scaling, or a plain enlargement.
    fn enlarge_with_look(&self, buf: &[u32], w: usize, h: usize, k: usize) -> Frame {
        if (self.crt || self.lcd) && k >= 2 {
            if self.crt {
                crt::apply_with(buf, w, h, k)
            } else {
                lcd::apply_with(buf, w, h, k)
            }
        } else {
            scale::enlarge(buf, w, h, k)
        }
    }

    /// The CRT or LCD look at the size that suits the picture, for a window that stretches it.
    fn look_without_whole_numbers(&self, buf: &[u32], w: usize, h: usize) -> Frame {
        if self.crt {
            crt::apply(buf, w, h)
        } else if self.lcd {
            lcd::apply(buf, w, h)
        } else {
            (buf.to_vec(), w, h)
        }
    }

    /// Tell the player when the game cannot keep up, and when it can again.
    fn judge_speed(&mut self, work: Duration) {
        let Some(change) = self.perf.record(work) else {
            return;
        };
        let msg = match change {
            Status::Slow { fps } => {
                self.slow_fps = Some(fps);
                format!(
                    "running slowly: about {fps} fps. This computer may be too slow for this game"
                )
            }
            Status::Smooth => {
                self.slow_fps = None;
                "running smoothly again".to_string()
            }
        };
        log::warn!("{msg}");
        self.notice = Some((msg, Instant::now()));
    }

    /// Keep a state every few frames so the game can be rewound.
    fn remember_for_rewind(&mut self) {
        let Some(rewind) = self.rewind.as_mut() else {
            return;
        };
        self.frames_run += 1;
        if self.frames_run.is_multiple_of(REWIND_EVERY) {
            if let Some(state) = self.emulator.save_state() {
                rewind.push(state);
            }
        }
    }

    /// Go back one step: load the newest remembered state and draw one frame of it.
    fn step_back(&mut self) {
        let Some(state) = self.rewind.as_mut().and_then(Rewind::pop) else {
            return; // nothing older is left: the picture stays
        };
        if self.emulator.load_state(&state) {
            self.emulator.run();
        }
        self.notice = Some(("rewinding".to_string(), Instant::now()));
    }

    /// Feed input, run the core (several times when fast-forwarding) and show its frame.
    fn run_core(&mut self) -> Result<()> {
        let started = Instant::now();
        // fast-forward, slow motion and rewind do more or less than one frame of work: not judged
        let mut normal = true;
        self.controller.poll(&*self.screen);
        if self.rewind.is_some() && self.rewind_held {
            self.step_back();
            normal = false;
        } else {
            normal &= self.plan.runs == 1 && self.plan.stretch == 1;
            for _ in 0..self.plan.runs {
                self.emulator.run();
                self.remember_for_rewind();
            }
        }

        match self.frames.take_new() {
            Some((buf, w, h)) if w > 0 && h > 0 => {
                self.show(&buf, w, h)?;
                self.last_frame = Some((buf, w, h));
            }
            _ => self.screen.idle(),
        }
        if normal {
            self.judge_speed(started.elapsed());
        }
        Ok(())
    }

    /// Save the preview image, the save RAM and, when asked, the place to carry on from.
    fn finish(&self) {
        if let Some(path) = &self.cover {
            let mut best = cover::Best::default();
            let (buf, w, h) = self.frames.current();
            best.offer(&buf, w, h);
            if let Err(e) = best.save(path) {
                log::warn!("cannot save cover: {e}");
            }
        }
        match self.emulator.save_sram(&self.sram_path) {
            Ok(0) => {}
            Ok(n) => log::info!("saved {n} bytes of save RAM"),
            Err(e) => log::error!("cannot save save RAM: {e}"),
        }
        if self.settings.resume {
            match self.emulator.save_state() {
                Some(state) => match std::fs::write(self.auto_state_path(), state) {
                    Ok(_) => log::info!("saved the game to carry on from it next time"),
                    Err(e) => log::warn!("cannot save the resume state: {e}"),
                },
                None => log::info!("this core has no save states: cannot resume next time"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::{Hotkeys, Keyboard};
    use std::cell::RefCell;
    use std::collections::{HashSet, VecDeque};
    use std::rc::Rc;

    /// Everything the fakes record, shared with the test.
    #[derive(Default)]
    struct Log {
        disc_swaps: u32,
        /// Keys held down, and keys that count as freshly pressed on the next frame.
        down: HashSet<Key>,
        pressed: HashSet<Key>,
        menu: VecDeque<usize>,
        /// Frames the window will stay open for.
        open_frames: u32,
        active: bool,
        titles: Vec<String>,
        presented: usize,
        /// Size of each picture sent to the window.
        presented_dims: Vec<(usize, usize)>,
        /// The pixels of the last picture sent to the window.
        last_pixels: Vec<u32>,
        idled: usize,
        reopened: Vec<usize>,
        /// The view of each window opened, and the one the window has now.
        reopened_views: Vec<View>,
        view: View,
        win_size: (usize, usize),
        no_fullscreen: bool,
        /// What the fake gamepad asks for.
        pad_acts: Vec<Act>,
        pad_ff: bool,
        pad_rewind: bool,
        ran: usize,
        resets: usize,
        state: Option<Vec<u8>>,
        sram_saves: usize,
        polls: usize,
        new_frame: Option<Frame>,
    }
    type Shared = Rc<RefCell<Log>>;

    struct FakeScreen(Shared);
    impl Keyboard for FakeScreen {
        fn key_down(&self, key: Key) -> bool {
            self.0.borrow().down.contains(&key)
        }
    }
    impl Screen for FakeScreen {
        fn is_open(&self) -> bool {
            let mut l = self.0.borrow_mut();
            if l.open_frames == 0 {
                return false;
            }
            l.open_frames -= 1;
            true
        }
        fn key_pressed(&self, key: Key) -> bool {
            self.0.borrow_mut().pressed.remove(&key)
        }
        fn menu_pressed(&mut self) -> Option<usize> {
            self.0.borrow_mut().menu.pop_front()
        }
        fn is_active(&mut self) -> bool {
            self.0.borrow().active
        }
        fn present(&mut self, pixels: &[u32], w: usize, h: usize) -> Result<()> {
            let mut l = self.0.borrow_mut();
            l.presented += 1;
            l.last_pixels = pixels.to_vec();
            l.presented_dims.push((w, h));
            Ok(())
        }
        fn idle(&mut self) {
            self.0.borrow_mut().idled += 1;
        }
        fn set_title(&mut self, t: &str) {
            self.0.borrow_mut().titles.push(t.to_string());
        }
        fn reopen(&mut self, _: &str, _: usize, _: usize, factor: usize, view: View) -> Result<()> {
            let mut l = self.0.borrow_mut();
            l.reopened.push(factor);
            l.reopened_views.push(view);
            l.view = view;
            Ok(())
        }
        fn size(&self) -> (usize, usize) {
            self.0.borrow().win_size
        }
        fn view(&self) -> View {
            self.0.borrow().view
        }
        fn can_fullscreen(&self) -> bool {
            !self.0.borrow().no_fullscreen
        }
    }

    struct FakeEmu(Shared);
    impl Emulator for FakeEmu {
        fn next_disc(&self) -> Option<String> {
            let mut log = self.0.borrow_mut();
            log.disc_swaps += 1;
            Some(format!("disc {} of 2", log.disc_swaps % 2 + 1))
        }
        fn run(&self) {
            self.0.borrow_mut().ran += 1;
        }
        fn reset(&self) {
            self.0.borrow_mut().resets += 1;
        }
        fn save_state(&self) -> Option<Vec<u8>> {
            Some(vec![7, 7])
        }
        fn load_state(&self, data: &[u8]) -> bool {
            self.0.borrow_mut().state = Some(data.to_vec());
            true
        }
        fn save_sram(&self, _: &std::path::Path) -> Result<usize> {
            self.0.borrow_mut().sram_saves += 1;
            Ok(0)
        }
    }

    struct FakePad(Shared);
    impl Controller for FakePad {
        fn poll(&mut self, _: &dyn Screen) {
            self.0.borrow_mut().polls += 1;
        }
        fn hotkeys(&mut self, _: &dyn Screen) -> Hotkeys {
            let mut l = self.0.borrow_mut();
            Hotkeys {
                acts: std::mem::take(&mut l.pad_acts),
                fast_forward: l.pad_ff,
                rewind: l.pad_rewind,
            }
        }
    }

    struct FakeFrames(Shared);
    impl FrameSource for FakeFrames {
        fn take_new(&mut self) -> Option<Frame> {
            self.0.borrow_mut().new_frame.take()
        }
        fn current(&self) -> Frame {
            (
                // blocks of many colours, like a game screen: worth a cover
                (0..10_000usize)
                    .map(|i| {
                        ((i % 100 / 5 + i / 100 / 5 * 20) as u32).wrapping_mul(2654435761) >> 8
                    })
                    .collect(),
                100,
                100,
            )
        }
    }

    struct Rig {
        log: Shared,
        gain: Arc<AtomicU32>,
        speed: Arc<AtomicU32>,
        dir: tempfile::TempDir,
    }

    fn rig(open_frames: u32) -> (Session, Rig) {
        rig_with(open_frames, GameSettings::default())
    }

    fn rig_with(open_frames: u32, settings: GameSettings) -> (Session, Rig) {
        let dir = tempfile::tempdir().unwrap();
        rig_in(dir, open_frames, settings, View::default())
    }

    fn rig_in(
        dir: tempfile::TempDir,
        open_frames: u32,
        settings: GameSettings,
        view: View,
    ) -> (Session, Rig) {
        let log: Shared = Rc::new(RefCell::new(Log {
            open_frames,
            active: true,
            view,
            win_size: (6, 4),
            ..Default::default()
        }));
        let gain = Arc::new(AtomicU32::new(100));
        let speed = Arc::new(AtomicU32::new(100));
        let session = Session::new(
            Parts {
                screen: Box::new(FakeScreen(log.clone())),
                emulator: Box::new(FakeEmu(log.clone())),
                controller: Box::new(FakePad(log.clone())),
                frames: Box::new(FakeFrames(log.clone())),
                gain: gain.clone(),
                speed: speed.clone(),
                keep_alive: Box::new(()),
            },
            Config {
                rom_name: "game".into(),
                save_dir: dir.path().to_path_buf(),
                sram_path: dir.path().join("game.srm"),
                shots_dir: dir.path().join("shots"),
                settings,
                settings_path: dir.path().join("game.json"),
                overrides_path: dir.path().join("overrides.json"),
                system: "snes".into(),
                cover: None,
                base_w: 2,
                base_h: 2,
                factor: 2,
                fps: 1000.0,
                start_slot: None,
            },
        );
        (
            session,
            Rig {
                log,
                gain,
                speed,
                dir,
            },
        )
    }

    #[test]
    fn runs_the_core_once_per_frame_until_the_window_closes() {
        let (s, r) = rig(5);
        r.log.borrow_mut().new_frame = Some((vec![0; 4], 2, 2));
        s.run().unwrap();
        let l = r.log.borrow();
        assert_eq!(l.ran, 5);
        assert_eq!(l.polls, 5);
        assert_eq!(l.presented, 1); // only one new frame was produced
        assert_eq!(l.idled, 4);
        assert_eq!(l.sram_saves, 1); // saved once on exit
    }

    #[test]
    fn the_quit_menu_ends_the_session_at_once() {
        let (s, r) = rig(100);
        r.log.borrow_mut().menu.push_back(window::act_id(Act::Quit));
        s.run().unwrap();
        // the frame that handled the menu choice still runs; then the loop ends
        assert_eq!(r.log.borrow().ran, 1);
    }

    #[test]
    fn escape_asks_first_and_a_second_escape_quits() {
        let (mut s, r) = rig(100);
        s.frame().unwrap();
        assert_eq!(r.log.borrow().ran, 1);
        // the first Esc stops the game and asks; nothing quits
        r.log.borrow_mut().pressed.insert(Key::Escape);
        s.frame().unwrap();
        assert!(s.asking && !s.quit);
        assert_eq!(
            r.log.borrow().ran,
            1,
            "the game waits while the question is open"
        );
        assert_eq!(r.gain.load(Ordering::Relaxed), 0, "and is muted");
        s.frame().unwrap();
        assert!(!s.quit && r.log.borrow().ran == 1, "it keeps waiting");
        // a second Esc quits
        r.log.borrow_mut().pressed.insert(Key::Escape);
        s.frame().unwrap();
        assert!(s.quit);
    }

    #[test]
    fn enter_space_or_n_goes_on_playing() {
        for key in [Key::Enter, Key::Space, Key::N] {
            let (mut s, r) = rig(100);
            r.log.borrow_mut().pressed.insert(Key::Escape);
            s.frame().unwrap();
            assert!(s.asking);
            r.log.borrow_mut().pressed.insert(key);
            s.frame().unwrap();
            assert!(!s.asking && !s.quit, "{key:?}");
            let before = r.log.borrow().ran;
            s.frame().unwrap();
            assert_eq!(
                r.log.borrow().ran,
                before + 1,
                "playing again after {key:?}"
            );
            assert_eq!(r.gain.load(Ordering::Relaxed), 100, "sound is back");
        }
    }

    #[test]
    fn while_asking_other_hotkeys_wait_and_the_menu_quit_still_works() {
        let (mut s, r) = rig(100);
        r.log.borrow_mut().pressed.insert(Key::Escape);
        s.frame().unwrap();
        // F2 (reset) is ignored while the question is open
        r.log.borrow_mut().pressed.insert(Key::F2);
        s.frame().unwrap();
        assert_eq!(r.log.borrow().resets, 0);
        r.log.borrow_mut().menu.push_back(window::act_id(Act::Quit));
        s.frame().unwrap();
        assert!(s.quit);
    }

    #[test]
    fn the_question_is_drawn_on_the_dimmed_picture() {
        let (w, h) = (320, 224);
        let out = window::quit_overlay(&vec![0x00FF_FFFF; w * h], w, h);
        assert_eq!(out.len(), w * h);
        // the picture is dark, and the text is white on it
        assert_eq!(out[0], 0x003F_3F3F);
        assert!(out.contains(&0x00FF_FFFF), "the text is drawn");
        // a picture too small for the text is just dimmed, never indexed out of range
        let tiny = window::quit_overlay(&[0x00FF_FFFF; 4], 2, 2);
        assert!(tiny.iter().all(|&p| p == 0x003F_3F3F));
    }

    #[test]
    fn fast_forward_runs_the_core_several_times() {
        let (s, r) = rig(2);
        r.log.borrow_mut().down.insert(Key::Tab);
        s.run().unwrap();
        assert_eq!(r.log.borrow().ran, 2 * 4, "4x is the default speed");
    }

    #[test]
    fn pause_stops_the_core_mutes_and_resumes() {
        let (mut s, r) = rig(100);
        r.log.borrow_mut().pressed.insert(Key::P);
        s.frame().unwrap();
        assert_eq!(r.log.borrow().ran, 0, "paused: the core must not run");
        assert_eq!(r.gain.load(Ordering::Relaxed), 0, "paused: muted");
        assert!(r.log.borrow().titles.last().unwrap().contains("paused"));

        r.log.borrow_mut().pressed.insert(Key::P);
        s.frame().unwrap();
        assert_eq!(r.log.borrow().ran, 1);
        assert_eq!(r.gain.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn losing_focus_pauses_only_when_enabled() {
        let (mut s, r) = rig(100);
        r.log.borrow_mut().active = false;
        s.frame().unwrap();
        assert_eq!(r.log.borrow().ran, 1, "off by default");

        s.settings.pause_on_blur = true;
        s.frame().unwrap();
        assert_eq!(r.log.borrow().ran, 1, "now paused in the background");
    }

    #[test]
    fn mute_toggles_gain() {
        let (mut s, r) = rig(100);
        r.log.borrow_mut().pressed.insert(Key::F8);
        s.frame().unwrap();
        assert_eq!(r.gain.load(Ordering::Relaxed), 0);
        r.log.borrow_mut().pressed.insert(Key::F8);
        s.frame().unwrap();
        assert_eq!(r.gain.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn save_and_load_state_use_the_current_slot_file() {
        let (mut s, r) = rig(100);
        r.log.borrow_mut().pressed.insert(Key::F6); // slot 2
        s.frame().unwrap();
        r.log.borrow_mut().pressed.insert(Key::F5); // save
        s.frame().unwrap();
        let file = r.dir.path().join("game.state2");
        assert_eq!(std::fs::read(&file).unwrap(), vec![7, 7]);

        r.log.borrow_mut().pressed.insert(Key::F7);
        s.frame().unwrap();
        assert_eq!(r.log.borrow().state, Some(vec![7, 7]));
    }

    #[test]
    fn saving_also_writes_a_thumbnail_png_for_the_slot() {
        let (mut s, r) = rig(100);
        s.apply(Act::SaveState).unwrap();
        let png = std::fs::read(r.dir.path().join("game.state1.png")).unwrap();
        assert_eq!(&png[1..4], b"PNG");
    }

    #[test]
    fn a_start_slot_is_loaded_before_the_first_frame() {
        let (mut s, r) = rig(1);
        std::fs::write(r.dir.path().join("game.state3"), [9, 9]).unwrap();
        s.start_slot = Some(3);
        s.run().unwrap();
        assert_eq!(r.log.borrow().state, Some(vec![9, 9]));
    }

    #[test]
    fn f11_changes_the_disc_and_says_so() {
        let (mut s, r) = rig(100);
        r.log.borrow_mut().pressed.insert(Key::F11);
        s.frame().unwrap();
        assert_eq!(r.log.borrow().disc_swaps, 1);
        assert_eq!(s.notice.as_ref().unwrap().0, "disc 2 of 2");
    }

    #[test]
    fn a_game_that_cannot_keep_up_is_flagged_in_the_title() {
        let (mut s, _r) = rig(100);
        // an impossible target makes every frame "late"
        s.perf = PerfMonitor::new(Duration::ZERO, 3);
        for _ in 0..4 {
            s.frame().unwrap();
        }
        assert!(s.slow_fps.is_some());
        assert!(s.shown_title.contains("slow: about"), "{}", s.shown_title);
        assert!(s
            .notice
            .as_ref()
            .unwrap()
            .0
            .contains("too slow for this game"));
    }

    #[test]
    fn a_game_on_time_is_not_flagged() {
        let (mut s, _r) = rig(100);
        s.perf = PerfMonitor::new(Duration::from_secs(10), 3);
        for _ in 0..10 {
            s.frame().unwrap();
        }
        assert!(s.slow_fps.is_none());
    }

    #[test]
    fn the_crt_effect_toggles_and_is_remembered() {
        let (mut s, r) = rig(100);
        assert_eq!(s.apply(Act::Crt).unwrap().unwrap(), "CRT effect on");
        assert!(vuretro_settings::load(&r.dir.path().join("game.json")).crt);
        assert_eq!(s.apply(Act::Crt).unwrap().unwrap(), "CRT effect off");
        assert!(!vuretro_settings::load(&r.dir.path().join("game.json")).crt);
    }

    #[test]
    fn holding_backspace_rewinds_through_remembered_states() {
        let (mut s, r) = rig(100);
        s.rewind = Some(Rewind::new(1000));
        for _ in 0..4 {
            s.frame().unwrap();
        }
        assert_eq!(
            s.rewind.as_ref().unwrap().len(),
            2,
            "one state every 2 frames"
        );
        assert_eq!(r.log.borrow().state, None);

        r.log.borrow_mut().down.insert(Key::Backspace);
        s.frame().unwrap();
        assert_eq!(
            r.log.borrow().state,
            Some(vec![7, 7]),
            "an old state was loaded"
        );
        assert_eq!(s.rewind.as_ref().unwrap().len(), 1);
        s.frame().unwrap();
        s.frame().unwrap(); // nothing older left: no crash, no new states
        assert_eq!(s.rewind.as_ref().unwrap().len(), 0);
    }

    #[test]
    fn rewind_is_off_unless_the_setting_is_on() {
        let (mut s, r) = rig(100);
        assert!(s.rewind.is_none());
        r.log.borrow_mut().down.insert(Key::Backspace);
        s.frame().unwrap();
        assert_eq!(r.log.borrow().state, None);
    }

    #[test]
    fn loading_an_empty_slot_reports_it_and_slots_wrap() {
        let (mut s, _r) = rig(100);
        assert_eq!(s.apply(Act::LoadState).unwrap().unwrap(), "slot 1 is empty");
        for _ in 0..SLOTS {
            s.apply(Act::NextSlot).unwrap();
        }
        assert_eq!(s.slot, 1);
    }

    #[test]
    fn resizing_reopens_the_window_clamps_and_remembers() {
        let (mut s, r) = rig(100);
        s.apply(Act::Size(3)).unwrap();
        assert_eq!(r.log.borrow().reopened, vec![3]);
        let saved = vuretro_settings::load(&r.dir.path().join("game.json"));
        assert_eq!(saved.window_scale, 3);

        // same size again: nothing reopens
        s.apply(Act::Size(3)).unwrap();
        assert_eq!(r.log.borrow().reopened.len(), 1);

        // the step buttons never go past the limits
        for _ in 0..20 {
            s.apply(Act::SizeStep(1)).unwrap();
        }
        assert_eq!(s.factor, vuretro_settings::MAX_SCALE as usize);
        for _ in 0..20 {
            s.apply(Act::SizeStep(-1)).unwrap();
        }
        assert_eq!(s.factor, vuretro_settings::MIN_SCALE as usize);
    }

    #[test]
    fn screenshot_needs_a_frame_then_writes_a_png() {
        let (mut s, r) = rig(100);
        assert_eq!(
            s.apply(Act::Screenshot).unwrap().unwrap(),
            "nothing to capture yet"
        );
        r.log.borrow_mut().new_frame = Some((vec![0x00FF_0000; 4], 2, 2));
        s.frame().unwrap();
        assert!(s
            .apply(Act::Screenshot)
            .unwrap()
            .unwrap()
            .starts_with("screenshot saved"));
        let shots: Vec<_> = std::fs::read_dir(r.dir.path().join("shots"))
            .unwrap()
            .collect();
        assert_eq!(shots.len(), 1);
    }

    #[test]
    fn reset_reaches_the_core() {
        let (mut s, r) = rig(100);
        s.apply(Act::Reset).unwrap();
        assert_eq!(r.log.borrow().resets, 1);
    }

    #[test]
    fn closing_saves_a_cover_when_asked() {
        let (mut s, r) = rig(1);
        let cover = r.dir.path().join("cover.png");
        s.cover = Some(cover.clone());
        s.run().unwrap();
        // the fake's frame is colourful, so a preview is written
        assert!(cover.exists());
    }

    // ---------- speed ----------

    fn settings_with(f: impl FnOnce(&mut GameSettings)) -> GameSettings {
        let mut s = GameSettings::default();
        f(&mut s);
        s
    }

    #[test]
    fn the_chosen_fast_forward_speed_sets_how_many_frames_run() {
        for speed in [2u8, 3, 8] {
            let (s, r) = rig_with(2, settings_with(|s| s.fast_forward = speed));
            r.log.borrow_mut().down.insert(Key::Tab);
            s.run().unwrap();
            assert_eq!(r.log.borrow().ran, 2 * speed as usize, "{speed}x");
        }
    }

    #[test]
    fn f1_switches_fast_forward_on_and_off_without_holding_a_key() {
        let (mut s, r) = rig(100);
        r.log.borrow_mut().pressed.insert(Key::F1);
        s.frame().unwrap();
        assert_eq!(r.log.borrow().ran, 4);
        s.frame().unwrap();
        assert_eq!(r.log.borrow().ran, 8, "still on");
        assert_eq!(
            r.speed.load(Ordering::Relaxed),
            400,
            "the audio plays 4x too"
        );
        assert!(r
            .log
            .borrow()
            .titles
            .last()
            .unwrap()
            .contains("fast-forward 4×"));

        r.log.borrow_mut().pressed.insert(Key::F1);
        s.frame().unwrap();
        assert_eq!(r.log.borrow().ran, 9, "back to one frame");
        assert_eq!(r.speed.load(Ordering::Relaxed), 100);
    }

    #[test]
    fn fast_forward_is_silent_unless_the_player_wants_to_hear_it() {
        let (mut s, r) = rig(100);
        r.log.borrow_mut().down.insert(Key::Tab);
        s.frame().unwrap();
        assert_eq!(r.gain.load(Ordering::Relaxed), 0, "muted by default");
        r.log.borrow_mut().down.remove(&Key::Tab);
        s.frame().unwrap();
        assert_eq!(
            r.gain.load(Ordering::Relaxed),
            100,
            "sound is back at normal speed"
        );

        let (mut s, r) = rig_with(100, settings_with(|s| s.ff_sound = true));
        r.log.borrow_mut().down.insert(Key::Tab);
        s.frame().unwrap();
        assert_eq!(r.gain.load(Ordering::Relaxed), 100);
        assert_eq!(r.speed.load(Ordering::Relaxed), 400);
    }

    #[test]
    fn slow_motion_runs_one_frame_per_pass_and_is_always_silent() {
        let (mut s, r) = rig_with(100, settings_with(|s| s.ff_sound = true));
        {
            let mut l = r.log.borrow_mut();
            l.down.insert(Key::LeftShift);
            l.pressed.insert(Key::F1);
        }
        s.frame().unwrap();
        assert_eq!(r.log.borrow().ran, 1);
        assert_eq!(r.speed.load(Ordering::Relaxed), 50);
        assert_eq!(r.gain.load(Ordering::Relaxed), 0);
        assert_eq!(s.plan.stretch, 2, "each pass lasts two normal frames");
        assert!(r
            .log
            .borrow()
            .titles
            .last()
            .unwrap()
            .contains("slow motion"));

        // holding fast-forward wins over slow motion
        r.log.borrow_mut().down.insert(Key::Tab);
        s.frame().unwrap();
        assert_eq!(r.speed.load(Ordering::Relaxed), 400);
    }

    #[test]
    fn picking_a_speed_applies_at_once_and_is_remembered() {
        let (mut s, r) = rig(100);
        assert_eq!(
            s.apply(Act::Speed(2)).unwrap().unwrap(),
            "fast-forward speed 2×"
        );
        r.log.borrow_mut().down.insert(Key::Tab);
        s.frame().unwrap();
        assert_eq!(r.log.borrow().ran, 2);
        let saved = vuretro_settings::load(&r.dir.path().join("game.json"));
        assert_eq!(saved.fast_forward, 2);
        // a speed that is not offered snaps to one that is
        s.apply(Act::Speed(7)).unwrap();
        assert_eq!(s.settings.fast_forward, 4);
    }

    // ---------- carrying on ----------

    #[test]
    fn a_game_with_resume_saves_on_exit_and_carries_on_next_time() {
        let on = settings_with(|s| s.resume = true);
        let (s, r) = rig_with(1, on.clone());
        s.run().unwrap();
        let auto = r.dir.path().join("game.stateauto");
        assert_eq!(std::fs::read(&auto).unwrap(), vec![7, 7]);

        // next time: a new session finds that file and loads it before the first frame
        let (s2, r2) = rig_with(1, on);
        std::fs::write(r2.dir.path().join("game.stateauto"), [9u8, 9]).unwrap();
        s2.run().unwrap();
        assert_eq!(r2.log.borrow().state, Some(vec![9, 9]));
    }

    #[test]
    fn without_resume_nothing_is_saved_or_loaded() {
        let (s, r) = rig(1);
        std::fs::write(r.dir.path().join("game.stateauto"), [9u8, 9]).unwrap();
        s.run().unwrap();
        assert_eq!(r.log.borrow().state, None, "an old auto state is ignored");
        assert_eq!(
            std::fs::read(r.dir.path().join("game.stateauto")).unwrap(),
            vec![9, 9],
            "and left alone"
        );
    }

    #[test]
    fn starting_from_a_chosen_slot_wins_over_carrying_on() {
        let (mut s, r) = rig_with(1, settings_with(|s| s.resume = true));
        std::fs::write(r.dir.path().join("game.stateauto"), [1u8]).unwrap();
        std::fs::write(r.dir.path().join("game.state3"), [3u8]).unwrap();
        s.start_slot = Some(3);
        s.run().unwrap();
        // the slot was loaded first; on exit the resume state is written over the old one
        assert_eq!(
            std::fs::read(r.dir.path().join("game.stateauto")).unwrap(),
            vec![7, 7]
        );
        assert_eq!(r.log.borrow().state, Some(vec![3]));
    }

    // ---------- gamepad hotkeys ----------

    #[test]
    fn gamepad_hotkeys_run_the_same_actions_as_the_keyboard() {
        let (mut s, r) = rig(100);
        r.log.borrow_mut().pad_acts = vec![Act::Reset, Act::NextSlot];
        s.frame().unwrap();
        assert_eq!(r.log.borrow().resets, 1);
        assert_eq!(s.slot, 2);
        s.frame().unwrap();
        assert_eq!(r.log.borrow().resets, 1, "an act is delivered once");

        r.log.borrow_mut().pad_acts = vec![Act::PrevSlot, Act::PrevSlot];
        s.frame().unwrap();
        assert_eq!(s.slot, SLOTS, "backwards past slot 1 wraps to the last");
    }

    #[test]
    fn a_gamepad_chord_can_quit_the_game() {
        let (s, r) = rig(100);
        r.log.borrow_mut().pad_acts = vec![Act::Quit];
        s.run().unwrap();
        assert!(r.log.borrow().ran <= 1);
    }

    #[test]
    fn holding_the_gamepad_fast_forward_chord_speeds_the_game_up() {
        let (mut s, r) = rig(100);
        r.log.borrow_mut().pad_ff = true;
        s.frame().unwrap();
        assert_eq!(r.log.borrow().ran, 4);
        r.log.borrow_mut().pad_ff = false;
        s.frame().unwrap();
        assert_eq!(r.log.borrow().ran, 5);
    }

    #[test]
    fn gamepad_hotkeys_still_work_while_paused() {
        let (mut s, r) = rig(100);
        s.apply(Act::Pause).unwrap();
        s.frame().unwrap();
        assert_eq!(r.log.borrow().ran, 0);
        r.log.borrow_mut().pad_acts = vec![Act::Pause];
        s.frame().unwrap();
        assert_eq!(r.log.borrow().ran, 1, "unpaused from the gamepad");
    }

    #[test]
    fn the_pad_rewind_chord_rewinds_like_backspace() {
        let (mut s, r) = rig_with(100, settings_with(|s| s.rewind = true));
        for _ in 0..6 {
            s.frame().unwrap();
        }
        let ran = r.log.borrow().ran;
        r.log.borrow_mut().pad_rewind = true;
        s.frame().unwrap();
        // stepping back loads a remembered state and draws one frame of it
        assert!(r.log.borrow().state.is_some());
        assert_eq!(r.log.borrow().ran, ran + 1);
    }

    // ---------- picture: full screen, whole-number scaling, looks ----------

    #[test]
    fn alt_enter_toggles_full_screen_and_remembers_it() {
        let (mut s, r) = rig(100);
        {
            let mut l = r.log.borrow_mut();
            l.down.insert(Key::LeftAlt);
            l.pressed.insert(Key::Enter);
        }
        s.frame().unwrap();
        assert!(r.log.borrow().reopened_views.last().unwrap().fullscreen);
        assert!(vuretro_settings::load(&r.dir.path().join("game.json")).fullscreen);

        r.log.borrow_mut().pressed.insert(Key::Enter);
        s.frame().unwrap();
        assert!(!r.log.borrow().reopened_views.last().unwrap().fullscreen);
        assert!(!vuretro_settings::load(&r.dir.path().join("game.json")).fullscreen);
    }

    #[test]
    fn full_screen_is_refused_politely_where_it_cannot_be_made() {
        let (mut s, r) = rig(100);
        r.log.borrow_mut().no_fullscreen = true;
        let msg = s.apply(Act::Fullscreen).unwrap().unwrap();
        assert!(msg.contains("not available"), "{msg}");
        assert!(
            r.log.borrow().reopened.is_empty(),
            "the window is left alone"
        );
    }

    #[test]
    fn choosing_a_window_size_leaves_full_screen() {
        let (mut s, r) = rig(100);
        s.apply(Act::Fullscreen).unwrap();
        s.apply(Act::Size(3)).unwrap();
        let l = r.log.borrow();
        assert!(!l.reopened_views.last().unwrap().fullscreen);
        assert_eq!(*l.reopened.last().unwrap(), 3);
    }

    #[test]
    fn whole_number_scaling_enlarges_the_picture_to_fit_the_window() {
        let view = View {
            integer: true,
            fullscreen: false,
        };
        let (mut s, r) = rig_in(
            tempfile::tempdir().unwrap(),
            100,
            GameSettings::default(),
            view,
        );
        // window 6x4, picture 2x2: the largest whole factor is 2
        r.log.borrow_mut().new_frame = Some((vec![5; 4], 2, 2));
        s.frame().unwrap();
        assert_eq!(*r.log.borrow().presented_dims.last().unwrap(), (4, 4));

        // without the option the picture goes to the window as it is
        let (mut s, r) = rig(100);
        r.log.borrow_mut().new_frame = Some((vec![5; 4], 2, 2));
        s.frame().unwrap();
        assert_eq!(*r.log.borrow().presented_dims.last().unwrap(), (2, 2));
    }

    #[test]
    fn whole_number_scaling_with_a_look_draws_the_look_at_that_factor() {
        let view = View {
            integer: true,
            fullscreen: false,
        };
        let (mut s, r) = rig_in(
            tempfile::tempdir().unwrap(),
            100,
            settings_with(|s| s.lcd = true),
            view,
        );
        r.log.borrow_mut().new_frame = Some((vec![0x00FF_FFFF; 4], 2, 2));
        s.frame().unwrap();
        assert_eq!(*r.log.borrow().presented_dims.last().unwrap(), (4, 4));
    }

    /// A session with whole-number scaling in a window of `win` pixels.
    fn integer_rig(win: (usize, usize), f: impl FnOnce(&mut GameSettings)) -> (Session, Rig) {
        let view = View {
            integer: true,
            fullscreen: false,
        };
        let (s, r) = rig_in(tempfile::tempdir().unwrap(), 100, settings_with(f), view);
        r.log.borrow_mut().win_size = win;
        (s, r)
    }

    fn presented(s: &mut Session, r: &Rig, pic: (Vec<u32>, usize, usize)) -> (usize, usize) {
        r.log.borrow_mut().new_frame = Some(pic);
        s.frame().unwrap();
        *r.log.borrow().presented_dims.last().unwrap()
    }

    #[test]
    fn smoothing_never_makes_the_picture_smaller_whatever_the_whole_factor() {
        let pic = || (vec![5u32; 3 * 2], 3, 2);
        // window 9x6 fits the 3x2 picture 3 times: an odd factor (the bug: it came out 2 times)
        let (mut plain, r) = integer_rig((9, 6), |_| {});
        let plain_dims = presented(&mut plain, &r, pic());
        let (mut smooth, r) = integer_rig((9, 6), |s| s.scale2x = true);
        assert_eq!(presented(&mut smooth, &r, pic()), plain_dims);
        assert_eq!(plain_dims, (9, 6));
        // an even factor, and a bigger odd one
        for win in [(12, 8), (15, 10), (6, 4)] {
            let (mut p, r) = integer_rig(win, |_| {});
            let want = presented(&mut p, &r, pic());
            let (mut s, r) = integer_rig(win, |s| s.scale2x = true);
            assert_eq!(presented(&mut s, &r, pic()), want, "window {win:?}");
        }
        // a window with room for the picture only once cannot show a doubled one: as before
        let (mut s, r) = integer_rig((3, 2), |s| s.scale2x = true);
        assert_eq!(presented(&mut s, &r, pic()), (3, 2));
    }

    #[test]
    fn the_looks_keep_their_cells_on_a_smoothed_picture_at_an_odd_factor() {
        // 2x2 white picture, window 6x6: factor 3, cells of 3x3 with the last row dimmed
        let white = || (vec![0x00FF_FFFFu32; 4], 2, 2);
        let (mut s, r) = integer_rig((6, 6), |s| {
            s.scale2x = true;
            s.crt = true;
        });
        assert_eq!(presented(&mut s, &r, white()), (6, 6));
        let f = r.log.borrow().last_pixels.clone();
        let row = |y: usize| f[y * 6];
        assert_eq!((row(0), row(1)), (0x00FF_FFFF, 0x00FF_FFFF));
        assert!(
            row(2) < 0x00FF_FFFF,
            "the last row of each cell is the scanline gap"
        );
        assert_eq!(row(5), row(2));
    }

    #[test]
    fn crt_and_lcd_looks_exclude_each_other_and_are_remembered() {
        let (mut s, r) = rig(100);
        s.apply(Act::Lcd).unwrap();
        assert!(s.lcd && !s.crt);
        s.apply(Act::Crt).unwrap();
        assert!(s.crt && !s.lcd, "turning CRT on turns LCD off");
        let saved = vuretro_settings::load(&r.dir.path().join("game.json"));
        assert!(saved.crt && !saved.lcd);
        s.apply(Act::Lcd).unwrap();
        assert!(s.lcd && !s.crt);
    }

    #[test]
    fn shift_f4_is_the_lcd_look_and_f4_the_crt_look() {
        let (mut s, r) = rig(100);
        r.log.borrow_mut().pressed.insert(Key::F4);
        s.frame().unwrap();
        assert!(s.crt);
        {
            let mut l = r.log.borrow_mut();
            l.down.insert(Key::RightShift);
            l.pressed.insert(Key::F4);
        }
        s.frame().unwrap();
        assert!(s.lcd && !s.crt);
    }

    #[test]
    fn shift_f9_toggles_scale2x_and_plain_f9_still_shrinks_the_window() {
        let (mut s, r) = rig(100);
        let before = s.factor;
        {
            let mut l = r.log.borrow_mut();
            l.down.insert(Key::LeftShift);
            l.pressed.insert(Key::F9);
        }
        s.frame().unwrap();
        assert!(s.scale2x, "Shift+F9 turns it on");
        assert_eq!(s.factor, before, "and does not touch the window size");
        assert!(vuretro_settings::load(&r.dir.path().join("game.json")).scale2x);
        assert!(s
            .notice
            .as_ref()
            .unwrap()
            .0
            .contains("Scale2x smoothing on"));

        {
            let mut l = r.log.borrow_mut();
            l.down.clear();
            l.pressed.insert(Key::F9);
        }
        s.frame().unwrap();
        assert!(s.scale2x, "plain F9 leaves it alone");
        assert_eq!(s.factor, before - 1, "plain F9 makes the window smaller");
    }

    #[test]
    fn scale2x_doubles_the_picture_before_the_looks_and_off_leaves_it_as_it_is() {
        let (mut s, _r) = rig(100);
        let pic = vec![0x00ff_0000u32; 6 * 4];
        assert_eq!(s.compose(&pic, 6, 4).1, 6);
        s.scale2x = true;
        let (out, w, h) = s.compose(&pic, 6, 4);
        assert_eq!((w, h, out.len()), (12, 8, 96));
        // a look on top works on the doubled picture, so it is bigger again
        s.crt = true;
        let (_, cw, ch) = s.compose(&pic, 6, 4);
        assert!(cw >= 12 && ch >= 8, "{cw}x{ch}");
    }
}
