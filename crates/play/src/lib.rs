//! vuretro play: the rules and the frame loop of one game session. It knows what a game
//! window, an emulator, a controller and a frame source do (`ports`) but not how: windows,
//! audio, gamepads and libretro cores are adapters in `vuretro-runner`, so everything here
//! runs against fakes in tests.
//!
//! Dependency rule: no window, audio, gamepad, FFI or image-encoding crates in here.

pub mod cover;
pub mod crt;
pub mod keys;
pub mod lcd;
pub mod neogeo;
pub mod perf;
pub mod ports;
pub mod rewind;
pub mod rumble;
pub mod scale;
pub mod scale2x;
pub mod session;
pub mod speed;
pub mod window;

pub use keys::Key;
