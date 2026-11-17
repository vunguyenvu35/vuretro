//! What Tauri manages for the commands: the use-case layer plus the live pad list.

use std::sync::{Arc, Mutex};
use vuretro_controls::PadInfo;
use vuretro_core::service::App;

pub struct Shell {
    pub app: Arc<App>,
    pub pads: Arc<Mutex<Vec<PadInfo>>>,
}
