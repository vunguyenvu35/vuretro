//! The backdrop of a course: the sky, the far hills and the ground at the horizon.
//!
//! `IMAGES/HORIZONS/<NAME>.BOB` is a picture of 720x320 with no header (230,400 bytes), the
//! 720 columns going once all the way round the rider: it scrolls sideways as the road turns.
//! In the 640x480 picture the horizon line is on row 180, which is where the road starts, so
//! row `y` of the picture is row `y` of this one. The courses use other names for their
//! horizons than for themselves.

use crate::disc::Disc;
use crate::image::Rgb;
use crate::palette::Palette;

pub const WIDTH: usize = 720;
pub const HEIGHT: usize = 320;

/// The horizon that goes with a course.
pub fn name_for(course: &str) -> String {
    match course.to_uppercase().as_str() {
        "HIWAY" => "SIERRA",
        "MEDLY" => "PENIN",
        "CANYN" => "PCH",
        other => return other.to_string(),
    }
    .to_string()
}

/// The horizon of a course, in true colour.
pub fn load(disc: &Disc, course: &str, palette: &Palette) -> Result<Rgb, String> {
    let bytes = disc.read(&format!("IMAGES/HORIZONS/{}.BOB", name_for(course)))?;
    if bytes.len() != WIDTH * HEIGHT {
        return Err(format!(
            "a horizon is {} bytes, this one {}",
            WIDTH * HEIGHT,
            bytes.len()
        ));
    }
    let mut pixels = Vec::with_capacity(bytes.len() * 3);
    for &i in &bytes {
        pixels.extend_from_slice(&palette.0[i as usize]);
    }
    Ok(Rgb {
        width: WIDTH as u32,
        height: HEIGHT as u32,
        pixels,
    })
}

/// The part of the horizon in front of the rider as a picture of 320x240 (every second pixel
/// of the 640x480 one). `scroll` is how many columns of the horizon the view has turned to the
/// right; the picture is the middle 640 columns of the 720.
pub fn backdrop(horizon: &Rgb, scroll: i64) -> Rgb {
    let (w, h) = (horizon.width as usize, horizon.height as usize);
    let mut pixels = vec![0u8; 320 * 240 * 3];
    for y in 0..240 {
        let sy = (y * 2).min(h - 1);
        for x in 0..320 {
            let sx = (x as i64 * 2 + (w as i64 - 640) / 2 + scroll).rem_euclid(w as i64) as usize;
            let from = (sy * w + sx) * 3;
            let to = (y * 320 + x) * 3;
            pixels[to..to + 3].copy_from_slice(&horizon.pixels[from..from + 3]);
        }
    }
    Rgb {
        width: 320,
        height: 240,
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn courses_name_their_horizons_differently() {
        assert_eq!(name_for("NAPA"), "NAPA");
        assert_eq!(name_for("city"), "CITY");
        assert_eq!(name_for("HIWAY"), "SIERRA");
        assert_eq!(name_for("MEDLY"), "PENIN");
        assert_eq!(name_for("CANYN"), "PCH");
    }

    /// A horizon whose pixel colour says which column it is in.
    fn ramp() -> Rgb {
        let mut pixels = Vec::new();
        for _ in 0..HEIGHT {
            for x in 0..WIDTH {
                pixels.extend_from_slice(&[(x % 256) as u8, (x / 256) as u8, 0]);
            }
        }
        Rgb {
            width: WIDTH as u32,
            height: HEIGHT as u32,
            pixels,
        }
    }
    fn column(p: &Rgb, x: usize) -> usize {
        let at = x * 3;
        p.pixels[at] as usize + 256 * p.pixels[at + 1] as usize
    }

    #[test]
    fn the_picture_is_the_middle_of_the_horizon_and_turns_with_the_scroll() {
        let b = backdrop(&ramp(), 0);
        assert_eq!((b.width, b.height), (320, 240));
        assert_eq!(column(&b, 0), 40, "the middle 640 of 720 start at 40");
        assert_eq!(column(&b, 1), 42, "every second column");
        assert_eq!(
            column(&backdrop(&ramp(), 10), 0),
            50,
            "turning right shows what is to the right"
        );
        assert_eq!(column(&backdrop(&ramp(), -50), 0), 710, "and it goes round");
    }

    #[test]
    fn rows_below_the_horizon_image_keep_its_last_row() {
        let mut h = ramp();
        let last = ((HEIGHT - 1) * WIDTH + 40) * 3; // column 40 is the left edge of the picture
        h.pixels[last..last + 3].copy_from_slice(&[7, 7, 7]);
        let b = backdrop(&h, 0);
        assert_eq!(&b.pixels[(239 * 320) * 3..(239 * 320) * 3 + 3], &[7, 7, 7]);
    }
}
