//! Whole-number scaling of the picture: the largest factor that fits the window, and the
//! nearest-neighbour enlargement itself. Pure, so it is unit-tested without a window.

/// The largest whole factor (at least 1) that keeps a `w` x `h` picture inside a window of
/// `win_w` x `win_h`.
pub fn integer_factor(w: usize, h: usize, win_w: usize, win_h: usize) -> usize {
    if w == 0 || h == 0 {
        return 1;
    }
    (win_w / w).min(win_h / h).max(1)
}

/// Enlarge by a whole factor, copying pixels (no blur).
pub fn enlarge(buf: &[u32], w: usize, h: usize, k: usize) -> (Vec<u32>, usize, usize) {
    if k <= 1 || w == 0 || buf.len() < w * h {
        return (buf.to_vec(), w, h);
    }
    let (ow, oh) = (w * k, h * k);
    let mut out = vec![0u32; ow * oh];
    for y in 0..h {
        let src = &buf[y * w..(y + 1) * w];
        let first = y * k * ow;
        for (x, &p) in src.iter().enumerate() {
            out[first + x * k..first + (x + 1) * k].fill(p);
        }
        for r in 1..k {
            out.copy_within(first..first + ow, first + r * ow);
        }
    }
    (out, ow, oh)
}

/// Resize to `ow` x `oh` by picking the nearest source pixel. Used when a smoothed picture has
/// to reach an odd whole factor: its doubled pixels then get 1 or 2 output pixels in turn.
pub fn resample(buf: &[u32], w: usize, h: usize, ow: usize, oh: usize) -> (Vec<u32>, usize, usize) {
    if w == 0 || h == 0 || ow == 0 || oh == 0 || buf.len() < w * h {
        return (buf.to_vec(), w, h);
    }
    let mut out = Vec::with_capacity(ow * oh);
    for y in 0..oh {
        let row = &buf[(y * h / oh) * w..][..w];
        out.extend((0..ow).map(|x| row[x * w / ow]));
    }
    (out, ow, oh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factor_is_the_largest_that_fits_both_ways() {
        assert_eq!(integer_factor(240, 160, 720, 480), 3);
        assert_eq!(integer_factor(240, 160, 1000, 480), 3, "height limits it");
        assert_eq!(integer_factor(240, 160, 720, 1000), 3, "width limits it");
        assert_eq!(integer_factor(256, 224, 1920, 1080), 4);
    }

    #[test]
    fn factor_never_drops_below_one_or_divides_by_zero() {
        assert_eq!(integer_factor(240, 160, 100, 100), 1);
        assert_eq!(integer_factor(0, 0, 100, 100), 1);
    }

    #[test]
    fn enlarging_repeats_pixels_in_both_directions() {
        let (out, w, h) = enlarge(&[1, 2, 3, 4], 2, 2, 2);
        assert_eq!((w, h), (4, 4));
        assert_eq!(out, vec![1, 1, 2, 2, 1, 1, 2, 2, 3, 3, 4, 4, 3, 3, 4, 4]);
    }

    #[test]
    fn resampling_picks_the_nearest_pixel_so_odd_factors_alternate_one_and_two() {
        // 2 pixels stretched to 5: 1 1 1 2 2 (each source pixel gets 2 or 3, none is lost)
        let (out, w, h) = resample(&[1, 2], 2, 1, 5, 1);
        assert_eq!((w, h, out), (5, 1, vec![1, 1, 1, 2, 2]));
        // equal size is the identity; bad input comes back unchanged
        assert_eq!(resample(&[1, 2, 3, 4], 2, 2, 2, 2).0, vec![1, 2, 3, 4]);
        assert_eq!(resample(&[1], 2, 2, 4, 4).0, vec![1]);
    }

    #[test]
    fn enlarging_by_one_or_a_malformed_picture_changes_nothing() {
        assert_eq!(enlarge(&[1, 2], 2, 1, 1).0, vec![1, 2]);
        assert_eq!(enlarge(&[1, 2, 3], 2, 2, 3).0, vec![1, 2, 3]);
    }
}
