//! A minimal libretro core used to test the frontend without real games.
//! Draws moving color bars (XRGB8888), plays a 440 Hz tone, and moves a
//! square with the D-pad. Supports save states (square position + frame).
#![allow(clippy::missing_safety_doc)]

use libretro_sys::*;
use std::os::raw::{c_char, c_uint, c_void};
use std::sync::Mutex;

const W: usize = 320;
const H: usize = 224;
const RATE: f64 = 44_100.0;
const FPS: f64 = 60.0;

struct State {
    env: Option<retro_environment_t>,
    video: Option<retro_video_refresh_t>,
    audio_batch: Option<retro_audio_sample_batch_t>,
    poll: Option<retro_input_poll_t>,
    input: Option<retro_input_state_t>,
    frame: u64,
    x: i32,
    y: i32,
    phase: f64,
    fb: Vec<u32>,
    sram: [u8; 64],
    square: usize,
    /// Set when the core was asked to draw with OpenGL (`VURETRO_TEST_HW=1`).
    hw: Option<Hw>,
}

/// The OpenGL side of the core: what the frontend gave it, and the GL functions it uses.
#[derive(Clone, Copy)]
struct Hw {
    get_framebuffer: retro_hw_get_current_framebuffer_t,
    get_proc: retro_hw_get_proc_address_t,
    /// `glBindFramebuffer`, `glViewport`, `glClearColor`, `glClear`, `glEnable`, `glScissor`
    /// once the context is ready.
    gl: Option<Gl>,
}

#[derive(Clone, Copy)]
struct Gl {
    bind_framebuffer: unsafe extern "C" fn(c_uint, c_uint),
    viewport: unsafe extern "C" fn(i32, i32, i32, i32),
    clear_color: unsafe extern "C" fn(f32, f32, f32, f32),
    clear: unsafe extern "C" fn(c_uint),
    enable: unsafe extern "C" fn(c_uint),
    scissor: unsafe extern "C" fn(i32, i32, i32, i32),
}

const GL_FRAMEBUFFER: c_uint = 0x8D40;
const GL_COLOR_BUFFER_BIT: c_uint = 0x4000;
const GL_SCISSOR_TEST: c_uint = 0x0C11;

static S: Mutex<State> = Mutex::new(State {
    env: None,
    video: None,
    audio_batch: None,
    poll: None,
    input: None,
    frame: 0,
    x: 150,
    y: 100,
    phase: 0.0,
    fb: Vec::new(),
    sram: [0; 64],
    square: 20,
    hw: None,
});

#[no_mangle]
pub extern "C" fn retro_api_version() -> c_uint {
    RETRO_API_VERSION
}
#[no_mangle]
pub extern "C" fn retro_set_environment(cb: retro_environment_t) {
    S.lock().unwrap().env = Some(cb);
}
#[no_mangle]
pub extern "C" fn retro_set_video_refresh(cb: retro_video_refresh_t) {
    S.lock().unwrap().video = Some(cb);
}
#[no_mangle]
pub extern "C" fn retro_set_audio_sample(_cb: retro_audio_sample_t) {}
#[no_mangle]
pub extern "C" fn retro_set_audio_sample_batch(cb: retro_audio_sample_batch_t) {
    S.lock().unwrap().audio_batch = Some(cb);
}
#[no_mangle]
pub extern "C" fn retro_set_input_poll(cb: retro_input_poll_t) {
    S.lock().unwrap().poll = Some(cb);
}
#[no_mangle]
pub extern "C" fn retro_set_input_state(cb: retro_input_state_t) {
    S.lock().unwrap().input = Some(cb);
}
#[no_mangle]
pub extern "C" fn retro_init() {
    let env = {
        let mut s = S.lock().unwrap();
        s.fb = vec![0; W * H];
        s.env
    };
    // Announce one core option so the runner's option handling can be tested.
    if let Some(env) = env {
        let mut vars = [
            retro_variable {
                key: c"vuretro_test_square".as_ptr(),
                value: c"Square size; small|large".as_ptr(),
            },
            retro_variable {
                key: std::ptr::null(),
                value: std::ptr::null(),
            },
        ];
        unsafe {
            env(
                RETRO_ENVIRONMENT_SET_VARIABLES,
                vars.as_mut_ptr() as *mut c_void,
            )
        };
    }
}
#[no_mangle]
pub extern "C" fn retro_deinit() {}
#[no_mangle]
pub extern "C" fn retro_set_controller_port_device(_port: c_uint, _device: c_uint) {}
#[no_mangle]
pub extern "C" fn retro_reset() {
    let mut s = S.lock().unwrap();
    s.frame = 0;
    s.x = 150;
    s.y = 100;
}
#[no_mangle]
pub extern "C" fn retro_unload_game() {}

