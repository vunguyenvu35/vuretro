//! Use-case tests with in-memory fakes: no disk, no processes, no Tauri.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use vuretro_controls::{macros::MacroConfig, ControlsConfig};
use vuretro_core::model::*;
use vuretro_core::paths::Paths;
use vuretro_core::ports::*;
use vuretro_core::registry::{self, System};
use vuretro_core::service::App;
use vuretro_settings::GameSettings;

#[derive(Default)]
struct World {
    controls: Option<ControlsConfig>,
    game_settings: Option<GameSettings>,
    settings: Settings,
    meta: HashMap<String, GameMeta>,
    covers: HashSet<PathBuf>,
    scans: HashMap<PathBuf, Vec<GameEntry>>,
    bios_ok: bool,
    cores: HashSet<&'static str>,
    runner: bool,
    /// What a spawned runner does: `None` keeps running, `Some` ends at once.
    runner_exit: Option<Exit>,
    removed: Vec<PathBuf>,
    spawned: Vec<LaunchSpec>,
    cover_ok: bool,
    scratch_cleared: bool,
}

#[derive(Clone, Default)]
struct Fake(Arc<Mutex<World>>);

impl Store for Fake {
    fn load_settings(&self) -> Settings {
        self.0.lock().unwrap().settings.clone()
    }
    fn save_settings(&self, s: &Settings) {
        self.0.lock().unwrap().settings = s.clone();
    }
    fn load_meta(&self) -> HashMap<String, GameMeta> {
        self.0.lock().unwrap().meta.clone()
    }
    fn save_meta(&self, m: &HashMap<String, GameMeta>) {
        self.0.lock().unwrap().meta = m.clone();
    }
    fn load_controls(&self) -> ControlsConfig {
        self.0.lock().unwrap().controls.clone().unwrap_or_default()
    }
    fn save_controls(&self, c: &ControlsConfig) -> Result<(), String> {
        self.0.lock().unwrap().controls = Some(c.clone());
        Ok(())
    }
    fn load_macros(&self) -> MacroConfig {
        MacroConfig::default()
    }
    fn save_macros(&self, _: &MacroConfig) -> Result<(), String> {
        Err("disk full".into())
    }
    fn load_game_settings(&self) -> GameSettings {
        self.0
            .lock()
            .unwrap()
            .game_settings
            .clone()
            .unwrap_or_default()
    }
    fn save_game_settings(&self, s: &GameSettings) -> Result<(), String> {
        self.0.lock().unwrap().game_settings = Some(s.clone());
        Ok(())
    }
    fn read_cover(&self, rom: &Path) -> Option<Vec<u8>> {
        self.0
            .lock()
            .unwrap()
            .covers
            .contains(rom)
            .then(|| vec![1, 2, 3])
    }
    fn cover_exists(&self, rom: &Path) -> bool {
        self.0.lock().unwrap().covers.contains(rom)
    }
    fn remove_cover(&self, rom: &Path) {
        self.0.lock().unwrap().covers.remove(rom);
    }
    fn clear_cover_scratch(&self) {
        self.0.lock().unwrap().scratch_cleared = true;
    }
    fn ensure_demo_rom(&self) -> Result<PathBuf, String> {
        Ok(PathBuf::from("/data/demo.test"))
    }
}

impl Catalog for Fake {
    fn scan(&self, dir: &Path) -> Vec<GameEntry> {
        self.0
            .lock()
            .unwrap()
            .scans
            .get(dir)
            .cloned()
            .unwrap_or_default()
    }
    fn bios_status(&self, system: &'static System) -> Vec<BiosStatus> {
        let ok = self.0.lock().unwrap().bios_ok;
        system
            .bios
            .iter()
            .map(|b| BiosStatus {
                system: system.id,
                file: b.file,
                required: b.required,
                note: b.note,
                state: if ok {
                    BiosState::Ok
                } else {
                    BiosState::Missing
                },
            })
            .collect()
    }
    fn import_bios(&self, src: &Path) -> Result<ImportedBios, String> {
        if src.ends_with("neogeo.zip") {
            Ok(ImportedBios {
                system: "neogeo",
                file: "neogeo.zip",
                verified: true,
            })
        } else {
            Err("not a BIOS".into())
        }
    }
    fn is_folder(&self, path: &Path) -> bool {
        path.extension().is_none()
    }
    fn import_rom(&self, src: &Path, _system: Option<&str>) -> Result<ImportedRom, String> {
        Ok(ImportedRom {
            system: "snes",
            dest: src.to_path_buf(),
            already_there: src.ends_with("dup.sfc"),
        })
    }
    fn import_folder(&self, _dir: &Path) -> Vec<(PathBuf, Result<ImportedRom, String>)> {
        Vec::new()
    }
    fn remove_rom(&self, path: &Path) -> Result<(), String> {
        if path.starts_with("/data/roms") {
            self.0.lock().unwrap().removed.push(path.to_path_buf());
            Ok(())
        } else {
            Err("only ROMs imported into the library can be removed".into())
        }
    }
}

