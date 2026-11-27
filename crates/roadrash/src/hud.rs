//! The instrument panel and the figures on the picture of the ride.
//!
//! The panel is a picture of its own (`IMAGES/BIKES/<class>/KHIDASH.BOB`, 640 wide, index 0
//! see-through) with the two dials printed on it but no needles. Both dials were measured on
//! the picture: the speed dial's hub is at (248, 93) and its figures run from 0 km/h on the
//! left, over the top (160) to 320 on the right; the rev counter's hub is at (385, 80) and runs
//! from 0 to 12 (thousands of turns a minute) the same way. The needles are drawn here.

use crate::bob::Frame;
use crate::image::Rgb;
use crate::palette::Palette;

pub const PANEL_WIDTH: u32 = 640;
const SPEED_HUB: (f32, f32) = (248.0, 93.0);
const REV_HUB: (f32, f32) = (385.0, 80.0);
/// The top of the scale of the two dials.
const SPEED_SCALE: f32 = 320.0;
const REV_SCALE: f32 = 12_000.0;

/// The panel as a sprite, from the bytes of the `.BOB` file.
pub fn panel(bytes: &[u8]) -> Result<Frame, String> {
    if bytes.is_empty() || bytes.len() % PANEL_WIDTH as usize != 0 {
        return Err(format!(
            "{} bytes do not make rows of {PANEL_WIDTH}",
            bytes.len()
        ));
    }
    Ok(Frame {
        width: PANEL_WIDTH,
        height: (bytes.len() / PANEL_WIDTH as usize) as u32,
        indices: bytes.to_vec(),
    })
}

/// Turns of the engine in a minute for a speed in km/h: five gears, each taking the engine from
/// 4000 to 10000 turns, so the needle falls back at every change.
pub fn revs(kmh: f32) -> f32 {
    const GEAR_TOP: [f32; 5] = [60.0, 110.0, 170.0, 240.0, 320.0];
    let mut low = 0.0;
    for top in GEAR_TOP {
        if kmh <= top {
            return 4000.0 + 6000.0 * ((kmh - low) / (top - low)).clamp(0.0, 1.0);
        }
        low = top;
    }
    10_000.0
}

/// The direction of a needle in degrees (0 pointing right, 90 up, 180 left) for a value on a
/// dial that runs from the left over the top to the right.
pub fn needle_angle(value: f32, scale: f32) -> f32 {
    180.0 - 180.0 * (value / scale).clamp(0.0, 1.0)
}

fn set(picture: &mut Rgb, x: i64, y: i64, c: [u8; 3]) {
    if x >= 0 && y >= 0 && (x as u32) < picture.width && (y as u32) < picture.height {
        let at = ((y as u32 * picture.width + x as u32) * 3) as usize;
        picture.pixels[at..at + 3].copy_from_slice(&c);
    }
}

/// A line two pixels thick from the hub in a direction.
fn needle(picture: &mut Rgb, hub: (f32, f32), length: f32, degrees: f32, c: [u8; 3]) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    for step in 0..=(length * 2.0) as i32 {
        let d = step as f32 / 2.0;
        let (x, y) = (hub.0 + cos * d, hub.1 - sin * d);
        for (ox, oy) in [(0, 0), (1, 0), (0, 1)] {
            set(picture, x as i64 + ox, y as i64 + oy, c);
        }
    }
}

