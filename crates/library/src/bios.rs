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