impl Locator for Fake {
    fn runner(&self) -> Option<PathBuf> {
        self.0.lock().unwrap().runner.then(|| "/bin/runner".into())
    }
    fn core(&self, stem: &str) -> Option<PathBuf> {
        let w = self.0.lock().unwrap();
        w.cores
            .iter()
            .find(|c| **c == stem)
            .map(|c| PathBuf::from(format!("/cores/{c}")))
    }
    fn test_core(&self) -> Option<PathBuf> {
        Some("/cores/test".into())
    }
}

struct FakeProcess(Option<Exit>);
impl GameProcess for FakeProcess {
    fn poll(&mut self) -> Result<Option<Exit>, String> {
        Ok(self.0.clone())
    }
    fn wait(&mut self) {}
}

impl Launcher for Fake {
    fn spawn(&self, spec: &LaunchSpec) -> Result<Box<dyn GameProcess>, String> {
        let mut w = self.0.lock().unwrap();
        w.spawned.push(spec.clone());
        Ok(Box::new(FakeProcess(w.runner_exit.clone())))
    }
    fn capture_cover(&self, _job: &CoverJob) -> bool {
        self.0.lock().unwrap().cover_ok
    }
}

fn app(fake: &Fake) -> App {
    App::new(
        Paths::new("/data"),
        Box::new(fake.clone()),
        Box::new(fake.clone()),
        Box::new(fake.clone()),
        Box::new(fake.clone()),
    )
    .with_launch_grace(Duration::from_millis(1))
}

fn entry(system: &str, path: &str) -> GameEntry {
    GameEntry {
        system: system.into(),
        title: "g".into(),
        path: path.into(),
        size: 1,
    }
}

fn ready_world() -> Fake {
    let f = Fake::default();
    {
        let mut w = f.0.lock().unwrap();
        w.runner = true;
        w.bios_ok = true;
        w.cores.extend(registry::systems().iter().map(|s| s.core));
    }
    f
}

#[test]
fn overview_merges_library_and_linked_folder_without_duplicates() {
    let f = ready_world();
    {
        let mut w = f.0.lock().unwrap();
        w.settings.rom_dir = Some("/linked".into());
        w.scans.insert(
            "/data/roms".into(),
            vec![entry("snes", "/data/roms/snes/a.sfc")],
        );
        w.scans.insert(
            "/linked".into(),
            vec![
                entry("snes", "/data/roms/snes/a.sfc"), // same file seen twice
                entry("snes", "/linked/b.sfc"),
            ],
        );
    }
    let o = app(&f).overview();
    assert_eq!(o.games.len(), 2);
    assert!(
        o.games
            .iter()
            .find(|g| g.path.ends_with("a.sfc"))
            .unwrap()
            .imported
    );
    assert!(
        !o.games
            .iter()
            .find(|g| g.path.ends_with("b.sfc"))
            .unwrap()
            .imported
    );
    assert_eq!(
        o.systems
            .iter()
            .find(|s| s.id == "snes")
            .unwrap()
            .game_count,
        2
    );
    assert!(o.runner_found && o.demo_available);
}

#[test]
fn overview_blocks_games_of_a_system_without_its_core() {
    let f = ready_world();
    {
        let mut w = f.0.lock().unwrap();
        w.cores.remove("snes9x_libretro");
        w.scans.insert(
            "/data/roms".into(),
            vec![entry("snes", "/data/roms/snes/a.sfc")],
        );
    }
    let o = app(&f).overview();
    assert!(o.games[0]
        .blocked
        .as_ref()
        .unwrap()
        .contains("not installed"));
}

#[test]
fn switching_a_system_off_persists_and_blocks_it() {
    let f = ready_world();
    f.0.lock().unwrap().scans.insert(
        "/data/roms".into(),
        vec![entry("snes", "/data/roms/snes/a.sfc")],
    );
    let a = app(&f);
    a.set_system_enabled("snes".into(), false);
    assert_eq!(f.0.lock().unwrap().settings.disabled, vec!["snes"]);
    assert_eq!(
        a.overview().games[0].blocked.as_deref(),
        Some("Super Nintendo is switched off")
    );
    a.set_system_enabled("snes".into(), true);
    assert!(f.0.lock().unwrap().settings.disabled.is_empty());
}

#[test]
fn favorites_toggle_and_persist() {
    let f = ready_world();
    let a = app(&f);
    assert!(a.toggle_favorite("/x".into()));
    assert!(f.0.lock().unwrap().meta["/x"].favorite);
    assert!(!a.toggle_favorite("/x".into()));
}

