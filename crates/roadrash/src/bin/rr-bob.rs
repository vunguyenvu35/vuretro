//! Writes a headerless picture (`.BOB`) of the disc as a PNG, given its width (the height
//! follows from the size). Index 0 is shown as magenta.
//! Usage: rr-bob <disc.iso or folder> <path on the disc> <width> <picture.png> [x y w h scale]
//! (the last five numbers cut a part out and make it `scale` times as big, with a grid line
//! every 10 pixels of the original to read positions from)
use std::path::Path;
use vuretro_roadrash::disc::Disc;
use vuretro_roadrash::image::{colour, write_png};
use vuretro_roadrash::palette::{Order, Palette};

fn main() -> Result<(), String> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 4 {
        return Err("usage: rr-bob <disc.iso or folder> <path on the disc> <width> <picture.png> [x y w h scale]".into());
    }
    let (disc, path, width, out) = (&a[0], &a[1], &a[2], &a[3]);
    let disc = Disc::open(Path::new(disc))?;
    let palette = Palette::parse(&disc.read("DATA/PALETTE.RAW")?, Order::Rgbx)?;
    let bytes = disc.read(path)?;
    let width: u32 = width.parse().map_err(|_| "the width is a number")?;
    if width == 0 || bytes.len() % width as usize != 0 {
        return Err(format!("{} bytes do not make rows of {width}", bytes.len()));
    }
    let picture = colour(&bytes, width, &palette);
    println!("{} x {}", picture.width, picture.height);
    if a.len() == 9 {
        let n: Vec<usize> = a[4..].iter().map(|v| v.parse().unwrap_or(0)).collect();
        let (x0, y0, w, h, k) = (n[0], n[1], n[2], n[3], n[4].max(1));
        let (pw, ph) = (picture.width as usize, picture.height as usize);
        let mut pixels = vec![0u8; w * k * h * k * 3];
        for y in 0..h * k {
            for x in 0..w * k {
                let (sx, sy) = (x0 + x / k, y0 + y / k);
                let mut c = if sx < pw && sy < ph {
                    let at = (sy * pw + sx) * 3;
                    [
                        picture.pixels[at],
                        picture.pixels[at + 1],
                        picture.pixels[at + 2],
                    ]
                } else {
                    [0, 0, 0]
                };
                if (x % (10 * k) == 0 && x / k % 1 == 0) || y % (10 * k) == 0 {
                    c = [c[0] / 2 + 100, c[1] / 2 + 100, c[2] / 2];
                }
                pixels[(y * w * k + x) * 3..][..3].copy_from_slice(&c);
            }
        }
        let zoom = vuretro_roadrash::image::Rgb {
            width: (w * k) as u32,
            height: (h * k) as u32,
            pixels,
        };
        return write_png(&zoom, Path::new(out));
    }
    write_png(&picture, Path::new(out))
}
