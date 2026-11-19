//! Domain types and the rules that need no I/O.

use crate::registry::System;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

/// App settings the user changes.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct Settings {
    /// A ROM folder the user linked (read in place, never copied).
    pub rom_dir: Option<String>,
    /// System ids the user switched off.
    #[serde(default)]
    pub disabled: Vec<String>,
    /// Download cover art for named games (sends game names to the libretro-thumbnails server).
    #[serde(default)]
    pub fetch_art: bool,
    /// ScreenScraper account of the user (stored as plain text in settings.json).
    #[serde(default)]
    pub screenscraper: Option<ScreenScraperLogin>,
    /// 3D systems the user switched on (they are off until then).
    #[serde(default)]
    pub hardware_on: Vec<String>,
    /// Core the user picked per system id (only values listed by the registry count).
    #[serde(default)]
    pub cores: BTreeMap<String, String>,
}

impl Settings {
    /// The core that runs `sys`: the user's pick if the registry lists it, else the default.
    pub fn core_for(&self, sys: &System) -> &'static str {
        self.cores
            .get(sys.id)
            .and_then(|c| sys.all_cores().find(|k| *k == c.as_str()))
            .unwrap_or(sys.core)
    }
}

/// A ScreenScraper account.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScreenScraperLogin {
    pub user: String,
    pub password: String,
}

/// What we remember about one game, keyed by its path.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct GameMeta {
    pub last_played: u64,
    pub play_secs: u64,
    pub favorite: bool,
    /// Official name found by hashing the file against the user's DAT files.
    #[serde(default)]
    pub name: Option<String>,
    /// Collections the user put the game in.
    #[serde(default)]
    pub tags: Vec<String>,
    /// The core this game runs on instead of its system's (stem, e.g. `mame2003_libretro`).
    #[serde(default)]
    pub core: Option<String>,
}

/// A ROM found on disk.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GameEntry {
    pub system: String,
    pub title: String,
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BiosState {
    Ok,
    Missing,
    /// Present but MD5 not in the accepted list.
    BadHash {
        md5: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct BiosStatus {
    pub system: &'static str,
    pub file: &'static str,
    pub required: bool,
    pub note: &'static str,
    pub state: BiosState,
}

/// True when every required BIOS is OK.
pub fn bios_ready(status: &[BiosStatus]) -> bool {
    status
        .iter()
        .all(|b| !b.required || b.state == BiosState::Ok)
}

/// A BIOS file that was installed.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ImportedBios {
    pub system: &'static str,
    pub file: &'static str,
    /// True when the content matched a known MD5 (or the name matched a presence-only set).
    pub verified: bool,
}

/// A ROM that was copied into the library.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedRom {
    pub system: &'static str,
    pub dest: PathBuf,
    /// The same file (same size) was already in the library; nothing was copied.
    pub already_there: bool,
}

/// Result of one item of a bulk operation (import, remove).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Outcome {
    pub source: String,
    pub ok: bool,
    pub message: String,
}

impl Outcome {
    pub fn ok(source: impl Into<String>, message: impl Into<String>) -> Self {
        Outcome {
            source: source.into(),
            ok: true,
            message: message.into(),
        }
    }
    pub fn failed(source: impl Into<String>, message: impl Into<String>) -> Self {
        Outcome {
            source: source.into(),
            ok: false,
            message: message.into(),
        }
    }
}

/// Number of save-state slots per game (the runner uses the same).
pub const SAVE_SLOTS: u8 = 4;

/// A save-state slot that holds a save.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SlotInfo {
    pub slot: u8,
    /// Unix seconds of the last save.
    pub saved_at: u64,
    pub has_thumb: bool,
}

/// Largest cover image accepted.
pub const MAX_ART_BYTES: usize = 8 * 1024 * 1024;

/// MIME type of a PNG or JPEG by its first bytes; `None` for anything else.
pub fn image_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG") {
        Some("image/png")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else {
        None
    }
}

/// What the computer offers, as far as it can be measured.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MachineInfo {
    pub cpu_name: String,
    pub cpu_cores: u32,
    pub ram_gb: f32,
    pub os: String,
}