#[test]
fn import_reports_one_outcome_per_item() {
    let f = ready_world();
    let out = app(&f).import_roms(
        vec![
            "/in/new.sfc".into(),
            "/in/dup.sfc".into(),
            "/in/emptydir".into(),
        ],
        None,
    );
    assert_eq!(out[0].message, "added to snes");
    assert_eq!(out[1].message, "already in the library");
    assert!(!out[2].ok && out[2].message.contains("no games"));

    let b = app(&f).import_bios(vec!["/x/neogeo.zip".into(), "/x/other.bin".into()]);
    assert!(b[0].ok && b[0].message == "Installed neogeo.zip");
    assert!(!b[1].ok);
}

#[test]
fn removing_a_rom_forgets_its_meta_and_cover() {
    let f = ready_world();
    {
        let mut w = f.0.lock().unwrap();
        w.meta
            .insert("/data/roms/snes/a.sfc".into(), GameMeta::default());
        w.covers.insert("/data/roms/snes/a.sfc".into());
    }
    let a = app(&f);
    let out = a.remove_roms(vec![
        "/data/roms/snes/a.sfc".into(),
        "/elsewhere/b.sfc".into(),
    ]);
    assert!(out[0].ok && !out[1].ok);
    let w = f.0.lock().unwrap();
    assert!(w.meta.is_empty() && w.covers.is_empty());
}

#[test]
fn launch_rejects_missing_bios_and_core_before_spawning() {
    let f = ready_world();
    let a = app(&f);
    // PlayStation needs a required BIOS
    f.0.lock().unwrap().bios_ok = false;
    let err = a.launch_game("/data/roms/psx/g.cue".into()).err().unwrap();
    assert!(err.starts_with("BIOS missing"), "{err}");

    {
        let mut w = f.0.lock().unwrap();
        w.bios_ok = true;
        w.cores.remove("pcsx_rearmed_libretro");
    }
    let err = a.launch_game("/data/roms/psx/g.cue".into()).err().unwrap();
    assert!(err.contains("not installed"), "{err}");

    let err = a.launch_game("/x/readme.txt".into()).err().unwrap();
    assert_eq!(err, "unknown system for this file");
    assert!(f.0.lock().unwrap().spawned.is_empty());
}

#[test]
fn launch_reports_an_early_crash_with_the_log_tail() {
    let f = ready_world();
    f.0.lock().unwrap().runner_exit = Some(Exit {
        success: false,
        detail: "bad rom".into(),
    });
    let err = app(&f)
        .launch_game("/data/roms/snes/a.sfc".into())
        .err()
        .unwrap();
    assert_eq!(err, "bad rom");
}

#[test]
fn launch_tracks_play_time_until_exit() {
    let f = ready_world();
    let a = app(&f);
    let running = a
        .launch_game("/data/roms/snes/a.sfc".into())
        .unwrap()
        .expect("still running after the grace period");
    let spawn = f.0.lock().unwrap().spawned[0].clone();
    assert_eq!(spawn.rom, PathBuf::from("/data/roms/snes/a.sfc"));
    assert_eq!(spawn.system_dir, PathBuf::from("/data/system"));
    assert!(f.0.lock().unwrap().meta["/data/roms/snes/a.sfc"].last_played > 0);

    a.wait_for_exit(running);
    assert!(f
        .0
        .lock()
        .unwrap()
        .meta
        .contains_key("/data/roms/snes/a.sfc"));
}

#[test]
fn demo_runs_without_tracking() {
    let f = ready_world();
    let a = app(&f);
    let running = a.launch_demo().unwrap().unwrap();
    a.wait_for_exit(running);
    assert!(f.0.lock().unwrap().meta.is_empty());
    assert_eq!(
        f.0.lock().unwrap().spawned[0].rom,
        PathBuf::from("/data/demo.test")
    );
}

#[test]
fn covers_are_planned_once_and_failures_are_not_retried() {
    let f = ready_world();
    let a = app(&f);
    let games = vec![
        ("/data/roms/snes/a.sfc".to_string(), "snes".to_string()),
        ("/data/roms/snes/b.sfc".to_string(), "snes".to_string()),
    ];
    f.0.lock()
        .unwrap()
        .covers
        .insert("/data/roms/snes/b.sfc".into()); // already has one

    let jobs = a.start_covers(games.clone()).expect("one job");
    assert_eq!(jobs.len(), 1);
    // a second request while the first is running does nothing
    assert!(a.start_covers(games.clone()).is_none());

    // the capture fails: reported nowhere, but remembered
    let made = Mutex::new(Vec::new());
    a.run_covers(jobs, |k| made.lock().unwrap().push(k.to_string()));
    assert!(made.lock().unwrap().is_empty());
    assert!(f.0.lock().unwrap().scratch_cleared);
    assert!(
        a.start_covers(games).is_none(),
        "failed game is not retried"
    );
}

