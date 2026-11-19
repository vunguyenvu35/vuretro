//! vuretro core: the innermost layer. It holds the domain (systems, games, settings), the
//! use cases (`service::App`) and the ports (`ports`) those use cases need. It does no
//! file, process or UI work itself, so it depends on neither Tauri, nor minifb, nor the
//! disk; adapters in `vuretro-library` and the Tauri shell plug into the ports.
//!
//! Dependency rule: everything else depends on this crate, never the other way round.

pub mod backup;
pub mod detect;
pub mod links;
pub mod model;
pub mod patch;
pub mod paths;
pub mod ports;
pub mod registry;
pub mod service;
