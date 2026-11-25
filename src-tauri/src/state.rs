//! What Tauri manages for the commands: the use-case layer plus the live pad list.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use vuretro_controls::padnames::PadNames;
use vuretro_controls::PadInfo;
use vuretro_core::service::App;

pub struct Shell {
    pub app: Arc<App>,
    /// The connected pads, with the names the system gives them.
    pub pads: Arc<Mutex<Vec<PadInfo>>>,
    /// A 2.4G receiver is plugged in with no pad awake behind it.
    pub dongle_waiting: Arc<AtomicBool>,
    /// The names the user gave to pads, and the file they are kept in.
    pub pad_names: Arc<Mutex<PadNames>>,
    pub pad_names_file: PathBuf,
}

impl Shell {
    /// The connected pads as the user should see them: their own names where they gave one.
    pub fn named_pads(&self) -> Vec<PadInfo> {
        vuretro_controls::padnames::apply(
            &self.pads.lock().unwrap(),
            &self.pad_names.lock().unwrap(),
        )
    }
}
