//! Use cases of the app. `App` owns the in-memory state and talks to the outside world
//! only through the ports in `ports`, so everything here runs in tests with fakes.

use crate::detect;
use crate::model::{game_views, GameMeta, Outcome, Overview, Settings, SystemView};
use crate::paths::Paths;
use crate::ports::{Catalog, CoverJob, GameProcess, LaunchSpec, Launcher, Locator, Store};
use crate::registry::systems;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use vuretro_controls::{macros::MacroConfig, ControlsConfig};
use vuretro_settings::GameSettings;

/// How long a freshly started game is watched for an immediate failure.
const LAUNCH_GRACE: Duration = Duration::from_millis(1500);
const LAUNCH_POLL: Duration = Duration::from_millis(50);

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A game that is running; hand it back to [`App::wait_for_exit`] on a thread.
pub struct Running {
    process: Box<dyn GameProcess>,
    track: Option<String>,
    started: Instant,
}

pub struct App {
    paths: Paths,
    store: Box<dyn Store>,
    catalog: Box<dyn Catalog>,
    locator: Box<dyn Locator>,
    launcher: Box<dyn Launcher>,
    launch_grace: Duration,
    settings: Mutex<Settings>,
    meta: Mutex<HashMap<String, GameMeta>>,
    /// A background run that makes preview images is going.
    covering: AtomicBool,
    /// Games a preview could not be made for (not retried until the app restarts).
    cover_failed: Mutex<HashSet<String>>,
}

impl App {
    pub fn new(
        paths: Paths,
        store: Box<dyn Store>,
        catalog: Box<dyn Catalog>,
        locator: Box<dyn Locator>,
        launcher: Box<dyn Launcher>,
    ) -> Self {
        let settings = Mutex::new(store.load_settings());
        let meta = Mutex::new(store.load_meta());
        App {
            paths,
            store,
            catalog,
            locator,
            launcher,
            launch_grace: LAUNCH_GRACE,
            settings,
            meta,
            covering: AtomicBool::new(false),
            cover_failed: Mutex::new(HashSet::new()),
        }
    }

    /// Change how long a new game is watched for an immediate failure (tests).
    pub fn with_launch_grace(mut self, grace: Duration) -> Self {
        self.launch_grace = grace;
        self
    }

    pub fn paths(&self) -> &Paths {
        &self.paths
    }

    fn save_settings(&self) {
        self.store.save_settings(&self.settings.lock().unwrap());
    }

    fn save_meta(&self) {
        self.store.save_meta(&self.meta.lock().unwrap());
    }

    // ---------- library overview ----------

    /// Everything the library screen shows. Scans the disk, so it can take a moment.
    pub fn overview(&self) -> Overview {
        let settings = self.settings.lock().unwrap().clone();
        let meta = self.meta.lock().unwrap().clone();
        let system_dir = self.paths.system_dir();
        let roms_dir = self.paths.roms_dir();

        // The app's own library plus, if the user still links one, that folder read in place.
        let mut scanned = self.catalog.scan(&roms_dir);
        if let Some(dir) = settings.rom_dir.as_deref() {
            for g in self.catalog.scan(Path::new(dir)) {
                if !scanned.iter().any(|x| x.path == g.path) {
                    scanned.push(g);
                }
            }
        }

        let systems: Vec<SystemView> = systems()
            .iter()
            .filter(|s| s.enabled)
            .map(|s| {
                SystemView::new(
                    s,
                    &settings,
                    self.locator.core(s.core).is_some(),
                    self.catalog.bios_status(s),
                    scanned.iter().filter(|g| g.system == s.id).count(),
                )
            })
            .collect();
        let games = game_views(scanned, &systems, &meta, &roms_dir);

        Overview {
            data_dir: self.paths.data_dir().to_string_lossy().into_owned(),
            system_dir: system_dir.to_string_lossy().into_owned(),
            rom_dir: settings.rom_dir,
            systems,
            games,
            runner_found: self.locator.runner().is_some(),
            demo_available: self.locator.test_core().is_some(),
        }
    }

    // ---------- settings and favorites ----------

    pub fn set_rom_dir(&self, path: Option<String>) {
        self.settings.lock().unwrap().rom_dir = path;
        self.save_settings();
    }

    pub fn set_system_enabled(&self, id: String, enabled: bool) {
        {
            let mut s = self.settings.lock().unwrap();
            s.disabled.retain(|d| *d != id);
            if !enabled {
                s.disabled.push(id);
            }
        }
        self.save_settings();
    }