const GLYPHS: [(char, [&str; 7]); 26] = [
    (
        '0',
        [
            "01110", "10001", "10011", "10101", "11001", "10001", "01110",
        ],
    ),
    (
        '1',
        [
            "00100", "01100", "00100", "00100", "00100", "00100", "01110",
        ],
    ),
    (
        '2',
        [
            "01110", "10001", "00001", "00010", "00100", "01000", "11111",
        ],
    ),
    (
        '3',
        [
            "11110", "00001", "00001", "01110", "00001", "00001", "11110",
        ],
    ),
    (
        '4',
        [
            "00010", "00110", "01010", "10010", "11111", "00010", "00010",
        ],
    ),
    (
        '5',
        [
            "11111", "10000", "11110", "00001", "00001", "10001", "01110",
        ],
    ),
    (
        '6',
        [
            "00110", "01000", "10000", "11110", "10001", "10001", "01110",
        ],
    ),
    (
        '7',
        [
            "11111", "00001", "00010", "00100", "01000", "01000", "01000",
        ],
    ),
    (
        '8',
        [
            "01110", "10001", "10001", "01110", "10001", "10001", "01110",
        ],
    ),
    (
        '9',
        [
            "01110", "10001", "10001", "01111", "00001", "00010", "01100",
        ],
    ),
    (
        '/',
        [
            "00001", "00010", "00010", "00100", "01000", "01000", "10000",
        ],
    ),
    (
        'P',
        [
            "11110", "10001", "10001", "11110", "10000", "10000", "10000",
        ],
    ),
    (
        'O',
        [
            "01110", "10001", "10001", "10001", "10001", "10001", "01110",
        ],
    ),
    (
        'S',
        [
            "01111", "10000", "10000", "01110", "00001", "00001", "11110",
        ],
    ),
    (
        'L',
        [
            "10000", "10000", "10000", "10000", "10000", "10000", "11111",
        ],
    ),
    (
        'I',
        [
            "01110", "00100", "00100", "00100", "00100", "00100", "01110",
        ],
    ),
    (
        'C',
        [
            "01110", "10001", "10000", "10000", "10000", "10001", "01110",
        ],
    ),
    (
        'E',
        [
            "11111", "10000", "10000", "11110", "10000", "10000", "11111",
        ],
    ),
    (
        'B',
        [
            "11110", "10001", "10001", "11110", "10001", "10001", "11110",
        ],
    ),
    (
        'U',
        [
            "10001", "10001", "10001", "10001", "10001", "10001", "01110",
        ],
    ),
    (
        'T',
        [
            "11111", "00100", "00100", "00100", "00100", "00100", "00100",
        ],
    ),
    (
        'D',
        [
            "11110", "10001", "10001", "10001", "10001", "10001", "11110",
        ],
    ),
    (
        'F',
        [
            "11111", "10000", "10000", "11110", "10000", "10000", "10000",
        ],
    ),
    (
        'H',
        [
            "10001", "10001", "10001", "11111", "10001", "10001", "10001",
        ],
    ),
    (
        'A',
        [
            "01110", "10001", "10001", "11111", "10001", "10001", "10001",
        ],
    ),
    (
        'N',
        [
            "10001", "11001", "10101", "10101", "10011", "10001", "10001",
        ],
    ),
];

/// Write letters (`0-9 / P O S L I C E B U T D F H A N` and spaces) with a dark shadow, each dot `size` pixels wide.
pub fn text(picture: &mut Rgb, x: i64, y: i64, size: i64, words: &str, c: [u8; 3]) {
    let mut left = x;
    for ch in words.chars() {
        if let Some((_, rows)) = GLYPHS.iter().find(|g| g.0 == ch) {
            for (row, bits) in rows.iter().enumerate() {
                for (col, bit) in bits.chars().enumerate() {
                    if bit == '1' {
                        for dy in 0..size {
                            for dx in 0..size {
                                let (px, py) =
                                    (left + col as i64 * size + dx, y + row as i64 * size + dy);
                                set(picture, px + size / 2, py + size / 2, [20, 20, 20]);
                                set(picture, px, py, c);
                            }
                        }
                    }
                }
            }
        }
        left += 6 * size;
    }
}

