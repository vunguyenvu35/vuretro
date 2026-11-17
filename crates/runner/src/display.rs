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
    fn matches_close_rates_only() {
        assert_eq!(match_rate(59.18, 60.0), Some(60.0));
        assert_eq!(match_rate(59.18, 120.0), Some(60.0)); // every 2nd refresh
        assert_eq!(match_rate(60.0, 144.0), None); // 72 is 20 % off
        assert_eq!(match_rate(50.0, 60.0), None); // PAL on 60 Hz: 20 % too fast
        assert_eq!(match_rate(60.0, 60.0), Some(60.0));
    }
}
