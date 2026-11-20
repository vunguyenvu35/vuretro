//! A badge for the game window's icon that says which system the game belongs to: a chip in
//! the system maker's colour with its short name ("NG", "PS2"...) in the corner of the app
//! icon. The letters are drawn as round-ended strokes, like the strokes of the logo. Pure
//! pixel work on an RGBA buffer, so it is unit-tested without a window; the runner decodes
//! the app icon and shows the result.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Badge {
    pub label: &'static str,
    pub color: [u8; 3],
}

const SONY: [u8; 3] = [70, 90, 230];
const NINTENDO: [u8; 3] = [225, 30, 45];
const SEGA: [u8; 3] = [0, 140, 215];
const SNK: [u8; 3] = [225, 140, 0];
const ATARI: [u8; 3] = [225, 85, 35];
const NEC: [u8; 3] = [30, 160, 90];
const ARCADE: [u8; 3] = [130, 70, 200];
const BANDAI: [u8; 3] = [200, 70, 160];

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

type Stroke = Vec<(f32, f32)>;

/// Points of an elliptic arc (angles in radians, y points down: 0 is right, pi/2 bottom).
fn arc(cx: f32, cy: f32, rx: f32, ry: f32, t0: f32, t1: f32) -> Stroke {
    let n = 14;
    (0..=n)
        .map(|i| {
            let t = t0 + (t1 - t0) * i as f32 / n as f32;
            (cx + rx * t.cos(), cy + ry * t.sin())
        })
        .collect()
}

fn join(parts: &[Stroke]) -> Stroke {
    parts.iter().flatten().copied().collect()
}

fn curve(p0: (f32, f32), c: (f32, f32), p1: (f32, f32)) -> Stroke {
    let n = 12;
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            (
                u * u * p0.0 + 2.0 * u * t * c.0 + t * t * p1.0,
                u * u * p0.1 + 2.0 * u * t * c.1 + t * t * p1.1,
            )
        })
        .collect()
}

