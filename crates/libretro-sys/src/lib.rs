//! Minimal FFI types for the libretro API (subset of libretro.h).
//! Only what the vuretro frontend uses; extend as cores need more.
#![allow(non_camel_case_types)]

use std::os::raw::{c_char, c_int, c_uint, c_void};

pub const RETRO_API_VERSION: c_uint = 1;

// Environment commands
/// Flag the C header puts on some command numbers. It is part of the number: the header has
/// two commands numbered 44 (`SET_SERIALIZATION_QUIRKS` and `SET_HW_SHARED_CONTEXT |
/// EXPERIMENTAL`), so it must never be stripped before matching.
pub const RETRO_ENVIRONMENT_EXPERIMENTAL: c_uint = 0x10000;
pub const RETRO_ENVIRONMENT_SET_ROTATION: c_uint = 1;
pub const RETRO_ENVIRONMENT_GET_OVERSCAN: c_uint = 2;
pub const RETRO_ENVIRONMENT_GET_CAN_DUPE: c_uint = 3;
pub const RETRO_ENVIRONMENT_SET_MESSAGE: c_uint = 6;
pub const RETRO_ENVIRONMENT_SHUTDOWN: c_uint = 7;
pub const RETRO_ENVIRONMENT_SET_HW_RENDER: c_uint = 14;
pub const RETRO_ENVIRONMENT_SET_SUPPORT_NO_GAME: c_uint = 18;
pub const RETRO_ENVIRONMENT_GET_INPUT_DEVICE_CAPABILITIES: c_uint = 24;
pub const RETRO_ENVIRONMENT_SET_SYSTEM_AV_INFO: c_uint = 32;
pub const RETRO_ENVIRONMENT_SET_SUBSYSTEM_INFO: c_uint = 34;
pub const RETRO_ENVIRONMENT_SET_CONTROLLER_INFO: c_uint = 35;
pub const RETRO_ENVIRONMENT_SET_MEMORY_MAPS: c_uint = 36 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_SET_GEOMETRY: c_uint = 37;
pub const RETRO_ENVIRONMENT_GET_USERNAME: c_uint = 38;
pub const RETRO_ENVIRONMENT_GET_LANGUAGE: c_uint = 39;
pub const RETRO_ENVIRONMENT_SET_SERIALIZATION_QUIRKS: c_uint = 44;
pub const RETRO_ENVIRONMENT_SET_HW_SHARED_CONTEXT: c_uint = 44 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_GET_PREFERRED_HW_RENDER: c_uint = 56;
pub const RETRO_ENVIRONMENT_GET_AUDIO_VIDEO_ENABLE: c_uint = 47 | RETRO_ENVIRONMENT_EXPERIMENTAL;
pub const RETRO_ENVIRONMENT_GET_MESSAGE_INTERFACE_VERSION: c_uint = 59;
pub const RETRO_ENVIRONMENT_SET_MESSAGE_EXT: c_uint = 60;
pub const RETRO_ENVIRONMENT_GET_INPUT_MAX_USERS: c_uint = 61;
pub const RETRO_ENVIRONMENT_SET_VARIABLE: c_uint = 70;
/// Bit of the input-capabilities mask for `RETRO_DEVICE_JOYPAD`.
pub const RETRO_DEVICE_JOYPAD_BIT: u64 = 1 << 1;
pub const RETRO_ENVIRONMENT_SET_PERFORMANCE_LEVEL: c_uint = 8;
pub const RETRO_ENVIRONMENT_GET_SYSTEM_DIRECTORY: c_uint = 9;
pub const RETRO_ENVIRONMENT_SET_PIXEL_FORMAT: c_uint = 10;
pub const RETRO_ENVIRONMENT_SET_INPUT_DESCRIPTORS: c_uint = 11;
pub const RETRO_ENVIRONMENT_SET_DISK_CONTROL_INTERFACE: c_uint = 13;
pub const RETRO_ENVIRONMENT_GET_RUMBLE_INTERFACE: c_uint = 23;
pub const RETRO_ENVIRONMENT_GET_VARIABLE: c_uint = 15;
pub const RETRO_ENVIRONMENT_SET_VARIABLES: c_uint = 16;
pub const RETRO_ENVIRONMENT_GET_VARIABLE_UPDATE: c_uint = 17;
pub const RETRO_ENVIRONMENT_GET_LOG_INTERFACE: c_uint = 27;
pub const RETRO_ENVIRONMENT_GET_CORE_ASSETS_DIRECTORY: c_uint = 30;
pub const RETRO_ENVIRONMENT_GET_SAVE_DIRECTORY: c_uint = 31;

