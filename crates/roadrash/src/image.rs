//! Pictures: a raw 8-bit picture (`.BOB`) and a PNG writer to look at the results.

use crate::palette::Palette;

/// A picture in true colour, 3 bytes per pixel.
#[derive(Debug, Clone, PartialEq)]
pub struct Rgb {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// A `.BOB` file holds the pixels of a picture, one palette index per byte, row after row,
/// with no header (to be confirmed on the pictures): its size is `width * height`.
pub fn decode_bob(bytes: &[u8], width: u32, palette: &Palette) -> Result<Rgb, String> {
    if width == 0 || bytes.len() % width as usize != 0 {
        return Err(format!("{} bytes do not make rows of {width}", bytes.len()));
    }
    let height = (bytes.len() / width as usize) as u32;
    let mut pixels = Vec::with_capacity(bytes.len() * 3);
    for &index in bytes {
        pixels.extend_from_slice(&palette.0[index as usize]);
    }
    Ok(Rgb {
        width,
        height,
        pixels,
    })
}

/// Colour a picture of palette indices; index 0 is shown as magenta so see-through shows.
pub fn colour(indices: &[u8], width: u32, palette: &Palette) -> Rgb {
    let height = indices.len().checked_div(width as usize).unwrap_or(0) as u32;
    let mut pixels = Vec::with_capacity(indices.len() * 3);
    for &i in indices {
        pixels.extend_from_slice(if i == 0 {
            &[255, 0, 255]
        } else {
            &palette.0[i as usize]
        });
    }
    Rgb {
        width,
        height,
        pixels,
    }
}

/// Lay pictures out in rows of `columns` cells, each cell as big as the biggest picture.
pub fn sheet(pictures: &[Rgb], columns: usize) -> Rgb {
    let cell_w = pictures.iter().map(|p| p.width).max().unwrap_or(1).max(1) as usize + 2;
    let cell_h = pictures.iter().map(|p| p.height).max().unwrap_or(1).max(1) as usize + 2;
    let columns = columns.max(1);
    let rows = pictures.len().div_ceil(columns).max(1);
    let (width, height) = (cell_w * columns, cell_h * rows);
    let mut pixels = vec![32u8; width * height * 3];
    for (n, p) in pictures.iter().enumerate() {
        let (x0, y0) = ((n % columns) * cell_w + 1, (n / columns) * cell_h + 1);
        for y in 0..p.height as usize {
            let from = y * p.width as usize * 3;
            let to = ((y0 + y) * width + x0) * 3;
            pixels[to..to + p.width as usize * 3]
                .copy_from_slice(&p.pixels[from..from + p.width as usize * 3]);
        }
    }
    Rgb {
        width: width as u32,
        height: height as u32,
        pixels,
    }
}

/// Write a picture as a PNG.
pub fn write_png(picture: &Rgb, path: &std::path::Path) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut encoder =
        png::Encoder::new(std::io::BufWriter::new(file), picture.width, picture.height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    writer
        .write_image_data(&picture.pixels)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::Order;

    fn palette() -> Palette {
        let mut bytes = vec![0u8; 1024];
        bytes[4..8].copy_from_slice(&[10, 20, 30, 0]);
        Palette::parse(&bytes, Order::Rgbx).unwrap()
    }

    #[test]
    fn indices_become_colours_and_the_height_follows_from_the_size() {
        let picture = decode_bob(&[0, 1, 1, 0, 0, 1], 3, &palette()).unwrap();
        assert_eq!((picture.width, picture.height), (3, 2));
        assert_eq!(&picture.pixels[3..6], &[10, 20, 30]);
        assert_eq!(picture.pixels.len(), 18);
    }

    #[test]
    fn a_size_that_does_not_fit_the_width_is_refused() {
        assert!(decode_bob(&[0; 7], 3, &palette()).is_err());
        assert!(decode_bob(&[0; 6], 0, &palette()).is_err());
    }
}
