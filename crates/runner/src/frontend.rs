//! Global frontend state shared with the libretro C callbacks.
//! libretro callbacks carry no user pointer, so state lives in a global.

use libretro_sys::*;
use std::collections::{HashMap, VecDeque};
use std::ffi::CString;
use std::os::raw::{c_char, c_uint, c_void};
use std::sync::{Arc, Mutex, OnceLock};

pub const MAX_PORTS: usize = vuretro_controls::PLAYERS;

pub struct Frontend {
    pub pixel_format: c_uint,
    /// Last frame as 0RGB u32 pixels (minifb format).
    pub frame: Vec<u32>,
    pub width: usize,
    pub height: usize,
    pub frame_dirty: bool,
    /// Interleaved stereo i16 samples at the core's sample rate.
    pub audio: Arc<Mutex<VecDeque<i16>>>,
    pub buttons: [[bool; JOYPAD_BUTTON_COUNT]; MAX_PORTS],
    pub system_dir: CString,
    pub save_dir: CString,
    /// User values for core options (`GET_VARIABLE`), kept alive for the core.
    pub options: HashMap<String, CString>,
    /// Options announced by the core (`SET_VARIABLES`).
    pub definitions: Vec<crate::options::OptionDef>,
}

static FRONTEND: OnceLock<Mutex<Frontend>> = OnceLock::new();

pub fn init(
    system_dir: &str,
    save_dir: &str,
    audio: Arc<Mutex<VecDeque<i16>>>,
    options: HashMap<String, String>,
) {
    let fe = Frontend {
        pixel_format: RETRO_PIXEL_FORMAT_0RGB1555,
        frame: Vec::new(),
        width: 0,
        height: 0,
        frame_dirty: false,
        audio,
        buttons: [[false; JOYPAD_BUTTON_COUNT]; MAX_PORTS],
        system_dir: CString::new(system_dir).unwrap_or_default(),
        save_dir: CString::new(save_dir).unwrap_or_default(),
        options: options
            .into_iter()
            .filter_map(|(k, v)| Some((k, CString::new(v).ok()?)))
            .collect(),
        definitions: Vec::new(),
    };
    let _ = FRONTEND.set(Mutex::new(fe));
}

pub fn with<R>(f: impl FnOnce(&mut Frontend) -> R) -> R {
    let mut guard = FRONTEND
        .get()
        .expect("frontend not initialised")
        .lock()
        .unwrap();
    f(&mut guard)
}

/// Max buffered audio (stereo samples) before old samples are dropped,
/// so latency stays bounded when video pacing drifts or fast-forward is on.
const AUDIO_MAX_SAMPLES: usize = 48_000 / 4 * 2;

fn push_audio(fe: &mut Frontend, samples: &[i16]) {
    let mut q = fe.audio.lock().unwrap();
    q.extend(samples.iter().copied());
    let overflow = q.len().saturating_sub(AUDIO_MAX_SAMPLES);
    if overflow > 0 {
        q.drain(..overflow + (overflow & 1));
    }
}

unsafe fn cstr_lossy(p: *const c_char) -> String {
    if p.is_null() {
        String::new()
    } else {
        std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned()
    }
}

pub unsafe extern "C" fn environment_cb(cmd: c_uint, data: *mut c_void) -> bool {
    match cmd {
        RETRO_ENVIRONMENT_GET_CAN_DUPE => {
            *(data as *mut bool) = true;
            true
        }
        RETRO_ENVIRONMENT_SET_PIXEL_FORMAT => {
            let fmt = *(data as *const c_uint);
            if fmt <= RETRO_PIXEL_FORMAT_RGB565 {
                with(|fe| fe.pixel_format = fmt);
                true
            } else {
                false
            }
        }
        RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY | RETRO_ENVIRONMENT_GET_CORE_ASSETS_DIRECTORY => {
            with(|fe| *(data as *mut *const c_char) = fe.system_dir.as_ptr());
            true
        }
        RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY => {
            with(|fe| *(data as *mut *const c_char) = fe.save_dir.as_ptr());
            true
        }
        RETRO_ENVIRONMENT_GET_VARIABLE => {
            let var = &mut *(data as *mut retro_variable);
            let key = std::ffi::CStr::from_ptr(var.key)
                .to_string_lossy()
                .into_owned();
            with(|fe| match fe.options.get(&key) {
                Some(v) => {
                    var.value = v.as_ptr();
                    true
                }
                None => {
                    var.value = std::ptr::null();
                    false // core falls back to its default
                }
            })
        }
        RETRO_ENVIRONMENT_SET_VARIABLES => {
            let mut p = data as *const retro_variable;
            let mut defs = Vec::new();
            while !p.is_null() && !(*p).key.is_null() {
                let (k, v) = (cstr_lossy((*p).key), cstr_lossy((*p).value));
                defs.push(crate::options::parse_definition(&k, &v));
                p = p.add(1);
            }
            with(|fe| fe.definitions = defs);
            true
        }
        RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE => {
            *(data as *mut bool) = false;
            true
        }
        RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS | RETRO_ENVIRONMENT_SET_PERFORMANCE_LEVEL => true,
        _ => {
            log::trace!("unhandled environment cmd {cmd}");
            false
        }
    }
}

