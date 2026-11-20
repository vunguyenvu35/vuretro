//! A badge for the game window's icon that says which system the game belongs to (a pill with
//! "NG" or "PS" in the corner of the app icon). Pure pixel work on an RGBA buffer, so it is
//! unit-tested without a window; the runner decodes the app icon and shows the result.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Badge {
    pub label: &'static str,
    pub color: [u8; 3],
}

const SONY: [u8; 3] = [0, 55, 145];
const NINTENDO: [u8; 3] = [214, 18, 30];
const SEGA: [u8; 3] = [0, 96, 168];
const SNK: [u8; 3] = [196, 150, 0];
const ATARI: [u8; 3] = [200, 80, 30];
const NEC: [u8; 3] = [214, 110, 0];
const ARCADE: [u8; 3] = [120, 50, 170];
const BANDAI: [u8; 3] = [0, 130, 130];

/// The badge of a system (by its id in the registry).
pub fn for_system(id: &str) -> Option<Badge> {
    let (label, color) = match id {
        "neogeo" => ("NG", SNK),
        "ngp" => ("NGP", SNK),
        "psx" => ("PS", SONY),
        "ps2" => ("PS2", SONY),
        "psp" => ("PSP", SONY),
        "nes" => ("NES", NINTENDO),
        "snes" => ("SNES", NINTENDO),
        "n64" => ("N64", NINTENDO),
        "gamecube" => ("GC", NINTENDO),
        "gb" => ("GB", NINTENDO),
        "gbc" => ("GBC", NINTENDO),
        "gba" => ("GBA", NINTENDO),
        "nds" => ("NDS", NINTENDO),
        "3ds" => ("3DS", NINTENDO),
        "virtualboy" => ("VB", NINTENDO),
        "genesis" => ("MD", SEGA),
        "sms" => ("SMS", SEGA),
        "gg" => ("GG", SEGA),
        "sega32x" => ("32X", SEGA),
        "saturn" => ("SS", SEGA),
        "pce" => ("PCE", NEC),
        "atari2600" => ("2600", ATARI),
        "atari7800" => ("7800", ATARI),
        "lynx" => ("LYNX", ATARI),
        "wonderswan" => ("WS", BANDAI),
        "arcade" => ("MAME", ARCADE),
        _ => return None,
    };
    Some(Badge { label, color })
}

