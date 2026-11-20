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
    /// Rumble the core asked for, per port: `[strong, weak]` strength (0 = off).
    pub rumble: [[u16; 2]; MAX_PORTS],
    pub system_dir: CString,
    pub save_dir: CString,
    /// User values for core options (`GET_VARIABLE`), kept alive for the core.
    pub options: HashMap<String, CString>,
    /// Options announced by the core (`SET_VARIABLES`).
    pub definitions: Vec<crate::options::OptionDef>,
    /// Disc control of a multi-disc core, if it announced one.
    pub disk: Option<retro_disk_control_callback>,
    /// Screen rotation the core asked for, in quarter turns counter-clockwise (0..=3).
    pub rotation: u8,
    /// The core asked to quit (`SHUTDOWN`), for example from its own menu.
    pub shutdown: bool,
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
        rumble: [[0; 2]; MAX_PORTS],
        system_dir: CString::new(system_dir).unwrap_or_default(),
        save_dir: CString::new(save_dir).unwrap_or_default(),
        options: options
            .into_iter()
            .filter_map(|(k, v)| Some((k, CString::new(v).ok()?)))
            .collect(),
        definitions: Vec::new(),
        disk: None,
        rotation: 0,
        shutdown: false,
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
    // some commands carry a flag that only says "experimental"; it is not part of the number
    let cmd = cmd & !RETRO_ENVIRONMENT_EXPERIMENTAL;
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
            with(|fe| {
                // A value the user has not chosen is the core's default, its first one. Cores
                // like MAME 2003-Plus do not pick it themselves: asked for nothing they take a
                // sound rate of 0 Hz and play no sound.
                for d in &defs {
                    if let (false, Some(v)) = (fe.options.contains_key(&d.key), d.values.first()) {
                        if let Ok(c) = CString::new(v.as_str()) {
                            fe.options.insert(d.key.clone(), c);
                        }
                    }
                }
                fe.definitions = defs;
            });
            true
        }
        // Many cores take for granted that the frontend offers logging, and crash on a null
        // function pointer when it does not (MAME 2003-Plus does).
        RETRO_ENVIRONMENT_GET_LOG_INTERFACE => {
            (*(data as *mut retro_log_callback)).log = Some(core_log);
            true
        }
        RETRO_ENVIRONMENT_GET_RUMBLE_INTERFACE => {
            (*(data as *mut retro_rumble_interface)).set_rumble_state = Some(core_rumble);
            true
        }
        RETRO_ENVIRONMENT_SET_DISK_CONTROL_INTERFACE => {
            let cb = *(data as *const retro_disk_control_callback);
            with(|fe| fe.disk = Some(cb));
            true
        }
        RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE => {
            *(data as *mut bool) = false;
            true
        }
        RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS => {
            log_input_descriptors(data as *const retro_input_descriptor);
            true
        }
        RETRO_ENVIRONMENT_SET_PERFORMANCE_LEVEL => true,
        RETRO_ENVIRONMENT_SET_ROTATION => {
            let turns = *(data as *const c_uint);
            if turns > 3 {
                return false;
            }
            with(|fe| fe.rotation = turns as u8);
            true
        }
        RETRO_ENVIRONMENT_GET_OVERSCAN => {
            // we show the whole picture, overscan included
            *(data as *mut bool) = true;
            true
        }
        RETRO_ENVIRONMENT_SHUTDOWN => {
            with(|fe| fe.shutdown = true);
            true
        }
        RETRO_ENVIRONMENT_SET_MESSAGE => {
            let m = &*(data as *const retro_message);
            log::info!(target: "core", "message: {}", cstr_lossy(m.msg));
            true
        }
        RETRO_ENVIRONMENT_SET_MESSAGE_EXT => {
            let m = &*(data as *const retro_message_ext);
            let text = cstr_lossy(m.msg);
            match m.level {
                0 => log::debug!(target: "core", "message: {text}"),
                1 => log::info!(target: "core", "message: {text}"),
                2 => log::warn!(target: "core", "message: {text}"),
                _ => log::error!(target: "core", "message: {text}"),
            }
            true
        }
        RETRO_ENVIRONMENT_GET_MESSAGE_INTERFACE_VERSION => {
            *(data as *mut c_uint) = 1;
            true
        }
        RETRO_ENVIRONMENT_GET_LANGUAGE => {
            *(data as *mut c_uint) = 0; // RETRO_LANGUAGE_ENGLISH
            true
        }
        RETRO_ENVIRONMENT_GET_USERNAME => {
            *(data as *mut *const c_char) = c"vuretro".as_ptr();
            true
        }
        RETRO_ENVIRONMENT_GET_INPUT_DEVICE_CAPABILITIES => {
            *(data as *mut u64) = RETRO_DEVICE_JOYPAD_BIT;
            true
        }
        RETRO_ENVIRONMENT_GET_INPUT_MAX_USERS => {
            *(data as *mut c_uint) = MAX_PORTS as c_uint;
            true
        }
        RETRO_ENVIRONMENT_GET_AUDIO_VIDEO_ENABLE => {
            *(data as *mut i32) = 0b11; // video and audio both wanted
            true
        }
        RETRO_ENVIRONMENT_SET_VARIABLE => {
            // a core changes one of its own options; null data only asks "is this supported?"
            if data.is_null() {
                return true;
            }
            let var = &*(data as *const retro_variable);
            if var.key.is_null() || var.value.is_null() {
                return false;
            }
            let key = cstr_lossy(var.key);
            let value = std::ffi::CStr::from_ptr(var.value).to_owned();
            with(|fe| match fe.options.get_mut(&key) {
                Some(slot) => {
                    *slot = value;
                    true
                }
                None => false,
            })
        }
        // Acknowledged, nothing to do: the picture size comes with every frame, and we do not
        // use the controller, subsystem, memory-map or quirk information.
        RETRO_ENVIRONMENT_SET_GEOMETRY
        | RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME
        | RETRO_ENVIRONMENT_SET_CONTROLLER_INFO
        | RETRO_ENVIRONMENT_SET_SUBSYSTEM_INFO
        | RETRO_ENVIRONMENT_SET_MEMORY_MAPS
        | RETRO_ENVIRONMENT_SET_SERIALIZATION_QUIRKS => true,
        RETRO_ENVIRONMENT_SET_HW_RENDER => {
            log::warn!("the core wants a GPU renderer, which vuretro does not offer yet");
            false
        }
        RETRO_ENVIRONMENT_SET_SYSTEM_AV_INFO => {
            // a new frame rate or sample rate would need the pacing and audio to follow
            log::warn!("the core changed its video or audio timing; vuretro keeps the old one");
            false
        }
        _ => {
            log::trace!("unhandled environment cmd {cmd}");
            false
        }
    }
}

