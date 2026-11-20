//! The session's keys as winit's physical key codes (a key is the place on the keyboard, not
//! the letter it types, so bindings do not move with the layout).

use vuretro_play::Key;
use winit::keyboard::KeyCode;

/// The winit key for `key`; `None` for [`Key::Unknown`].
pub fn to_code(key: Key) -> Option<KeyCode> {
    Some(match key {
        Key::Key0 => KeyCode::Digit0,
        Key::Key1 => KeyCode::Digit1,
        Key::Key2 => KeyCode::Digit2,
        Key::Key3 => KeyCode::Digit3,
        Key::Key4 => KeyCode::Digit4,
        Key::Key5 => KeyCode::Digit5,
        Key::Key6 => KeyCode::Digit6,
        Key::Key7 => KeyCode::Digit7,
        Key::Key8 => KeyCode::Digit8,
        Key::Key9 => KeyCode::Digit9,
        Key::A => KeyCode::KeyA,
        Key::B => KeyCode::KeyB,
        Key::C => KeyCode::KeyC,
        Key::D => KeyCode::KeyD,
        Key::E => KeyCode::KeyE,
        Key::F => KeyCode::KeyF,
        Key::G => KeyCode::KeyG,
        Key::H => KeyCode::KeyH,
        Key::I => KeyCode::KeyI,
        Key::J => KeyCode::KeyJ,
        Key::K => KeyCode::KeyK,
        Key::L => KeyCode::KeyL,
        Key::M => KeyCode::KeyM,
        Key::N => KeyCode::KeyN,
        Key::O => KeyCode::KeyO,
        Key::P => KeyCode::KeyP,
        Key::Q => KeyCode::KeyQ,
        Key::R => KeyCode::KeyR,
        Key::S => KeyCode::KeyS,
        Key::T => KeyCode::KeyT,
        Key::U => KeyCode::KeyU,
        Key::V => KeyCode::KeyV,
        Key::W => KeyCode::KeyW,
        Key::X => KeyCode::KeyX,
        Key::Y => KeyCode::KeyY,
        Key::Z => KeyCode::KeyZ,
        Key::F1 => KeyCode::F1,
        Key::F2 => KeyCode::F2,
        Key::F3 => KeyCode::F3,
        Key::F4 => KeyCode::F4,
        Key::F5 => KeyCode::F5,
        Key::F6 => KeyCode::F6,
        Key::F7 => KeyCode::F7,
        Key::F8 => KeyCode::F8,
        Key::F9 => KeyCode::F9,
        Key::F10 => KeyCode::F10,
        Key::F11 => KeyCode::F11,
        Key::F12 => KeyCode::F12,
        Key::F13 => KeyCode::F13,
        Key::F14 => KeyCode::F14,
        Key::F15 => KeyCode::F15,
        Key::Down => KeyCode::ArrowDown,
        Key::Left => KeyCode::ArrowLeft,
        Key::Right => KeyCode::ArrowRight,
        Key::Up => KeyCode::ArrowUp,
        Key::Apostrophe => KeyCode::Quote,
        Key::Backquote => KeyCode::Backquote,
        Key::Backslash => KeyCode::Backslash,
        Key::Comma => KeyCode::Comma,
        Key::Equal => KeyCode::Equal,
        Key::LeftBracket => KeyCode::BracketLeft,
        Key::Minus => KeyCode::Minus,
        Key::Period => KeyCode::Period,
        Key::RightBracket => KeyCode::BracketRight,
        Key::Semicolon => KeyCode::Semicolon,
        Key::Slash => KeyCode::Slash,
        Key::Backspace => KeyCode::Backspace,
        Key::Delete => KeyCode::Delete,
        Key::End => KeyCode::End,
        Key::Enter => KeyCode::Enter,
        Key::Escape => KeyCode::Escape,
        Key::Home => KeyCode::Home,
        Key::Insert => KeyCode::Insert,
        Key::Menu => KeyCode::ContextMenu,
        Key::PageDown => KeyCode::PageDown,
        Key::PageUp => KeyCode::PageUp,
        Key::Pause => KeyCode::Pause,
        Key::Space => KeyCode::Space,
        Key::Tab => KeyCode::Tab,
        Key::NumLock => KeyCode::NumLock,
        Key::CapsLock => KeyCode::CapsLock,
        Key::ScrollLock => KeyCode::ScrollLock,
        Key::LeftShift => KeyCode::ShiftLeft,
        Key::RightShift => KeyCode::ShiftRight,
        Key::LeftCtrl => KeyCode::ControlLeft,
        Key::RightCtrl => KeyCode::ControlRight,
        Key::NumPad0 => KeyCode::Numpad0,
        Key::NumPad1 => KeyCode::Numpad1,
        Key::NumPad2 => KeyCode::Numpad2,
        Key::NumPad3 => KeyCode::Numpad3,
        Key::NumPad4 => KeyCode::Numpad4,
        Key::NumPad5 => KeyCode::Numpad5,
        Key::NumPad6 => KeyCode::Numpad6,
        Key::NumPad7 => KeyCode::Numpad7,
        Key::NumPad8 => KeyCode::Numpad8,
        Key::NumPad9 => KeyCode::Numpad9,
        Key::NumPadDot => KeyCode::NumpadDecimal,
        Key::NumPadSlash => KeyCode::NumpadDivide,
        Key::NumPadAsterisk => KeyCode::NumpadMultiply,
        Key::NumPadMinus => KeyCode::NumpadSubtract,
        Key::NumPadPlus => KeyCode::NumpadAdd,
        Key::NumPadEnter => KeyCode::NumpadEnter,
        Key::LeftAlt => KeyCode::AltLeft,
        Key::RightAlt => KeyCode::AltRight,
        Key::LeftSuper => KeyCode::SuperLeft,
        Key::RightSuper => KeyCode::SuperRight,
        Key::Unknown => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_but_unknown_has_its_own_code() {
        let mut seen = std::collections::HashSet::new();
        macro_rules! check {
            ($($k:ident),* $(,)?) => {
                $(if let Some(c) = to_code(Key::$k) { assert!(seen.insert(c), "{} shares a code", stringify!($k)); })*
            };
        }
        vuretro_play::with_keys!(check);
        assert_eq!(to_code(Key::Unknown), None);
        assert_eq!(to_code(Key::A), Some(KeyCode::KeyA));
        assert_eq!(to_code(Key::Key1), Some(KeyCode::Digit1));
        assert_eq!(to_code(Key::Left), Some(KeyCode::ArrowLeft));
    }
}
