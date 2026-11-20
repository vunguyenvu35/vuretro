//! Backup and export: what goes into a backup, the rules a backup file must follow to be
//! restored safely, and the library list as CSV. Pure; the zip work is an adapter.

use crate::model::GameView;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

/// Version of the backup layout. A newer one is refused by an older app.
pub const FORMAT: u32 = 1;

/// Top-level entries of the data folder a backup may hold. Anything else in a backup file
/// is ignored on restore (ROMs, BIOS and cores are never part of one).
pub const ALLOWED: &[&str] = &[
    "settings.json",
    "library.json",
    "config",
    "dats",
    "saves",
    "covers",
];

/// What the user chose to include.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default)]
pub struct BackupOptions {
    /// Save RAM, save states and slot pictures.
    pub saves: bool,
    /// Preview pictures and downloaded or chosen cover art (large, and the previews can be
    /// made again, so off by default).
    pub covers: bool,
}

impl Default for BackupOptions {
    fn default() -> Self {
        BackupOptions {
            saves: true,
            covers: false,
        }
    }
}

impl BackupOptions {
    /// The data-folder entries these options put in a backup.
    pub fn includes(&self) -> Vec<&'static str> {
        let mut v = vec!["settings.json", "library.json", "config", "dats"];
        if self.saves {
            v.push("saves");
        }
        if self.covers {
            v.push("covers");
        }
        v
    }
}

/// First file of every backup: says what it is, so a wrong file is refused early.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupManifest {
    pub format: u32,
    pub app_version: String,
    /// Unix seconds.
    pub created: u64,
    pub includes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BackupSummary {
    pub path: String,
    pub files: usize,
    pub bytes: u64,
}

/// The path of a backup entry inside the data folder, if it is safe to write there: it must
/// start with `data/`, stay inside it (no `..`, no absolute or drive paths, no backslash
/// tricks) and begin with one of the [`ALLOWED`] entries.
pub fn restore_target(entry: &str) -> Option<PathBuf> {
    let rel = entry.strip_prefix("data/")?;
    if rel.is_empty() || rel.contains('\\') || rel.contains('\0') || rel.contains(':') {
        return None;
    }
    let path = Path::new(rel);
    let mut parts = path.components();
    let first = match parts.next()? {
        Component::Normal(p) => p.to_str()?,
        _ => return None,
    };
    if !ALLOWED.contains(&first) || !parts.all(|c| matches!(c, Component::Normal(_))) {
        return None;
    }
    Some(path.to_path_buf())
}

/// Refuse a manifest this version cannot restore.
pub fn check_manifest(m: &BackupManifest) -> Result<(), String> {
    if m.format > FORMAT {
        return Err(format!(
            "this backup was made by a newer Vuretro (backup format {}); update the app to restore it",
            m.format
        ));
    }
    Ok(())
}

fn csv_field(s: &str) -> String {
    // Spreadsheets run text that starts with = + - @ as a formula: defuse it.
    let s = if s.starts_with(['=', '+', '-', '@']) {
        format!("'{s}")
    } else {
        s.to_string()
    };
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

/// The library as a spreadsheet: one row per game.
pub fn library_csv(games: &[GameView], system_name: impl Fn(&str) -> String) -> String {
    let mut out = String::from(
        "title,system,file,size_bytes,play_minutes,last_played_unix,favorite,duplicate,collections\n",
    );
    for g in games {
        let cells = [
            csv_field(&g.title),
            csv_field(&system_name(&g.system)),
            csv_field(&g.path),
            g.size.to_string(),
            (g.play_secs / 60).to_string(),
            g.last_played.to_string(),
            g.favorite.to_string(),
            g.duplicate.to_string(),
            csv_field(&g.tags.join("; ")),
        ];
        out.push_str(&cells.join(","));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_choose_what_goes_in() {
        assert_eq!(
            BackupOptions::default().includes(),
            ["settings.json", "library.json", "config", "dats", "saves"]
        );
        let all = BackupOptions {
            saves: true,
            covers: true,
        };
        assert!(all.includes().contains(&"covers"));
        let min = BackupOptions {
            saves: false,
            covers: false,
        };
        assert_eq!(
            min.includes(),
            ["settings.json", "library.json", "config", "dats"]
        );
        assert!(all.includes().iter().all(|i| ALLOWED.contains(i)));
    }

    #[test]
    fn only_safe_entries_inside_the_data_folder_are_restored() {
        let ok = |e: &str| restore_target(e).map(|p| p.to_string_lossy().into_owned());
        assert_eq!(ok("data/settings.json").as_deref(), Some("settings.json"));
        assert_eq!(
            ok("data/config/controls/games/a b.json").as_deref(),
            Some("config/controls/games/a b.json")
        );
        assert_eq!(
            ok("data/saves/mslug.srm").as_deref(),
            Some("saves/mslug.srm")
        );
        for bad in [
            "manifest.json",          // not under data/
            "data/",                  // nothing
            "data/../etc/passwd",     // escapes
            "data/config/../../x",    // escapes
            "/data/settings.json",    // absolute
            "data//etc/x",            // empty component becomes absolute-ish
            "data/roms/game.zip",     // ROMs are never in a backup
            "data/system/neogeo.zip", // neither is BIOS
            "data/cores/x.dylib",     // nor cores
            "data/config\\..\\x",     // backslash tricks
            "data/C:/Windows/x",      // drive paths
            "data/settings.json\0.png",
        ] {
            assert_eq!(ok(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_backup_from_a_newer_format_is_refused_with_a_reason() {
        let m = |format| BackupManifest {
            format,
            app_version: "9".into(),
            created: 0,
            includes: vec![],
        };
        assert!(check_manifest(&m(FORMAT)).is_ok());
        assert!(check_manifest(&m(FORMAT + 1))
            .unwrap_err()
            .contains("newer Vuretro"));
    }

    fn game(title: &str, tags: &[&str]) -> GameView {
        GameView {
            system: "snes".into(),
            title: title.into(),
            original_title: title.into(),
            renamed: false,
            path: "/roms/snes/a.sfc".into(),
            size: 1000,
            last_played: 5,
            play_secs: 190,
            favorite: true,
            blocked: None,
            imported: true,
            identified: false,
            duplicate: false,
            tags: tags.iter().map(|t| t.to_string()).collect(),
            core: None,
        }
    }

    #[test]
    fn the_library_csv_quotes_commas_and_defuses_formulas() {
        let csv = library_csv(
            &[
                game("Zelda, the \"Link\"", &["RPG", "Co-op"]),
                game("=HYPERLINK(\"x\")", &[]),
            ],
            |id| {
                if id == "snes" {
                    "Super Nintendo".into()
                } else {
                    id.into()
                }
            },
        );
        let lines: Vec<&str> = csv.lines().collect();
        assert!(lines[0].starts_with("title,system,file"));
        assert_eq!(lines[1], "\"Zelda, the \"\"Link\"\"\",Super Nintendo,/roms/snes/a.sfc,1000,3,5,true,false,RPG; Co-op");
        assert!(
            lines[2].starts_with("\"'=HYPERLINK(\"\"x\"\")\","),
            "{}",
            lines[2]
        );
    }
}
