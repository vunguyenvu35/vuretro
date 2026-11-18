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

/// What we remember about one game, keyed by its path.
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct GameMeta {
    pub last_played: u64,
    pub play_secs: u64,
    pub favorite: bool,
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

/// One option a core offers. The first value is the core's default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoreOptionDef {
    pub key: String,
    pub description: String,
    pub values: Vec<String>,
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
                let default = d.values[0].clone();
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
            enabled: !settings.disabled.iter().any(|d| d == sys.id),
            core_installed,
            bios_ready: bios_ready(&bios),
            bios,
            game_count,
        }
    }

    /// Why games of this system cannot be played right now, if they cannot.
    pub fn blocked_reason(&self) -> Option<String> {
        if !self.enabled {
            Some(format!("{} is switched off", self.name))
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
}

/// Join scanned games with their system status and bookkeeping. Games of a system that
/// is not in `systems` are dropped.
pub fn game_views(
    scanned: Vec<GameEntry>,
    systems: &[SystemView],
    meta: &HashMap<String, GameMeta>,
    roms_dir: &Path,
) -> Vec<GameView> {
    scanned
        .into_iter()
        .filter_map(|g| {
            let sys = systems.iter().find(|s| s.id == g.system)?;
            let key = g.path.to_string_lossy().into_owned();
            let m = meta.get(&key).cloned().unwrap_or_default();
            Some(GameView {
                blocked: sys.blocked_reason(),
                imported: g.path.starts_with(roms_dir),
                system: g.system,
                title: g.title,
                path: key,
                size: g.size,
                last_played: m.last_played,
                play_secs: m.play_secs,
                favorite: m.favorite,
            })
        })
        .collect()
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

    #[test]
    fn blocked_reason_order_is_off_then_core_then_bios() {
        let on = Settings::default();
        let off = Settings {
            disabled: vec!["psx".into()],
            ..Default::default()
        };
        assert_eq!(
            view("psx", &off, false, false, 0).blocked_reason().unwrap(),
            "PlayStation is switched off"
        );
        assert!(view("psx", &on, false, false, 0)
            .blocked_reason()
            .unwrap()
            .starts_with("core pcsx_rearmed_libretro not installed"));
        assert_eq!(
            view("psx", &on, true, false, 0).blocked_reason().unwrap(),
            "BIOS missing"
        );
        assert_eq!(view("psx", &on, true, true, 0).blocked_reason(), None);
        // SNES needs no BIOS
        assert_eq!(view("snes", &on, true, false, 0).blocked_reason(), None);
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
