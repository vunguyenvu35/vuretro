//! Known systems: which core runs them, which files they accept, which BIOS they need.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize)]
pub struct BiosSpec {
    pub file: &'static str,
    /// Accepted MD5s (lowercase). Empty = only check presence (e.g. zip sets).
    pub md5: &'static [&'static str],
    pub required: bool,
    pub note: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct System {
    pub id: &'static str,
    pub name: &'static str,
    /// Core file stem; platform suffix (.dylib/.dll/.so) is added at runtime.
    pub core: &'static str,
    /// Other cores that can run the system (the user may pick one later).
    pub alt_cores: &'static [&'static str],
    pub extensions: &'static [&'static str],
    /// Folder names under the ROM root that force this system.
    pub folders: &'static [&'static str],
    pub bios: &'static [BiosSpec],
    pub enabled: bool,
}

/// Raw shape of `data/systems.json`; leaked once into the `'static` registry.
#[derive(Deserialize)]
struct RawBios {
    file: String,
    #[serde(default)]
    md5: Vec<String>,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    note: String,
}

#[derive(Deserialize)]
struct RawSystem {
    id: String,
    name: String,
    core: String,
    #[serde(default)]
    alt_cores: Vec<String>,
    extensions: Vec<String>,
    #[serde(default)]
    folders: Vec<String>,
    #[serde(default)]
    bios: Vec<RawBios>,
    #[serde(default = "yes")]
    enabled: bool,
}

fn yes() -> bool {
    true
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn leak_all(v: Vec<String>) -> &'static [&'static str] {
    Box::leak(
        v.into_iter()
            .map(leak)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    )
}

/// Parse a registry document. Errors name the problem so a bad data file is easy to fix.
pub fn parse_systems(json: &str) -> Result<Vec<System>, String> {
    let raw: Vec<RawSystem> = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(raw.len());
    for r in raw {
        if !seen.insert(r.id.clone()) {
            return Err(format!("duplicate system id {}", r.id));
        }
        if r.extensions.is_empty() {
            return Err(format!("system {} has no extensions", r.id));
        }
        let bios: Vec<BiosSpec> = r
            .bios
            .into_iter()
            .map(|b| BiosSpec {
                file: leak(b.file),
                md5: leak_all(b.md5),
                required: b.required,
                note: leak(b.note),
            })
            .collect();
        out.push(System {
            id: leak(r.id),
            name: leak(r.name),
            core: leak(r.core),
            alt_cores: leak_all(r.alt_cores),
            extensions: leak_all(r.extensions),
            folders: leak_all(r.folders),
            bios: Box::leak(bios.into_boxed_slice()),
            enabled: r.enabled,
        });
    }
    Ok(out)
}

static SYSTEMS: OnceLock<Vec<System>> = OnceLock::new();

/// All known systems, from the embedded `data/systems.json`.
pub fn systems() -> &'static [System] {
    SYSTEMS.get_or_init(|| {
        parse_systems(include_str!("../data/systems.json"))
            .expect("data/systems.json is valid (checked by tests)")
    })
}

pub fn by_id(id: &str) -> Option<&'static System> {
    systems().iter().find(|s| s.id == id)
}

/// File name of a core for the current OS.
pub fn core_filename(core: &str) -> String {
    let ext = if cfg!(target_os = "macos") {
        "dylib"
    } else if cfg!(target_os = "windows") {
        "dll"
    } else {
        "so"
    };
    format!("{core}.{ext}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_data_is_valid() {
        let all = systems();
        assert!(all.len() >= 4);
        for s in all {
            assert!(s.core.ends_with("_libretro"), "{}", s.id);
            assert!(s.alt_cores.iter().all(|c| c.ends_with("_libretro")));
            assert!(s.extensions.iter().all(|e| *e == e.to_ascii_lowercase()));
            assert!(s.folders.iter().all(|f| *f == f.to_ascii_lowercase()));
        }
        assert_eq!(by_id("gb").unwrap().core, "gambatte_libretro");
    }

    #[test]
    fn bad_documents_are_rejected_with_a_reason() {
        let dup = r#"[{"id":"a","name":"A","core":"x","extensions":["a"]},
                      {"id":"a","name":"B","core":"y","extensions":["b"]}]"#;
        assert!(parse_systems(dup).unwrap_err().contains("duplicate"));
        let none = r#"[{"id":"a","name":"A","core":"x","extensions":[]}]"#;
        assert!(parse_systems(none).unwrap_err().contains("no extensions"));
        assert!(parse_systems("nope").is_err());
    }
}