#[test]
fn covers_that_succeed_are_announced() {
    let f = ready_world();
    f.0.lock().unwrap().cover_ok = true;
    let a = app(&f);
    let jobs = a
        .start_covers(vec![("/g.sfc".into(), "snes".into())])
        .unwrap();
    let made = Mutex::new(Vec::new());
    a.run_covers(jobs, |k| made.lock().unwrap().push(k.to_string()));
    assert_eq!(*made.lock().unwrap(), vec!["/g.sfc"]);
    // finished: a new run may start
    assert!(a.covers(vec!["/g.sfc".into()]).is_empty());
}

#[test]
fn game_settings_are_clamped_before_saving() {
    let f = ready_world();
    let a = app(&f);
    let saved = a
        .set_game_settings(GameSettings {
            window_scale: 99,
            volume: 250,
            ..GameSettings::default()
        })
        .unwrap();
    assert_eq!((saved.window_scale, saved.volume), (8, 100));
    assert_eq!(f.0.lock().unwrap().game_settings.as_ref().unwrap(), &saved);
    assert_eq!(a.game_settings(), saved);
}

#[test]
fn controls_round_trip_and_save_errors_reach_the_caller() {
    let f = ready_world();
    let a = app(&f);
    assert_eq!(a.controls(), a.default_controls());
    let c = a.set_controls(a.default_controls()).unwrap();
    assert_eq!(a.controls(), c);
    // the fake's macro store always fails: the error is not swallowed
    assert_eq!(
        a.set_macros(MacroConfig::default()).unwrap_err(),
        "disk full"
    );
}

#[test]
fn a_system_can_be_switched_to_another_listed_core() {
    let f = ready_world();
    let a = app(&f);
    // default first
    let nes = |a: &App| {
        a.overview()
            .systems
            .into_iter()
            .find(|s| s.id == "nes")
            .unwrap()
    };
    assert_eq!(nes(&a).core, "fceumm_libretro");
    assert_eq!(nes(&a).core_choices.len(), 3);

    a.set_system_core("nes", "mesen_libretro").unwrap();
    assert_eq!(nes(&a).core, "mesen_libretro");
    assert_eq!(f.0.lock().unwrap().settings.cores["nes"], "mesen_libretro");
    // not installed -> the system reports its chosen core as missing
    f.0.lock().unwrap().cores.remove("mesen_libretro");
    assert!(!nes(&a).core_installed);

    // going back to the default clears the entry
    a.set_system_core("nes", "fceumm_libretro").unwrap();
    assert!(f.0.lock().unwrap().settings.cores.is_empty());
    // a core the registry does not list for that system is refused
    assert!(a.set_system_core("nes", "snes9x_libretro").is_err());
    assert!(a.set_system_core("nope", "x").is_err());
}

struct FakeInstaller {
    fail: bool,
    calls: Arc<Mutex<Vec<String>>>,
}

impl CoreInstaller for FakeInstaller {
    fn install(&self, stem: &str, progress: &dyn Fn(u8)) -> Result<std::path::PathBuf, String> {
        self.calls.lock().unwrap().push(stem.to_string());
        progress(50);
        if self.fail {
            Err("download failed: offline".into())
        } else {
            Ok(format!("/data/cores/{stem}").into())
        }
    }
}

#[test]
fn cores_install_through_the_port_and_only_known_ones() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let make = |fail| {
        app(&ready_world()).with_installer(Box::new(FakeInstaller {
            fail,
            calls: calls.clone(),
        }))
    };
    let seen = Mutex::new(Vec::new());
    make(false)
        .install_core("fceumm_libretro", |p| seen.lock().unwrap().push(p))
        .unwrap();
    assert_eq!(*seen.lock().unwrap(), vec![50]);
    assert_eq!(*calls.lock().unwrap(), vec!["fceumm_libretro"]);

    // a name outside the registry never reaches the installer
    let err = make(false).install_core("../evil", |_| {}).unwrap_err();
    assert!(err.contains("not a core"));
    assert_eq!(calls.lock().unwrap().len(), 1);

    // failures reach the caller, and the core can be tried again afterwards
    let a = make(true);
    assert!(a
        .install_core("fceumm_libretro", |_| {})
        .unwrap_err()
        .contains("offline"));
    assert!(a
        .install_core("fceumm_libretro", |_| {})
        .unwrap_err()
        .contains("offline"));

    // without an installer the app says so
    assert!(app(&ready_world())
        .install_core("fceumm_libretro", |_| {})
        .unwrap_err()
        .contains("cannot download"));
}