    /// Flip the favorite flag; returns the new value.
    pub fn toggle_favorite(&self, path: String) -> bool {
        let fav = {
            let mut meta = self.meta.lock().unwrap();
            let m = meta.entry(path).or_default();
            m.favorite = !m.favorite;
            m.favorite
        };
        self.save_meta();
        fav
    }

    // ---------- controls, macros and game settings ----------
    // Saved as given after `sanitized()`; the sanitized value is returned so the UI shows
    // what was really stored.

    pub fn controls(&self) -> ControlsConfig {
        self.store.load_controls()
    }

    pub fn default_controls(&self) -> ControlsConfig {
        ControlsConfig::default()
    }

    pub fn set_controls(&self, config: ControlsConfig) -> Result<ControlsConfig, String> {
        let config = config.sanitized();
        self.store.save_controls(&config)?;
        Ok(config)
    }

    pub fn macros(&self) -> MacroConfig {
        self.store.load_macros()
    }

    pub fn set_macros(&self, config: MacroConfig) -> Result<MacroConfig, String> {
        let config = config.sanitized();
        self.store.save_macros(&config)?;
        Ok(config)
    }

    pub fn game_settings(&self) -> GameSettings {
        self.store.load_game_settings()
    }

    pub fn set_game_settings(&self, settings: GameSettings) -> Result<GameSettings, String> {
        let settings = settings.sanitized();
        self.store.save_game_settings(&settings)?;
        Ok(settings)
    }

    // ---------- importing and removing ----------

    pub fn import_bios(&self, paths: Vec<String>) -> Vec<Outcome> {
        paths
            .into_iter()
            .map(|p| match self.catalog.import_bios(Path::new(&p)) {
                Ok(r) => Outcome::ok(p, format!("Installed {}", r.file)),
                Err(message) => Outcome::failed(p, message),
            })
            .collect()
    }

    /// Copy ROM files, or every ROM found inside the folders given, into the app's library.
    /// `system` is the system chosen in the sidebar; it settles archives (.zip) whose
    /// system cannot be told from the file.
    pub fn import_roms(&self, paths: Vec<String>, system: Option<String>) -> Vec<Outcome> {
        let describe = |r: Result<crate::model::ImportedRom, String>, source: String| match r {
            Ok(i) if i.already_there => Outcome::ok(source, "already in the library"),
            Ok(i) => Outcome::ok(source, format!("added to {}", i.system)),
            Err(message) => Outcome::failed(source, message),
        };
        let mut out = Vec::new();
        for p in paths {
            let path = PathBuf::from(&p);
            if self.catalog.is_folder(&path) {
                let results = self.catalog.import_folder(&path);
                if results.is_empty() {
                    out.push(Outcome::failed(p, "no games it recognizes in that folder"));
                }
                for (src, r) in results {
                    out.push(describe(r, src.to_string_lossy().into_owned()));
                }
            } else {
                out.push(describe(
                    self.catalog.import_rom(&path, system.as_deref()),
                    p,
                ));
            }
        }
        out
    }

    /// Delete an imported ROM together with its bookkeeping and preview.
    fn remove_one(&self, path: &str) -> Result<(), String> {
        self.catalog.remove_rom(Path::new(path))?;
        self.meta.lock().unwrap().remove(path);
        self.store.remove_cover(Path::new(path));
        Ok(())
    }

    pub fn remove_rom(&self, path: &str) -> Result<(), String> {
        let r = self.remove_one(path);
        self.save_meta();
        r
    }

    /// Remove several imported ROMs at once; one result per game.
    pub fn remove_roms(&self, paths: Vec<String>) -> Vec<Outcome> {
        let out = paths
            .into_iter()
            .map(|p| match self.remove_one(&p) {
                Ok(()) => Outcome::ok(p, "removed"),
                Err(message) => Outcome::failed(p, message),
            })
            .collect();
        self.save_meta();
        out
    }

    // ---------- preview images ----------

    /// Preview images that exist, as raw PNG bytes keyed by game path.
    pub fn covers(&self, paths: Vec<String>) -> HashMap<String, Vec<u8>> {
        paths
            .into_iter()
            .filter_map(|p| {
                let bytes = self.store.read_cover(Path::new(&p))?;
                Some((p, bytes))
            })
            .collect()
    }

