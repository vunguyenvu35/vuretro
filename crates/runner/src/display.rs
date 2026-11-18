//! Display refresh rate, so video can be paced to the screen instead of the core's own
//! rate. Neo Geo runs at 59.18 fps; on a 60 Hz screen that shows one frame twice every
//! 1.2 s, a periodic judder. Running the game at exactly 60 fps removes it.

/// Refresh rate of the main display in Hz, if the OS tells us.
pub fn refresh_rate() -> Option<f64> {
    #[cfg(target_os = "macos")]
    {
        use std::os::raw::c_void;
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGMainDisplayID() -> u32;
            fn CGDisplayCopyDisplayMode(display: u32) -> *const c_void;
            fn CGDisplayModeGetRefreshRate(mode: *const c_void) -> f64;
            fn CGDisplayModeRelease(mode: *const c_void);
        }
        // SAFETY: plain CoreGraphics queries; the mode is released after use.
        unsafe {
            let mode = CGDisplayCopyDisplayMode(CGMainDisplayID());
            if mode.is_null() {
                return None;
            }
            let hz = CGDisplayModeGetRefreshRate(mode);
            CGDisplayModeRelease(mode);
            (hz >= 24.0).then_some(hz) // 0 means "unknown" (some variable-rate panels)
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

/// Size of the main display in the units a window is sized in (points on macOS), when the
/// OS tells us. Without it a full-screen window cannot be made.
pub fn screen_size() -> Option<(usize, usize)> {
    #[cfg(target_os = "macos")]
    {
        #[repr(C)]
        struct CGPoint {
            x: f64,
            y: f64,
        }
        #[repr(C)]
        struct CGSize {
            width: f64,
            height: f64,
        }
        #[repr(C)]
        struct CGRect {
            origin: CGPoint,
            size: CGSize,
        }
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGMainDisplayID() -> u32;
            fn CGDisplayBounds(display: u32) -> CGRect;
        }
        // SAFETY: plain CoreGraphics query with no pointers involved.
        let r = unsafe { CGDisplayBounds(CGMainDisplayID()) };
        sane_size(r.size.width, r.size.height)
    }
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        extern "system" {
            fn GetSystemMetrics(index: i32) -> i32;
        }
        // SAFETY: plain Win32 query; SM_CXSCREEN = 0, SM_CYSCREEN = 1.
        let (w, h) = unsafe { (GetSystemMetrics(0), GetSystemMetrics(1)) };
        sane_size(f64::from(w), f64::from(h))
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        None
    }
}

#[allow(dead_code)] // unused on Linux
fn sane_size(w: f64, h: f64) -> Option<(usize, usize)> {
    (w.is_finite() && h.is_finite() && w >= 320.0 && h >= 200.0).then_some((w as usize, h as usize))
}

/// The frame rate to run video at: the display's rate (or half or a third of it, for
/// 120 and 180 Hz screens) when that is within 5 % of the core's, otherwise `None`.
pub fn match_rate(core_fps: f64, display_hz: f64) -> Option<f64> {
    (1..=3)
        .map(|n| display_hz / n as f64)
        .find(|hz| ((hz / core_fps) - 1.0).abs() <= 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonsense_screen_sizes_are_refused() {
        assert_eq!(sane_size(1920.0, 1080.0), Some((1920, 1080)));
        assert_eq!(sane_size(0.0, 0.0), None);
        assert_eq!(sane_size(f64::NAN, 1080.0), None);
        assert_eq!(sane_size(100.0, 100.0), None);
    }

    #[test]
    fn matches_close_rates_only() {
        assert_eq!(match_rate(59.18, 60.0), Some(60.0));
        assert_eq!(match_rate(59.18, 120.0), Some(60.0)); // every 2nd refresh
        assert_eq!(match_rate(60.0, 144.0), None); // 72 is 20 % off
        assert_eq!(match_rate(50.0, 60.0), None); // PAL on 60 Hz: 20 % too fast
        assert_eq!(match_rate(60.0, 60.0), Some(60.0));
    }
}