pub unsafe extern "C" fn video_cb(
    data: *const c_void,
    width: c_uint,
    height: c_uint,
    pitch: usize,
) {
    if data.is_null() {
        return; // duplicate frame: keep the previous one
    }
    let (w, h) = (width as usize, height as usize);
    with(|fe| {
        if fe.frame.len() != w * h {
            fe.frame = vec![0; w * h];
        }
        fe.width = w;
        fe.height = h;
        let src = data as *const u8;
        for y in 0..h {
            let row = src.add(y * pitch);
            let dst = &mut fe.frame[y * w..(y + 1) * w];
            match fe.pixel_format {
                RETRO_PIXEL_FORMAT_XRGB8888 => {
                    let px = std::slice::from_raw_parts(row as *const u32, w);
                    for (d, s) in dst.iter_mut().zip(px) {
                        *d = s & 0x00FF_FFFF;
                    }
                }
                RETRO_PIXEL_FORMAT_RGB565 => {
                    let px = std::slice::from_raw_parts(row as *const u16, w);
                    for (d, &p) in dst.iter_mut().zip(px) {
                        *d = rgb565_to_rgb888(p);
                    }
                }
                _ => {
                    let px = std::slice::from_raw_parts(row as *const u16, w);
                    for (d, &p) in dst.iter_mut().zip(px) {
                        *d = rgb1555_to_rgb888(p);
                    }
                }
            }
        }
        fe.frame_dirty = true;
    });
}

#[inline]
pub fn rgb565_to_rgb888(p: u16) -> u32 {
    let r = ((p >> 11) & 0x1F) as u32;
    let g = ((p >> 5) & 0x3F) as u32;
    let b = (p & 0x1F) as u32;
    ((r << 3 | r >> 2) << 16) | ((g << 2 | g >> 4) << 8) | (b << 3 | b >> 2)
}

#[inline]
pub fn rgb1555_to_rgb888(p: u16) -> u32 {
    let r = ((p >> 10) & 0x1F) as u32;
    let g = ((p >> 5) & 0x1F) as u32;
    let b = (p & 0x1F) as u32;
    ((r << 3 | r >> 2) << 16) | ((g << 3 | g >> 2) << 8) | (b << 3 | b >> 2)
}

pub unsafe extern "C" fn audio_sample_cb(left: i16, right: i16) {
    with(|fe| push_audio(fe, &[left, right]));
}

pub unsafe extern "C" fn audio_batch_cb(data: *const i16, frames: usize) -> usize {
    let samples = std::slice::from_raw_parts(data, frames * 2);
    with(|fe| push_audio(fe, samples));
    frames
}

pub unsafe extern "C" fn input_poll_cb() {
    // Input is sampled once per frame in the main loop before retro_run.
}

pub unsafe extern "C" fn input_state_cb(
    port: c_uint,
    device: c_uint,
    _index: c_uint,
    id: c_uint,
) -> i16 {
    if device != RETRO_DEVICE_JOYPAD
        || port as usize >= MAX_PORTS
        || id as usize >= JOYPAD_BUTTON_COUNT
    {
        return 0;
    }
    with(|fe| fe.buttons[port as usize][id as usize] as i16)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_conversion_extremes() {
        assert_eq!(rgb565_to_rgb888(0xFFFF), 0x00FF_FFFF);
        assert_eq!(rgb565_to_rgb888(0xF800), 0x00FF_0000);
        assert_eq!(rgb1555_to_rgb888(0x7FFF), 0x00FF_FFFF);
        assert_eq!(rgb1555_to_rgb888(0x001F), 0x0000_00FF);
    }
}