    /// Decide which games still need a preview. Returns `None` when a run is already going
    /// or there is nothing to do; otherwise the caller must pass the jobs to [`App::run_covers`].
    pub fn start_covers(&self, games: Vec<(String, String)>) -> Option<Vec<CoverJob>> {
        if self.covering.swap(true, Ordering::SeqCst) {
            return None; // already running; the page asks again when it finishes
        }
        let jobs = self.plan_covers(games);
        if jobs.is_empty() {
            self.covering.store(false, Ordering::SeqCst);
            return None; // nothing to do, and no event: the page would only ask again
        }
        Some(jobs)
    }

    /// `games` are `(path, system id)` pairs.
    fn plan_covers(&self, games: Vec<(String, String)>) -> Vec<CoverJob> {
        let Some(runner) = self.locator.runner() else {
            return Vec::new();
        };
        let mut jobs = Vec::new();
        for (path, system) in games {
            let rom = PathBuf::from(&path);
            let failed = self.cover_failed.lock().unwrap().contains(&path);
            let core = systems()
                .iter()
                .find(|s| s.id == system)
                .and_then(|s| self.locator.core(s.core));
            if let (false, false, Some(core)) = (failed, self.store.cover_exists(&rom), core) {
                jobs.push(CoverJob {
                    key: path,
                    out: self.paths.cover_file(&rom),
                    runner: runner.clone(),
                    core,
                    rom,
                    system_dir: self.paths.system_dir(),
                    scratch_dir: self.paths.cover_scratch_dir(),
                });
            }
        }
        jobs
    }

    /// Make the previews (slow: run it on a thread). `on_made` gets the key of each game
    /// whose image now exists.
    pub fn run_covers(&self, jobs: Vec<CoverJob>, on_made: impl Fn(&str)) {
        for job in jobs {
            if self.launcher.capture_cover(&job) {
                on_made(&job.key);
            } else {
                self.cover_failed.lock().unwrap().insert(job.key);
            }
        }
        self.store.clear_cover_scratch();
        self.covering.store(false, Ordering::SeqCst);
    }

    // ---------- launching ----------

    /// Start a game from the library. `Ok(None)` means it already finished successfully.
    pub fn launch_game(&self, path: String) -> Result<Option<Running>, String> {
        let rom_dir = self
            .settings
            .lock()
            .unwrap()
            .rom_dir
            .clone()
            .unwrap_or_default();
        let rom = PathBuf::from(&path);
        let sys = detect::detect_system(Path::new(&rom_dir), &rom)
            .ok_or("unknown system for this file")?;
        if !crate::model::bios_ready(&self.catalog.bios_status(sys)) {
            return Err("BIOS missing: add it in Cores & BIOS".into());
        }
        let core = self
            .locator
            .core(sys.core)
            .ok_or_else(|| format!("core {} not installed (run: make cores)", sys.core))?;
        self.start(&core, &rom, Some(path))
    }

    /// Run the fake test core so the app can be tried without any ROM.
    pub fn launch_demo(&self) -> Result<Option<Running>, String> {
        let core = self
            .locator
            .test_core()
            .ok_or("test core not built. Run `make build` first.")?;
        let rom = self.store.ensure_demo_rom()?;
        self.start(&core, &rom, None)
    }

    fn start(
        &self,
        core: &Path,
        rom: &Path,
        track: Option<String>,
    ) -> Result<Option<Running>, String> {
        let runner = self
            .locator
            .runner()
            .ok_or("vuretro-runner not found. Run `make build` first.")?;
        let mut process = self.launcher.spawn(&LaunchSpec {
            runner,
            core: core.to_path_buf(),
            rom: rom.to_path_buf(),
            system_dir: self.paths.system_dir(),
            cover: self.paths.cover_file(rom),
        })?;

        // Failing fast (missing BIOS, bad file) should be reported, not silently ignored.
        let started = Instant::now();
        loop {
            if let Some(exit) = process.poll()? {
                return if exit.success {
                    Ok(None)
                } else {
                    Err(exit.detail)
                };
            }
            if started.elapsed() >= self.launch_grace {
                break;
            }
            std::thread::sleep(LAUNCH_POLL.min(self.launch_grace));
        }

        if let Some(key) = &track {
            self.meta
                .lock()
                .unwrap()
                .entry(key.clone())
                .or_default()
                .last_played = now();
        }
        self.save_meta();
        Ok(Some(Running {
            process,
            track,
            started,
        }))
    }

    /// Block until the game ends, then add its play time to the library.
    pub fn wait_for_exit(&self, mut running: Running) {
        running.process.wait();
        if let Some(key) = running.track {
            self.meta.lock().unwrap().entry(key).or_default().play_secs +=
                running.started.elapsed().as_secs();
            self.save_meta();
        }
    }
}