/// Draw the panel at the bottom of the picture, the needles for a speed in km/h, and the
/// place in the race ("3/7") at the top left.
pub fn draw(picture: &mut Rgb, panel: &Frame, palette: &Palette, kmh: f32, place: (usize, usize)) {
    let top = picture.height as i64 - panel.height as i64;
    for y in 0..panel.height as i64 {
        for x in 0..panel.width.min(picture.width) as i64 {
            let i = panel.indices[(y * panel.width as i64 + x) as usize];
            if i != 0 {
                set(picture, x, top + y, palette.0[i as usize]);
            }
        }
    }
    let red = [225, 30, 30];
    let at = |hub: (f32, f32)| (hub.0, hub.1 + top as f32);
    needle(
        picture,
        at(SPEED_HUB),
        34.0,
        needle_angle(kmh, SPEED_SCALE),
        red,
    );
    needle(
        picture,
        at(REV_HUB),
        28.0,
        needle_angle(revs(kmh), REV_SCALE),
        red,
    );
    text(
        picture,
        12,
        10,
        3,
        &format!("POS {}/{}", place.0, place.1),
        [250, 230, 60],
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blank(w: u32, h: u32) -> Rgb {
        Rgb {
            width: w,
            height: h,
            pixels: vec![0; (w * h * 3) as usize],
        }
    }
    fn lit(p: &Rgb) -> usize {
        p.pixels
            .chunks_exact(3)
            .filter(|c| c.iter().any(|&v| v != 0))
            .count()
    }

    #[test]
    fn the_needles_sweep_from_the_left_over_the_top_to_the_right() {
        assert_eq!(needle_angle(0.0, 320.0), 180.0);
        assert_eq!(needle_angle(160.0, 320.0), 90.0);
        assert_eq!(needle_angle(320.0, 320.0), 0.0);
        assert_eq!(
            needle_angle(999.0, 320.0),
            0.0,
            "past the end stays at the end"
        );
        assert_eq!(needle_angle(-5.0, 320.0), 180.0);
    }

    #[test]
    fn the_engine_falls_back_at_every_change_of_gear() {
        assert!(revs(0.0) >= 4000.0 && revs(320.0) <= 10_000.0);
        assert!(revs(59.0) > revs(30.0), "rises in a gear");
        assert!(revs(61.0) < revs(59.0), "falls after the change");
        assert!(revs(500.0) <= 10_000.0);
    }

    #[test]
    fn the_panel_is_made_of_rows_of_640() {
        let p = panel(&vec![1; 640 * 3]).unwrap();
        assert_eq!((p.width, p.height), (640, 3));
        assert!(panel(&[1; 100]).is_err());
        assert!(panel(&[]).is_err());
    }

    #[test]
    fn a_needle_is_drawn_from_its_hub_in_its_direction() {
        let mut up = blank(100, 100);
        needle(&mut up, (50.0, 50.0), 20.0, 90.0, [255, 0, 0]);
        let at = |p: &Rgb, x: usize, y: usize| p.pixels[(y * 100 + x) * 3];
        assert_eq!(at(&up, 50, 35), 255, "straight up");
        assert_eq!(at(&up, 50, 80), 0, "not downwards");
        let mut right = blank(100, 100);
        needle(&mut right, (50.0, 50.0), 20.0, 0.0, [255, 0, 0]);
        assert_eq!(at(&right, 65, 50), 255);
    }

    #[test]
    fn figures_are_written_and_unknown_letters_are_skipped() {
        let mut p = blank(200, 40);
        text(&mut p, 2, 2, 2, "3/7", [255, 255, 255]);
        assert!(lit(&p) > 30);
        let mut q = blank(200, 40);
        text(&mut q, 2, 2, 2, "?#", [255, 255, 255]);
        assert_eq!(lit(&q), 0);
    }

    #[test]
    fn the_panel_goes_at_the_bottom_and_see_through_stays_see_through() {
        let palette = Palette([[9, 9, 9]; 256]);
        let mut indices = vec![0u8; 640 * 4];
        indices[0] = 7; // the top left dot of the panel
        let panel = Frame {
            width: 640,
            height: 4,
            indices,
        };
        let mut p = blank(640, 20);
        draw(&mut p, &panel, &palette, 0.0, (1, 1));
        let at = |x: usize, y: usize| p.pixels[(y * 640 + x) * 3];
        assert_eq!(at(0, 16), 9, "the dot of the panel, 4 rows from the bottom");
        assert_eq!(at(1, 16), 0, "see-through leaves the picture as it was");
    }
}
