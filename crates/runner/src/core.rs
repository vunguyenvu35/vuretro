//! Dynamic loading of a libretro core (.dylib / .dll / .so).

use anyhow::{bail, Context, Result};
use libloading::Library;
use libretro_sys::*;
use std::ffi::{CStr, CString};
use std::os::raw::{c_uint, c_void};
use std::path::Path;

type FnVoid = unsafe extern "C" fn();
type FnApiVersion = unsafe extern "C" fn() -> c_uint;
type FnSystemInfo = unsafe extern "C" fn(*mut retro_system_info);
type FnAvInfo = unsafe extern "C" fn(*mut retro_system_av_info);
type FnLoadGame = unsafe extern "C" fn(*const retro_game_info) -> bool;
type FnSerializeSize = unsafe extern "C" fn() -> usize;
type FnSerialize = unsafe extern "C" fn(*mut c_void, usize) -> bool;
type FnUnserialize = unsafe extern "C" fn(*const c_void, usize) -> bool;
type FnSetPortDevice = unsafe extern "C" fn(c_uint, c_uint);

pub struct SystemInfo {
    pub name: String,
    pub version: String,
    pub extensions: String,
    pub need_fullpath: bool,
}

/// A loaded core. Field order matters: `_lib` must drop last.
pub struct Core {
    retro_deinit: FnVoid,
    retro_run: FnVoid,
    retro_reset: FnVoid,
    retro_unload_game: FnVoid,
    retro_get_system_av_info: FnAvInfo,
    retro_load_game: FnLoadGame,
    retro_serialize_size: FnSerializeSize,
    retro_serialize: FnSerialize,
    retro_unserialize: FnUnserialize,
    retro_set_controller_port_device: FnSetPortDevice,
    pub info: SystemInfo,
    game_loaded: bool,
    // Keeps ROM bytes alive for cores that don't need a full path.
    _rom_data: Option<Vec<u8>>,
    _rom_path: Option<CString>,
    _lib: Library,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &str) -> Result<T> {
    let s: libloading::Symbol<T> = lib
        .get(name.as_bytes())
        .with_context(|| format!("core is missing symbol {name}"))?;
    Ok(*s)
}

fn cstr(p: *const std::os::raw::c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
    }
}

impl Core {
    /// Load the core library and register frontend callbacks.
    pub fn load(path: &Path) -> Result<Self> {
        unsafe {
            let lib = Library::new(path)
                .with_context(|| format!("cannot load core {}", path.display()))?;

            let api: FnApiVersion = sym(&lib, "retro_api_version")?;
            if api() != RETRO_API_VERSION {
                bail!("unsupported libretro API version {}", api());
            }

            // Callbacks must be set before retro_init (environment) per spec.
            let set_env: unsafe extern "C" fn(retro_environment_t) =
                sym(&lib, "retro_set_environment")?;
            let set_video: unsafe extern "C" fn(retro_video_refresh_t) =
                sym(&lib, "retro_set_video_refresh")?;
            let set_audio: unsafe extern "C" fn(retro_audio_sample_t) =
                sym(&lib, "retro_set_audio_sample")?;
            let set_audio_batch: unsafe extern "C" fn(retro_audio_sample_batch_t) =
                sym(&lib, "retro_set_audio_sample_batch")?;
            let set_poll: unsafe extern "C" fn(retro_input_poll_t) =
                sym(&lib, "retro_set_input_poll")?;
            let set_state: unsafe extern "C" fn(retro_input_state_t) =
                sym(&lib, "retro_set_input_state")?;

            set_env(crate::frontend::environment_cb);
            set_video(crate::frontend::video_cb);
            set_audio(crate::frontend::audio_sample_cb);
            set_audio_batch(crate::frontend::audio_batch_cb);
            set_poll(crate::frontend::input_poll_cb);
            set_state(crate::frontend::input_state_cb);

            let get_info: FnSystemInfo = sym(&lib, "retro_get_system_info")?;
            let mut raw = std::mem::zeroed::<retro_system_info>();
            get_info(&mut raw);
            let info = SystemInfo {
                name: cstr(raw.library_name),
                version: cstr(raw.library_version),
                extensions: cstr(raw.valid_extensions),
                need_fullpath: raw.need_fullpath,
            };

            let retro_init: FnVoid = sym(&lib, "retro_init")?;
            retro_init();

            Ok(Core {
                retro_deinit: sym(&lib, "retro_deinit")?,
                retro_run: sym(&lib, "retro_run")?,
                retro_reset: sym(&lib, "retro_reset")?,
                retro_unload_game: sym(&lib, "retro_unload_game")?,
                retro_get_system_av_info: sym(&lib, "retro_get_system_av_info")?,
                retro_load_game: sym(&lib, "retro_load_game")?,
                retro_serialize_size: sym(&lib, "retro_serialize_size")?,
                retro_serialize: sym(&lib, "retro_serialize")?,
                retro_unserialize: sym(&lib, "retro_unserialize")?,
                retro_set_controller_port_device: sym(&lib, "retro_set_controller_port_device")?,
                info,
                game_loaded: false,
                _rom_data: None,
                _rom_path: None,
                _lib: lib,
            })
        }
    }

    pub fn load_game(&mut self, rom: &Path) -> Result<retro_system_av_info> {
        let path_c = CString::new(rom.to_string_lossy().as_bytes())?;
        let data = if self.info.need_fullpath {
            None
        } else {
            Some(std::fs::read(rom).with_context(|| format!("cannot read {}", rom.display()))?)
        };
        let game = retro_game_info {
            path: path_c.as_ptr(),
            data: data
                .as_ref()
                .map_or(std::ptr::null(), |d| d.as_ptr() as *const c_void),
            size: data.as_ref().map_or(0, |d| d.len()),
            meta: std::ptr::null(),
        };
        let ok = unsafe { (self.retro_load_game)(&game) };
        if !ok {
            bail!(
                "core refused to load {} (missing BIOS or unsupported file?)",
                rom.display()
            );
        }
        self._rom_data = data;
        self._rom_path = Some(path_c);
        self.game_loaded = true;
        unsafe {
            (self.retro_set_controller_port_device)(0, RETRO_DEVICE_JOYPAD);
            (self.retro_set_controller_port_device)(1, RETRO_DEVICE_JOYPAD);
        }
        let mut av = retro_system_av_info::default();
        unsafe { (self.retro_get_system_av_info)(&mut av) };
        Ok(av)
    }

    pub fn run(&self) {
        unsafe { (self.retro_run)() }
    }

    pub fn reset(&self) {
        unsafe { (self.retro_reset)() }
    }

    pub fn save_state(&self) -> Option<Vec<u8>> {
        unsafe {
            let size = (self.retro_serialize_size)();
            if size == 0 {
                return None;
            }
            let mut buf = vec![0u8; size];
            (self.retro_serialize)(buf.as_mut_ptr() as *mut c_void, size).then_some(buf)
        }
    }

    pub fn load_state(&self, data: &[u8]) -> bool {
        unsafe { (self.retro_unserialize)(data.as_ptr() as *const c_void, data.len()) }
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        unsafe {
            if self.game_loaded {
                (self.retro_unload_game)();
            }
            (self.retro_deinit)();
        }
    }
}