#[no_mangle]
pub unsafe extern "C" fn retro_get_system_info(info: *mut retro_system_info) {
    let info = &mut *info;
    info.library_name = c"vuretro-test-core".as_ptr();
    info.library_version = c"0.1".as_ptr();
    info.valid_extensions = c"bin|test".as_ptr() as *const c_char;
    info.need_fullpath = false;
    info.block_extract = false;
}

#[no_mangle]
pub unsafe extern "C" fn retro_get_system_av_info(info: *mut retro_system_av_info) {
    *info = retro_system_av_info {
        geometry: retro_game_geometry {
            base_width: W as c_uint,
            base_height: H as c_uint,
            max_width: W as c_uint,
            max_height: H as c_uint,
            aspect_ratio: 4.0 / 3.0,
        },
        timing: retro_system_timing {
            fps: FPS,
            sample_rate: RATE,
        },
    };
}

#[no_mangle]
pub unsafe extern "C" fn retro_load_game(_game: *const retro_game_info) -> bool {
    let env = S.lock().unwrap().env;
    let large = env.is_some_and(|env| {
        let mut var = retro_variable {
            key: c"vuretro_test_square".as_ptr(),
            value: std::ptr::null(),
        };
        env(
            RETRO_ENVIRONMENT_GET_VARIABLE,
            &mut var as *mut _ as *mut c_void,
        ) && !var.value.is_null()
            && std::ffi::CStr::from_ptr(var.value).to_bytes() == b"large"
    });
    S.lock().unwrap().square = if large { 40 } else { 20 };
    let mut fmt = RETRO_PIXEL_FORMAT_XRGB8888;
    let ok = match env {
        Some(env) => env(
            RETRO_ENVIRONMENT_SET_PIXEL_FORMAT,
            &mut fmt as *mut c_uint as *mut c_void,
        ),
        None => false,
    };
    // `VURETRO_TEST_HW=1`: draw with OpenGL, like a 3D core. A frontend that cannot give a
    // context makes the core fall back to the plain picture.
    if let (true, Some(env)) = (std::env::var_os("VURETRO_TEST_HW").is_some(), env) {
        let mut cb = retro_hw_render_callback {
            context_type: RETRO_HW_CONTEXT_OPENGL_CORE,
            context_reset: Some(hw_reset),
            get_current_framebuffer: None,
            get_proc_address: None,
            depth: false,
            stencil: false,
            bottom_left_origin: true,
            version_major: 3,
            version_minor: 3,
            cache_context: false,
            context_destroy: Some(hw_destroy),
            debug_context: false,
        };
        if env(
            RETRO_ENVIRONMENT_SET_HW_RENDER,
            &mut cb as *mut _ as *mut c_void,
        ) {
            S.lock().unwrap().hw = Some(Hw {
                get_framebuffer: cb.get_current_framebuffer,
                get_proc: cb.get_proc_address,
                gl: None,
            });
        }
    }
    ok
}

/// The context is ready: look the GL functions up.
unsafe extern "C" fn hw_reset() {
    let mut s = S.lock().unwrap();
    let Some(hw) = s.hw.as_mut() else { return };
    let Some(get) = hw.get_proc else { return };
    macro_rules! load {
        ($name:expr, $ty:ty) => {{
            let Some(f) = get($name.as_ptr()) else { return };
            std::mem::transmute::<unsafe extern "C" fn(), $ty>(f)
        }};
    }
    hw.gl = Some(Gl {
        bind_framebuffer: load!(c"glBindFramebuffer", unsafe extern "C" fn(c_uint, c_uint)),
        viewport: load!(c"glViewport", unsafe extern "C" fn(i32, i32, i32, i32)),
        clear_color: load!(c"glClearColor", unsafe extern "C" fn(f32, f32, f32, f32)),
        clear: load!(c"glClear", unsafe extern "C" fn(c_uint)),
        enable: load!(c"glEnable", unsafe extern "C" fn(c_uint)),
        scissor: load!(c"glScissor", unsafe extern "C" fn(i32, i32, i32, i32)),
    });
}

/// The context goes away: forget the functions.
unsafe extern "C" fn hw_destroy() {
    if let Some(hw) = S.lock().unwrap().hw.as_mut() {
        hw.gl = None;
    }
}

/// Draw the test picture with OpenGL: all red, and blue in the top half of the image (the
/// top half is the high rows, because OpenGL counts rows from the bottom).
unsafe fn draw_with_gl(hw: Hw, gl: Gl) {
    let fb = hw.get_framebuffer.map_or(0, |f| f()) as c_uint;
    (gl.bind_framebuffer)(GL_FRAMEBUFFER, fb);
    (gl.viewport)(0, 0, W as i32, H as i32);
    (gl.clear_color)(1.0, 0.0, 0.0, 1.0);
    (gl.clear)(GL_COLOR_BUFFER_BIT);
    (gl.enable)(GL_SCISSOR_TEST);
    (gl.scissor)(0, H as i32 / 2, W as i32, H as i32 / 2);
    (gl.clear_color)(0.0, 0.0, 1.0, 1.0);
    (gl.clear)(GL_COLOR_BUFFER_BIT);
}

