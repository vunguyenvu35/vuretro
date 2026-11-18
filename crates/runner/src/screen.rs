//! The real window: a minifb window behind the [`Screen`] trait.

use crate::ports::{Keyboard, Screen};
use crate::window::{act_id, Act, SLOTS};
use anyhow::Result;
use minifb::{Key, KeyRepeat, Menu, Scale, Window, WindowOptions};

const MENU_MOD: usize = if cfg!(target_os = "macos") {
    minifb::MENU_KEY_COMMAND
} else {
    minifb::MENU_KEY_CTRL
};

/// Add the game menus to the window (macOS and Windows; elsewhere minifb has none and
/// the hotkeys do the job). Call again for a new window.
pub fn install_menus(window: &mut Window) {
    let mut add = |name: &str, build: &dyn Fn(&mut Menu)| match Menu::new(name) {
        Ok(mut m) => {
            build(&mut m);
            window.add_menu(&m);
        }
        Err(e) => log::debug!("no {name} menu: {e}"),
    };
    let item = |m: &mut Menu, label: &str, act: Act, key: Option<Key>| {
        let mut it = m.add_item(label, act_id(act));
        match key {
            Some(k) => it.shortcut(k, MENU_MOD).build(),
            None => it.build(),
        };
    };
    add("Game", &|m| {
        item(m, "Pause / Resume (P or F3)", Act::Pause, Some(Key::P));
        item(m, "Reset (F2)", Act::Reset, Some(Key::R));
        m.add_separator();
        m.add_item("Fast Forward: hold Tab", 0)
            .enabled(false)
            .build();
        m.add_separator();
        item(m, "Screenshot (F12)", Act::Screenshot, Some(Key::T));
        item(m, "Sound On / Off (F8)", Act::Mute, Some(Key::M));
        m.add_separator();
        item(m, "Close Game (Esc)", Act::Quit, None);
    });
    add("State", &|m| {
        item(m, "Save State (F5)", Act::SaveState, Some(Key::S));
        item(m, "Load State (F7)", Act::LoadState, Some(Key::L));
        m.add_separator();
        item(m, "Next Slot (F6)", Act::NextSlot, None);
        for n in 1..=SLOTS {
            item(m, &format!("Use Slot {n}"), Act::Slot(n), None);
        }
    });
    add("Size", &|m| {
        item(m, "Smaller (F9)", Act::SizeStep(-1), Some(Key::Minus));
        item(m, "Bigger (F10)", Act::SizeStep(1), Some(Key::Equal));
        item(m, "CRT Effect (F4)", Act::Crt, None);
        m.add_separator();
        for n in 1..=4u8 {
            item(m, &format!("{n}×"), Act::Size(n), None);
        }
    });
}

/// minifb's own scaling only does 1, 2, 4, 8 and 16; for other sizes the window is
/// made that many times bigger and the frame is stretched to fit it.
fn open_window(title: &str, base_w: usize, base_h: usize, factor: usize) -> Result<Window> {
    let (scale, stretch) = match factor {
        1 => (Scale::X1, 1),
        2 => (Scale::X2, 1),
        4 => (Scale::X4, 1),
        8 => (Scale::X8, 1),
        n => (Scale::X1, n),
    };
    Ok(Window::new(
        title,
        base_w * stretch,
        base_h * stretch,
        WindowOptions {
            resize: true,
            scale,
            ..WindowOptions::default()
        },
    )?)
}

pub struct MiniFbScreen {
    window: Window,
}

impl MiniFbScreen {
    pub fn open(title: &str, base_w: usize, base_h: usize, factor: usize) -> Result<Self> {
        let mut window = open_window(title, base_w, base_h, factor)?;
        install_menus(&mut window);
        Ok(MiniFbScreen { window })
    }
}

impl Keyboard for MiniFbScreen {
    fn key_down(&self, key: Key) -> bool {
        self.window.is_key_down(key)
    }
}

impl Screen for MiniFbScreen {
    fn is_open(&self) -> bool {
        self.window.is_open()
    }
    fn key_pressed(&self, key: Key) -> bool {
        self.window.is_key_pressed(key, KeyRepeat::No)
    }
    fn menu_pressed(&mut self) -> Option<usize> {
        self.window.is_menu_pressed()
    }
    fn is_active(&mut self) -> bool {
        self.window.is_active()
    }
    fn present(&mut self, frame: &[u32], w: usize, h: usize) -> Result<()> {
        Ok(self.window.update_with_buffer(frame, w, h)?)
    }
    fn idle(&mut self) {
        self.window.update();
    }
    fn set_title(&mut self, title: &str) {
        self.window.set_title(title);
    }
    fn reopen(&mut self, title: &str, base_w: usize, base_h: usize, factor: usize) -> Result<()> {
        let at = self.window.get_position();
        // Open the new window before the old one closes so the app never has none.
        let mut window = open_window(title, base_w, base_h, factor)?;
        window.set_position(at.0, at.1);
        install_menus(&mut window);
        self.window = window;
        Ok(())
    }
}