/// The letter as strokes in a box 0.6 wide and 1 high (y down), or `None` for a character
/// the badges do not use.
fn strokes(c: char) -> Option<Vec<Stroke>> {
    use std::f32::consts::{PI, TAU};
    let line = |pts: &[(f32, f32)]| pts.to_vec();
    Some(match c {
        'A' => vec![
            line(&[(0.0, 1.0), (0.3, 0.0), (0.6, 1.0)]),
            line(&[(0.12, 0.68), (0.48, 0.68)]),
        ],
        'B' => vec![join(&[
            line(&[(0.0, 1.0), (0.0, 0.0), (0.3, 0.0)]),
            arc(0.3, 0.25, 0.25, 0.25, -PI / 2.0, PI / 2.0),
            line(&[(0.0, 0.5), (0.32, 0.5)]),
            arc(0.32, 0.75, 0.27, 0.25, -PI / 2.0, PI / 2.0),
            line(&[(0.0, 1.0)]),
        ])],
        'C' => vec![arc(0.32, 0.5, 0.29, 0.5, 0.6, TAU - 0.6)],
        'D' => vec![join(&[
            line(&[(0.0, 1.0), (0.0, 0.0), (0.2, 0.0)]),
            arc(0.2, 0.5, 0.4, 0.5, -PI / 2.0, PI / 2.0),
            line(&[(0.0, 1.0)]),
        ])],
        'E' => vec![
            line(&[(0.52, 0.0), (0.0, 0.0), (0.0, 1.0), (0.52, 1.0)]),
            line(&[(0.0, 0.5), (0.42, 0.5)]),
        ],
        'G' => vec![join(&[
            arc(0.32, 0.5, 0.29, 0.5, TAU - 0.6, 0.0),
            line(&[(0.34, 0.54)]),
        ])],
        'L' => vec![line(&[(0.0, 0.0), (0.0, 1.0), (0.5, 1.0)])],
        'M' => vec![line(&[
            (0.0, 1.0),
            (0.0, 0.0),
            (0.3, 0.58),
            (0.6, 0.0),
            (0.6, 1.0),
        ])],
        'N' => vec![line(&[(0.0, 1.0), (0.0, 0.0), (0.6, 1.0), (0.6, 0.0)])],
        'P' => vec![join(&[
            line(&[(0.0, 1.0), (0.0, 0.0), (0.3, 0.0)]),
            arc(0.3, 0.27, 0.28, 0.27, -PI / 2.0, PI / 2.0),
            line(&[(0.0, 0.54)]),
        ])],
        'S' => vec![join(&[
            arc(0.3, 0.25, 0.27, 0.25, TAU - 0.45, PI / 2.0),
            arc(0.3, 0.75, 0.29, 0.25, -PI / 2.0, PI - 0.45),
        ])],
        'V' => vec![line(&[(0.0, 0.0), (0.3, 1.0), (0.6, 0.0)])],
        'W' => vec![line(&[
            (0.0, 0.0),
            (0.15, 1.0),
            (0.3, 0.38),
            (0.45, 1.0),
            (0.6, 0.0),
        ])],
        'X' => vec![
            line(&[(0.0, 0.0), (0.6, 1.0)]),
            line(&[(0.6, 0.0), (0.0, 1.0)]),
        ],
        'Y' => vec![
            line(&[(0.0, 0.0), (0.3, 0.5), (0.6, 0.0)]),
            line(&[(0.3, 0.5), (0.3, 1.0)]),
        ],
        '0' => vec![arc(0.3, 0.5, 0.28, 0.5, 0.0, TAU)],
        '2' => vec![join(&[
            arc(0.3, 0.27, 0.27, 0.27, PI + 0.3, TAU + 0.5),
            line(&[(0.0, 1.0), (0.58, 1.0)]),
        ])],
        '3' => vec![join(&[
            arc(0.3, 0.25, 0.27, 0.25, PI + 0.4, TAU + PI / 2.0),
            arc(0.3, 0.75, 0.29, 0.25, -PI / 2.0, PI - 0.4),
        ])],
        '4' => vec![line(&[(0.45, 1.0), (0.45, 0.0), (0.0, 0.7), (0.6, 0.7)])],
        '6' => vec![
            arc(0.3, 0.68, 0.28, 0.32, 0.0, TAU),
            curve((0.02, 0.68), (0.0, 0.1), (0.5, 0.02)),
        ],
        '7' => vec![line(&[(0.0, 0.0), (0.6, 0.0), (0.2, 1.0)])],
        '8' => vec![
            arc(0.3, 0.25, 0.24, 0.25, 0.0, TAU),
            arc(0.3, 0.75, 0.29, 0.25, 0.0, TAU),
        ],
        _ => return None,
    })
}

/// Lay `src` (with coverage `a`, 0..=1) over the pixel at `i` of a non-premultiplied RGBA buffer.
fn blend(rgba: &mut [u8], i: usize, src: [f32; 3], a: f32) {
    if a <= 0.0 {
        return;
    }
    let px = &mut rgba[i * 4..i * 4 + 4];
    let dst_a = px[3] as f32 / 255.0;
    let out_a = a + dst_a * (1.0 - a);
    for c in 0..3 {
        let v = (src[c] * a + px[c] as f32 * dst_a * (1.0 - a)) / out_a;
        px[c] = v.round().clamp(0.0, 255.0) as u8;
    }
    px[3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
}

/// Distance from a point to a segment.
fn seg_dist(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    };
    ((p.0 - (a.0 + t * dx)).powi(2) + (p.1 - (a.1 + t * dy)).powi(2)).sqrt()
}

