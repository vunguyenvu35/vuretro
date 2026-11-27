//! Bring the window of a Wine game to the front when it starts. The game's window appears some
//! seconds after Play and, started from another program, macOS leaves it behind it (the user then
//! has to click its Dock icon). While the app is still the one in front, it hands the front to the
//! game; if the user has moved on to something else, nothing is taken from them.

/// Wait for the game's app to appear and hand it the front. Slow: run it on a thread.
#[cfg(target_os = "macos")]
pub fn raise_wine_game() {
    use std::ffi::{c_char, c_void, CStr, CString};
    use std::mem::transmute;
    use std::time::Duration;

    type Id = *mut c_void;
    type Sel = *mut c_void;

    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> Id;
        fn sel_registerName(name: *const c_char) -> Sel;
        fn objc_msgSend();
        fn objc_autoreleasePoolPush() -> *mut c_void;
        fn objc_autoreleasePoolPop(pool: *mut c_void);
    }
    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}

    // SAFETY: Objective-C calls with the exact signatures of the selectors used (all take
    // and give pointers, integers or booleans), made inside an autorelease pool.
    unsafe fn sel(name: &str) -> Sel {
        let c = CString::new(name).unwrap();
        sel_registerName(c.as_ptr())
    }
    unsafe fn msg(o: Id, name: &str) -> Id {
        let f: extern "C" fn(Id, Sel) -> Id = transmute(objc_msgSend as *const ());
        f(o, sel(name))
    }
    unsafe fn msg_int(o: Id, name: &str) -> isize {
        let f: extern "C" fn(Id, Sel) -> isize = transmute(objc_msgSend as *const ());
        f(o, sel(name))
    }
    unsafe fn msg_at(o: Id, name: &str, i: usize) -> Id {
        let f: extern "C" fn(Id, Sel, usize) -> Id = transmute(objc_msgSend as *const ());
        f(o, sel(name), i)
    }
    unsafe fn msg_opts(o: Id, name: &str, opts: usize) {
        let f: extern "C" fn(Id, Sel, usize) -> u8 = transmute(objc_msgSend as *const ());
        f(o, sel(name), opts);
    }
    unsafe fn text(s: Id) -> String {
        if s.is_null() {
            return String::new();
        }
        let p = msg(s, "UTF8String") as *const c_char;
        if p.is_null() {
            String::new()
        } else {
            CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    }

    let me = std::process::id() as isize;
    // up to a minute: the first start of a game sets it up before Wine opens its window
    for _ in 0..120 {
        std::thread::sleep(Duration::from_millis(500));
        // SAFETY: see above.
        let done = unsafe {
            let pool = objc_autoreleasePoolPush();
            let workspace = msg(objc_getClass(c"NSWorkspace".as_ptr()), "sharedWorkspace");
            let front = msg(workspace, "frontmostApplication");
            let still_ours = !front.is_null() && msg_int(front, "processIdentifier") == me;
            let mut result = None;
            if !still_ours {
                // the user is in another program: leave them there
                result = Some(());
            } else {
                let apps = msg(workspace, "runningApplications");
                let count = msg_int(apps, "count") as usize;
                for i in 0..count {
                    let app = msg_at(apps, "objectAtIndex:", i);
                    // a Wine program with a window: a regular app whose file is named wine
                    let regular = msg_int(app, "activationPolicy") == 0;
                    let url = msg(app, "executableURL");
                    let path = if url.is_null() {
                        String::new()
                    } else {
                        text(msg(url, "path"))
                    };
                    if regular && path.ends_with("/wine") {
                        // all windows, ignoring the other apps
                        msg_opts(app, "activateWithOptions:", 3);
                        result = Some(());
                        break;
                    }
                }
            }
            objc_autoreleasePoolPop(pool);
            result
        };
        if done.is_some() {
            return;
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn raise_wine_game() {}
