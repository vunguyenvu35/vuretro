//! Implementations of the `vuretro_core::ports` traits for a real machine.

mod catalog;
mod launcher;
mod locator;
mod store;

pub use catalog::FsCatalog;
pub use launcher::ProcessLauncher;
pub use locator::FsLocator;
pub use store::JsonStore;