/// How well a computer is likely to run a system, worst first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Level {
    /// Below the minimum: expect it not to run, or to lag badly.
    Poor,
    /// Meets the minimum only: some games will lag.
    Marginal,
    Good,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Assessment {
    pub level: Level,
    pub reasons: Vec<String>,
}

/// Judge whether `machine` can run `sys`. Only RAM and CPU cores are measured; the graphics
/// chip is not, so a "good" verdict still carries a reminder about it.
pub fn assess(sys: &System, machine: &MachineInfo) -> Assessment {
    let mut level = Level::Good;
    let mut reasons = Vec::new();
    let mut worse = |l: Level, why: String| {
        level = level.min(l);
        reasons.push(why);
    };
    if sys.min_ram_gb > 0 {
        let (have, need) = (machine.ram_gb, sys.min_ram_gb as f32);
        if have + 0.5 < need {
            worse(
                Level::Poor,
                format!("needs at least {need:.0} GB of memory, this computer has {have:.0} GB"),
            );
        } else if have < need * 1.5 {
            worse(
                Level::Marginal,
                format!(
                    "{have:.0} GB of memory is just enough; {:.0} GB or more is comfortable",
                    need * 1.5
                ),
            );
        }
    }
    if sys.min_cores > 0 {
        let (have, need) = (machine.cpu_cores, sys.min_cores);
        if have < need {
            worse(
                Level::Poor,
                format!("needs at least {need} processor cores, this computer has {have}"),
            );
        } else if have < need + 2 {
            worse(
                Level::Marginal,
                format!(
                    "{have} processor cores is just enough; {} or more is comfortable",
                    need + 2
                ),
            );
        }
    }
    if sys.hardware {
        reasons.push(
            "the graphics chip cannot be checked: an old or weak one can still make games lag"
                .to_string(),
        );
    }
    Assessment { level, reasons }
}

/// Settings as one scope sees them: what applies, what it would inherit, and what it overrides.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ScopedSettings {
    pub effective: vuretro_settings::GameSettings,
    /// What the "recommended look" button would set (a system only; `None` for all games and
    /// for systems with no recommendation).
    pub recommended: Option<vuretro_settings::scoped::Override>,
    /// The values this scope would use if it overrode nothing.
    pub inherited: vuretro_settings::GameSettings,
    pub overridden: vuretro_settings::scoped::Override,
}

/// Key bindings as one scope sees them.
#[derive(Debug, Clone, Serialize)]
pub struct ScopedControls {
    pub config: vuretro_controls::ControlsConfig,
    /// The system has bindings of its own (otherwise it uses the general ones).
    pub custom: bool,
}

/// One option a core offers. The first value is the core's default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoreOptionDef {
    pub key: String,
    pub description: String,
    pub values: Vec<String>,
    /// What the runner sets for this option unless the user chose otherwise (for example
    /// FBNeo in Neo Geo AES mode). It replaces the core's own default in what is shown.
    #[serde(default)]
    pub frontend_default: Option<String>,
}

/// An option as the options screen shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CoreOptionView {
    pub key: String,
    pub description: String,
    pub values: Vec<String>,
    pub default: String,
    /// The saved override, or the default.
    pub value: String,
}

impl CoreOptionView {
    /// Join what the core offers with what the user saved. A saved value the core no
    /// longer offers is ignored.
    pub fn merge(defs: Vec<CoreOptionDef>, saved: &BTreeMap<String, String>) -> Vec<Self> {
        defs.into_iter()
            .filter(|d| !d.values.is_empty())
            .map(|d| {
                // what a game really runs with: the runner's own default, else the core's
                let default = d
                    .frontend_default
                    .clone()
                    .filter(|v| d.values.contains(v))
                    .unwrap_or_else(|| d.values[0].clone());
                let value = saved
                    .get(&d.key)
                    .filter(|v| d.values.contains(v))
                    .cloned()
                    .unwrap_or_else(|| default.clone());
                CoreOptionView {
                    key: d.key,
                    description: d.description,
                    values: d.values,
                    default,
                    value,
                }
            })
            .collect()
    }
}

