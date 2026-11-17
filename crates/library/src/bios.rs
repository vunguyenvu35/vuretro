//! Check BIOS files in the system directory against the registry.

use crate::registry::System;
use serde::Serialize;
use std::path::Path;

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

pub fn check(system: &'static System, system_dir: &Path) -> Vec<BiosStatus> {
    system
        .bios
        .iter()
        .map(|spec| {
            let path = system_dir.join(spec.file);
            let state = match std::fs::read(&path) {
                Err(_) => BiosState::Missing,
                Ok(_) if spec.md5.is_empty() => BiosState::Ok,
                Ok(data) => {
                    let md5 = format!("{:x}", md5::compute(&data));
                    if spec.md5.contains(&md5.as_str()) {
                        BiosState::Ok
                    } else {
                        BiosState::BadHash { md5 }
                    }
                }
            };
            BiosStatus {
                system: system.id,
                file: spec.file,
                required: spec.required,
                note: spec.note,
                state,
            }
        })
        .collect()
}

/// True when every required BIOS for the system is OK.
pub fn ready(system: &'static System, system_dir: &Path) -> bool {
    check(system, system_dir)
        .iter()
        .all(|b| !b.required || b.state == BiosState::Ok)
}

/// Result of importing a BIOS file into the system directory.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ImportResult {
    pub system: &'static str,
    pub file: &'static str,
    /// True when the content matched a known MD5 (or the name matched a presence-only set).
    pub verified: bool,
}

/// Identify `src` as one of the registry's BIOS files and copy it into
/// `system_dir` under the expected name. Matching is by MD5 first (so a
/// renamed dump still works), then by file name.
pub fn import(src: &Path, system_dir: &Path) -> Result<ImportResult, String> {
    let data = std::fs::read(src).map_err(|e| format!("cannot read {}: {e}", src.display()))?;
    let md5 = format!("{:x}", md5::compute(&data));
    let name = src.file_name().and_then(|n| n.to_str()).unwrap_or_default();

    let all = || {
        crate::registry::SYSTEMS
            .iter()
            .flat_map(|s| s.bios.iter().map(move |b| (s, b)))
    };
    let (sys, spec, verified) = all()
        .find(|(_, b)| b.md5.contains(&md5.as_str()))
        .map(|(s, b)| (s, b, true))
        .or_else(|| {
            all()
                .find(|(_, b)| b.file.eq_ignore_ascii_case(name))
                .map(|(s, b)| (s, b, b.md5.is_empty()))
        })
        .ok_or_else(|| format!("{name} is not a BIOS file vuretro knows"))?;

    if !verified {
        return Err(format!(
            "{name} looks like {} but its MD5 ({md5}) is not an accepted dump",
            spec.file
        ));
    }
    std::fs::create_dir_all(system_dir).map_err(|e| e.to_string())?;
    std::fs::write(system_dir.join(spec.file), data).map_err(|e| e.to_string())?;
    Ok(ImportResult {
        system: sys.id,
        file: spec.file,
        verified,
    })
}
