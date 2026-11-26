//! The game menus (Game, Speed, State, Size) as native menus through `muda`: the macOS menu
//! bar and a Windows menu bar on the window. Elsewhere there is none and the hotkeys do the
//! job, as before. Items carry the same ids as `act_id`, so a choice goes back to the session
//! as the number it already understands.

/// macOS and Windows have a native menu bar. (`muda` pulls in GTK on Linux, which the
/// menu would not use, so it is only a dependency of these two.)
#[cfg(any(target_os = "macos", windows))]
mod native {
    use muda::accelerator::{Accelerator, Code, Modifiers};
    use muda::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
    use std::cell::RefCell;
    use vuretro_play::window::{act_id, Act, SLOTS};
    use winit::window::Window;

    /// The menu bar of one window. Keep it alive as long as the window.
    pub struct GameMenu {
        _menu: Menu,
        /// The "Use Slot n" items, to say which holds a save and which is current.
        slots: Vec<CheckMenuItem>,
    }

    /// Command on macOS, Control elsewhere.
    const MOD: Modifiers = if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    };

    struct Builder<'a> {
        menu: &'a Submenu,
    }

    impl Builder<'_> {
        fn item(&self, label: &str, act: Act, key: Option<Code>) {
            let it = MenuItem::with_id(
                MenuId::new(act_id(act).to_string()),
                label,
                true,
                key.map(|k| Accelerator::new(MOD, k)),
            );
            let _ = self.menu.append(&it);
        }
        /// A slot item: it carries a check mark for the current slot.
        fn slot(&self, n: u8) -> CheckMenuItem {
            let it = CheckMenuItem::with_id(
                MenuId::new(act_id(Act::Slot(n)).to_string()),
                format!("Use Slot {n}"),
                true,
                false,
                None,
            );
            let _ = self.menu.append(&it);
            it
        }
        fn note(&self, label: &str) {
            let _ = self
                .menu
                .append(&MenuItem::with_id(MenuId::new("0"), label, false, None));
        }
        fn line(&self) {
            let _ = self.menu.append(&PredefinedMenuItem::separator());
        }
    }

    fn submenu(menu: &Menu, name: &str, build: impl Fn(&Builder)) {
        let sub = Submenu::new(name, true);
        build(&Builder { menu: &sub });
        let _ = menu.append(&sub);
    }

    /// Every menu of the game window.
    fn build() -> (Menu, Vec<CheckMenuItem>) {
        let menu = Menu::new();
        let slots = RefCell::new(Vec::new());
        #[cfg(target_os = "macos")]
        submenu(&menu, "Vuretro", |m| {
            let _ = m.menu.append(&PredefinedMenuItem::hide(None));
            let _ = m.menu.append(&PredefinedMenuItem::hide_others(None));
            let _ = m.menu.append(&PredefinedMenuItem::show_all(None));
            m.line();
            // the app's own Quit, not the system's: it must close the game the normal way, so
            // the save RAM and the place to carry on from are written
            m.item("Quit Vuretro", Act::Quit, Some(Code::KeyQ));
        });
        submenu(&menu, "Game", |m| {
            m.item("Pause / Resume (P or F3)", Act::Pause, Some(Code::KeyP));
            m.item("Reset (F2)", Act::Reset, Some(Code::KeyR));
            m.item("Change Disc (F11)", Act::NextDisc, None);
            m.line();
            m.note("Fast Forward: hold Tab");
            m.item("Fast Forward On / Off (F1)", Act::FastForwardToggle, None);
            m.item("Slow Motion On / Off (Shift+F1)", Act::SlowMotion, None);
            m.line();
            m.item("Screenshot (F12)", Act::Screenshot, Some(Code::KeyT));
            m.item("Sound On / Off (F8)", Act::Mute, Some(Code::KeyM));
            m.line();
            m.item("Close Game (Esc)", Act::Quit, None);
        });
        submenu(&menu, "Speed", |m| {
            for n in vuretro_settings::FAST_FORWARD_SPEEDS {
                m.item(&format!("Fast Forward at {n}×"), Act::Speed(n), None);
            }
        });
        submenu(&menu, "State", |m| {
            m.item("Save State (F5)", Act::SaveState, Some(Code::KeyS));
            m.item("Load State (F7)", Act::LoadState, Some(Code::KeyL));
            m.line();
            m.item("Next Slot (F6)", Act::NextSlot, None);
            for n in 1..=SLOTS {
                slots.borrow_mut().push(m.slot(n));
            }
        });
        submenu(&menu, "Size", |m| {
            m.item("Smaller (F9)", Act::SizeStep(-1), Some(Code::Minus));
            m.item("Bigger (F10)", Act::SizeStep(1), Some(Code::Equal));
            m.item("CRT Effect (F4)", Act::Crt, None);
            m.item("LCD Effect (Shift+F4)", Act::Lcd, None);
            m.item("Smooth Pixels, Scale2x (Shift+F9)", Act::Scale2x, None);
            m.item("Full Screen (Alt+Enter)", Act::Fullscreen, Some(Code::KeyF));
            m.line();
            for n in 1..=4u8 {
                m.item(&format!("{n}×"), Act::Size(n), None);
            }
        });
        (menu, slots.into_inner())
    }

    impl GameMenu {
        /// Put the menus on the app (macOS) or on `window` (Windows); nothing elsewhere.
        pub fn new(window: &Window) -> Self {
            let _ = window; // used on Windows only
            #[cfg(target_os = "macos")]
            {
                let (menu, slots) = build();
                menu.init_for_nsapp();
                GameMenu { _menu: menu, slots }
            }
            #[cfg(windows)]
            {
                use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                let (menu, slots) = build();
                if let Ok(h) = window.window_handle() {
                    if let RawWindowHandle::Win32(w) = h.as_raw() {
                        // SAFETY: the handle is the live window this menu belongs to.
                        let _ = unsafe { menu.init_for_hwnd(w.hwnd.get()) };
                    }
                }
                GameMenu { _menu: menu, slots }
            }
        }

        /// What each slot holds (one text per slot), with a check mark on `current` (from 1).
        pub fn set_slots(&self, labels: &[String], current: u8) {
            for (i, item) in self.slots.iter().enumerate() {
                if let Some(text) = labels.get(i) {
                    item.set_text(text);
                }
                item.set_checked(i + 1 == current as usize);
            }
        }

        /// The next item the user chose, as the id `act_id` gave it.
        pub fn next_choice() -> Option<usize> {
            while let Ok(e) = MenuEvent::receiver().try_recv() {
                if let Ok(id) = e.id.0.parse::<usize>() {
                    if id != 0 {
                        return Some(id);
                    }
                }
            }
            None
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn item_ids_are_the_numbers_the_session_knows() {
            assert_eq!(MenuId::new(act_id(Act::Quit).to_string()).0, "30");
            assert_eq!(MenuId::new(act_id(Act::Slot(2)).to_string()).0, "12");
        }
    }
}

/// Elsewhere there is no menu bar: the hotkeys do the job.
#[cfg(not(any(target_os = "macos", windows)))]
mod native {
    use winit::window::Window;

    pub struct GameMenu;

    impl GameMenu {
        pub fn new(_window: &Window) -> Self {
            GameMenu
        }

        pub fn next_choice() -> Option<usize> {
            None
        }

        pub fn set_slots(&self, _labels: &[String], _current: u8) {}
    }
}

pub use native::GameMenu;