// ---------- what the UI shows ----------

#[derive(Debug, Clone, Serialize)]
pub struct CoreChoice {
    pub name: &'static str,
    pub installed: bool,
}

/// A move written in notation, as the sequence of steps it plays.
#[derive(Debug, Clone, Serialize)]
pub struct CompiledMove {
    pub steps: Vec<vuretro_controls::macros::Step>,
    pub total_ms: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemView {
    pub id: &'static str,
    pub name: &'static str,
    pub core: &'static str,
    /// Every core that can run this system, with whether it is installed.
    pub core_choices: Vec<CoreChoice>,
    pub extensions: &'static [&'static str],
    /// Switched on by the user (Cores & BIOS screen).
    pub enabled: bool,
    pub core_installed: bool,
    pub bios: Vec<BiosStatus>,
    pub bios_ready: bool,
    pub game_count: usize,
    /// A 3D system: needs a GPU renderer and a fast computer.
    pub hardware: bool,
    /// An arcade system: games need a coin before Start works.
    pub coin_op: bool,
    /// How this computer is likely to cope (3D systems only, when it could be measured).
    pub assessment: Option<Assessment>,
    /// False while the runner cannot draw this kind of system.
    pub renderer_ready: bool,
}

impl SystemView {
    pub fn new(
        sys: &'static System,
        settings: &Settings,
        core_installed: bool,
        core_choices: Vec<CoreChoice>,
        bios: Vec<BiosStatus>,
        game_count: usize,
    ) -> Self {
        SystemView {
            id: sys.id,
            name: sys.name,
            core: settings.core_for(sys),
            core_choices,
            extensions: sys.extensions,
            enabled: if sys.hardware {
                settings.hardware_on.iter().any(|d| d == sys.id)
            } else {
                !settings.disabled.iter().any(|d| d == sys.id)
            },
            core_installed,
            bios_ready: bios_ready(&bios),
            bios,
            game_count,
            hardware: sys.hardware,
            coin_op: sys.coin_op,
            assessment: None,
            renderer_ready: !sys.hardware,
        }
    }

