//! Draws a run of frames of a sprite set of the disc (`DATA/BIKERS/*.DAT`), each with its
//! number written under it, in the colours of a rider.
//! Usage: rr-frames <disc.iso or folder> <set, e.g. HIBOB> <first> <count> <picture.png> [rider colour 0-14]
use std::path::Path;
use vuretro_roadrash::disc::Disc;
use vuretro_roadrash::image::{colour, sheet, write_png, Rgb};
use vuretro_roadrash::palette::{Order, Palette};
use vuretro_roadrash::{bob, exe, hud};

fn main() -> Result<(), String> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 5 {
        return Err("usage: rr-frames <disc> <set> <first> <count> <picture.png> [colour]".into());
    }
    let disc = Disc::open(Path::new(&a[0]))?;
    let (first, count): (usize, usize) = (
        a[2].parse().map_err(|_| "first is a number")?,
        a[3].parse().map_err(|_| "count is a number")?,
    );
    let rider: usize = a
        .get(5)
        .map_or(Ok(0), |c| c.parse())
        .map_err(|_| "colour is a number")?;
    let palette = Palette::parse(&disc.read("DATA/PALETTE.RAW")?, Order::Rgbx)?;
    let remap = exe::rider_remaps(&disc.read("ROADRASH.EXE")?)?[rider.min(14)];
    let frames = bob::read_frames(&disc.read(&format!("DATA/BIKERS/{}.DAT", a[1]))?)?;
    let mut pictures = Vec::new();
    for n in first..(first + count).min(frames.len()) {
        let f = &frames[n];
        let mapped: Vec<u8> = f.indices.iter().map(|&i| remap[i as usize]).collect();
        let mut p = colour(&mapped, f.width.max(1), &palette);
        if f.width == 0 || f.height == 0 {
            p = Rgb {
                width: 8,
                height: 8,
                pixels: vec![60; 8 * 8 * 3],
            };
        }
        // a strip under the picture for the number
        let (w, h) = (p.width.max(40) as usize, p.height as usize + 14);
        let mut pixels = vec![32u8; w * h * 3];
        for y in 0..p.height as usize {
            let from = y * p.width as usize * 3;
            pixels[(y * w) * 3..(y * w + p.width as usize) * 3]
                .copy_from_slice(&p.pixels[from..from + p.width as usize * 3]);
        }
        let mut cell = Rgb {
            width: w as u32,
            height: h as u32,
            pixels,
        };
        hud::text(
            &mut cell,
            2,
            p.height as i64 + 2,
            1,
            &n.to_string(),
            [250, 230, 60],
        );
        pictures.push(cell);
    }
    write_png(&sheet(&pictures, 8), Path::new(&a[4]))
}
