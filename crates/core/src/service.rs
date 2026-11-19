//! Use cases of the app. `App` owns the in-memory state and talks to the outside world
//! only through the ports in `ports`, so everything here runs in tests with fakes.

use crate::backup::{self, BackupManifest, BackupOptions, BackupSummary};
use crate::detect;
use crate::model::{
    assess, CoreChoice, CoreOptionView, MachineInfo, ScopedControls, ScopedSettings, SlotInfo,
};
use crate::model::{game_views, CompiledMove, GameMeta, Outcome, Overview, Settings, SystemView};
use crate::paths::Paths;
use crate::ports::{
    Archive, ArtQuery, ArtSource, Catalog, CoreInstaller, CoverJob, GameProcess, LaunchSpec,
    Launcher, Locator, Machine, Store,
};
use crate::registry::{systems, System};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use vuretro_controls::{macros::MacroConfig, ControlsConfig};
use vuretro_settings::core_options::is_storable;
use vuretro_settings::GameSettings;

/// How long a freshly started game is watched for an immediate failure.
const LAUNCH_GRACE: Duration = Duration::from_millis(1500);
/// Whether the runner can draw 3D (GPU) systems. Switch on when it can.
const HARDWARE_RENDERER: bool = false;
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

/// Previews waiting to be made, and whether a worker is making them.
#[derive(Default)]
struct CoverQueue {
    pending: std::collections::VecDeque<CoverJob>,
    running: bool,
}