/// Say what the core calls each button of the first controller (for example `A`, `Button B`).
/// Shown with `RUST_LOG=debug`: it is how to see which RetroPad button is which game button.
///
/// # Safety
/// `list` is the array the core passed to the environment call, ended by a null description.
unsafe fn log_input_descriptors(list: *const retro_input_descriptor) {
    if list.is_null() || !log::log_enabled!(log::Level::Debug) {
        return;
    }
    let mut p = list;
    while !(*p).description.is_null() {
        let d = &*p;
        if d.port == 0 && d.device == RETRO_DEVICE_JOYPAD {
            log::debug!(
                "input: RetroPad id {} is \"{}\"",
                d.id,
                cstr_lossy(d.description)
            );
        }
        p = p.add(1);
    }
}

/// A core asks for a controller to rumble (or stop). Remembered here; the input code
/// passes it on to the gamepad.
unsafe extern "C" fn core_rumble(port: c_uint, effect: c_uint, strength: u16) -> bool {
    if port as usize >= MAX_PORTS || effect > RETRO_RUMBLE_WEAK {
        return false;
    }
    with(|fe| fe.rumble[port as usize][effect as usize] = strength);
    true
}

/// What a core's log calls end up as. Levels follow libretro: 0 debug, 1 info, 2 warn, 3 error.
unsafe extern "C" fn core_log(level: std::os::raw::c_int, fmt: *const c_char) {
    if fmt.is_null() {
        return;
    }
    let text = std::ffi::CStr::from_ptr(fmt).to_string_lossy();
    let text = text.trim_end();
    match level {
        0 => log::debug!(target: "core", "{text}"),
        1 => log::info!(target: "core", "{text}"),
        2 => log::warn!(target: "core", "{text}"),
        _ => log::error!(target: "core", "{text}"),
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
        if fe.rotation != 0 {
            let (rotated, rw, rh) = rotate(&fe.frame, w, h, fe.rotation);
            fe.frame = rotated;
            fe.width = rw;
            fe.height = rh;
        }
        fe.frame_dirty = true;
    });
}

