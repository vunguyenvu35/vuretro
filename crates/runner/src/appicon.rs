//! Dock icon and app name for the game window. The runner is a bare executable, so
//! macOS would show a generic "exec" icon and the file name; this gives it the vuretro
//! icon instead. No-op elsewhere.

/// The icon shown in the Dock (the app's own 256 px icon).
const ICON_PNG: &[u8] = include_bytes!("../../../src-tauri/icons/128x128@2x.png");

#[cfg(target_os = "macos")]
pub fn apply() {
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

        let data = send_bytes(
            cls("NSData"),
            sel("dataWithBytes:length:"),
            ICON_PNG.as_ptr(),
            ICON_PNG.len(),
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

        let name = CString::new("vuretro").unwrap();
        let ns_name = send_cstr(cls("NSString"), sel("stringWithUTF8String:"), name.as_ptr());
        let info = send0(cls("NSProcessInfo"), sel("processInfo"));
        send1(info, sel("setProcessName:"), ns_name);
    }
}

#[cfg(not(target_os = "macos"))]
pub fn apply() {
    let _ = ICON_PNG;
}
