//! What a [`Session`](crate::session::Session) needs from the machine: a window, an
//! emulator, a controller and a source of frames. The real ones are in `screen.rs` and
//! `real.rs`; the session tests use fakes.

use crate::window::Act;
use anyhow::Result;
use minifb::Key;
use std::path::Path;

/// A decoded video frame: 0RGB pixels, width, height.
pub type Frame = (Vec<u32>, usize, usize);

/// Keyboard state, as the controller polls it.
pub trait Keyboard {
    fn key_down(&self, key: Key) -> bool;
}

/// How the picture sits in the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct View {
    /// A borderless window the size of the display.
    pub fullscreen: bool,
    /// Whole-number scaling with black borders; the session prepares the enlarged picture.
    pub integer: bool,
}

/// The game window.
pub trait Screen: Keyboard {
    fn is_open(&self) -> bool;
    /// True once for each new press of `key`.
    fn key_pressed(&self, key: Key) -> bool;
    /// Next chosen menu item (its id), if any.
    fn menu_pressed(&mut self) -> Option<usize>;
    fn is_active(&mut self) -> bool;
    fn present(&mut self, frame: &[u32], w: usize, h: usize) -> Result<()>;
    /// Keep the window alive without a new picture.
    fn idle(&mut self);
    fn set_title(&mut self, title: &str);
    /// Replace the window by one of another size or view, at the same place (a full-screen
    /// window covers the display instead).
    fn reopen(
        &mut self,
        title: &str,
        base_w: usize,
        base_h: usize,
        factor: usize,
        view: View,
    ) -> Result<()>;
    /// Size of the area the picture is drawn in, in pixels.
    fn size(&self) -> (usize, usize);
    /// The view the window really has (a full screen that could not be made is not one).
    fn view(&self) -> View;
    /// Whether this system can show a full-screen window.
    fn can_fullscreen(&self) -> bool {
        true
    }
}

/// The libretro core.
pub trait Emulator {
    fn run(&self);
    fn reset(&self);
    fn save_state(&self) -> Option<Vec<u8>>;
    fn load_state(&self, data: &[u8]) -> bool;
    fn save_sram(&self, path: &Path) -> Result<usize>;
    /// Insert the next disc of a multi-disc game; the text says what happened.
    /// `None` when the core has no disc control.
    fn next_disc(&self) -> Option<String> {
        None
    }
}

/// Frontend actions asked for from a gamepad (a chord on the Select button).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Hotkeys {
    /// Actions asked for since the last call (one per press).
    pub acts: Vec<Act>,
    /// Fast-forward is held.
    pub fast_forward: bool,
    /// Rewind is held.
    pub rewind: bool,
}

/// Turns keyboard and gamepads into the game's buttons.
pub trait Controller {
    fn poll(&mut self, screen: &dyn Screen);
    /// Gamepad hotkeys. Called every frame, also while the game is paused.
    fn hotkeys(&mut self, _screen: &dyn Screen) -> Hotkeys {
        Hotkeys::default()
    }
}

/// Video frames the core has produced.
pub trait FrameSource {
    /// The frame produced since the last call, if there is one.
    fn take_new(&mut self) -> Option<Frame>;
    /// The latest frame, whether or not it was seen before.
    fn current(&self) -> Frame;
}
