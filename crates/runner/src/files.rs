//! The session's files on the real disk, and PNG encoding.

use anyhow::{Context, Result};
use std::path::Path;
use vuretro_play::ports::Files;

pub struct DiskFiles;

impl Files for DiskFiles {
    fn read(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }
    fn write(&self, path: &Path, data: &[u8]) -> std::io::Result<()> {
        std::fs::write(path, data)
    }
    fn write_png(&self, path: &Path, buf: &[u32], w: usize, h: usize) -> Result<()> {
        write_png(path, buf, w, h)
    }
}

/// Write 0RGB pixels as an 8-bit RGB PNG, creating the folder if needed.
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
        .flat_map(|&p| [(p >> 16) as u8, (p >> 8) as u8, p as u8])
        .collect();
    writer.write_image_data(&rgb)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_png_and_makes_its_folder() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a/b.png");
        write_png(&path, &[0x00ff00; 4], 2, 2).unwrap();
        assert_eq!(&std::fs::read(&path).unwrap()[1..4], b"PNG");
    }
}
