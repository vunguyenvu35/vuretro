//! Implementations of the `vuretro_core::ports` traits for a real machine.

mod archive;
mod art;
mod bundle;
mod catalog;
mod installer;
mod launcher;
mod locator;
mod machine;
mod screenscraper;
mod store;

pub use archive::ZipBackup;
pub use art::ThumbnailsArt;
pub use catalog::FsCatalog;
pub use installer::BuildbotInstaller;
pub use launcher::ProcessLauncher;
pub use locator::FsLocator;
pub use machine::SysMachine;
pub use screenscraper::{DevLogin, ScreenScraperArt};
pub use store::JsonStore;