// Rumble effects (`retro_rumble_interface::set_rumble_state`)
pub const RETRO_RUMBLE_STRONG: c_uint = 0;
pub const RETRO_RUMBLE_WEAK: c_uint = 1;

// Pixel formats
// Memory regions
pub const RETRO_MEMORY_SAVE_RAM: c_uint = 0;

pub const RETRO_PIXEL_FORMAT_0RGB1555: c_uint = 0;
pub const RETRO_PIXEL_FORMAT_XRGB8888: c_uint = 1;
pub const RETRO_PIXEL_FORMAT_RGB565: c_uint = 2;

// Devices
pub const RETRO_DEVICE_JOYPAD: c_uint = 1;

// Joypad ids
pub const RETRO_DEVICE_ID_JOYPAD_B: c_uint = 0;
pub const RETRO_DEVICE_ID_JOYPAD_Y: c_uint = 1;
pub const RETRO_DEVICE_ID_JOYPAD_SELECT: c_uint = 2;
pub const RETRO_DEVICE_ID_JOYPAD_START: c_uint = 3;
pub const RETRO_DEVICE_ID_JOYPAD_UP: c_uint = 4;
pub const RETRO_DEVICE_ID_JOYPAD_DOWN: c_uint = 5;
pub const RETRO_DEVICE_ID_JOYPAD_LEFT: c_uint = 6;
pub const RETRO_DEVICE_ID_JOYPAD_RIGHT: c_uint = 7;
pub const RETRO_DEVICE_ID_JOYPAD_A: c_uint = 8;
pub const RETRO_DEVICE_ID_JOYPAD_X: c_uint = 9;
pub const RETRO_DEVICE_ID_JOYPAD_L: c_uint = 10;
pub const RETRO_DEVICE_ID_JOYPAD_R: c_uint = 11;
pub const RETRO_DEVICE_ID_JOYPAD_L2: c_uint = 12;
pub const RETRO_DEVICE_ID_JOYPAD_R2: c_uint = 13;
pub const RETRO_DEVICE_ID_JOYPAD_L3: c_uint = 14;
pub const RETRO_DEVICE_ID_JOYPAD_R3: c_uint = 15;
pub const JOYPAD_BUTTON_COUNT: usize = 16;

#[repr(C)]
pub struct retro_game_info {
    pub path: *const c_char,
    pub data: *const c_void,
    pub size: usize,
    pub meta: *const c_char,
}