pub struct App {
    paths: Paths,
    store: Box<dyn Store>,
    catalog: Box<dyn Catalog>,
    locator: Box<dyn Locator>,
    launcher: Box<dyn Launcher>,
    installer: Option<Box<dyn CoreInstaller>>,
    archive: Option<Box<dyn Archive>>,
    /// What this computer offers, measured once at start-up.
    machine: Option<MachineInfo>,
    art: Vec<Box<dyn ArtSource>>,
    /// Games the art server had nothing for (not asked again until restart).
    art_missing: Mutex<HashSet<String>>,
    /// Cores being downloaded right now.
    installing: Mutex<HashSet<String>>,
    /// The game that is open now (its name): only one can be at a time.
    playing: Mutex<Option<String>>,
    launch_grace: Duration,
    settings: Mutex<Settings>,
    meta: Mutex<HashMap<String, GameMeta>>,
    /// A background run that makes preview images is going.
    covers: Mutex<CoverQueue>,
    /// Games a preview could not be made for (not retried until the app restarts).
    cover_failed: Mutex<HashSet<String>>,
    /// Games already looked up in the DAT files (cleared when a DAT is added).
    identify_checked: Mutex<HashSet<String>>,
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
            installer: None,
            archive: None,
            machine: None,
            art: Vec::new(),
            art_missing: Mutex::new(HashSet::new()),
            installing: Mutex::new(HashSet::new()),
            playing: Mutex::new(None),
            launch_grace: LAUNCH_GRACE,
            settings,
            meta,
            covers: Mutex::new(CoverQueue::default()),
            cover_failed: Mutex::new(HashSet::new()),
            identify_checked: Mutex::new(HashSet::new()),
        }
    }

    /// Let the app write backups and exports.
    pub fn with_archive(mut self, archive: Box<dyn Archive>) -> Self {
        self.archive = Some(archive);
        self
    }

    fn archive(&self) -> Result<&dyn Archive, String> {
        self.archive
            .as_deref()
            .ok_or_else(|| "this build cannot write backups".to_string())
    }

    /// Back up settings, favorites, controls, cheats and the rest of what the user set up
    /// (and, if chosen, saves and cover pictures). ROMs, BIOS files and cores are never
    /// included: they are big, and they are the user's own files.
    pub fn create_backup(
        &self,
        dest: &str,
        options: BackupOptions,
    ) -> Result<BackupSummary, String> {
        self.archive()?.create_backup(
            Path::new(dest),
            &options.includes(),
            env!("CARGO_PKG_VERSION"),
            now(),
        )
    }

    /// What a backup file holds, before restoring it.
    pub fn inspect_backup(&self, src: &str) -> Result<BackupManifest, String> {
        let m = self.archive()?.inspect_backup(Path::new(src))?;
        backup::check_manifest(&m)?;
        Ok(m)
    }

    /// Restore a backup. A safety copy of the current state is made first, next to the data
    /// (`backups/before-restore-<time>.zip`), and what the app holds in memory is reloaded.
    pub fn restore_backup(&self, src: &str) -> Result<BackupSummary, String> {
        let archive = self.archive()?;
        self.inspect_backup(src)?;
        let safety = self
            .paths
            .data_dir()
            .join("backups")
            .join(format!("before-restore-{}.zip", now()));
        archive
            .create_backup(
                &safety,
                &BackupOptions {
                    saves: true,
                    covers: false,
                }
                .includes(),
                env!("CARGO_PKG_VERSION"),
                now(),
            )
            .map_err(|e| {
                format!("could not make a safety copy first, nothing was restored: {e}")
            })?;
        let summary = archive.restore_backup(Path::new(src))?;
        *self.settings.lock().unwrap() = self.store.load_settings();
        *self.meta.lock().unwrap() = self.store.load_meta();
        Ok(summary)
    }

    /// Write the library as a CSV spreadsheet.
    pub fn export_library(&self, dest: &str) -> Result<usize, String> {
        let ov = self.overview();
        let name = |id: &str| {
            ov.systems
                .iter()
                .find(|s| s.id == id)
                .map_or_else(|| id.to_string(), |s| s.name.to_string())
        };
        let csv = backup::library_csv(&ov.games, name);
        self.archive()?.write_text(Path::new(dest), &csv)?;
        Ok(ov.games.len())
    }

    /// Measure the computer so 3D systems can be judged against it.
    pub fn with_machine(mut self, machine: Box<dyn Machine>) -> Self {
        self.machine = Some(machine.info());
        self
    }

    /// Let the app download cores itself.
    pub fn with_installer(mut self, installer: Box<dyn CoreInstaller>) -> Self {
        self.installer = Some(installer);
        self
    }

    /// Add a place to download cover art from; sources are tried in the order added.
    pub fn with_art_source(mut self, art: Box<dyn ArtSource>) -> Self {
        self.art.push(art);
        self
    }

    /// Save the user's ScreenScraper login; an empty user name forgets it.
    pub fn set_screenscraper(&self, user: String, password: String) {
        let user = user.trim().to_string();
        self.settings.lock().unwrap().screenscraper =
            (!user.is_empty()).then_some(crate::model::ScreenScraperLogin { user, password });
        self.save_settings();
    }

    pub fn set_fetch_art(&self, enabled: bool) {
        self.settings.lock().unwrap().fetch_art = enabled;
        self.save_settings();
    }

    /// Download box art for games that have an official name. Does nothing unless the user
    /// switched it on. Slow: run it on a thread. `on_made` gets the key of each game that
    /// now has art; returns how many.
    pub fn fetch_art(&self, on_made: impl Fn(&str)) -> usize {
        if self.art.is_empty() {
            return 0;
        }
        let settings = self.settings.lock().unwrap().clone();
        if !settings.fetch_art {
            return 0;
        }
        let mut made = 0;
        for g in self.scan_all(&settings) {
            let key = g.path.to_string_lossy().into_owned();
            let name = self
                .meta
                .lock()
                .unwrap()
                .get(&key)
                .and_then(|m| m.name.clone());
            let sys = crate::registry::by_id(&g.system);
            let folder = sys.map(|s| s.thumbnails).unwrap_or("");
            let ss_id = sys.map(|s| s.screenscraper).unwrap_or(0);
            let (Some(name), false) = (name, folder.is_empty() && ss_id == 0) else {
                continue;
            };
            if self.store.art_exists(&g.path)
                || !self.art_missing.lock().unwrap().insert(key.clone())
            {
                continue;
            }
            let query = ArtQuery {
                name: &name,
                thumbnails: folder,
                screenscraper_id: ss_id,
                login: settings
                    .screenscraper
                    .as_ref()
                    .map(|l| (l.user.as_str(), l.password.as_str())),
            };
            // first source that has a picture wins
            let found = self.art.iter().find_map(|src| match src.boxart(&query) {
                Ok(Some(img)) if crate::model::image_mime(&img).is_some() => Some(img),
                _ => None,
            });
            if let Some(img) = found {
                if self.store.save_art(&g.path, &img).is_ok() {
                    self.art_missing.lock().unwrap().remove(&key);
                    on_made(&key);
                    made += 1;
                }
            }
        }
        made
    }

    /// Download a core the registry knows. Slow: run it on a thread. `progress` gets 0..=100.
    pub fn install_core(&self, stem: &str, progress: impl Fn(u8)) -> Result<(), String> {
        if !systems().iter().any(|s| s.all_cores().any(|c| c == stem)) {
            return Err(format!("{stem} is not a core vuretro knows"));
        }
        let installer = self
            .installer
            .as_ref()
            .ok_or("this build cannot download cores")?;
        if !self.installing.lock().unwrap().insert(stem.to_string()) {
            return Err(format!("{stem} is already being installed"));
        }
        let result = installer.install(stem, &progress);
        self.installing.lock().unwrap().remove(stem);
        result.map(|_| ())
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

    /// The app's own library plus, if the user still links one, that folder read in place.
    fn scan_all(&self, settings: &Settings) -> Vec<crate::model::GameEntry> {
        let mut scanned = self.catalog.scan(&self.paths.roms_dir());
        if let Some(dir) = settings.rom_dir.as_deref() {
            for g in self.catalog.scan(Path::new(dir)) {
                if !scanned.iter().any(|x| x.path == g.path) {
                    scanned.push(g);
                }
            }
        }
        scanned
    }

    /// Everything the library screen shows. Scans the disk, so it can take a moment.
    pub fn overview(&self) -> Overview {
        let settings = self.settings.lock().unwrap().clone();
        let meta = self.meta.lock().unwrap().clone();
        let system_dir = self.paths.system_dir();
        let roms_dir = self.paths.roms_dir();

        let scanned = self.scan_all(&settings);

        let systems: Vec<SystemView> = systems()
            .iter()
            .filter(|s| s.enabled)
            .map(|s| {
                let mut view = SystemView::new(
                    s,
                    &settings,
                    self.locator.core(settings.core_for(s)).is_some(),
                    s.all_cores()
                        .map(|c| CoreChoice {
                            name: c,
                            installed: self.locator.core(c).is_some(),
                        })
                        .collect(),
                    self.catalog.bios_status(s),
                    scanned.iter().filter(|g| g.system == s.id).count(),
                );
                if s.hardware {
                    view.renderer_ready = HARDWARE_RENDERER;
                    view.assessment = self.machine.as_ref().map(|m| assess(s, m));
                }
                view
            })
            .collect();
        let mut collections: Vec<String> = scanned
            .iter()
            .filter_map(|g| meta.get(g.path.to_string_lossy().as_ref()))
            .flat_map(|m| m.tags.iter().cloned())
            .collect();
        collections.sort();
        collections.dedup();
        let games = game_views(scanned, &systems, &meta, &roms_dir);

        Overview {
            data_dir: self.paths.data_dir().to_string_lossy().into_owned(),
            system_dir: system_dir.to_string_lossy().into_owned(),
            rom_dir: settings.rom_dir,
            systems,
            games,
            runner_found: self.locator.runner().is_some(),
            demo_available: self.locator.test_core().is_some(),
            machine: self.machine.clone(),
            dat_entries: self.catalog.dat_entries(),
            fetch_art: settings.fetch_art,
            art_sources: self.art.iter().map(|a| a.id()).collect(),
            screenscraper_user: settings.screenscraper.as_ref().map(|l| l.user.clone()),
            collections,
        }
    }

    // ---------- settings and favorites ----------

    pub fn set_rom_dir(&self, path: Option<String>) {
        self.settings.lock().unwrap().rom_dir = path;
        self.save_settings();
    }

    /// The options a core offers, with the saved choices. Slow (starts the runner): use a thread.
    pub fn core_options(&self, stem: &str) -> Result<Vec<CoreOptionView>, String> {
        self.check_core_name(stem)?;
        let runner = self.locator.runner().ok_or("runner not found")?;
        let core = self
            .locator
            .core(stem)
            .ok_or_else(|| format!("core {stem} not installed"))?;
        // a game of a system that uses this core, so cores that announce their options only
        // after loading one (FBNeo) can tell them
        let settings = self.settings.lock().unwrap().clone();
        let sample = self
            .scan_all(&settings)
            .into_iter()
            .find(|g| {
                crate::registry::by_id(&g.system).is_some_and(|s| s.all_cores().any(|c| c == stem))
            })
            .map(|g| g.path);
        let defs = self
            .launcher
            .list_core_options(&runner, &core, sample.as_deref())?;
        Ok(CoreOptionView::merge(
            defs,
            &self.store.load_core_options(stem),
        ))
    }

    /// Save the options that differ from the core's defaults (an empty map resets them all).
    pub fn set_core_options(
        &self,
        stem: &str,
        overrides: BTreeMap<String, String>,
    ) -> Result<(), String> {
        self.check_core_name(stem)?;
        if let Some(bad) = overrides
            .iter()
            .find(|(k, v)| k.is_empty() || !is_storable(k) || !is_storable(v) || k.contains('='))
        {
            return Err(format!("cannot save option {:?}", bad.0));
        }
        self.store.save_core_options(stem, &overrides)
    }

    fn check_core_name(&self, stem: &str) -> Result<(), String> {
        if systems().iter().any(|s| s.all_cores().any(|c| c == stem)) {
            Ok(())
        } else {
            Err(format!("{stem} is not a core vuretro knows"))
        }
    }

    /// The slots of a game that hold a save.
    /// Forget every screenshot preview so they are made again (with the current rules for what
    /// makes a good one). Cover art that was downloaded or chosen is kept.
    pub fn regenerate_previews(&self) -> usize {
        let n = self.store.remove_all_previews();
        self.cover_failed.lock().unwrap().clear();
        n
    }

    /// Use a picture the user chose as the game's cover.
    pub fn set_custom_art(&self, path: &str, image: &str) -> Result<(), String> {
        self.store.import_art(Path::new(path), Path::new(image))
    }

    /// Drop downloaded or chosen cover art; the screenshot preview comes back.
    pub fn clear_art(&self, path: &str) {
        self.store.remove_art(Path::new(path));
    }

    /// The cheat codes of a game as text (see `vuretro_settings::cheats`).
    pub fn cheats(&self, path: &str) -> String {
        self.store.load_cheats(Path::new(path))
    }

    /// Save a game's cheat codes; they apply the next time it starts.
    pub fn set_cheats(&self, path: &str, text: &str) -> Result<(), String> {
        if text.len() > 64 * 1024 {
            return Err("cheat list is too long".into());
        }
        self.store.save_cheats(Path::new(path), text)
    }

    pub fn save_slots(&self, path: &str) -> Vec<SlotInfo> {
        self.store.list_slots(Path::new(path))
    }

    /// The picture taken when a slot was saved.
    pub fn slot_thumbnail(&self, path: &str, slot: u8) -> Option<Vec<u8>> {
        self.store.read_slot_thumb(Path::new(path), slot)
    }

    /// Pick which core runs a system. Only cores the registry lists for it are accepted.
    pub fn set_system_core(&self, id: &str, core: &str) -> Result<(), String> {
        let sys = crate::registry::by_id(id).ok_or("unknown system")?;
        if !sys.all_cores().any(|c| c == core) {
            return Err(format!("{core} cannot run {}", sys.name));
        }
        {
            let mut s = self.settings.lock().unwrap();
            if core == sys.core {
                s.cores.remove(id);
            } else {
                s.cores.insert(id.to_string(), core.to_string());
            }
        }
        self.save_settings();
        Ok(())
    }

    pub fn set_system_enabled(&self, id: String, enabled: bool) {
        {
            let mut s = self.settings.lock().unwrap();
            // 3D systems are off unless listed; the others are on unless listed
            let hardware = crate::registry::by_id(&id).is_some_and(|sys| sys.hardware);
            if hardware {
                s.hardware_on.retain(|d| *d != id);
                if enabled {
                    s.hardware_on.push(id);
                }
                drop(s);
                self.save_settings();
                return;
            }
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

    /// Check a move or combo written in fighting-game notation and show what it would play.
    pub fn compile_move(&self, notation: &str, mirror: bool) -> Result<CompiledMove, String> {
        let steps = vuretro_controls::moves::compile(notation, mirror)?;
        let total_ms = vuretro_controls::moves::total_ms(&steps);
        Ok(CompiledMove { steps, total_ms })
    }

    /// Choose which macro profile of a game plays (`None`: only the shared ones). `game` is
    /// the game's path or name; profiles are told apart by file name like the macros.
    pub fn set_active_macro_profile(
        &self,
        game: &str,
        profile: Option<String>,
    ) -> Result<MacroConfig, String> {
        let key = vuretro_controls::macros::game_key(game);
        let mut config = self.store.load_macros();
        match profile {
            Some(id) => {
                let exists = config
                    .profiles
                    .get(&key)
                    .is_some_and(|l| l.iter().any(|p| p.id == id));
                if !exists {
                    return Err("that macro profile does not exist".into());
                }
                config.active.insert(key, id);
            }
            None => {
                config.active.remove(&key);
            }
        }
        self.set_macros(config)
    }

    pub fn set_macros(&self, config: MacroConfig) -> Result<MacroConfig, String> {
        let config = config.sanitized();
        self.store.save_macros(&config)?;
        Ok(config)
    }

    fn known_system(system: &str) -> Result<(), String> {
        crate::registry::by_id(system)
            .map(|_| ())
            .ok_or_else(|| format!("unknown system {system}"))
    }

    /// Settings for everything (`None`) or for one system. A system shows what it overrides
    /// and what it would otherwise inherit.
    pub fn scoped_settings(&self, system: Option<&str>) -> Result<ScopedSettings, String> {
        let base = self.store.load_game_settings();
        let Some(system) = system else {
            return Ok(ScopedSettings {
                effective: base.clone(),
                inherited: base,
                overridden: Default::default(),
                recommended: None,
            });
        };
        Self::known_system(system)?;
        let ov = self
            .store
            .load_overrides()
            .systems
            .remove(system)
            .unwrap_or_default();
        Ok(ScopedSettings {
            effective: ov.apply(base.clone()),
            inherited: base,
            overridden: ov,
            recommended: crate::registry::by_id(system)
                .and_then(crate::model::recommended_override),
        })
    }

    /// Replace what one system overrides (`None` fields inherit again).
    pub fn set_system_override(
        &self,
        system: &str,
        ov: vuretro_settings::scoped::Override,
    ) -> Result<ScopedSettings, String> {
        Self::known_system(system)?;
        let mut all = self.store.load_overrides();
        all.set_system(system, ov);
        self.store.save_overrides(&all)?;
        self.scoped_settings(Some(system))
    }

    /// What a game would use if it overrode nothing: general settings plus its system's.
    fn system_of(&self, rom: &Path) -> Option<&'static crate::registry::System> {
        let rom_dir = self
            .settings
            .lock()
            .unwrap()
            .rom_dir
            .clone()
            .unwrap_or_default();
        detect::detect_system(Path::new(&rom_dir), rom)
    }

    fn inherited_for(&self, rom: &Path) -> GameSettings {
        let base = self.store.load_game_settings();
        match self.system_of(rom) {
            Some(sys) => {
                let mut o = self.store.load_overrides();
                o.games.clear();
                o.resolve(
                    &base,
                    vuretro_settings::scoped::Scope {
                        system: sys.id,
                        game: "",
                    },
                )
            }
            None => base,
        }
    }

    /// Settings of one game: what applies, what it would inherit from its system, and what
    /// the game itself overrides. Games are told apart by file name without extension.
    pub fn game_override(&self, path: &str) -> ScopedSettings {
        let rom = Path::new(path);
        let key = Self::game_key(rom);
        let inherited = self.inherited_for(rom);
        let ov = self
            .store
            .load_overrides()
            .games
            .remove(&key)
            .unwrap_or_default();
        ScopedSettings {
            effective: ov.apply(inherited.clone()),
            inherited,
            overridden: ov,
            recommended: None,
        }
    }

    /// Replace what one game overrides (`None` fields inherit again).
    pub fn set_game_override(
        &self,
        path: &str,
        ov: vuretro_settings::scoped::Override,
    ) -> Result<ScopedSettings, String> {
        let mut all = self.store.load_overrides();
        all.set_game(&Self::game_key(Path::new(path)), ov);
        self.store.save_overrides(&all)?;
        Ok(self.game_override(path))
    }

    /// The runner names a game by its file name without extension; so does the override.
    fn game_key(rom: &Path) -> String {
        rom.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Key bindings for everything (`None`) or for one system.
    pub fn controls_for(&self, system: Option<&str>) -> Result<ScopedControls, String> {
        let Some(system) = system else {
            return Ok(ScopedControls {
                config: self.store.load_controls(),
                custom: false,
            });
        };
        Self::known_system(system)?;
        Ok(match self.store.load_system_controls(system) {
            Some(config) => ScopedControls {
                config,
                custom: true,
            },
            None => ScopedControls {
                config: self.store.load_controls(),
                custom: false,
            },
        })
    }

    /// Key bindings of one game: its own if it has them, else its system's, else the general
    /// ones. `custom` says the game has its own.
    pub fn controls_for_game(&self, path: &str) -> ScopedControls {
        let rom = Path::new(path);
        if let Some(config) = self.store.load_game_controls(rom) {
            return ScopedControls {
                config,
                custom: true,
            };
        }
        let inherited = self
            .system_of(rom)
            .and_then(|s| self.store.load_system_controls(s.id))
            .unwrap_or_else(|| self.store.load_controls());
        ScopedControls {
            config: inherited,
            custom: false,
        }
    }

    pub fn set_controls_for_game(
        &self,
        path: &str,
        config: ControlsConfig,
    ) -> Result<ScopedControls, String> {
        let config = config.sanitized();
        self.store.save_game_controls(Path::new(path), &config)?;
        Ok(ScopedControls {
            config,
            custom: true,
        })
    }

    /// Drop a game's own key bindings: it follows its system again.
    pub fn reset_controls_for_game(&self, path: &str) -> Result<ScopedControls, String> {
        self.store.remove_game_controls(Path::new(path))?;
        Ok(self.controls_for_game(path))
    }

    /// Give a system key bindings of its own.
    pub fn set_controls_for(
        &self,
        system: &str,
        config: ControlsConfig,
    ) -> Result<ScopedControls, String> {
        Self::known_system(system)?;
        let config = config.sanitized();
        self.store.save_system_controls(system, &config)?;
        Ok(ScopedControls {
            config,
            custom: true,
        })
    }

    /// Drop a system's own key bindings: it uses the general ones again.
    pub fn reset_controls_for(&self, system: &str) -> Result<ScopedControls, String> {
        Self::known_system(system)?;
        self.store.remove_system_controls(system)?;
        self.controls_for(Some(system))
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

    /// Install one BIOS file; the message starts with "BIOS installed" so the page can tell.
    fn install_bios(&self, file: &Path) -> Outcome {
        let source = file.to_string_lossy().into_owned();
        match self.catalog.import_bios(file) {
            Ok(r) => Outcome::ok(source, format!("BIOS installed: {}", r.file)),
            Err(message) => Outcome::failed(source, message),
        }
    }

    /// Add DAT files (No-Intro, Redump style XML) so games get their official names.
    pub fn import_dats(&self, paths: Vec<String>) -> Vec<Outcome> {
        let out = paths
            .into_iter()
            .map(|p| match self.catalog.import_dat(Path::new(&p)) {
                Ok(n) => Outcome::ok(p, format!("{n} dumps added")),
                Err(message) => Outcome::failed(p, message),
            })
            .collect();
        self.identify_checked.lock().unwrap().clear();
        out
    }

    /// Look games up in the DAT files and remember their official names. Slow (reads
    /// files): run it on a thread. Returns how many games got a name now.
    pub fn identify_games(&self) -> usize {
        if self.catalog.dat_entries() == 0 {
            return 0;
        }
        let settings = self.settings.lock().unwrap().clone();
        let mut found = 0;
        for g in self.scan_all(&settings) {
            let key = g.path.to_string_lossy().into_owned();
            let known = self
                .meta
                .lock()
                .unwrap()
                .get(&key)
                .is_some_and(|m| m.name.is_some());
            if known || !self.identify_checked.lock().unwrap().insert(key.clone()) {
                continue;
            }
            if let Some(name) = self.catalog.identify(&g.path) {
                self.meta.lock().unwrap().entry(key).or_default().name = Some(name);
                found += 1;
            }
        }
        if found > 0 {
            self.save_meta();
        }
        found
    }

    /// Put a game in collections (replaces its list). Names are trimmed; blanks and repeats go.
    pub fn set_tags(&self, path: String, tags: Vec<String>) -> Vec<String> {
        let mut clean: Vec<String> = Vec::new();
        for t in tags {
            let t: String = t.trim().chars().take(40).collect();
            if !t.is_empty() && !clean.iter().any(|c| c.eq_ignore_ascii_case(&t)) {
                clean.push(t);
            }
        }
        self.meta.lock().unwrap().entry(path).or_default().tags = clean.clone();
        self.save_meta();
        clean
    }

    /// Run one game on a core other than its system's (`None` goes back to the system's).
    /// Only cores the registry lists for the game's system are accepted.
    pub fn set_game_core(&self, path: String, core: Option<String>) -> Result<(), String> {
        let rom_dir = self.settings.lock().unwrap().rom_dir.clone().unwrap_or_default();
        let sys = detect::detect_system(Path::new(&rom_dir), Path::new(&path))
            .ok_or("unknown system for this file")?;
        let core = match core.filter(|c| !c.is_empty()) {
            Some(c) if sys.all_cores().any(|k| k == c) => Some(c),
            Some(c) => return Err(format!("{c} cannot run {}", sys.name)),
            None => None,
        };
        self.meta.lock().unwrap().entry(path).or_default().core = core;
        self.save_meta();
        Ok(())
    }

    /// The core a game runs on: its own pick, else the one of its system.
    fn core_of(&self, path: &str, sys: &System) -> &'static str {
        let own = self
            .meta
            .lock()
            .unwrap()
            .get(path)
            .and_then(|m| m.core.clone());
        own.and_then(|c| sys.all_cores().find(|k| *k == c))
            .unwrap_or_else(|| self.settings.lock().unwrap().core_for(sys))
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
                // BIOS files in the collection are not games: install them where cores look
                let bios = self.catalog.find_bios(&path);
                if results.is_empty() && bios.is_empty() {
                    out.push(Outcome::failed(p, "no games it recognizes in that folder"));
                }
                for (src, r) in results {
                    out.push(describe(r, src.to_string_lossy().into_owned()));
                }
                for file in bios {
                    out.push(self.install_bios(&file));
                }
            } else if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(detect::is_bios_file)
            {
                // chosen by mistake as a game: it is a BIOS, so treat it as one
                out.push(self.install_bios(&path));
            } else {
                out.push(describe(
                    self.catalog.import_rom(&path, system.as_deref()),
                    p,
                ));
            }
        }
        out
    }

    /// Make a patched copy of a game (a translation or ROM hack shared as an IPS, UPS or
    /// BPS file) in the library. The original game is left as it is.
    pub fn apply_patch(&self, rom: String, patch: String) -> Outcome {
        match self.catalog.apply_patch(Path::new(&rom), Path::new(&patch)) {
            Ok(i) if i.already_there => Outcome::ok(rom, "already in the library"),
            Ok(i) => {
                let file = i
                    .dest
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                Outcome::ok(rom, format!("added to {} as {file}", i.system))
            }
            Err(message) => Outcome::failed(rom, message),
        }
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

    /// Ask for previews of the games on screen. A game that already has a picture (downloaded,
    /// chosen, or made before) is skipped; the rest are made one at a time in the background,
    /// in the order given. **A new request replaces the one still waiting**: scrolling to other
    /// games makes those the next ones made, and the ones scrolled past are dropped. Returns
    /// true when the caller must start a worker thread that calls [`App::run_covers`]; false
    /// when nothing is needed or a worker is already running and will pick the new list up.
    pub fn request_covers(&self, games: Vec<(String, String)>) -> bool {
        let jobs = self.plan_covers(games);
        let mut q = self.covers.lock().unwrap();
        q.pending = jobs.into();
        if q.pending.is_empty() || q.running {
            return false;
        }
        q.running = true;
        true
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
                .and_then(|s| self.locator.core(self.core_of(&path, s)));
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

    /// The worker of [`App::request_covers`] (slow: run it on a thread). Makes each waiting
    /// preview in turn until none is left; `on_made` gets the key of each game whose image now
    /// exists. A request that arrives meanwhile changes what is made next.
    pub fn run_covers(&self, on_made: impl Fn(&str)) {
        loop {
            let job = {
                let mut q = self.covers.lock().unwrap();
                match q.pending.pop_front() {
                    Some(job) => job,
                    None => {
                        // decided under the lock, so a request cannot slip in unseen
                        q.running = false;
                        break;
                    }
                }
            };
            if self.launcher.capture_cover(&job) {
                on_made(&job.key);
            } else {
                self.cover_failed.lock().unwrap().insert(job.key);
            }
        }
        self.store.clear_cover_scratch();
    }

    // ---------- launching ----------

    /// Start a game from the library. `Ok(None)` means it already finished successfully.
    pub fn launch_game(&self, path: String, slot: Option<u8>) -> Result<Option<Running>, String> {
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
        let chosen = self.core_of(&path, sys);
        let own = self
            .meta
            .lock()
            .unwrap()
            .get(&path)
            .is_some_and(|m| m.core.is_some());
        // The system's core may still be downloading (or was never installed): play with
        // another of its cores that is there rather than not at all. A core picked for this
        // game alone is the user's decision, so that one is never swapped.
        let core = match self.locator.core(chosen) {
            Some(c) => c,
            None if !own => sys
                .all_cores()
                .find_map(|c| self.locator.core(c))
                .ok_or_else(|| {
                    format!("core {chosen} not installed (install it in Cores & BIOS)")
                })?,
            None => {
                return Err(format!(
                    "core {chosen} not installed (install it in Cores & BIOS)"
                ))
            }
        };
        self.start(&core, &rom, Some(path), slot, sys.id)
    }

    /// Run the fake test core so the app can be tried without any ROM.
    pub fn launch_demo(&self) -> Result<Option<Running>, String> {
        let core = self
            .locator
            .test_core()
            .ok_or("test core not built. Run `make build` first.")?;
        let rom = self.store.ensure_demo_rom()?;
        self.start(&core, &rom, None, None, "")
    }

    fn start(
        &self,
        core: &Path,
        rom: &Path,
        track: Option<String>,
        load_slot: Option<u8>,
        system: &str,
    ) -> Result<Option<Running>, String> {
        let runner = self
            .locator
            .runner()
            .ok_or("vuretro-runner not found. Run `make build` first.")?;
        // One game at a time: two windows would fight over the sound, the gamepad and the
        // saves of the same game. The place is taken before the game starts, so two quick
        // presses of Play cannot both get through.
        let name = rom
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "a game".into());
        {
            let mut playing = self.playing.lock().unwrap();
            if let Some(open) = playing.as_ref() {
                return Err(format!(
                    "{open} is already open. Close it before you open another game."
                ));
            }
            *playing = Some(name);
        }
        let free = |app: &App| *app.playing.lock().unwrap() = None;
        let mut process = match self.launcher.spawn(&LaunchSpec {
            runner,
            core: core.to_path_buf(),
            rom: rom.to_path_buf(),
            system_dir: self.paths.system_dir(),
            load_slot,
            system: system.to_string(),
        }) {
            Ok(p) => p,
            Err(e) => {
                free(self);
                return Err(e);
            }
        };

        // Failing fast (missing BIOS, bad file) should be reported, not silently ignored.
        let started = Instant::now();
        loop {
            let polled = process.poll();
            let exit = match polled {
                Ok(exit) => exit,
                Err(e) => {
                    free(self);
                    return Err(e);
                }
            };
            if let Some(exit) = exit {
                free(self);
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
        *self.playing.lock().unwrap() = None;
        if let Some(key) = running.track {
            self.meta.lock().unwrap().entry(key).or_default().play_secs +=
                running.started.elapsed().as_secs();
            self.save_meta();
        }
    }
}
