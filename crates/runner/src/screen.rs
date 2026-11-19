//! The real window: a minifb window behind the [`Screen`] trait.

use crate::display;
use crate::ports::{Keyboard, Screen, View};
use crate::window::{act_id, Act, SLOTS};
use anyhow::Result;
use minifb::{Key, KeyRepeat, Menu, Scale, ScaleMode, Window, WindowOptions};

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
        item(m, "Change Disc (F11)", Act::NextDisc, None);
        m.add_separator();
        m.add_item("Fast Forward: hold Tab", 0)
            .enabled(false)
            .build();
        item(
            m,
            "Fast Forward On / Off (F1)",
            Act::FastForwardToggle,
            None,
        );
        item(m, "Slow Motion On / Off (Shift+F1)", Act::SlowMotion, None);
        m.add_separator();
        item(m, "Screenshot (F12)", Act::Screenshot, Some(Key::T));
        item(m, "Sound On / Off (F8)", Act::Mute, Some(Key::M));
        m.add_separator();
        item(m, "Close Game (Esc)", Act::Quit, None);
    });
    add("Speed", &|m| {
        for n in vuretro_settings::FAST_FORWARD_SPEEDS {
            item(m, &format!("Fast Forward at {n}×"), Act::Speed(n), None);
        }
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
        item(m, "LCD Effect (Shift+F4)", Act::Lcd, None);
        item(m, "Full Screen (Alt+Enter)", Act::Fullscreen, Some(Key::F));
        m.add_separator();
        for n in 1..=4u8 {
            item(m, &format!("{n}×"), Act::Size(n), None);
        }
    });
}

/// Full-screen windows are made from a borderless window the size of the display.
fn open_window(
    title: &str,
    base_w: usize,
    base_h: usize,
    factor: usize,
    view: View,
) -> Result<(Window, View)> {
    if view.fullscreen {
        if let Some((sw, sh)) = display::screen_size() {
            let mut window = Window::new(
                title,
                sw,
                sh,
                WindowOptions {
                    borderless: true,
                    scale: Scale::X1,
                    scale_mode: if view.integer {
                        ScaleMode::Center
                    } else {
                        ScaleMode::AspectRatioStretch
                    },
                    ..WindowOptions::default()
                },
            )?;
            window.set_position(0, 0);
            return Ok((window, view));
        }
        log::warn!("full screen is not available here; using a window");
    }
    let view = View {
        fullscreen: false,
        ..view
    };
    if view.integer {
        // the session enlarges the picture by whole numbers; the window shows it as it is
        let window = Window::new(
            title,
            base_w * factor,
            base_h * factor,
            WindowOptions {
                resize: true,
                scale: Scale::X1,
                scale_mode: ScaleMode::Center,
                ..WindowOptions::default()
            },
        )?;
        return Ok((window, view));
    }
    // minifb's own scaling only does 1, 2, 4, 8 and 16; for other sizes the window is
    // made that many times bigger and the frame is stretched to fit it.
    let (scale, stretch) = match factor {
        1 => (Scale::X1, 1),
        2 => (Scale::X2, 1),
        4 => (Scale::X4, 1),
        8 => (Scale::X8, 1),
        n => (Scale::X1, n),
    };
    let window = Window::new(
        title,
        base_w * stretch,
        base_h * stretch,
        WindowOptions {
            resize: true,
            scale,
            ..WindowOptions::default()
        },
    )?;
    Ok((window, view))
}

pub struct MiniFbScreen {
    window: Window,
    view: View,
}

impl MiniFbScreen {
    pub fn open(
        title: &str,
        base_w: usize,
        base_h: usize,
        factor: usize,
        view: View,
    ) -> Result<Self> {
        let (mut window, view) = open_window(title, base_w, base_h, factor, view)?;
        install_menus(&mut window);
        // Show a black picture now. The first frame of a game can take a while, and until the
        // window has handled its first update the Dock still shows the generic "exec" icon and
        // the window shows whatever was in graphics memory.
        let _ = window.update_with_buffer(&vec![0u32; base_w * base_h], base_w, base_h);
        Ok(MiniFbScreen { window, view })
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
    fn reopen(
        &mut self,
        title: &str,
        base_w: usize,
        base_h: usize,
        factor: usize,
        view: View,
    ) -> Result<()> {
        let at = self.window.get_position();
        // Open the new window before the old one closes so the app never has none.
        let (mut window, view) = open_window(title, base_w, base_h, factor, view)?;
        if !view.fullscreen {
            window.set_position(at.0, at.1);
        }
        install_menus(&mut window);
        self.window = window;
        self.view = view;
        Ok(())
    }
    fn size(&self) -> (usize, usize) {
        self.window.get_size()
    }
    fn view(&self) -> View {
        self.view
    }
    fn can_fullscreen(&self) -> bool {
        display::screen_size().is_some()
    }
}
