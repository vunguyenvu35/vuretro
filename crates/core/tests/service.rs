//! Use-case tests with in-memory fakes: no disk, no processes, no Tauri.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use vuretro_controls::{macros::MacroConfig, ControlsConfig};
use vuretro_core::model::*;
use vuretro_core::paths::Paths;
use vuretro_core::ports::*;
use vuretro_core::registry::{self, System};
use vuretro_core::service::App;
use vuretro_settings::scoped::{Override, Overrides};
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
    core_options: HashMap<String, BTreeMap<String, String>>,
    offered: Vec<CoreOptionDef>,
    slots: Vec<SlotInfo>,
    /// Official names the fake DAT knows, by file path.
    names: HashMap<PathBuf, String>,
    art: HashSet<PathBuf>,
    cheats: HashMap<PathBuf, String>,
    overrides: Overrides,
    system_controls: HashMap<String, ControlsConfig>,
    game_controls: HashMap<PathBuf, ControlsConfig>,
    /// What the fake finds when a folder is imported.
    folder_games: Vec<(PathBuf, Result<ImportedRom, String>)>,
    folder_bios: Vec<PathBuf>,
    art_calls: Vec<(String, String)>,
    dat_entries: usize,
}

#[derive(Clone, Default)]
struct Fake(Arc<Mutex<World>>);

