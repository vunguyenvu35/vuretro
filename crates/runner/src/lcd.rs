//! A handheld LCD look done on the CPU: every source pixel becomes a small square and the
//! last row and column of each square are dimmed, like the grid between the pixels of a
//! Game Boy screen. Pure, so it is unit-tested without a window.

/// Brightness of the grid lines, in 256ths.
const GRID: u32 = 190;

fn dim(p: u32, level: u32) -> u32 {
    let ch = |shift: u32| (((p >> shift) & 0xff) * level / 256) << shift;
    ch(16) | ch(8) | ch(0)
}

/// Whole-number enlargement for the grid: small pictures need a bigger one to show it.
pub fn factor_for(height: usize) -> usize {
    match height {
        0..=300 => 3,
        301..=600 => 2,
        _ => 1,
    }
}

/// Enlarge by the factor that suits the picture size.
pub fn apply(buf: &[u32], w: usize, h: usize) -> (Vec<u32>, usize, usize) {
    apply_with(buf, w, h, factor_for(h))
}

/// Enlarge by `k` (at least 2) and draw the grid. Smaller `k` or a malformed picture is
/// returned as it is.
pub fn apply_with(buf: &[u32], w: usize, h: usize, k: usize) -> (Vec<u32>, usize, usize) {
    if k < 2 || w == 0 || buf.len() < w * h {
        return (buf.to_vec(), w, h);
    }
    let (ow, oh) = (w * k, h * k);
    let mut out = vec![0u32; ow * oh];
    for y in 0..h {
        for (x, &p) in buf[y * w..(y + 1) * w].iter().enumerate() {
            let grid = dim(p, GRID);
            for r in 0..k {
                let row = &mut out[(y * k + r) * ow + x * k..][..k];
                if r == k - 1 {
                    row.fill(grid);
                } else {
                    row.fill(p);
                    row[k - 1] = grid;
                }
            }
        }
    }
    (out, ow, oh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_lines_are_the_last_row_and_column_of_each_cell() {
        let buf = vec![0x00FF_FFFF];
        let (out, w, h) = apply_with(&buf, 1, 1, 3);
        assert_eq!((w, h), (3, 3));
        let lit = 0x00FF_FFFF;
        let line = dim(lit, GRID);
        assert!(line < lit && line != 0);
        // rows 0 and 1: two lit pixels then a grid pixel; row 2: all grid
        assert_eq!(&out[0..3], &[lit, lit, line]);
        assert_eq!(&out[3..6], &[lit, lit, line]);
        assert_eq!(&out[6..9], &[line, line, line]);
    }

    #[test]
    fn each_source_pixel_keeps_its_own_colour() {
        let buf = vec![0x00FF_0000, 0x0000_FF00];
        let (out, w, _) = apply_with(&buf, 2, 1, 2);
        assert_eq!(w, 4);
        assert_eq!(out[0], 0x00FF_0000);
        assert_eq!(out[2], 0x0000_FF00);
        // dimming keeps hue: a dimmed red pixel has no green
        assert_eq!(out[1] & 0xff00, 0);
    }

    #[test]
    fn factor_depends_on_the_size_and_tiny_factors_pass_through() {
        assert_eq!(factor_for(144), 3);
        assert_eq!(factor_for(480), 2);
        assert_eq!(factor_for(1080), 1);
        assert_eq!(apply_with(&[1, 2], 2, 1, 1).0, vec![1, 2]);
        assert_eq!(apply_with(&[1, 2, 3], 4, 2, 3).0, vec![1, 2, 3]);
    }
}
