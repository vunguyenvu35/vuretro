//! An interactive game session: the window, the frame loop and the player's actions.
//! Built from ports (see `ports.rs`), so it runs against fakes in tests; `real::open`
//! builds the real one. [`Session::run`] plays until the window closes.

use crate::cover;
use crate::crt;
use crate::ports::{Controller, Emulator, Frame, FrameSource, Screen};
use crate::rewind::Rewind;
use crate::window::{self, clamp_scale, gain_for, pace, paused_overlay, window_title, Act, SLOTS};
use anyhow::Result;
use minifb::Key;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use vuretro_settings::GameSettings;

/// Rewind keeps a state every this many frames, up to this much memory.
const REWIND_EVERY: u64 = 2;
const REWIND_BYTES: usize = 64 * 1024 * 1024;
const FAST_FORWARD_FRAMES: usize = 4;
const SRAM_AUTOSAVE: Duration = Duration::from_secs(30);

/// What a session needs, built either from the real machine (`real::open`) or from fakes.
pub struct Parts {
    pub screen: Box<dyn Screen>,
    pub emulator: Box<dyn Emulator>,
    pub controller: Box<dyn Controller>,
    pub frames: Box<dyn FrameSource>,
    /// Output level in percent, read by the audio thread.
    pub gain: Arc<AtomicU32>,
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

pub struct Session {
    screen: Box<dyn Screen>,
    emulator: Box<dyn Emulator>,
    controller: Box<dyn Controller>,
    frames: Box<dyn FrameSource>,
    gain: Arc<AtomicU32>,
    _keep_alive: Box<dyn std::any::Any>,
    settings: GameSettings,
    settings_path: PathBuf,
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

    frame_time: Duration,
    deadline: Instant,
    last_sram_save: Instant,

    rewind: Option<Rewind>,
    frames_run: u64,
    crt: bool,
    slot: u8,
    start_slot: Option<u8>,
    sound_on: bool,
    /// Pause: F3, or the window losing focus when that is switched on in the settings.
    paused: bool,
    /// Paused or in the background: emulation is stopped.
    holding: bool,
    last_frame: Option<Frame>,
    dimmed: Option<Vec<u32>>,
    notice: Option<(String, Instant)>,
    quit: bool,
}

impl Session {
    pub fn new(parts: Parts, cfg: Config) -> Self {
        let title = format!("vuretro — {}", cfg.rom_name);
        let crt = cfg.settings.crt;
        let rewind = cfg.settings.rewind.then(|| Rewind::new(REWIND_BYTES));
        Session {
            screen: parts.screen,
            emulator: parts.emulator,
            controller: parts.controller,
            frames: parts.frames,
            gain: parts.gain,
            _keep_alive: parts.keep_alive,
            sound_on: cfg.settings.sound,
            settings: cfg.settings,
            settings_path: cfg.settings_path,
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
            // Frame pacing is done against absolute deadlines: minifb's own limiter
            // sleeps a fixed amount per frame, so it drifts (57 fps instead of 59.18)
            // and the audio then underruns.
            frame_time: Duration::from_secs_f64(1.0 / cfg.fps),
            deadline: Instant::now(),
            last_sram_save: Instant::now(),
            rewind,
            frames_run: 0,
            crt,
            slot: 1,
            start_slot: cfg.start_slot.filter(|n| (1..=SLOTS).contains(n)),
            paused: false,
            holding: false,
            last_frame: None,
            dimmed: None,
            notice: None,
            quit: false,
        }
    }

    /// Play until the window closes, then save.
    pub fn run(mut self) -> Result<()> {
        if let Some(n) = self.start_slot.take() {
            self.slot = n;
            let note = self.apply(Act::LoadState)?;
            self.notice = note.map(|t| (t, Instant::now()));
        }
        while self.screen.is_open() && !self.quit && !self.screen.key_down(Key::Escape) {
            self.frame()?;
        }
        self.finish();
        Ok(())
    }

