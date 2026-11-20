//! Check BIOS files in the system directory against the registry.

use std::path::Path;
use vuretro_core::model::{bios_ready, ImportedBios};
pub use vuretro_core::model::{BiosState, BiosStatus};
use vuretro_core::registry::{systems, System};

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
    bios_ready(&check(system, system_dir))
}

/// What a file is, as far as the BIOS list can tell.
pub struct Identified {
    pub system: &'static System,
    pub spec: &'static vuretro_core::registry::BiosSpec,
    /// Its content is an accepted dump (MD5 matches, or the entry has no MD5 to check).
    pub verified: bool,
    pub md5: String,
}

/// Match a file against the registry's BIOS files: by MD5 first (so a renamed dump still
/// works), then by file name. `Err` when no entry fits.
pub fn identify(data: &[u8], name: &str) -> Result<Identified, String> {
    let md5 = format!("{:x}", md5::compute(data));
    let all = || {
        systems()
            .iter()
            .flat_map(|s| s.bios.iter().map(move |b| (s, b)))
    };
    all()
        .find(|(_, b)| b.md5.contains(&md5.as_str()))
        .map(|(system, spec)| Identified {
            system,
            spec,
            verified: true,
            md5: md5.clone(),
        })
        .or_else(|| {
            all()
                .find(|(_, b)| b.file.eq_ignore_ascii_case(name))
                .map(|(system, spec)| Identified {
                    system,
                    spec,
                    verified: spec.md5.is_empty(),
                    md5: md5.clone(),
                })
        })
        .ok_or_else(|| format!("{name} is not a BIOS file vuretro knows"))
}

/// Identify `src` as one of the registry's BIOS files and copy it into
/// `system_dir` under the expected name. Matching is by MD5 first (so a
/// renamed dump still works), then by file name.
pub fn import(src: &Path, system_dir: &Path) -> Result<ImportedBios, String> {
    let data = std::fs::read(src).map_err(|e| format!("cannot read {}: {e}", src.display()))?;
    let name = src.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let found = identify(&data, name)?;
    if !found.verified {
        return Err(format!(
            "{name} looks like {} but its MD5 ({}) is not an accepted dump",
            found.spec.file, found.md5
        ));
    }
    std::fs::create_dir_all(system_dir).map_err(|e| e.to_string())?;
    std::fs::write(system_dir.join(found.spec.file), data).map_err(|e| e.to_string())?;
    Ok(ImportedBios {
        system: found.system.id,
        file: found.spec.file,
        verified: found.verified,
    })
}