/// Signed distance from a point to a stadium (pill) between `(x0, cy)` and `(x1, cy)`.
fn pill_dist(p: (f32, f32), x0: f32, x1: f32, cy: f32, r: f32) -> f32 {
    let nx = p.0.clamp(x0, x1);
    ((p.0 - nx).powi(2) + (p.1 - cy).powi(2)).sqrt() - r
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Draw the badge in the bottom right corner of a square RGBA picture `size` pixels wide.
pub fn draw(rgba: &mut [u8], size: usize, badge: &Badge) {
    assert_eq!(rgba.len(), size * size * 4, "not a square RGBA picture");
    let letters: Vec<Vec<Stroke>> = badge.label.chars().filter_map(strokes).collect();
    let n = letters.len();
    if n == 0 {
        return;
    }
    let sz = size as f32;
    // letter height `cap`; letters are 0.6 wide with 0.3 between them, round stroke 0.17
    let chip_h = sz * 0.27;
    let mut cap = chip_h * 0.50;
    let text_w = |cap: f32| (n as f32 * 0.6 + (n as f32 - 1.0) * 0.3) * cap;
    let max_w = sz * 0.58;
    if text_w(cap) + cap > max_w {
        cap = max_w / (n as f32 * 0.6 + (n as f32 - 1.0) * 0.3 + 1.0);
    }
    let pad = cap * 0.5 + chip_h * 0.08;
    let w = text_w(cap) + 2.0 * pad;
    let h = chip_h;
    let right = sz * 0.96;
    let bottom = sz * 0.96;
    let (left, top) = (right - w, bottom - h);
    let r = h / 2.0;
    let cy = top + r;
    let (x0, x1) = (left + r, right - r);
    let stroke = cap * 0.17;

    // the letters, as segments in pixels
    let mut segs: Vec<((f32, f32), (f32, f32))> = Vec::new();
    let ox = left + (w - text_w(cap)) / 2.0;
    let oy = cy - cap / 2.0;
    for (i, strokes) in letters.iter().enumerate() {
        let lx = ox + i as f32 * 0.9 * cap;
        for pts in strokes {
            let px: Vec<(f32, f32)> = pts
                .iter()
                .map(|&(x, y)| (lx + x * cap, oy + y * cap))
                .collect();
            if px.len() == 1 {
                segs.push((px[0], px[0]));
            }
            segs.extend(px.windows(2).map(|p| (p[0], p[1])));
        }
    }

    let base = [
        badge.color[0] as f32,
        badge.color[1] as f32,
        badge.color[2] as f32,
    ];
    let hi = mix(base, [255.0; 3], 0.20);
    let lo = mix(base, [0.0; 3], 0.16);
    let blur = sz * 0.03;
    let shadow_dy = sz * 0.014;
    let ring = (sz * 0.008).max(1.0);
    let y_from = (top - 2.0 * blur).max(0.0) as usize;
    let x_from = (left - 2.0 * blur).max(0.0) as usize;
    for y in y_from..size {
        for x in x_from..size {
            let p = (x as f32 + 0.5, y as f32 + 0.5);
            let i = y * size + x;
            // soft shadow under the chip
            let sd = pill_dist((p.0, p.1 - shadow_dy), x0, x1, cy, r);
            let shadow = (1.0 - (sd / blur).clamp(0.0, 1.0)).powi(2) * 0.55;
            blend(rgba, i, [0.0; 3], if sd > 0.0 { shadow } else { 0.0 });
            let d = pill_dist(p, x0, x1, cy, r);
            let cover = (0.5 - d).clamp(0.0, 1.0);
            if cover <= 0.0 {
                continue;
            }
            let fill = mix(hi, lo, ((p.1 - top) / h).clamp(0.0, 1.0));
            blend(rgba, i, fill, cover);
            // a light edge inside the chip
            let edge = (1.0 - (0.5 - (d + ring)).clamp(0.0, 1.0)).min(cover);
            blend(rgba, i, [255.0; 3], edge * 0.55);
            let dist = segs
                .iter()
                .map(|&(a, b)| seg_dist(p, a, b))
                .fold(f32::MAX, f32::min);
            let ink = (0.5 + stroke / 2.0 - dist).clamp(0.0, 1.0);
            blend(rgba, i, [255.0; 3], ink);
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
            assert!(b.label.chars().all(|c| strokes(c).is_some()), "{}", b.label);
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
        let any = (120..250).any(|x| (170..250).any(|y| alpha(x, y) == 255));
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