#[no_mangle]
pub unsafe extern "C" fn retro_run() {
    // Copy callbacks out so we don't hold the lock while calling the frontend.
    let (poll, input) = {
        let s = S.lock().unwrap();
        (s.poll, s.input)
    };
    if let Some(p) = poll {
        p();
    }
    let pressed = |id: c_uint| input.is_some_and(|f| f(0, RETRO_DEVICE_JOYPAD, 0, id) != 0);
    let (dx, dy) = (
        pressed(RETRO_DEVICE_ID_JOYPAD_RIGHT) as i32 - pressed(RETRO_DEVICE_ID_JOYPAD_LEFT) as i32,
        pressed(RETRO_DEVICE_ID_JOYPAD_DOWN) as i32 - pressed(RETRO_DEVICE_ID_JOYPAD_UP) as i32,
    );

    let mut s = S.lock().unwrap();
    s.frame += 1;
    s.sram[0] = s.sram[0].wrapping_add(1); // proves save RAM round-trips
    let sq = s.square;
    s.x = (s.x + dx * 2).clamp(0, W as i32 - 20);
    s.y = (s.y + dy * 2).clamp(0, H as i32 - 20);
    let (frame, sx, sy) = (s.frame as usize, s.x as usize, s.y as usize);
    const BARS: [u32; 7] = [
        0xFFFFFF, 0xFFFF00, 0x00FFFF, 0x00FF00, 0xFF00FF, 0xFF0000, 0x0000FF,
    ];
    for y in 0..H {
        for x in 0..W {
            let bar = ((x + frame) / (W / 7 + 1)) % 7;
            let inside = x >= sx && x < sx + sq && y >= sy && y < sy + sq;
            s.fb[y * W + x] = if inside { 0x101010 } else { BARS[bar] };
        }
    }

    let n = (RATE / FPS) as usize;
    let mut audio = Vec::with_capacity(n * 2);
    for _ in 0..n {
        let v = (s.phase.sin() * 3000.0) as i16;
        audio.push(v);
        audio.push(v);
        s.phase =
            (s.phase + 2.0 * std::f64::consts::PI * 440.0 / RATE) % (2.0 * std::f64::consts::PI);
    }
    let fb = s.fb.clone();
    let (video, batch) = (s.video, s.audio_batch);
    drop(s);

    let hw = S.lock().unwrap().hw;
    if let Some(v) = video {
        match hw.and_then(|h| h.gl.map(|gl| (h, gl))) {
            Some((hw, gl)) => {
                draw_with_gl(hw, gl);
                v(
                    RETRO_HW_FRAME_BUFFER_VALID as *const c_void,
                    W as c_uint,
                    H as c_uint,
                    0,
                );
            }
            None => v(
                fb.as_ptr() as *const c_void,
                W as c_uint,
                H as c_uint,
                W * 4,
            ),
        }
    }
    if let Some(b) = batch {
        b(audio.as_ptr(), n);
    }
}

#[no_mangle]
pub extern "C" fn retro_get_memory_data(id: c_uint) -> *mut c_void {
    if id == RETRO_MEMORY_SAVE_RAM {
        S.lock().unwrap().sram.as_mut_ptr() as *mut c_void
    } else {
        std::ptr::null_mut()
    }
}

#[no_mangle]
pub extern "C" fn retro_get_memory_size(id: c_uint) -> usize {
    if id == RETRO_MEMORY_SAVE_RAM {
        64
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn retro_serialize_size() -> usize {
    16
}

#[no_mangle]
pub unsafe extern "C" fn retro_serialize(data: *mut c_void, size: usize) -> bool {
    if size < 16 {
        return false;
    }
    let s = S.lock().unwrap();
    let out = std::slice::from_raw_parts_mut(data as *mut u8, 16);
    out[0..8].copy_from_slice(&s.frame.to_le_bytes());
    out[8..12].copy_from_slice(&s.x.to_le_bytes());
    out[12..16].copy_from_slice(&s.y.to_le_bytes());
    true
}

#[no_mangle]
pub unsafe extern "C" fn retro_unserialize(data: *const c_void, size: usize) -> bool {
    if size < 16 {
        return false;
    }
    let d = std::slice::from_raw_parts(data as *const u8, 16);
    let mut s = S.lock().unwrap();
    s.frame = u64::from_le_bytes(d[0..8].try_into().unwrap());
    s.x = i32::from_le_bytes(d[8..12].try_into().unwrap());
    s.y = i32::from_le_bytes(d[12..16].try_into().unwrap());
    true
}
