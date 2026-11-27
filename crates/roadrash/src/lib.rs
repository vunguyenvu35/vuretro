//! Reader for the data on the Road Rash PC disc (the 1996 Windows release).
//!
//! Everything here is worked out from the disc's own files, which the user supplies; no game
//! data is included. What is known about each format is written next to its reader.

pub mod adpcm;
pub mod avi;
pub mod bob;
pub mod cel;
pub mod cinepak;
pub mod course;
pub mod disc;
pub mod exe;
pub mod image;
pub mod iso;
pub mod palette;
pub mod ride;
pub mod road;
pub mod rsc;
