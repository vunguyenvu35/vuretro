//! Scale2x (also called AdvMAME2x or EPX): doubles a picture and smooths the stairs on
//! diagonal edges without blurring or inventing colours. Every pixel becomes four; each of
//! the four copies the pixel itself, unless its two neighbours on that corner agree with each
//! other and not with the opposite side, in which case it takes their colour.
//!
//! ```text
//!    A          1 2        1 = A if C == A and C != D and A != B  (else P)
//!  C P B   ->   3 4        2 = B if A == B and A != C and B != D
//!    D                     3 = C if D == C and D != B and C != A
//!                          4 = D if B == D and B != A and D != C
//! ```
//!
//! Pure, so it is unit-tested; it runs on the CPU like the CRT and LCD looks.

/// Frames taller than this are not enlarged: they are already detailed (PlayStation hi-res,
/// 3D games) and the doubled picture would only cost time.
pub const MAX_HEIGHT: usize = 480;

/// The doubled picture with its new width and height. A buffer that does not match `w * h`,
/// an empty one, or a frame taller than [`MAX_HEIGHT`] is returned unchanged.
pub fn apply(buf: &[u32], w: usize, h: usize) -> (Vec<u32>, usize, usize) {
    if w == 0 || h == 0 || buf.len() != w * h || h > MAX_HEIGHT {
        return (buf.to_vec(), w, h);
    }
    let (ow, oh) = (w * 2, h * 2);
    let mut out = vec![0u32; ow * oh];
    for y in 0..h {
        let (up, down) = (y.saturating_sub(1), (y + 1).min(h - 1));
        for x in 0..w {
            let (left, right) = (x.saturating_sub(1), (x + 1).min(w - 1));
            let p = buf[y * w + x];
            let a = buf[up * w + x];
            let b = buf[y * w + right];
            let c = buf[y * w + left];
            let d = buf[down * w + x];
            let (mut e1, mut e2, mut e3, mut e4) = (p, p, p, p);
            if c == a && c != d && a != b {
                e1 = a;
            }
            if a == b && a != c && b != d {
                e2 = b;
            }
            if d == c && d != b && c != a {
                e3 = c;
            }
            if b == d && b != a && d != c {
                e4 = d;
            }
            let (ox, oy) = (x * 2, y * 2);
            out[oy * ow + ox] = e1;
            out[oy * ow + ox + 1] = e2;
            out[(oy + 1) * ow + ox] = e3;
            out[(oy + 1) * ow + ox + 1] = e4;
        }
    }
    (out, ow, oh)
}

#[cfg(test)]
mod tests {
    use super::*;

    const X: u32 = 0x00FF_FFFF;
    const O: u32 = 0x0000_0000;

    fn rows(buf: &[u32], w: usize) -> Vec<Vec<u32>> {
        buf.chunks(w).map(<[u32]>::to_vec).collect()
    }

    #[test]
    fn the_picture_doubles_in_both_directions() {
        let (out, w, h) = apply(&[1, 2, 3, 4, 5, 6], 3, 2);
        assert_eq!((w, h, out.len()), (6, 4, 24));
    }

    #[test]
    fn a_flat_picture_stays_flat_and_a_lone_pixel_is_not_smoothed_away() {
        let (flat, ..) = apply(&[7; 9], 3, 3);
        assert!(flat.iter().all(|&p| p == 7));
        // a single dot has no two neighbours agreeing on a corner: it just becomes 2x2
        let mut dot = [O; 9];
        dot[4] = X;
        let (out, w, _) = apply(&dot, 3, 3);
        let r = rows(&out, w);
        assert_eq!(&r[2][2..4], &[X, X]);
        assert_eq!(&r[3][2..4], &[X, X]);
        assert_eq!(
            out.iter().filter(|&&p| p == X).count(),
            4,
            "no pixel grew or vanished"
        );
    }

    #[test]
    fn a_diagonal_stair_gets_its_corners_filled_in() {
        // X in the bottom-left half, O in the top-right half: a staircase
        //   O O O
        //   X O O      (the middle pixel row/col is the one checked)
        //   X X O
        let src = [O, O, O, X, O, O, X, X, O];
        let (out, w, _) = apply(&src, 3, 3);
        let r = rows(&out, w);
        // the centre pixel is O with C (left) = X, D (below) = X, A (up) = O, B (right) = O:
        // corner 3 (bottom-left) sees D == C (both X) and D != B, C != A: it becomes X
        assert_eq!(
            r[3][2], X,
            "corner 3 took the colour of its two X neighbours"
        );
        // the other three corners keep the centre colour
        assert_eq!((r[2][2], r[2][3], r[3][3]), (O, O, O));
    }

    #[test]
    fn it_never_makes_a_colour_that_was_not_in_the_picture() {
        let src: Vec<u32> = (0..64).map(|i| (i * 7 % 5) as u32 * 0x0010_0000).collect();
        let (out, ..) = apply(&src, 8, 8);
        assert!(out.iter().all(|p| src.contains(p)));
    }

    #[test]
    fn edges_use_the_pixel_itself_and_odd_input_passes_through() {
        // 1x1 and 1-wide pictures must not index outside the buffer
        assert_eq!(apply(&[5], 1, 1).0, vec![5, 5, 5, 5]);
        let (col, w, h) = apply(&[1, 2, 3], 1, 3);
        assert_eq!((w, h, col.len()), (2, 6, 12));
        // wrong size, empty and very tall pictures come back unchanged
        assert_eq!(apply(&[1, 2, 3], 2, 2).0, vec![1, 2, 3]);
        assert_eq!(apply(&[], 0, 0), (vec![], 0, 0));
        let tall = vec![1u32; 4 * (MAX_HEIGHT + 1)];
        assert_eq!(apply(&tall, 4, MAX_HEIGHT + 1).0.len(), tall.len());
    }
}