/// Turn a picture by `turns` quarter turns counter-clockwise (libretro's `SET_ROTATION`);
/// returns the pixels and the new width and height.
pub fn rotate(src: &[u32], w: usize, h: usize, turns: u8) -> (Vec<u32>, usize, usize) {
    match turns % 4 {
        0 => (src.to_vec(), w, h),
        1 => {
            // new (x, y) comes from old (w - 1 - y, x)
            let mut out = vec![0; w * h];
            for y in 0..w {
                for x in 0..h {
                    out[y * h + x] = src[x * w + (w - 1 - y)];
                }
            }
            (out, h, w)
        }
        2 => (src.iter().rev().copied().collect(), w, h),
        _ => {
            // new (x, y) comes from old (y, h - 1 - x)
            let mut out = vec![0; w * h];
            for y in 0..w {
                for x in 0..h {
                    out[y * h + x] = src[(h - 1 - x) * w + y];
                }
            }
            (out, h, w)
        }
    }
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

#[cfg(test)]
mod env_tests {
    use super::*;

    fn call<T>(cmd: c_uint, value: &mut T) -> bool {
        unsafe { environment_cb(cmd, value as *mut T as *mut c_void) }
    }

    #[test]
    fn rotation_turns_counter_clockwise() {
        // 3 wide, 2 tall:  1 2 3 / 4 5 6
        let src = [1, 2, 3, 4, 5, 6];
        assert_eq!(rotate(&src, 3, 2, 0), (src.to_vec(), 3, 2));
        assert_eq!(rotate(&src, 3, 2, 1), (vec![3, 6, 2, 5, 1, 4], 2, 3));
        assert_eq!(rotate(&src, 3, 2, 2), (vec![6, 5, 4, 3, 2, 1], 3, 2));
        assert_eq!(rotate(&src, 3, 2, 3), (vec![4, 1, 5, 2, 6, 3], 2, 3));
    }

    #[test]
    fn simple_questions_get_simple_answers() {
        let mut language: c_uint = 99;
        assert!(call(RETRO_ENVIRONMENT_GET_LANGUAGE, &mut language));
        assert_eq!(language, 0);
        let mut caps = 0u64;
        assert!(call(
            RETRO_ENVIRONMENT_GET_INPUT_DEVICE_CAPABILITIES,
            &mut caps
        ));
        assert_eq!(caps, 1 << RETRO_DEVICE_JOYPAD);
        let mut users: c_uint = 0;
        assert!(call(RETRO_ENVIRONMENT_GET_INPUT_MAX_USERS, &mut users));
        assert_eq!(users as usize, MAX_PORTS);
        let mut name: *const c_char = std::ptr::null();
        assert!(call(RETRO_ENVIRONMENT_GET_USERNAME, &mut name));
        assert_eq!(unsafe { cstr_lossy(name) }, "vuretro");
        let mut av = 0i32;
        assert!(call(RETRO_ENVIRONMENT_GET_AUDIO_VIDEO_ENABLE, &mut av));
        assert_eq!(av, 3);
    }

    #[test]
    fn the_experimental_flag_is_not_part_of_the_command() {
        let mut av = 0i32;
        assert!(call(
            RETRO_ENVIRONMENT_GET_AUDIO_VIDEO_ENABLE | RETRO_ENVIRONMENT_EXPERIMENTAL,
            &mut av
        ));
        assert_eq!(av, 3);
    }

    #[test]
    fn rotation_shutdown_and_set_variable_are_remembered() {
        init(
            "/s",
            "/v",
            Arc::new(Mutex::new(VecDeque::new())),
            HashMap::new(),
        );
        // `init` only counts the first time in a test run, so put the option in directly
        with(|fe| fe.options.insert("opt".into(), CString::new("a").unwrap()));
        let mut turns: c_uint = 1;
        assert!(call(RETRO_ENVIRONMENT_SET_ROTATION, &mut turns));
        assert_eq!(with(|fe| fe.rotation), 1);
        let mut bad: c_uint = 4;
        assert!(!call(RETRO_ENVIRONMENT_SET_ROTATION, &mut bad));

        let mut none = ();
        assert!(call(RETRO_ENVIRONMENT_SHUTDOWN, &mut none));
        assert!(with(|fe| fe.shutdown));
        with(|fe| {
            fe.shutdown = false; // the state is shared with other tests
            fe.rotation = 0;
        });

        let (k, v, unknown) = (c"opt", c"b", c"nope");
        let mut var = retro_variable {
            key: k.as_ptr(),
            value: v.as_ptr(),
        };
        assert!(call(RETRO_ENVIRONMENT_SET_VARIABLE, &mut var));
        assert_eq!(
            with(|fe| fe.options["opt"].to_str().unwrap().to_string()),
            "b"
        );
        var.key = unknown.as_ptr();
        assert!(!call(RETRO_ENVIRONMENT_SET_VARIABLE, &mut var));
    }

    #[test]
    fn what_we_cannot_do_is_refused() {
        let mut none = ();
        assert!(!call(RETRO_ENVIRONMENT_SET_HW_RENDER, &mut none));
        assert!(!call(RETRO_ENVIRONMENT_SET_SYSTEM_AV_INFO, &mut none));
    }
}

#[cfg(test)]
mod log_tests {
    use super::*;

    #[test]
    fn a_core_asking_for_logging_gets_a_working_log_function() {
        let mut cb = retro_log_callback { log: None };
        let ok = unsafe {
            environment_cb(
                RETRO_ENVIRONMENT_GET_LOG_INTERFACE,
                &mut cb as *mut _ as *mut c_void,
            )
        };
        assert!(
            ok,
            "a core that assumes logging exists would crash on a null pointer"
        );
        let log = cb.log.expect("the function is filled in");
        let msg = CString::new("hello from a core\n").unwrap();
        for level in 0..5 {
            unsafe { log(level, msg.as_ptr()) };
        }
        unsafe { log(1, std::ptr::null()) }; // a null format must not crash either
    }

    #[test]
    fn a_core_can_ask_for_rumble_and_the_request_is_remembered() {
        init(
            "/s",
            "/v",
            Arc::new(Mutex::new(VecDeque::new())),
            HashMap::new(),
        );
        let mut cb = retro_rumble_interface {
            set_rumble_state: None,
        };
        let ok = unsafe {
            environment_cb(
                RETRO_ENVIRONMENT_GET_RUMBLE_INTERFACE,
                &mut cb as *mut _ as *mut c_void,
            )
        };
        assert!(ok);
        let set = cb.set_rumble_state.expect("the function is filled in");
        unsafe {
            assert!(set(0, RETRO_RUMBLE_STRONG, 40_000));
            assert!(set(0, RETRO_RUMBLE_WEAK, 100));
            assert!(
                !set(MAX_PORTS as c_uint, RETRO_RUMBLE_STRONG, 1),
                "no such port"
            );
            assert!(!set(0, 7, 1), "no such effect");
        }
        assert_eq!(with(|fe| fe.rumble[0]), [40_000, 100]);
        unsafe { set(0, RETRO_RUMBLE_STRONG, 0) };
        assert_eq!(with(|fe| fe.rumble[0]), [0, 100]);
    }
}
