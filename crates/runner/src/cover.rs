//! Game preview images ("covers"): the library shows a screenshot of each game.

use anyhow::{Context, Result};
use std::path::Path;

/// A frame needs at least this many distinct colours to be worth showing (a black or
/// single-colour boot screen has 1 to 3).
pub const MIN_COLOURS: usize = 16;

/// If two colours cover more of the picture than this, it is a flat fill, not a game screen.
/// A game that did not start properly often shows stripes of two colours with a few
/// corrupted tiles on top; those tiles add many colours but the picture is still not one.
const MAX_TOP2_SHARE: f64 = 0.88;

/// If almost every neighbouring pixel differs, it is static, not a picture.
const MAX_NOISE: f64 = 0.92;

/// How good a frame is as a preview: the number of distinct colours in a sample of its
/// pixels (capped), or 0 when it is not a plausible game screen: too few colours, a flat
/// fill, or noise.
pub fn richness(buf: &[u32], w: usize) -> usize {
    use std::collections::HashMap;
    let mut counts: HashMap<u32, usize> = HashMap::new();
    let mut sampled = 0usize;
    for &p in buf.iter().step_by(5) {
        *counts.entry(p).or_default() += 1;
        sampled += 1;
    }
    if counts.len() < MIN_COLOURS || sampled == 0 {
        return 0;
    }
    let mut shares: Vec<usize> = counts.values().copied().collect();
    shares.sort_unstable_by(|a, b| b.cmp(a));
    let top2 = shares.iter().take(2).sum::<usize>() as f64 / sampled as f64;
    if top2 > MAX_TOP2_SHARE {
        return 0;
    }
    // neighbouring pixels along rows (a row of the picture is `w` pixels)
    let (mut pairs, mut differ) = (0usize, 0usize);
    if w > 1 {
        for row in buf.chunks_exact(w).step_by(3) {
            for pair in row.windows(2).step_by(2) {
                pairs += 1;
                differ += usize::from(pair[0] != pair[1]);
            }
        }
    }
    if pairs > 0 && differ as f64 / pairs as f64 > MAX_NOISE {
        return 0;
    }
    counts.len().min(4096)
}

/// A glitch that shows for a single frame (the picture half drawn, memory shown as colours)
/// is gone on the next frame. A real screen, even a moving one, mostly stays: this is the
/// share of pixels that are the same in two frames one after the other.
pub fn same_share(a: &[u32], b: &[u32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let same = a.iter().zip(b).filter(|(x, y)| x == y).count();
    same as f64 / a.len() as f64
}

/// A frame is kept only if at least this much of it is still there on the next frame.
pub const MIN_STABLE: f64 = 0.5;

pub fn write_png(path: &Path, buf: &[u32], w: usize, h: usize) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let file = std::fs::File::create(path).with_context(|| path.display().to_string())?;
    let mut enc = png::Encoder::new(std::io::BufWriter::new(file), w as u32, h as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header()?;
    let rgb: Vec<u8> = buf
        .iter()
        .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8])
        .collect();
    writer.write_image_data(&rgb)?;
    Ok(())
}

/// Keeps the most colourful frame seen.
#[derive(Default)]
pub struct Best {
    score: usize,
    frame: Option<(Vec<u32>, usize, usize)>,
}

impl Best {
    /// Offer `frame` together with the frame that followed it: it counts only if it lasted.
    pub fn offer_stable(&mut self, frame: &[u32], next: &[u32], w: usize, h: usize) {
        if same_share(frame, next) >= MIN_STABLE {
            self.offer(frame, w, h);
        }
    }

    pub fn offer(&mut self, buf: &[u32], w: usize, h: usize) {
        let score = richness(buf, w);
        if score > self.score {
            self.score = score;
            self.frame = Some((buf.to_vec(), w, h));
        }
    }

    /// Write the best frame, if there was one. Returns whether a file was written.
    pub fn save(&self, path: &Path) -> Result<bool> {
        match &self.frame {
            Some((buf, w, h)) => write_png(path, buf, *w, *h).map(|_| true),
            None => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_one_frame_glitch_is_not_kept_but_a_lasting_screen_is() {
        let good = scene();
        let glitch: Vec<u32> = (0..W * H)
            .map(|i| (i as u32).wrapping_mul(2654435761) >> 8)
            .collect();
        let mut best = Best::default();
        best.offer_stable(&glitch, &good, W, H);
        assert!(best.frame.is_none(), "the next frame is nothing like it");
        best.offer_stable(&good, &good, W, H);
        assert!(best.frame.is_some());
        // a screen that moves a little still counts
        let mut moved = good.clone();
        for p in moved.iter_mut().take(W * H / 5) {
            *p ^= 0x00ff00;
        }
        assert!(same_share(&good, &moved) >= MIN_STABLE);
        assert_eq!(
            same_share(&good, &good[1..]),
            0.0,
            "frames of different sizes differ"
        );
    }

    const W: usize = 64;
    const H: usize = 48;

    /// Something like a game screen: blocks of many different colours.
    fn scene() -> Vec<u32> {
        (0..W * H)
            .map(|i| {
                let (x, y) = (i % W, i / W);
                let block = (x / 4 + y / 4 * 5) as u32;
                block.wrapping_mul(2654435761) >> 8
            })
            .collect()
    }

    /// What a game that did not start properly can show: stripes of two greens with a patch
    /// of corrupted tiles (many colours, but not a picture).
    fn stripes_with_garbage() -> Vec<u32> {
        (0..W * H)
            .map(|i| {
                let (x, y) = (i % W, i / W);
                if x < 10 && y > 30 {
                    (i as u32).wrapping_mul(2654435761) >> 8
                } else if y % 2 == 0 {
                    0x0060_d040
                } else {
                    0x0050_b838
                }
            })
            .collect()
    }

    fn static_noise() -> Vec<u32> {
        (0..W * H)
            .map(|i| (i as u32).wrapping_mul(2654435761) >> 8)
            .collect()
    }

    #[test]
    fn only_plausible_game_screens_score() {
        assert!(richness(&scene(), W) >= MIN_COLOURS);
        assert_eq!(richness(&vec![0; W * H], W), 0, "blank");
        assert_eq!(
            richness(&stripes_with_garbage(), W),
            0,
            "two-colour stripes with a corrupted patch"
        );
        assert_eq!(richness(&static_noise(), W), 0, "static");
    }

    #[test]
    fn keeps_the_real_screen_even_when_a_broken_frame_has_more_colours() {
        let mut best = Best::default();
        best.offer(&vec![0u32; W * H], W, H);
        assert!(best.frame.is_none(), "blank screens are never kept");
        best.offer(&stripes_with_garbage(), W, H);
        best.offer(&static_noise(), W, H);
        assert!(best.frame.is_none(), "broken frames are not kept either");
        best.offer(&scene(), W, H);
        assert!(best.frame.is_some());

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b.png");
        assert!(best.save(&path).unwrap());
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[1..4], b"PNG");
        assert!(!Best::default().save(&dir.path().join("none.png")).unwrap());
    }
}
