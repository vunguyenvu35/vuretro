//! What a [`Session`](crate::session::Session) needs from the machine: a window, an
//! emulator, a controller, a source of frames and somewhere to keep files. The real ones are
//! adapters in `vuretro-runner`; the session tests use fakes.

use crate::keys::Key;
use crate::window::Act;
use anyhow::Result;
use std::path::Path;
use vuretro_core::protocol::Event;

/// A decoded video frame: 0RGB pixels, width, height.
///
/// A frame with no pixels but a size is one that is still on the graphics card (a 3D core drew
/// it there): see [`Screen::present_gpu`] and [`FrameSource::current`].
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

/// A look the window can draw itself, over the picture as the game made it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Look {
    #[default]
    Plain,
    /// Scanlines: the last row of every enlarged pixel is darker.
    Crt,
    /// A handheld grid: the last row and column of every enlarged pixel are darker.
    Lcd,
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
    /// Say in the State menu which save slot is the current one (`current`, from 1) and what
    /// each holds: one label per slot. Windows without a menu do nothing.
    fn set_slot_menu(&mut self, _labels: &[String], _current: u8) {}
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
    /// Whether the window draws the CRT and LCD looks itself (on the graphics card). When it
    /// does, the session sends the picture as the game made it and says which look to draw
    /// with [`set_look`](Screen::set_look); otherwise the session draws the look into the
    /// picture it sends.
    fn gpu_look(&self) -> bool {
        false
    }
    /// The look to draw over the pictures that follow. Only called when `gpu_look` is true.
    fn set_look(&mut self, _look: Look) {}
    /// Show the `w` x `h` picture a 3D core has just drawn, straight from the graphics card.
    /// Returns false when this window cannot (the session then asks for the pixels).
    fn present_gpu(&mut self, _w: usize, _h: usize) -> Result<bool> {
        Ok(false)
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
    /// The frame produced since the last call, if there is one. A 3D core's frame may come
    /// with no pixels (see [`Frame`]): the picture is then still on the graphics card.
    fn take_new(&mut self) -> Option<Frame>;
    /// The latest frame, whether or not it was seen before, always with its pixels (a frame
    /// on the graphics card is read back now, which takes time: ask only when needed).
    fn current(&self) -> Frame;
}

/// Where the session tells the outside what happens while it plays (see
/// [`vuretro_core::protocol`]): save slots used, screenshots, play time.
pub trait Events {
    fn emit(&self, event: Event);
}

/// Nobody is told anything.
pub struct NoEvents;

impl Events for NoEvents {
    fn emit(&self, _event: Event) {}
}

/// Where the session keeps what it saves: save states and pictures.
pub trait Files {
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>>;
    fn write(&self, path: &Path, data: &[u8]) -> std::io::Result<()>;
    /// When the file was last written, `None` when it does not exist (or the adapter cannot
    /// tell). The State menu shows which slots hold a save with it.
    fn modified(&self, _path: &Path) -> Option<std::time::SystemTime> {
        None
    }
    /// Write a picture of 0RGB pixels as a PNG file, creating its folder if needed.
    fn write_png(&self, path: &Path, buf: &[u32], w: usize, h: usize) -> Result<()>;
}
