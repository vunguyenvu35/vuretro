//! Where the picture goes in the window.

/// A rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Place a `picture` (w, h) in a `window` (w, h, physical pixels), centred.
///
/// With `whole: Some(scale)` the picture is shown at `scale` window pixels per picture pixel
/// (the session has already enlarged it by whole numbers; `scale` is the display's
/// pixel ratio), unless that does not fit. Otherwise it is stretched as far as the window
/// allows with its proportions kept.
pub fn fit_rect(picture: (usize, usize), window: (usize, usize), whole: Option<f64>) -> Rect {
    let (pw, ph) = (picture.0.max(1) as f64, picture.1.max(1) as f64);
    let (ww, wh) = (window.0 as f64, window.1 as f64);
    let (mut w, mut h) = match whole {
        Some(s) => (pw * s, ph * s),
        None => (pw, ph),
    };
    if whole.is_none() || w > ww || h > wh {
        let k = (ww / pw).min(wh / ph);
        w = pw * k;
        h = ph * k;
    }
    let (w, h) = (w.round() as i32, h.round() as i32);
    Rect {
        x: (window.0 as i32 - w) / 2,
        y: (window.1 as i32 - h) / 2,
        w,
        h,
    }
}

/// Place a `picture` in a `window` at the largest whole number of window pixels per picture
/// pixel that fits (at least 1), centred. Returns the rectangle and that number.
pub fn whole_fit(picture: (usize, usize), window: (usize, usize)) -> (Rect, usize) {
    let (pw, ph) = (picture.0.max(1), picture.1.max(1));
    let k = (window.0 / pw).min(window.1 / ph).max(1);
    let (w, h) = ((pw * k) as i32, (ph * k) as i32);
    let rect = Rect {
        x: (window.0 as i32 - w) / 2,
        y: (window.1 as i32 - h) / 2,
        w,
        h,
    };
    (rect, k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stretches_to_the_window_keeping_proportions() {
        // 4:3 picture in a wide window: bars left and right
        let r = fit_rect((320, 240), (1600, 900), None);
        assert_eq!(
            r,
            Rect {
                x: 200,
                y: 0,
                w: 1200,
                h: 900
            }
        );
        // tall window: bars above and below
        let r = fit_rect((320, 240), (800, 800), None);
        assert_eq!(
            r,
            Rect {
                x: 0,
                y: 100,
                w: 800,
                h: 600
            }
        );
    }

    #[test]
    fn whole_numbers_keep_their_size_and_sit_in_the_middle() {
        // already enlarged to 640x480, shown at 2 pixels per pixel on a Retina display
        let r = fit_rect((640, 480), (1920, 1200), Some(2.0));
        assert_eq!(
            r,
            Rect {
                x: 320,
                y: 120,
                w: 1280,
                h: 960
            }
        );
    }

    #[test]
    fn a_picture_that_does_not_fit_is_shrunk_to_fit() {
        let r = fit_rect((640, 480), (320, 240), Some(1.0));
        assert_eq!(
            r,
            Rect {
                x: 0,
                y: 0,
                w: 320,
                h: 240
            }
        );
    }

    #[test]
    fn whole_fit_takes_the_largest_whole_factor_and_centres() {
        // 320x224 in 1920x1080: 6x wide, 4x tall: 4 is the most that fits both ways
        let (r, k) = whole_fit((320, 224), (1920, 1080));
        assert_eq!(k, 4);
        assert_eq!(
            r,
            Rect {
                x: 320,
                y: 92,
                w: 1280,
                h: 896
            }
        );
        // a window smaller than the picture still shows it at 1x (and clips)
        assert_eq!(whole_fit((320, 224), (100, 100)).1, 1);
    }

    #[test]
    fn an_empty_picture_does_not_divide_by_zero() {
        let r = fit_rect((0, 0), (100, 100), None);
        assert!(r.w >= 0 && r.h >= 0);
    }
}
