//! A cheap CRT look done on the CPU: the picture is enlarged by a whole factor and every
//! enlarged row but the first gets darker, like the gaps between scanlines. Pure, so it
//! is unit-tested without a window. (A GPU shader replaces it once the renderer does.)

/// Whole-number enlargement: small pictures need more rows to show scanlines.
pub fn factor_for(height: usize) -> usize {
    match height {
        0..=300 => 3,
        301..=600 => 2,
        _ => 1,
    }
}

/// Brightness of the gap rows, in 256ths.
const GAP: u32 = 150;

fn dim(p: u32, level: u32) -> u32 {
    let ch = |shift: u32| (((p >> shift) & 0xff) * level / 256) << shift;
    ch(16) | ch(8) | ch(0)
}

/// Returns the enlarged picture with its new width and height.
pub fn apply(buf: &[u32], w: usize, h: usize) -> (Vec<u32>, usize, usize) {
    apply_with(buf, w, h, factor_for(h))
}

/// Like [`apply`], enlarging by a whole factor `k` chosen by the caller (the window
/// scaling it fits). With `k` below 2 there is no room for a gap, so nothing changes.
pub fn apply_with(buf: &[u32], w: usize, h: usize, k: usize) -> (Vec<u32>, usize, usize) {
    if k < 2 || buf.len() < w * h || w == 0 {
        return (buf.to_vec(), w, h);
    }
    let (ow, oh) = (w * k, h * k);
    let mut out = vec![0u32; ow * oh];
    for y in 0..h {
        let src = &buf[y * w..(y + 1) * w];
        for r in 0..k {
            // the last row of each group is the dark gap
            let level = if r == k - 1 { GAP } else { 256 };
            let row = &mut out[(y * k + r) * ow..(y * k + r + 1) * ow];
            for (x, &p) in src.iter().enumerate() {
                let v = if level == 256 { p } else { dim(p, level) };
                row[x * k..(x + 1) * k].fill(v);
            }
        }
    }
    (out, ow, oh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_pictures_are_enlarged_by_3_medium_by_2_large_not_at_all() {
        assert_eq!(factor_for(160), 3);
        assert_eq!(factor_for(480), 2);
        assert_eq!(factor_for(1080), 1);
    }

    #[test]
    fn rows_are_repeated_and_the_last_one_of_each_group_is_darker() {
        let buf = vec![0x00FF_FFFF, 0x00FF_0000];
        let (out, w, h) = apply(&buf, 2, 1);
        assert_eq!((w, h, out.len()), (6, 3, 18));
        // first two rows keep the colour, three pixels wide per source pixel
        assert_eq!(
            &out[0..6],
            &[0xFFFFFF, 0xFFFFFF, 0xFFFFFF, 0xFF0000, 0xFF0000, 0xFF0000]
        );
        assert_eq!(&out[6..12], &out[0..6]);
        // the gap row is dimmer but not black
        let gap = out[12];
        assert!(gap != 0 && gap < 0xFFFFFF);
        assert_eq!(gap & 0xff, 0x95, "150/256 of 255");
        assert_eq!(out[15] & 0xff00, 0, "dimming keeps a red pixel red");
        assert!(out[15] >> 16 > 0);
    }

    #[test]
    fn the_caller_can_choose_the_factor() {
        let buf = vec![0x00FF_FFFF; 4];
        let (out, w, h) = apply_with(&buf, 2, 2, 2);
        assert_eq!((w, h), (4, 4));
        assert_eq!(out[0], 0xFFFFFF); // first row of a group: full
        assert!(out[4] < 0xFFFFFF); // last row of a group: the gap
        assert_eq!(apply_with(&buf, 2, 2, 1).0, buf, "1x: nothing to do");
    }

    #[test]
    fn large_or_malformed_input_passes_through() {
        let big = vec![1u32; 4 * 700];
        assert_eq!(apply(&big, 4, 700).0, big);
        assert_eq!(
            apply(&[1, 2, 3], 4, 2).0,
            vec![1, 2, 3],
            "short buffer is returned as is"
        );
    }
}
