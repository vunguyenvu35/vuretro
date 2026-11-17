//! Game preview images ("covers"): the library shows a screenshot of each game.

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;

/// A frame needs at least this many distinct colours to be worth showing (a black or
/// single-colour boot screen has 1 to 3).
pub const MIN_COLOURS: usize = 16;

/// How colourful a frame is: distinct colours in a sample of its pixels, capped.
pub fn richness(buf: &[u32]) -> usize {
    let mut seen = HashSet::new();
    for &p in buf.iter().step_by(5) {
        seen.insert(p);
        if seen.len() >= 4096 {
            break;
        }
    }
    seen.len()
}

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
    pub fn offer(&mut self, buf: &[u32], w: usize, h: usize) {
        let score = richness(buf);
        if score >= MIN_COLOURS && score > self.score {
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
    fn picks_the_colourful_frame_and_writes_a_png() {
        let blank = vec![0u32; 64 * 48];
        let busy: Vec<u32> = (0..64 * 48)
            .map(|i| (i as u32).wrapping_mul(2654435761) >> 8)
            .collect();
        let mut best = Best::default();
        best.offer(&blank, 64, 48);
        assert!(best.frame.is_none()); // blank screens are never kept
        best.offer(&busy, 64, 48);
        assert!(best.frame.is_some());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b.png");
        assert!(best.save(&path).unwrap());
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[1..4], b"PNG");
        assert!(!Best::default().save(&dir.path().join("none.png")).unwrap());
    }
}