/// 5x7 pixel letters and digits; each row holds 5 bits, the leftmost pixel is bit 4.
const GLYPHS: &[(char, [u8; 7])] = &[
    (
        '0',
        [
            0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
        ],
    ),
    (
        '1',
        [
            0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
    ),
    (
        '2',
        [
            0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
        ],
    ),
    (
        '3',
        [
            0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
    ),
    (
        '4',
        [
            0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
        ],
    ),
    (
        '5',
        [
            0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110,
        ],
    ),
    (
        '6',
        [
            0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
        ],
    ),
    (
        '7',
        [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
        ],
    ),
    (
        '8',
        [
            0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
        ],
    ),
    (
        '9',
        [
            0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100,
        ],
    ),
    (
        'A',
        [
            0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
    ),
    (
        'B',
        [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110,
        ],
    ),
    (
        'C',
        [
            0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110,
        ],
    ),
    (
        'D',
        [
            0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110,
        ],
    ),
    (
        'E',
        [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111,
        ],
    ),
    (
        'F',
        [
            0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
    ),
    (
        'G',
        [
            0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01111,
        ],
    ),
    (
        'H',
        [
            0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001,
        ],
    ),
    (
        'I',
        [
            0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
        ],
    ),
    (
        'J',
        [
            0b00111, 0b00010, 0b00010, 0b00010, 0b00010, 0b10010, 0b01100,
        ],
    ),
    (
        'K',
        [
            0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001,
        ],
    ),
    (
        'L',
        [
            0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111,
        ],
    ),
    (
        'M',
        [
            0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001,
        ],
    ),
    (
        'N',
        [
            0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001,
        ],
    ),
    (
        'O',
        [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
    ),
    (
        'P',
        [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000,
        ],
    ),
    (
        'Q',
        [
            0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101,
        ],
    ),
    (
        'R',
        [
            0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001,
        ],
    ),
    (
        'S',
        [
            0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110,
        ],
    ),
    (
        'T',
        [
            0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
    ),
    (
        'U',
        [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110,
        ],
    ),
    (
        'V',
        [
            0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100,
        ],
    ),
    (
        'W',
        [
            0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001,
        ],
    ),
    (
        'X',
        [
            0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001,
        ],
    ),
    (
        'Y',
        [
            0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100,
        ],
    ),
    (
        'Z',
        [
            0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111,
        ],
    ),
];

fn glyph(c: char) -> Option<&'static [u8; 7]> {
    GLYPHS.iter().find(|(g, _)| *g == c).map(|(_, rows)| rows)
}

/// Lay `src` (with coverage `a`, 0..=1) over the pixel at `i` of a non-premultiplied RGBA buffer.
fn blend(rgba: &mut [u8], i: usize, src: [u8; 3], a: f32) {
    if a <= 0.0 {
        return;
    }
    let px = &mut rgba[i * 4..i * 4 + 4];
    let dst_a = px[3] as f32 / 255.0;
    let out_a = a + dst_a * (1.0 - a);
    for c in 0..3 {
        let v = (src[c] as f32 * a + px[c] as f32 * dst_a * (1.0 - a)) / out_a;
        px[c] = v.round().clamp(0.0, 255.0) as u8;
    }
    px[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}

/// How much of a pixel is inside the stadium (a pill) from `x0` to `x1` at height `cy`,
/// with radius `r`; one pixel of soft edge.
fn coverage(px: f32, py: f32, x0: f32, x1: f32, cy: f32, r: f32) -> f32 {
    let nx = px.clamp(x0, x1);
    let d = ((px - nx).powi(2) + (py - cy).powi(2)).sqrt() - r;
    (0.5 - d).clamp(0.0, 1.0)
}

/// Draw the badge in the bottom right corner of a square RGBA picture `size` pixels wide.
pub fn draw(rgba: &mut [u8], size: usize, badge: &Badge) {
    assert_eq!(rgba.len(), size * size * 4, "not a square RGBA picture");
    let n = badge.label.chars().count();
    if n == 0 {
        return;
    }
    // the unit is one pixel of the 5x7 letters; it shrinks for longer labels
    let unit = (size as f32 / 256.0
        * if n <= 2 {
            7.0
        } else if n == 3 {
            6.0
        } else {
            5.0
        })
    .max(1.0)
    .floor();
    let text_w = n as f32 * 5.0 * unit + (n as f32 - 1.0) * unit;
    let text_h = 7.0 * unit;
    let pad_x = 2.0 * unit;
    let pad_y = 1.6 * unit;
    let ring = (size as f32 / 256.0 * 3.0).max(1.0);
    let h = text_h + 2.0 * pad_y;
    let w = text_w + 2.0 * pad_x;
    let margin = size as f32 * 0.07;
    let right = size as f32 - margin;
    let bottom = size as f32 - margin;
    let (left, top) = (right - w, bottom - h);
    let r = h / 2.0;
    let cy = top + r;

    let light = [255, 255, 255];
    let bright = badge.color.iter().map(|&c| c as u32).sum::<u32>() > 3 * 150;
    let ink = if bright { [0, 0, 0] } else { light };
    for y in 0..size {
        for x in 0..size {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let outer = coverage(px, py, left + r, right - r, cy, r + ring);
            blend(rgba, y * size + x, light, outer * 0.95);
            let inner = coverage(px, py, left + r, right - r, cy, r);
            blend(rgba, y * size + x, badge.color, inner);
        }
    }
    let mut x = left + pad_x;
    let y = top + pad_y;
    for ch in badge.label.chars() {
        if let Some(rows) = glyph(ch) {
            for (ry, row) in rows.iter().enumerate() {
                for rx in 0..5 {
                    if row >> (4 - rx) & 1 == 1 {
                        fill(
                            rgba,
                            size,
                            x + rx as f32 * unit,
                            y + ry as f32 * unit,
                            unit,
                            ink,
                        );
                    }
                }
            }
        }
        x += 6.0 * unit;
    }
}

fn fill(rgba: &mut [u8], size: usize, x: f32, y: f32, side: f32, color: [u8; 3]) {
    for py in y as usize..((y + side) as usize).min(size) {
        for px in x as usize..((x + side) as usize).min(size) {
            blend(rgba, py * size + px, color, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_system_has_a_badge_that_can_be_drawn() {
        for sys in vuretro_core::registry::systems() {
            let b = for_system(sys.id).unwrap_or_else(|| panic!("no badge for {}", sys.id));
            assert!(b.label.chars().all(|c| glyph(c).is_some()), "{}", b.label);
            assert!(b.label.len() <= 4);
        }
        assert_eq!(for_system("nope"), None);
    }

    #[test]
    fn the_badge_sits_in_the_bottom_right_and_leaves_the_rest_alone() {
        let size = 256;
        let mut img = vec![0u8; size * size * 4];
        draw(&mut img, size, &for_system("neogeo").unwrap());
        let alpha = |x: usize, y: usize| img[(y * size + x) * 4 + 3];
        assert_eq!(alpha(20, 20), 0, "top left is untouched");
        assert_eq!(alpha(20, 230), 0, "bottom left is untouched");
        assert_eq!(alpha(235, 20), 0, "top right is untouched");
        let any = (150..250).any(|x| (190..250).any(|y| alpha(x, y) == 255));
        assert!(any, "the pill is drawn");
    }

    #[test]
    fn different_systems_look_different() {
        let draw_of = |id: &str| {
            let mut img = vec![0u8; 64 * 64 * 4];
            draw(&mut img, 64, &for_system(id).unwrap());
            img
        };
        assert_ne!(draw_of("neogeo"), draw_of("psx"));
    }
}
