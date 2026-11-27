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

/// Make the window of the Wine game as big as the screen allows, as a double click on its title bar
/// does (the zoom button of the window). The game takes the size of the whole display for its
/// window, which on a Retina Mac is more than the screen holds. It needs the permission of
/// Accessibility for the app: the first time macOS asks for it, and until it is given nothing
/// is done (the game keeps its window). Slow: run it on a thread.
#[cfg(target_os = "macos")]
pub fn zoom_wine_window() {
    use std::ffi::{c_char, c_void, CString};
    use std::time::Duration;

    type Cf = *const c_void;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        static kAXTrustedCheckOptionPrompt: Cf;
        fn AXIsProcessTrustedWithOptions(options: Cf) -> u8;
        fn AXUIElementCreateApplication(pid: i32) -> Cf;
        fn AXUIElementCopyAttributeValue(element: Cf, attribute: Cf, value: *mut Cf) -> i32;
        fn AXUIElementPerformAction(element: Cf, action: Cf) -> i32;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        static kCFBooleanTrue: Cf;
        static kCFTypeDictionaryKeyCallBacks: u8;
        static kCFTypeDictionaryValueCallBacks: u8;
        fn CFStringCreateWithCString(alloc: Cf, s: *const c_char, encoding: u32) -> Cf;
        fn CFDictionaryCreate(
            alloc: Cf,
            keys: *const Cf,
            values: *const Cf,
            count: isize,
            key_callbacks: *const u8,
            value_callbacks: *const u8,
        ) -> Cf;
        fn CFArrayGetCount(array: Cf) -> isize;
        fn CFArrayGetValueAtIndex(array: Cf, index: isize) -> Cf;
        fn CFRelease(cf: Cf);
    }
    #[link(name = "objc")]
    extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        fn objc_msgSend();
        fn objc_autoreleasePoolPush() -> *mut c_void;
        fn objc_autoreleasePoolPop(pool: *mut c_void);
    }
    #[link(name = "AppKit", kind = "framework")]
    extern "C" {}

    // SAFETY: calls of the system's C and Objective-C interfaces with the signatures they are
    // documented with; the strings are made and given back here, and the elements that were
    // copied are released.
    unsafe {
        let string = |s: &str| {
            let c = CString::new(s).unwrap();
            CFStringCreateWithCString(std::ptr::null(), c.as_ptr(), 0x0800_0100)
        };
        // asks the user for the permission the first time (the prompt option)
        let key = kAXTrustedCheckOptionPrompt;
        let value = kCFBooleanTrue;
        let options = CFDictionaryCreate(
            std::ptr::null(),
            &key,
            &value,
            1,
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        );
        let trusted = AXIsProcessTrustedWithOptions(options) != 0;
        CFRelease(options);
        if !trusted {
            return;
        }

        // the pid of the Wine program that has windows
        let wine_pid = || -> Option<i32> {
            let sel = |n: &str| {
                let c = CString::new(n).unwrap();
                sel_registerName(c.as_ptr())
            };
            let msg: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_i: extern "C" fn(*mut c_void, *mut c_void) -> isize =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_at: extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let pool = objc_autoreleasePoolPush();
            let workspace = msg(
                objc_getClass(c"NSWorkspace".as_ptr()),
                sel("sharedWorkspace"),
            );
            let apps = msg(workspace, sel("runningApplications"));
            let mut found = None;
            for i in 0..msg_i(apps, sel("count")) as usize {
                let app = msg_at(apps, sel("objectAtIndex:"), i);
                let url = msg(app, sel("executableURL"));
                if url.is_null() || msg_i(app, sel("activationPolicy")) != 0 {
                    continue;
                }
                let path = msg(msg(url, sel("path")), sel("UTF8String")) as *const c_char;
                if !path.is_null()
                    && std::ffi::CStr::from_ptr(path)
                        .to_string_lossy()
                        .ends_with("/wine")
                {
                    found = Some(msg_i(app, sel("processIdentifier")) as i32);
                }
            }
            objc_autoreleasePoolPop(pool);
            found
        };

        let (windows, zoom_button, press) = (
            string("AXWindows"),
            string("AXZoomButton"),
            string("AXPress"),
        );
        // up to a minute and a half: the first start sets the game up before its window opens
        for _ in 0..180 {
            std::thread::sleep(Duration::from_millis(500));
            let Some(pid) = wine_pid() else { continue };
            let app = AXUIElementCreateApplication(pid);
            let mut list: Cf = std::ptr::null();
            if AXUIElementCopyAttributeValue(app, windows, &mut list) == 0 && !list.is_null() {
                let mut pressed = false;
                for i in 0..CFArrayGetCount(list) {
                    let window = CFArrayGetValueAtIndex(list, i);
                    let mut button: Cf = std::ptr::null();
                    if AXUIElementCopyAttributeValue(window, zoom_button, &mut button) == 0
                        && !button.is_null()
                    {
                        // let the window settle (the game sizes it as it starts), then zoom
                        std::thread::sleep(Duration::from_secs(2));
                        pressed = AXUIElementPerformAction(button, press) == 0;
                        CFRelease(button);
                        break;
                    }
                }
                CFRelease(list);
                if pressed {
                    CFRelease(app);
                    return;
                }
            }
            CFRelease(app);
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn zoom_wine_window() {}

#[cfg(not(target_os = "macos"))]
pub fn raise_wine_game() {}