    /// Why games of this system cannot be played right now, if they cannot.
    pub fn blocked_reason(&self) -> Option<String> {
        if !self.enabled {
            Some(format!("{} is switched off", self.name))
        } else if !self.renderer_ready {
            Some(format!(
                "{} needs the 3D renderer, which this version of vuretro does not have yet",
                self.name
            ))
        } else if !self.core_installed {
            Some(format!(
                "core {} not installed (install it in Cores & BIOS)",
                self.core
            ))
        } else if !self.bios_ready {
            Some("BIOS missing".to_string())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GameView {
    pub system: String,
    pub title: String,
    pub path: String,
    pub size: u64,
    pub last_played: u64,
    pub play_secs: u64,
    pub favorite: bool,
    /// Reason the Play button is locked, if any.
    pub blocked: Option<String>,
    /// Imported into the app's library (can be removed from it), not just linked.
    pub imported: bool,
    /// The title came from a DAT file, not from the file name.
    pub identified: bool,
    /// Another file in the library has the same official title.
    pub duplicate: bool,
    pub tags: Vec<String>,
    /// The core chosen for this game alone; `None` = the one of its system.
    pub core: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Overview {
    pub data_dir: String,
    pub system_dir: String,
    pub rom_dir: Option<String>,
    pub systems: Vec<SystemView>,
    pub games: Vec<GameView>,
    pub runner_found: bool,
    pub demo_available: bool,
    /// This computer, if it could be measured.
    pub machine: Option<MachineInfo>,
    /// Dumps known from the user's DAT files (0 = none added).
    pub dat_entries: usize,
    pub fetch_art: bool,
    /// Cover art sources this build offers, in the order they are tried.
    pub art_sources: Vec<&'static str>,
    /// The ScreenScraper user name saved, if any (never the password).
    pub screenscraper_user: Option<String>,
    /// Collection names in use, sorted.
    pub collections: Vec<String>,
}

/// Join scanned games with their system status and bookkeeping. Games of a system that
/// is not in `systems` are dropped.
pub fn game_views(
    scanned: Vec<GameEntry>,
    systems: &[SystemView],
    meta: &HashMap<String, GameMeta>,
    roms_dir: &Path,
) -> Vec<GameView> {
    let mut views: Vec<GameView> = scanned
        .into_iter()
        .filter_map(|g| {
            let sys = systems.iter().find(|s| s.id == g.system)?;
            let key = g.path.to_string_lossy().into_owned();
            let m = meta.get(&key).cloned().unwrap_or_default();
            Some(GameView {
                blocked: sys.blocked_reason(),
                imported: g.path.starts_with(roms_dir),
                system: g.system,
                title: m.name.clone().unwrap_or(g.title),
                path: key,
                size: g.size,
                last_played: m.last_played,
                play_secs: m.play_secs,
                favorite: m.favorite,
                identified: m.name.is_some(),
                duplicate: false,
                tags: m.tags,
                core: m.core,
            })
        })
        .collect();
    // Games with the same official title (in the same system) are probably the same dump.
    let mut seen: HashMap<(String, String), usize> = HashMap::new();
    for v in views.iter().filter(|v| v.identified) {
        *seen.entry((v.system.clone(), v.title.clone())).or_default() += 1;
    }
    for v in views.iter_mut().filter(|v| v.identified) {
        v.duplicate = seen[&(v.system.clone(), v.title.clone())] > 1;
    }
    views
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry;

    fn view(id: &str, settings: &Settings, core: bool, bios_ok: bool, n: usize) -> SystemView {
        let sys = registry::by_id(id).unwrap();
        let bios = sys
            .bios
            .iter()
            .map(|b| BiosStatus {
                system: sys.id,
                file: b.file,
                required: b.required,
                note: b.note,
                state: if bios_ok {
                    BiosState::Ok
                } else {
                    BiosState::Missing
                },
            })
            .collect();
        SystemView::new(sys, settings, core, Vec::new(), bios, n)
    }

    fn machine(ram_gb: f32, cores: u32) -> MachineInfo {
        MachineInfo {
            cpu_name: "Test CPU".into(),
            cpu_cores: cores,
            ram_gb,
            os: "test".into(),
        }
    }

    #[test]
    fn machines_are_judged_by_memory_and_cores_with_the_worst_verdict_winning() {
        let psp = registry::by_id("psp").unwrap(); // 8 GB, 4 cores
        assert_eq!(assess(psp, &machine(16.0, 8)).level, Level::Good);
        let a = assess(psp, &machine(8.0, 8));
        assert_eq!(a.level, Level::Marginal, "{a:?}");
        let a = assess(psp, &machine(4.0, 8));
        assert_eq!(a.level, Level::Poor);
        assert!(a.reasons[0].contains("at least 8 GB") && a.reasons[0].contains("4 GB"));
        assert_eq!(assess(psp, &machine(32.0, 2)).level, Level::Poor);
        assert_eq!(assess(psp, &machine(32.0, 5)).level, Level::Marginal);
        // both low: both reasons are listed
        let a = assess(psp, &machine(4.0, 2));
        assert_eq!(
            a.reasons.iter().filter(|r| r.contains("at least")).count(),
            2
        );
        // the graphics chip is never measured, so 3D systems always say so
        assert!(assess(psp, &machine(64.0, 16))
            .reasons
            .iter()
            .any(|r| r.contains("graphics")));
        // a system with no requirements is fine anywhere
        let nes = registry::by_id("nes").unwrap();
        let a = assess(nes, &machine(0.5, 1));
        assert_eq!((a.level, a.reasons.len()), (Level::Good, 0));
    }

    #[test]
    fn blocked_reason_order_is_off_then_core_then_bios() {
        let on = Settings::default();
        let off = Settings {
            disabled: vec!["neogeo".into()],
            ..Default::default()
        };
        assert_eq!(
            view("neogeo", &off, false, false, 0).blocked_reason().unwrap(),
            "Neo Geo is switched off"
        );
        assert!(view("neogeo", &on, false, false, 0)
            .blocked_reason()
            .unwrap()
            .starts_with("core fbneo_libretro not installed"));
        assert_eq!(
            view("neogeo", &on, true, false, 0).blocked_reason().unwrap(),
            "BIOS missing"
        );
        assert_eq!(view("neogeo", &on, true, true, 0).blocked_reason(), None);
        // SNES needs no BIOS
        assert_eq!(view("snes", &on, true, false, 0).blocked_reason(), None);
    }

    #[test]
    fn playstation_plays_without_a_bios_file() {
        // the core has a built-in one, so the files only make it more compatible
        let on = Settings::default();
        assert_eq!(view("psx", &on, true, false, 0).blocked_reason(), None);
        assert!(view("psx", &on, true, false, 0).bios_ready);
    }

    #[test]
    fn optional_bios_does_not_block() {
        let on = Settings::default();
        // gba's BIOS is optional, so "missing" is fine
        assert!(view("gba", &on, true, false, 0).bios_ready);
    }

    #[test]
    fn game_views_join_meta_and_flag_imported() {
        let on = Settings::default();
        let systems = vec![view("snes", &on, true, true, 2)];
        let entry = |sys: &str, path: &str| GameEntry {
            system: sys.into(),
            title: "t".into(),
            path: PathBuf::from(path),
            size: 1,
        };
        let mut meta = HashMap::new();
        meta.insert(
            "/lib/snes/a.sfc".to_string(),
            GameMeta {
                last_played: 5,
                play_secs: 60,
                favorite: true,
                ..Default::default()
            },
        );
        let games = game_views(
            vec![
                entry("snes", "/lib/snes/a.sfc"),
                entry("snes", "/elsewhere/b.sfc"),
                entry("psx", "/lib/psx/c.cue"), // system not shown: dropped
            ],
            &systems,
            &meta,
            Path::new("/lib"),
        );
        assert_eq!(games.len(), 2);
        assert!(games[0].imported && games[0].favorite && games[0].play_secs == 60);
        assert!(!games[1].imported && !games[1].favorite);
    }
}

/// The look the "Use the recommended look" button gives a system: whole-number scaling for
/// sharp pixels, plus the LCD grid for handhelds or CRT scanlines for TV and arcade systems.
/// `None` for systems with no recommendation (3D systems, which are not pixel art).
/// Full screen is a habit of the player, so it is not part of it.
pub fn recommended_override(sys: &System) -> Option<vuretro_settings::scoped::Override> {
    if sys.look.is_empty() {
        return None;
    }
    Some(vuretro_settings::scoped::Override {
        integer_scale: Some(true),
        crt: Some(sys.look == "crt"),
        lcd: Some(sys.look == "lcd"),
        ..Default::default()
    })
}

#[cfg(test)]
mod recommended_tests {
    use super::*;
    use crate::registry;

    fn rec(id: &str) -> Option<vuretro_settings::scoped::Override> {
        recommended_override(registry::by_id(id).unwrap())
    }

    #[test]
    fn handhelds_get_the_lcd_grid_and_tv_systems_get_scanlines() {
        let gba = rec("gba").unwrap();
        assert_eq!(
            (gba.integer_scale, gba.lcd, gba.crt),
            (Some(true), Some(true), Some(false))
        );
        let neo = rec("neogeo").unwrap();
        assert_eq!(
            (neo.integer_scale, neo.lcd, neo.crt),
            (Some(true), Some(false), Some(true))
        );
        // a system with no look of its own still gets sharp pixels, and neither effect
        let vb = rec("virtualboy").unwrap();
        assert_eq!(
            (vb.integer_scale, vb.lcd, vb.crt),
            (Some(true), Some(false), Some(false))
        );
        // only the display-look fields are touched: nothing about sound, speed or rewind
        assert_eq!(gba.rewind, None);
        assert_eq!(gba.window_scale, None);
    }

    #[test]
    fn three_d_systems_have_no_recommendation_and_every_pixel_system_has_one() {
        for s in registry::systems() {
            assert_eq!(recommended_override(s).is_some(), !s.hardware, "{}", s.id);
            assert!(["", "crt", "lcd", "none"].contains(&s.look), "{}", s.id);
        }
    }
}