#[repr(C)]
pub struct retro_system_info {
    pub library_name: *const c_char,
    pub library_version: *const c_char,
    pub valid_extensions: *const c_char,
    pub need_fullpath: bool,
    pub block_extract: bool,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct retro_game_geometry {
    pub base_width: c_uint,
    pub base_height: c_uint,
    pub max_width: c_uint,
    pub max_height: c_uint,
    pub aspect_ratio: f32,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct retro_system_timing {
    pub fps: f64,
    pub sample_rate: f64,
}

#[repr(C)]
#[derive(Default, Clone, Copy, Debug)]
pub struct retro_system_av_info {
    pub geometry: retro_game_geometry,
    pub timing: retro_system_timing,
}

#[repr(C)]
pub struct retro_variable {
    pub key: *const c_char,
    pub value: *const c_char,
}

/// Disc swapping, announced by multi-disc cores (`SET_DISK_CONTROL_INTERFACE`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct retro_disk_control_callback {
    pub set_eject_state: Option<unsafe extern "C" fn(ejected: bool) -> bool>,
    pub get_eject_state: Option<unsafe extern "C" fn() -> bool>,
    pub get_image_index: Option<unsafe extern "C" fn() -> c_uint>,
    pub set_image_index: Option<unsafe extern "C" fn(index: c_uint) -> bool>,
    pub get_num_images: Option<unsafe extern "C" fn() -> c_uint>,
    pub replace_image_index:
        Option<unsafe extern "C" fn(index: c_uint, info: *const c_void) -> bool>,
    pub add_image_index: Option<unsafe extern "C" fn() -> bool>,
}

pub type retro_environment_t = unsafe extern "C" fn(cmd: c_uint, data: *mut c_void) -> bool;
pub type retro_video_refresh_t =
    unsafe extern "C" fn(data: *const c_void, width: c_uint, height: c_uint, pitch: usize);
pub type retro_audio_sample_t = unsafe extern "C" fn(left: i16, right: i16);
pub type retro_audio_sample_batch_t =
    unsafe extern "C" fn(data: *const i16, frames: usize) -> usize;
pub type retro_input_poll_t = unsafe extern "C" fn();
pub type retro_input_state_t =
    unsafe extern "C" fn(port: c_uint, device: c_uint, index: c_uint, id: c_uint) -> i16;

/// The logging function a core asks the frontend for (`GET_LOG_INTERFACE`). In C it is
/// variadic (`printf`-style); declared here with its two fixed arguments only, which is how
/// it is read: extra arguments are ignored and the format string is logged as it is.
#[repr(C)]
pub struct retro_log_callback {
    pub log: Option<unsafe extern "C" fn(level: c_int, fmt: *const c_char)>,
}

/// One entry of the list a core sends with `SET_INPUT_DESCRIPTORS`: what a controller button
/// does in this game. The list ends with an entry whose `description` is null.
#[repr(C)]
pub struct retro_input_descriptor {
    pub port: c_uint,
    pub device: c_uint,
    pub index: c_uint,
    pub id: c_uint,
    pub description: *const c_char,
}

/// The rumble function a core asks the frontend for (`GET_RUMBLE_INTERFACE`).
/// `effect` is [`RETRO_RUMBLE_STRONG`] or [`RETRO_RUMBLE_WEAK`]; `strength` 0 stops it.
#[repr(C)]
pub struct retro_rumble_interface {
    pub set_rumble_state:
        Option<unsafe extern "C" fn(port: c_uint, effect: c_uint, strength: u16) -> bool>,
}

/// `SET_MESSAGE`: a short text the core wants shown for `frames` frames.
#[repr(C)]
pub struct retro_message {
    pub msg: *const c_char,
    pub frames: c_uint,
}

/// `SET_MESSAGE_EXT`: like [`retro_message`] with a duration in ms, priority and level
/// (0 debug, 1 info, 2 warn, 3 error). The trailing fields are read as `c_uint`s because
/// the C enums are 32 bits wide.
#[repr(C)]
pub struct retro_message_ext {
    pub msg: *const c_char,
    pub duration: c_uint,
    pub priority: c_uint,
    pub level: c_uint,
    pub target: c_uint,
    pub kind: c_uint,
    pub progress: i8,
}

// Hardware rendering: the core draws with OpenGL into a framebuffer the frontend owns.

pub const RETRO_HW_CONTEXT_NONE: c_uint = 0;
pub const RETRO_HW_CONTEXT_OPENGL: c_uint = 1;
pub const RETRO_HW_CONTEXT_OPENGLES2: c_uint = 2;
pub const RETRO_HW_CONTEXT_OPENGL_CORE: c_uint = 3;
pub const RETRO_HW_CONTEXT_OPENGLES3: c_uint = 4;
pub const RETRO_HW_CONTEXT_OPENGLES_VERSION: c_uint = 5;
pub const RETRO_HW_CONTEXT_VULKAN: c_uint = 6;

/// What a core passes to the video callback instead of pixels when it drew into the frontend's
/// framebuffer (`RETRO_HW_FRAME_BUFFER_VALID` is `(void*)-1`).
pub const RETRO_HW_FRAME_BUFFER_VALID: usize = usize::MAX;

pub type retro_hw_context_reset_t = Option<unsafe extern "C" fn()>;
pub type retro_hw_get_current_framebuffer_t = Option<unsafe extern "C" fn() -> usize>;
pub type retro_proc_address_t = Option<unsafe extern "C" fn()>;
pub type retro_hw_get_proc_address_t =
    Option<unsafe extern "C" fn(sym: *const c_char) -> retro_proc_address_t>;

/// `SET_HW_RENDER`: what the core wants (type, version, buffers) and the functions it gives
/// the frontend; the frontend fills in `get_current_framebuffer` and `get_proc_address`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct retro_hw_render_callback {
    pub context_type: c_uint,
    pub context_reset: retro_hw_context_reset_t,
    pub get_current_framebuffer: retro_hw_get_current_framebuffer_t,
    pub get_proc_address: retro_hw_get_proc_address_t,
    pub depth: bool,
    pub stencil: bool,
    /// The core draws with the origin at the bottom left (the OpenGL way), so the picture is
    /// upside down compared with a normal frame.
    pub bottom_left_origin: bool,
    pub version_major: c_uint,
    pub version_minor: c_uint,
    pub cache_context: bool,
    pub context_destroy: retro_hw_context_reset_t,
    pub debug_context: bool,
}
