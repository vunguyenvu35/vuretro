//! Keyboard keys, named like the `minifb` ones so the controls file and the window adapter
//! agree on them, but owned by this crate: the session knows no window library.

/// Calls `$m!` with the comma-separated list of every key, so an adapter can generate the
/// conversion to its own key type from the one list kept here.
#[macro_export]
macro_rules! with_keys {
    ($m:ident) => {
        $m! { Key0, Key1, Key2, Key3, Key4, Key5, Key6, Key7, Key8, Key9, A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z, F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12, F13, F14, F15, Down, Left, Right, Up, Apostrophe, Backquote, Backslash, Comma, Equal, LeftBracket, Minus, Period, RightBracket, Semicolon, Slash, Backspace, Delete, End, Enter, Escape, Home, Insert, Menu, PageDown, PageUp, Pause, Space, Tab, NumLock, CapsLock, ScrollLock, LeftShift, RightShift, LeftCtrl, RightCtrl, NumPad0, NumPad1, NumPad2, NumPad3, NumPad4, NumPad5, NumPad6, NumPad7, NumPad8, NumPad9, NumPadDot, NumPadSlash, NumPadAsterisk, NumPadMinus, NumPadPlus, NumPadEnter, LeftAlt, RightAlt, LeftSuper, RightSuper, Unknown }
    };
}

macro_rules! define_key {
    ($($k:ident),* $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Key {
            $($k),*
        }
    };
}

with_keys!(define_key);