impl Store for Fake {
    fn load_game_controls(&self, rom: &Path) -> Option<ControlsConfig> {
        self.0.lock().unwrap().game_controls.get(rom).cloned()
    }
    fn save_game_controls(&self, rom: &Path, c: &ControlsConfig) -> Result<(), String> {
        self.0
            .lock()
            .unwrap()
            .game_controls
            .insert(rom.to_path_buf(), c.clone());
        Ok(())
    }
    fn remove_game_controls(&self, rom: &Path) -> Result<(), String> {
        self.0.lock().unwrap().game_controls.remove(rom);
        Ok(())
    }
    fn load_overrides(&self) -> Overrides {
        self.0.lock().unwrap().overrides.clone()
    }
    fn save_overrides(&self, o: &Overrides) -> Result<(), String> {
        self.0.lock().unwrap().overrides = o.clone();
        Ok(())
    }
    fn load_system_controls(&self, system: &str) -> Option<ControlsConfig> {
        self.0.lock().unwrap().system_controls.get(system).cloned()
    }
    fn save_system_controls(&self, system: &str, c: &ControlsConfig) -> Result<(), String> {
        self.0
            .lock()
            .unwrap()
            .system_controls
            .insert(system.to_string(), c.clone());
        Ok(())
    }
    fn remove_system_controls(&self, system: &str) -> Result<(), String> {
        self.0.lock().unwrap().system_controls.remove(system);
        Ok(())
    }
    fn list_slots(&self, _: &Path) -> Vec<SlotInfo> {
        self.0.lock().unwrap().slots.clone()
    }
    fn read_slot_thumb(&self, _: &Path, slot: u8) -> Option<Vec<u8>> {
        (slot == 1).then(|| vec![0x89, b'P'])
    }
    fn load_cheats(&self, rom: &Path) -> String {
        self.0
            .lock()
            .unwrap()
            .cheats
            .get(rom)
            .cloned()
            .unwrap_or_default()
    }
    fn save_cheats(&self, rom: &Path, text: &str) -> Result<(), String> {
        self.0
            .lock()
            .unwrap()
            .cheats
            .insert(rom.to_path_buf(), text.to_string());
        Ok(())
    }
    fn load_core_options(&self, stem: &str) -> BTreeMap<String, String> {
        self.0
            .lock()
            .unwrap()
            .core_options
            .get(stem)
            .cloned()
            .unwrap_or_default()
    }
    fn save_core_options(&self, stem: &str, o: &BTreeMap<String, String>) -> Result<(), String> {
        self.0
            .lock()
            .unwrap()
            .core_options
            .insert(stem.to_string(), o.clone());
        Ok(())
    }
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
    fn save_art(&self, rom: &Path, _png: &[u8]) -> Result<(), String> {
        self.0.lock().unwrap().art.insert(rom.to_path_buf());
        Ok(())
    }
    fn art_exists(&self, rom: &Path) -> bool {
        self.0.lock().unwrap().art.contains(rom)
    }
    fn import_art(&self, rom: &Path, src: &Path) -> Result<(), String> {
        if src.extension().is_some_and(|e| e == "png") {
            self.0.lock().unwrap().art.insert(rom.to_path_buf());
            Ok(())
        } else {
            Err("not a PNG or JPEG image".into())
        }
    }
    fn remove_art(&self, rom: &Path) {
        self.0.lock().unwrap().art.remove(rom);
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
    fn identify(&self, path: &Path) -> Option<String> {
        self.0.lock().unwrap().names.get(path).cloned()
    }
    fn import_dat(&self, src: &Path) -> Result<usize, String> {
        if src.extension().is_some_and(|e| e == "dat") {
            self.0.lock().unwrap().dat_entries += 3;
            Ok(3)
        } else {
            Err("no ROM entries found".into())
        }
    }
    fn dat_entries(&self) -> usize {
        self.0.lock().unwrap().dat_entries
    }
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
    fn find_bios(&self, _dir: &Path) -> Vec<PathBuf> {
        self.0.lock().unwrap().folder_bios.clone()
    }
    fn import_folder(&self, _dir: &Path) -> Vec<(PathBuf, Result<ImportedRom, String>)> {
        self.0.lock().unwrap().folder_games.clone()
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
    fn list_core_options(&self, _: &Path, _: &Path) -> Result<Vec<CoreOptionDef>, String> {
        Ok(self.0.lock().unwrap().offered.clone())
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
    let err = a
        .launch_game("/data/roms/psx/g.cue".into(), None)
        .err()
        .unwrap();
    assert!(err.starts_with("BIOS missing"), "{err}");

    {
        let mut w = f.0.lock().unwrap();
        w.bios_ok = true;
        w.cores.remove("pcsx_rearmed_libretro");
    }
    let err = a
        .launch_game("/data/roms/psx/g.cue".into(), None)
        .err()
        .unwrap();
    assert!(err.contains("not installed"), "{err}");

    let err = a.launch_game("/x/readme.txt".into(), None).err().unwrap();
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
        .launch_game("/data/roms/snes/a.sfc".into(), None)
        .err()
        .unwrap();
    assert_eq!(err, "bad rom");
}

#[test]
fn launch_tracks_play_time_until_exit() {
    let f = ready_world();
    let a = app(&f);
    let running = a
        .launch_game("/data/roms/snes/a.sfc".into(), None)
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

struct FakeMachine(f32, u32);

impl Machine for FakeMachine {
    fn info(&self) -> MachineInfo {
        MachineInfo {
            cpu_name: "Test CPU".into(),
            cpu_cores: self.1,
            ram_gb: self.0,
            os: "test".into(),
        }
    }
}

#[test]
fn three_d_systems_are_off_by_default_judged_against_the_machine_and_blocked_without_a_renderer() {
    let f = ready_world();
    f.0.lock().unwrap().scans.insert(
        "/data/roms".into(),
        vec![entry("n64", "/data/roms/n64/g.z64")],
    );
    let a = app(&f).with_machine(Box::new(FakeMachine(4.0, 8)));
    let ov = a.overview();
    assert_eq!(ov.machine.as_ref().unwrap().cpu_cores, 8);
    let n64 = ov.systems.iter().find(|s| s.id == "n64").unwrap();
    assert!(
        n64.hardware && !n64.enabled,
        "off until the user turns it on"
    );
    let verdict = n64.assessment.as_ref().unwrap();
    assert_eq!(verdict.level, Level::Marginal, "{verdict:?}"); // 4 GB = the minimum
    assert!(ov
        .systems
        .iter()
        .find(|s| s.id == "nes")
        .unwrap()
        .assessment
        .is_none());

    a.set_system_enabled("n64".into(), true);
    assert_eq!(f.0.lock().unwrap().settings.hardware_on, vec!["n64"]);
    assert!(f.0.lock().unwrap().settings.disabled.is_empty());
    let ov = a.overview();
    assert!(ov.systems.iter().find(|s| s.id == "n64").unwrap().enabled);
    // switched on, but this build cannot draw 3D yet: the game says why
    assert!(ov.games[0]
        .blocked
        .as_deref()
        .unwrap()
        .contains("3D renderer"));
    a.set_system_enabled("n64".into(), false);
    assert!(f.0.lock().unwrap().settings.hardware_on.is_empty());

    // without a measurement there is no verdict
    let bare = app(&f).overview();
    assert!(bare.machine.is_none());
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

fn offered(key: &str, values: &[&str]) -> CoreOptionDef {
    CoreOptionDef {
        key: key.into(),
        description: format!("{key} option"),
        values: values.iter().map(|v| v.to_string()).collect(),
    }
}

#[test]
fn core_options_merge_saved_choices_and_save_only_overrides() {
    let f = ready_world();
    f.0.lock().unwrap().offered = vec![
        offered("region", &["auto", "pal", "ntsc"]),
        offered("overscan", &["off", "on"]),
        offered("empty", &[]),
    ];
    let a = app(&f);
    let opts = a.core_options("snes9x_libretro").unwrap();
    assert_eq!(opts.len(), 2, "options without values are hidden");
    assert!(opts.iter().all(|o| o.value == o.default));

    a.set_core_options(
        "snes9x_libretro",
        [("region".to_string(), "pal".to_string())].into(),
    )
    .unwrap();
    let opts = a.core_options("snes9x_libretro").unwrap();
    assert_eq!(opts[0].value, "pal");
    assert_eq!(opts[1].value, "off");

    // a saved value the core no longer offers falls back to the default
    f.0.lock().unwrap().core_options.insert(
        "snes9x_libretro".into(),
        [("region".to_string(), "mars".to_string())].into(),
    );
    assert_eq!(a.core_options("snes9x_libretro").unwrap()[0].value, "auto");

    // unknown cores and values that cannot be stored are refused
    assert!(a.core_options("../x").is_err());
    assert!(a.set_core_options("../x", BTreeMap::new()).is_err());
    let bad = [("k".to_string(), "a\nb=1".to_string())].into();
    assert!(a.set_core_options("snes9x_libretro", bad).is_err());
    let bad = [("a=b".to_string(), "x".to_string())].into();
    assert!(a.set_core_options("snes9x_libretro", bad).is_err());
}

#[test]
fn core_options_need_the_core_to_be_installed() {
    let f = Fake::default();
    f.0.lock().unwrap().runner = true;
    let err = app(&f).core_options("snes9x_libretro").unwrap_err();
    assert!(err.contains("not installed"));
}

#[test]
fn a_game_starts_from_the_chosen_save_slot() {
    let f = ready_world();
    f.0.lock().unwrap().slots = vec![SlotInfo {
        slot: 2,
        saved_at: 5,
        has_thumb: true,
    }];
    let a = app(&f);
    assert_eq!(a.save_slots("/data/roms/snes/a.sfc")[0].slot, 2);
    assert_eq!(
        a.slot_thumbnail("/data/roms/snes/a.sfc", 1),
        Some(vec![0x89, b'P'])
    );

    a.launch_game("/data/roms/snes/a.sfc".into(), Some(2)).ok();
    a.launch_game("/data/roms/snes/a.sfc".into(), None).ok();
    let spawned = f.0.lock().unwrap().spawned.clone();
    assert_eq!(spawned[0].load_slot, Some(2));
    assert_eq!(spawned[1].load_slot, None);
}

#[test]
fn games_get_official_names_from_dats_and_duplicates_are_flagged() {
    let f = ready_world();
    {
        let mut w = f.0.lock().unwrap();
        w.scans.insert(
            "/data/roms".into(),
            vec![
                entry("snes", "/data/roms/snes/a.sfc"),
                entry("snes", "/data/roms/snes/a copy.sfc"),
                entry("snes", "/data/roms/snes/b.sfc"),
            ],
        );
        w.names
            .insert("/data/roms/snes/a.sfc".into(), "Super A (USA)".into());
        w.names
            .insert("/data/roms/snes/a copy.sfc".into(), "Super A (USA)".into());
    }
    let a = app(&f);
    // no DAT added yet: nothing to look up
    assert_eq!(a.identify_games(), 0);
    assert!(a.overview().games.iter().all(|g| !g.identified));
    assert_eq!(a.overview().dat_entries, 0);

    let out = a.import_dats(vec!["/x/snes.dat".into(), "/x/readme.txt".into()]);
    assert!(out[0].ok && !out[1].ok);
    assert_eq!(a.identify_games(), 2);
    assert_eq!(a.identify_games(), 0, "already looked up: no second pass");

    let ov = a.overview();
    assert_eq!(ov.dat_entries, 3);
    let by = |p: &str| ov.games.iter().find(|g| g.path == p).unwrap().clone();
    let a1 = by("/data/roms/snes/a.sfc");
    assert_eq!(
        (a1.title.as_str(), a1.identified, a1.duplicate),
        ("Super A (USA)", true, true)
    );
    let b = by("/data/roms/snes/b.sfc");
    assert_eq!(
        (b.title.as_str(), b.identified, b.duplicate),
        ("g", false, false)
    );
    // the name is saved
    assert_eq!(
        f.0.lock().unwrap().meta["/data/roms/snes/a.sfc"]
            .name
            .as_deref(),
        Some("Super A (USA)")
    );
}

#[test]
fn collections_are_cleaned_saved_and_listed() {
    let f = ready_world();
    f.0.lock().unwrap().scans.insert(
        "/data/roms".into(),
        vec![entry("snes", "/data/roms/snes/a.sfc")],
    );
    let a = app(&f);
    let tags = a.set_tags(
        "/data/roms/snes/a.sfc".into(),
        vec![" RPG ".into(), "rpg".into(), "".into(), "Co-op".into()],
    );
    assert_eq!(tags, vec!["RPG", "Co-op"]);
    let ov = a.overview();
    assert_eq!(ov.games[0].tags, vec!["RPG", "Co-op"]);
    assert_eq!(ov.collections, vec!["Co-op", "RPG"]);
    assert!(a
        .set_tags("/data/roms/snes/a.sfc".into(), vec![])
        .is_empty());
    assert!(a.overview().collections.is_empty());
}

struct FakeArt {
    f: Fake,
    has: bool,
}

impl ArtSource for FakeArt {
    fn id(&self) -> &'static str {
        "fake"
    }
    fn boxart(&self, q: &ArtQuery) -> Result<Option<Vec<u8>>, String> {
        let (folder, name) = (q.thumbnails, q.name);
        self.f
            .0
            .lock()
            .unwrap()
            .art_calls
            .push((folder.to_string(), name.to_string()));
        Ok(self.has.then(|| b"\x89PNG".to_vec()))
    }
}

fn art_world() -> Fake {
    let f = ready_world();
    {
        let mut w = f.0.lock().unwrap();
        w.scans.insert(
            "/data/roms".into(),
            vec![
                entry("snes", "/data/roms/snes/a.sfc"),
                entry("snes", "/data/roms/snes/unnamed.sfc"),
            ],
        );
        w.meta.insert(
            "/data/roms/snes/a.sfc".into(),
            GameMeta {
                name: Some("Super A (USA)".into()),
                ..Default::default()
            },
        );
    }
    f
}

#[test]
fn art_is_only_downloaded_when_switched_on_and_only_for_named_games() {
    let f = art_world();
    let a = app(&f).with_art_source(Box::new(FakeArt {
        f: f.clone(),
        has: true,
    }));
    // off by default: nothing is sent anywhere
    assert_eq!(a.fetch_art(|_| {}), 0);
    assert!(f.0.lock().unwrap().art_calls.is_empty());

    a.set_fetch_art(true);
    assert!(a.overview().fetch_art);
    let made = Mutex::new(Vec::new());
    assert_eq!(a.fetch_art(|k| made.lock().unwrap().push(k.to_string())), 1);
    assert_eq!(*made.lock().unwrap(), vec!["/data/roms/snes/a.sfc"]);
    assert_eq!(
        f.0.lock().unwrap().art_calls,
        vec![(
            "Nintendo - Super Nintendo Entertainment System".to_string(),
            "Super A (USA)".to_string()
        )]
    );
    // art exists now: no second request
    assert_eq!(a.fetch_art(|_| {}), 0);
    assert_eq!(f.0.lock().unwrap().art_calls.len(), 1);
}

#[test]
fn games_the_server_has_no_art_for_are_not_asked_again() {
    let f = art_world();
    let a = app(&f).with_art_source(Box::new(FakeArt {
        f: f.clone(),
        has: false,
    }));
    a.set_fetch_art(true);
    assert_eq!(a.fetch_art(|_| {}), 0);
    assert_eq!(a.fetch_art(|_| {}), 0);
    assert_eq!(f.0.lock().unwrap().art_calls.len(), 1);
}

#[test]
fn cheats_are_saved_per_game_and_size_limited() {
    let f = ready_world();
    let a = app(&f);
    assert_eq!(a.cheats("/r/a.sfc"), "");
    a.set_cheats("/r/a.sfc", "7E0DBE05\n").unwrap();
    assert_eq!(a.cheats("/r/a.sfc"), "7E0DBE05\n");
    assert_eq!(a.cheats("/r/b.sfc"), "");
    assert!(a.set_cheats("/r/a.sfc", &"x".repeat(70_000)).is_err());
}

#[test]
fn a_chosen_cover_is_kept_and_never_replaced_by_a_download() {
    let f = art_world();
    let a = app(&f).with_art_source(Box::new(FakeArt {
        f: f.clone(),
        has: true,
    }));
    a.set_fetch_art(true);
    a.set_custom_art("/data/roms/snes/a.sfc", "/pics/mine.png")
        .unwrap();
    assert!(a
        .set_custom_art("/data/roms/snes/a.sfc", "/pics/doc.txt")
        .is_err());
    // a.sfc has a name and a server picture, but the user's choice wins
    assert_eq!(a.fetch_art(|_| {}), 0);
    assert!(f.0.lock().unwrap().art_calls.is_empty());
    a.clear_art("/data/roms/snes/a.sfc");
    assert_eq!(a.fetch_art(|_| {}), 1);
}

#[test]
fn image_types_are_told_by_their_first_bytes() {
    assert_eq!(image_mime(b"\x89PNG\r\n"), Some("image/png"));
    assert_eq!(image_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
    assert_eq!(image_mime(b"GIF89a"), None);
}

struct Tagged {
    f: Fake,
    id: &'static str,
    has: bool,
}

impl ArtSource for Tagged {
    fn id(&self) -> &'static str {
        self.id
    }
    fn boxart(&self, q: &ArtQuery) -> Result<Option<Vec<u8>>, String> {
        self.f.0.lock().unwrap().art_calls.push((
            self.id.to_string(),
            format!("{}|{}|{:?}", q.name, q.screenscraper_id, q.login),
        ));
        Ok(self.has.then(|| vec![0xFF, 0xD8, 0xFF, 1]))
    }
}

#[test]
fn art_sources_are_tried_in_order_and_get_the_users_login() {
    let f = art_world();
    let a = app(&f)
        .with_art_source(Box::new(Tagged {
            f: f.clone(),
            id: "first",
            has: false,
        }))
        .with_art_source(Box::new(Tagged {
            f: f.clone(),
            id: "second",
            has: true,
        }));
    a.set_fetch_art(true);
    a.set_screenscraper("  me ".into(), "pw".into());
    let ov = a.overview();
    assert_eq!(ov.art_sources, vec!["first", "second"]);
    assert_eq!(ov.screenscraper_user.as_deref(), Some("me"));

    assert_eq!(a.fetch_art(|_| {}), 1);
    let calls = f.0.lock().unwrap().art_calls.clone();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, "first");
    assert!(
        calls[1]
            .1
            .contains("Super A (USA)|4|Some((\"me\", \"pw\"))"),
        "{calls:?}"
    );

    // an empty user name forgets the login
    a.set_screenscraper(String::new(), String::new());
    assert!(a.overview().screenscraper_user.is_none());
}

#[test]
fn a_system_can_override_window_crt_and_rewind_while_the_rest_stays_general() {
    let f = ready_world();
    let a = app(&f);
    a.set_game_settings(GameSettings {
        window_scale: 2,
        volume: 60,
        ..Default::default()
    })
    .unwrap();

    // everything: no overrides
    let all = a.scoped_settings(None).unwrap();
    assert_eq!(
        (all.effective.window_scale, all.inherited.window_scale),
        (2, 2)
    );
    assert!(all.overridden.is_empty());

    // a system: its own size and CRT, the general volume
    let gba = a
        .set_system_override(
            "gba",
            Override {
                window_scale: Some(5),
                crt: Some(true),
                rewind: None,
            },
        )
        .unwrap();
    assert_eq!(
        (
            gba.effective.window_scale,
            gba.effective.crt,
            gba.effective.volume
        ),
        (5, true, 60)
    );
    assert_eq!(gba.inherited.window_scale, 2);
    assert_eq!(gba.overridden.window_scale, Some(5));
    assert!(a
        .scoped_settings(Some("psx"))
        .unwrap()
        .overridden
        .is_empty());

    // changing the general size later still reaches systems that did not override it
    a.set_game_settings(GameSettings {
        window_scale: 3,
        volume: 60,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(
        a.scoped_settings(Some("psx"))
            .unwrap()
            .effective
            .window_scale,
        3
    );
    assert_eq!(
        a.scoped_settings(Some("gba"))
            .unwrap()
            .effective
            .window_scale,
        5
    );

    // an empty override removes the system's entry; unknown systems are refused
    a.set_system_override("gba", Override::default()).unwrap();
    assert!(f.0.lock().unwrap().overrides.systems.is_empty());
    assert!(a.set_system_override("nope", Override::default()).is_err());
    assert!(a.scoped_settings(Some("nope")).is_err());
}

#[test]
fn a_system_can_have_its_own_key_bindings_and_go_back_to_the_general_ones() {
    let f = ready_world();
    let a = app(&f);
    let general = a.controls_for(None).unwrap();
    assert!(!general.custom);
    // no bindings of its own: the general ones, not marked custom
    let snes = a.controls_for(Some("snes")).unwrap();
    assert!(!snes.custom);

    let mut own = ControlsConfig::default();
    own.players[0].deadzone = 0.5;
    let saved = a.set_controls_for("snes", own.clone()).unwrap();
    assert!(saved.custom);
    assert_eq!(
        a.controls_for(Some("snes")).unwrap().config.players[0].deadzone,
        0.5
    );
    assert!(a.controls_for(Some("snes")).unwrap().custom);
    assert!(
        !a.controls_for(Some("gba")).unwrap().custom,
        "other systems are untouched"
    );

    let back = a.reset_controls_for("snes").unwrap();
    assert!(!back.custom);
    assert!(a.set_controls_for("nope", own).is_err());
}

#[test]
fn one_game_can_override_on_top_of_its_system_and_other_games_are_untouched() {
    let f = ready_world();
    let a = app(&f);
    a.set_game_settings(GameSettings {
        window_scale: 2,
        ..Default::default()
    })
    .unwrap();
    a.set_system_override(
        "snes",
        Override {
            window_scale: Some(4),
            ..Default::default()
        },
    )
    .unwrap();

    // a SNES game with no override of its own inherits the system's size
    let mario = "/data/roms/snes/Super Mario World.sfc";
    let g = a.game_override(mario);
    assert_eq!((g.effective.window_scale, g.inherited.window_scale), (4, 4));
    assert!(g.overridden.is_empty());

    // the game takes its own size and CRT; its system and sibling games keep theirs
    let g = a
        .set_game_override(
            mario,
            Override {
                window_scale: Some(6),
                crt: Some(true),
                rewind: None,
            },
        )
        .unwrap();
    assert_eq!((g.effective.window_scale, g.effective.crt), (6, true));
    assert_eq!(
        g.inherited.window_scale, 4,
        "what it falls back to is the system's size"
    );
    assert!(f
        .0
        .lock()
        .unwrap()
        .overrides
        .games
        .contains_key("Super Mario World"));
    let other = a.game_override("/data/roms/snes/Zelda.sfc");
    assert_eq!(
        (other.effective.window_scale, other.effective.crt),
        (4, false)
    );
    assert_eq!(
        a.scoped_settings(Some("snes"))
            .unwrap()
            .effective
            .window_scale,
        4
    );
    // a game of another system inherits the general size
    assert_eq!(
        a.game_override("/data/roms/gba/Metroid.gba")
            .effective
            .window_scale,
        2
    );

    // clearing everything removes the entry
    a.set_game_override(mario, Override::default()).unwrap();
    assert!(f.0.lock().unwrap().overrides.games.is_empty());
}

#[test]
fn key_bindings_resolve_game_then_system_then_general() {
    let f = ready_world();
    let a = app(&f);
    let zelda = "/data/roms/snes/Zelda.sfc";
    let mario = "/data/roms/snes/Mario.sfc";
    let with_dz = |v: f32| {
        let mut c = ControlsConfig::default();
        c.players[0].deadzone = v;
        c
    };
    let dz = |s: ScopedControls| (s.config.players[0].deadzone, s.custom);
    let general = ControlsConfig::default().players[0].deadzone;

    // nothing customised: the general bindings, not marked custom
    assert_eq!(dz(a.controls_for_game(zelda)), (general, false));

    // the SNES has its own: every SNES game follows it, other systems do not
    a.set_controls_for("snes", with_dz(0.3)).unwrap();
    assert_eq!(
        dz(a.controls_for_game(zelda)),
        (0.3, false),
        "inherited, not the game's own"
    );
    assert_eq!(
        dz(a.controls_for_game("/data/roms/gba/Metroid.gba")),
        (general, false)
    );

    // a game of its own wins over its system, and only for that game
    let saved = a.set_controls_for_game(zelda, with_dz(0.7)).unwrap();
    assert!(saved.custom);
    assert_eq!(dz(a.controls_for_game(zelda)), (0.7, true));
    assert_eq!(dz(a.controls_for_game(mario)), (0.3, false));

    // dropping the game's bindings falls back to the system's
    assert_eq!(dz(a.reset_controls_for_game(zelda).unwrap()), (0.3, false));
}

#[test]
fn importing_a_collection_also_installs_the_bios_files_inside_it() {
    let f = ready_world();
    {
        let mut w = f.0.lock().unwrap();
        w.folder_games = vec![(
            PathBuf::from("/dl/Neo-Geo/mslug.zip"),
            Ok(ImportedRom {
                system: "neogeo",
                dest: "/data/roms/neogeo/mslug.zip".into(),
                already_there: false,
            }),
        )];
        w.folder_bios = vec![PathBuf::from("/dl/Neo-Geo/neogeo.zip")];
    }
    let out = app(&f).import_roms(vec!["/dl/Neo-Geo".into()], None);
    assert_eq!(out.len(), 2);
    assert!(out[0].ok && out[0].message.starts_with("added"));
    assert!(out[1].ok, "{out:?}");
    assert_eq!(out[1].message, "BIOS installed: neogeo.zip");

    // a folder that holds nothing but a BIOS is not an error
    f.0.lock().unwrap().folder_games.clear();
    let out = app(&f).import_roms(vec!["/dl/Neo-Geo".into()], None);
    assert_eq!(out.len(), 1);
    assert!(out[0].ok);

    // a folder with neither still says so
    f.0.lock().unwrap().folder_bios.clear();
    let out = app(&f).import_roms(vec!["/dl/Empty".into()], None);
    assert!(!out[0].ok && out[0].message.contains("no games"));
}

#[test]
fn a_bios_file_chosen_as_a_game_is_installed_as_a_bios_instead_of_refused() {
    let f = ready_world();
    let out = app(&f).import_roms(vec!["/dl/Neo-Geo/neogeo.zip".into()], None);
    assert!(out[0].ok, "{out:?}");
    assert_eq!(out[0].message, "BIOS installed: neogeo.zip");
}