    /// One pass of the loop: handle input, then either show the paused picture or run the core.
    fn frame(&mut self) -> Result<()> {
        for act in self.pending_actions() {
            if let Some(msg) = self.apply(act)? {
                log::info!("{msg}");
                self.notice = Some((msg, Instant::now()));
            }
        }
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
        } else {
            self.autosave_sram();
            self.run_core()?;
        }
        pace(&mut self.deadline, self.frame_time);
        Ok(())
    }

    /// Hotkeys and menu choices end up as the same list of actions.
    fn pending_actions(&mut self) -> Vec<Act> {
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
            (Key::F4, Act::Crt),
        ]
        .into_iter()
        .filter(|(k, _)| self.screen.key_pressed(*k))
        .map(|(_, a)| a)
        .collect();
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
            Act::LoadState => Some(match std::fs::read(self.state_path(self.slot)) {
                Ok(data) if self.emulator.load_state(&data) => format!("loaded slot {}", self.slot),
                Ok(_) => format!("slot {} was rejected by the core", self.slot),
                Err(_) => format!("slot {} is empty", self.slot),
            }),
            Act::NextSlot => {
                self.slot = self.slot % SLOTS + 1;
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
                if !self.holding {
                    self.set_gain();
                }
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
                if wanted != self.factor {
                    self.resize(wanted)?;
                }
                Some(format!("size {}×", self.factor))
            }
            Act::Crt => {
                self.crt = !self.crt;
                self.settings.crt = self.crt;
                if let Err(e) = vuretro_settings::save(&self.settings_path, &self.settings) {
                    log::warn!("cannot save the CRT setting: {e}");
                }
                Some(format!(
                    "CRT effect {}",
                    if self.crt { "on" } else { "off" }
                ))
            }
            Act::Screenshot => Some(self.screenshot()),
            Act::Quit => {
                self.quit = true;
                None
            }
        })
    }

    fn set_gain(&self) {
        self.gain.store(
            gain_for(self.sound_on, self.settings.volume, self.holding),
            Ordering::Relaxed,
        );
    }

    fn resize(&mut self, wanted: usize) -> Result<()> {
        self.screen
            .reopen(&self.title, self.base_w, self.base_h, wanted)?;
        self.shown_title.clear();
        self.factor = wanted;
        self.settings.window_scale = wanted as u8;
        if let Err(e) = vuretro_settings::save(&self.settings_path, &self.settings) {
            log::warn!("cannot save window size: {e}");
        }
        Ok(())
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
        let hold = self.paused || (self.settings.pause_on_blur && !self.screen.is_active());
        if hold == self.holding {
            return;
        }
        self.holding = hold;
        self.set_gain();
        self.dimmed = self
            .last_frame
            .as_ref()
            .map(|(buf, w, h)| paused_overlay(buf, *w, *h));
        log::info!("{}", if hold { "paused" } else { "resumed" });
    }

    fn update_title(&mut self) {
        let wanted = window_title(&self.title, self.holding, &self.notice);
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

    /// Put a picture on the screen, with the CRT look when it is on.
    fn show(&mut self, buf: &[u32], w: usize, h: usize) -> Result<()> {
        if self.crt {
            let (out, ow, oh) = crt::apply(buf, w, h);
            self.screen.present(&out, ow, oh)
        } else {
            self.screen.present(buf, w, h)
        }
    }

    /// Keep a state every few frames so the game can be rewound.
    fn remember_for_rewind(&mut self) {
        let Some(rewind) = self.rewind.as_mut() else {
            return;
        };
        self.frames_run += 1;
        if self.frames_run % REWIND_EVERY == 0 {
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
        self.controller.poll(&*self.screen);
        if self.rewind.is_some() && self.screen.key_down(Key::Backspace) {
            self.step_back();
        } else {
            let runs = if self.screen.key_down(Key::Tab) {
                FAST_FORWARD_FRAMES
            } else {
                1
            };
            for _ in 0..runs {
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
        Ok(())
    }

    /// Save the preview image and the save RAM.
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::Keyboard;
    use std::cell::RefCell;
    use std::collections::{HashSet, VecDeque};
    use std::rc::Rc;

    /// Everything the fakes record, shared with the test.
    #[derive(Default)]
    struct Log {
        /// Keys held down, and keys that count as freshly pressed on the next frame.
        down: HashSet<Key>,
        pressed: HashSet<Key>,
        menu: VecDeque<usize>,
        /// Frames the window will stay open for.
        open_frames: u32,
        active: bool,
        titles: Vec<String>,
        presented: usize,
        idled: usize,
        reopened: Vec<usize>,
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
        fn present(&mut self, _: &[u32], _: usize, _: usize) -> Result<()> {
            self.0.borrow_mut().presented += 1;
            Ok(())
        }
        fn idle(&mut self) {
            self.0.borrow_mut().idled += 1;
        }
        fn set_title(&mut self, t: &str) {
            self.0.borrow_mut().titles.push(t.to_string());
        }
        fn reopen(&mut self, _: &str, _: usize, _: usize, factor: usize) -> Result<()> {
            self.0.borrow_mut().reopened.push(factor);
            Ok(())
        }
    }

    struct FakeEmu(Shared);
    impl Emulator for FakeEmu {
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
    }

    struct FakeFrames(Shared);
    impl FrameSource for FakeFrames {
        fn take_new(&mut self) -> Option<Frame> {
            self.0.borrow_mut().new_frame.take()
        }
        fn current(&self) -> Frame {
            ((0..10_000u32).collect(), 100, 100) // many colours: worth a cover
        }
    }

    struct Rig {
        log: Shared,
        gain: Arc<AtomicU32>,
        dir: tempfile::TempDir,
    }

    fn rig(open_frames: u32) -> (Session, Rig) {
        let dir = tempfile::tempdir().unwrap();
        let log: Shared = Rc::new(RefCell::new(Log {
            open_frames,
            active: true,
            ..Default::default()
        }));
        let gain = Arc::new(AtomicU32::new(100));
        let session = Session::new(
            Parts {
                screen: Box::new(FakeScreen(log.clone())),
                emulator: Box::new(FakeEmu(log.clone())),
                controller: Box::new(FakePad(log.clone())),
                frames: Box::new(FakeFrames(log.clone())),
                gain: gain.clone(),
                keep_alive: Box::new(()),
            },
            Config {
                rom_name: "game".into(),
                save_dir: dir.path().to_path_buf(),
                sram_path: dir.path().join("game.srm"),
                shots_dir: dir.path().join("shots"),
                settings: GameSettings::default(),
                settings_path: dir.path().join("game.json"),
                cover: None,
                base_w: 2,
                base_h: 2,
                factor: 2,
                fps: 1000.0,
                start_slot: None,
            },
        );
        (session, Rig { log, gain, dir })
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
    fn escape_or_the_quit_menu_ends_the_session() {
        let (s, r) = rig(100);
        r.log.borrow_mut().down.insert(Key::Escape);
        s.run().unwrap();
        assert_eq!(r.log.borrow().ran, 0);

        let (s, r) = rig(100);
        r.log.borrow_mut().menu.push_back(window::act_id(Act::Quit));
        s.run().unwrap();
        // the frame that handled the menu choice still runs; then the loop ends
        assert_eq!(r.log.borrow().ran, 1);
    }

    #[test]
    fn fast_forward_runs_the_core_several_times() {
        let (s, r) = rig(2);
        r.log.borrow_mut().down.insert(Key::Tab);
        s.run().unwrap();
        assert_eq!(r.log.borrow().ran, 2 * FAST_FORWARD_FRAMES);
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
}
