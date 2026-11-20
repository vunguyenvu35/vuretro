//! Dock icon and app name for the game window. The runner is a bare executable, so
//! macOS would show a generic "exec" icon and the file name; this gives it the vuretro
//! icon instead, with a small badge in the corner that says which system the game is for
//! (Neo Geo, PlayStation...), so several open games can be told apart. No-op elsewhere.

/// The icon shown in the Dock (the app's own 256 px icon).
const ICON_PNG: &[u8] = include_bytes!("../../../src-tauri/icons/128x128@2x.png");

/// The Dock icon for a game of `system`: the app icon with the system's badge. The plain app
/// icon when the system is unknown or the icon cannot be decoded.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn icon_png(system: &str) -> Vec<u8> {
    let Some(badge) = vuretro_play::badge::for_system(system) else {
        return ICON_PNG.to_vec();
    };
    badged(ICON_PNG, &badge).unwrap_or_else(|| ICON_PNG.to_vec())
}

/// `png` (8-bit RGBA, square) with `badge` drawn on it, as PNG bytes.
fn badged(png: &[u8], badge: &vuretro_play::badge::Badge) -> Option<Vec<u8>> {
    let mut reader = png::Decoder::new(png).read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let ok = info.color_type == png::ColorType::Rgba
        && info.bit_depth == png::BitDepth::Eight
        && info.width == info.height;
    if !ok {
        return None;
    }
    let size = info.width as usize;
    buf.truncate(size * size * 4);
    vuretro_play::badge::draw(&mut buf, size, badge);
    let mut out = Vec::new();
    let mut enc = png::Encoder::new(&mut out, info.width, info.height);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    enc.write_header().ok()?.write_image_data(&buf).ok()?;
    Some(out)
}

#[cfg(target_os = "macos")]
pub fn apply(system: &str) {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_void};
    type Id = *mut c_void;
    type Sel = *mut c_void;

    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> Id;
        fn sel_registerName(name: *const c_char) -> Sel;
        fn objc_msgSend();
    }
    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}

    // SAFETY: Objective-C calls with the exact signatures of the selectors used;
    // called on the main thread after the window exists, so NSApplication is set up.
    unsafe {
        let cls = |n: &str| {
            let n = CString::new(n).unwrap();
            objc_getClass(n.as_ptr())
        };
        let sel = |n: &str| {
            let n = CString::new(n).unwrap();
            sel_registerName(n.as_ptr())
        };
        let msg = objc_msgSend as *const ();
        let send0: unsafe extern "C" fn(Id, Sel) -> Id = std::mem::transmute(msg);
        let send1: unsafe extern "C" fn(Id, Sel, Id) -> Id = std::mem::transmute(msg);
        let send_bytes: unsafe extern "C" fn(Id, Sel, *const u8, usize) -> Id =
            std::mem::transmute(msg);
        let send_cstr: unsafe extern "C" fn(Id, Sel, *const c_char) -> Id =
            std::mem::transmute(msg);

        let icon = icon_png(system);
        let data = send_bytes(
            cls("NSData"),
            sel("dataWithBytes:length:"),
            icon.as_ptr(),
            icon.len(),
        );
        let image = send1(
            send0(cls("NSImage"), sel("alloc")),
            sel("initWithData:"),
            data,
        );
        if image.is_null() {
            return;
        }
        let app = send0(cls("NSApplication"), sel("sharedApplication"));
        send1(app, sel("setApplicationIconImage:"), image);

        let name = CString::new("Vuretro").unwrap();
        let ns_name = send_cstr(cls("NSString"), sel("stringWithUTF8String:"), name.as_ptr());
        let info = send0(cls("NSProcessInfo"), sel("processInfo"));
        send1(info, sel("setProcessName:"), ns_name);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn apply(_system: &str) {
    let _ = ICON_PNG;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_known_system_gets_its_own_icon_and_an_unknown_one_the_plain_icon() {
        let ng = icon_png("neogeo");
        let ps = icon_png("psx");
        assert_ne!(ng, ps);
        assert_ne!(ng, ICON_PNG);
        assert_eq!(icon_png(""), ICON_PNG);
        assert_eq!(icon_png("nope"), ICON_PNG);
        assert_eq!(&ng[1..4], b"PNG");
    }

    /// `ICON_SAMPLES=<dir> cargo test -p vuretro-runner icon_samples` writes the icons to look at.
    #[test]
    fn icon_samples() {
        let Ok(dir) = std::env::var("ICON_SAMPLES") else {
            return;
        };
        for id in [
            "neogeo",
            "psx",
            "ps2",
            "snes",
            "arcade",
            "atari2600",
            "wonderswan",
            "sega32x",
            "lynx",
            "gamecube",
        ] {
            std::fs::write(format!("{dir}/{id}.png"), icon_png(id)).unwrap();
        }
    }
}
