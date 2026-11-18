//! Changing discs in multi-disc games through the core's disc control interface:
//! open the tray, pick the next image, close the tray.

use libretro_sys::retro_disk_control_callback;

/// Insert the next disc (wrapping to the first). Returns what to tell the player.
pub fn swap_to_next(cb: &retro_disk_control_callback) -> String {
    let (Some(count), Some(index), Some(eject), Some(set)) = (
        cb.get_num_images,
        cb.get_image_index,
        cb.set_eject_state,
        cb.set_image_index,
    ) else {
        return "this core cannot change discs".into();
    };
    // SAFETY: the core handed us these functions for exactly this use.
    unsafe {
        let n = count();
        if n < 2 {
            return "this game has only one disc".into();
        }
        let next = (index() + 1) % n;
        eject(true);
        let ok = set(next);
        eject(false);
        if ok {
            format!("disc {} of {n}", next + 1)
        } else {
            format!("the core refused disc {}", next + 1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::raw::c_uint;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::SeqCst};

    // One fake "core" per test would need separate statics; the tests share these and run
    // one after the other under a lock.
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    static IMAGES: AtomicU32 = AtomicU32::new(3);
    static CURRENT: AtomicU32 = AtomicU32::new(0);
    static OPEN: AtomicBool = AtomicBool::new(false);
    static ACCEPT: AtomicBool = AtomicBool::new(true);

    unsafe extern "C" fn set_eject(e: bool) -> bool {
        OPEN.store(e, SeqCst);
        true
    }
    unsafe extern "C" fn get_index() -> c_uint {
        CURRENT.load(SeqCst)
    }
    unsafe extern "C" fn set_index(i: c_uint) -> bool {
        assert!(
            OPEN.load(SeqCst),
            "the tray must be open when the image changes"
        );
        if ACCEPT.load(SeqCst) {
            CURRENT.store(i, SeqCst);
        }
        ACCEPT.load(SeqCst)
    }
    unsafe extern "C" fn count() -> c_uint {
        IMAGES.load(SeqCst)
    }

    fn cb() -> retro_disk_control_callback {
        retro_disk_control_callback {
            set_eject_state: Some(set_eject),
            get_eject_state: None,
            get_image_index: Some(get_index),
            set_image_index: Some(set_index),
            get_num_images: Some(count),
            replace_image_index: None,
            add_image_index: None,
        }
    }

    #[test]
    fn walks_through_the_discs_with_the_tray_open_and_wraps() {
        let _g = LOCK.lock().unwrap();
        IMAGES.store(3, SeqCst);
        CURRENT.store(0, SeqCst);
        ACCEPT.store(true, SeqCst);
        assert_eq!(swap_to_next(&cb()), "disc 2 of 3");
        assert_eq!(swap_to_next(&cb()), "disc 3 of 3");
        assert_eq!(swap_to_next(&cb()), "disc 1 of 3");
        assert!(!OPEN.load(SeqCst), "the tray is closed again");
    }

    #[test]
    fn single_disc_refusal_and_missing_functions_are_reported() {
        let _g = LOCK.lock().unwrap();
        IMAGES.store(1, SeqCst);
        assert_eq!(swap_to_next(&cb()), "this game has only one disc");
        IMAGES.store(2, SeqCst);
        CURRENT.store(0, SeqCst);
        ACCEPT.store(false, SeqCst);
        assert_eq!(swap_to_next(&cb()), "the core refused disc 2");
        ACCEPT.store(true, SeqCst);
        let mut bare = cb();
        bare.set_image_index = None;
        assert_eq!(swap_to_next(&bare), "this core cannot change discs");
    }
}
