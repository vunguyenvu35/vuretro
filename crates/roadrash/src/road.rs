//! The road as the rider sees it: a picture of 320x240 with the horizon on row 90.
//!
//! This is a projection of my own, not the game's table. The rider is on the middle line,
//! 150 world units up, and slices of road half a sample (128 units) apart are projected with
//! a scale of `FOCAL / distance`. Walking away from the rider, the bend turns the road sideways
//! (the angle it stands for is a guess, `turn`) and the rise lifts it (`lift`: world units for one unit of
//! rise; 1 is the reading of the rise as a plain height change per sample). Slices are painted from near to far and a slice that
//! would be hidden behind a hill is skipped.

use crate::course::Sample;
use crate::image::Rgb;
use crate::terrain::{self, Ground, HillSlice, Texture};

pub const WIDTH: usize = 320;
pub const HEIGHT: usize = 240;
const HORIZON: f32 = 90.0;
const FOCAL: f32 = 128.0;
const CAMERA_HEIGHT: f32 = 150.0;
const LANE: f32 = 250.0;
const SAMPLE: f32 = 256.0;
const SLICES: usize = 240;

/// How the bend and rise are taken. See the module documentation.
#[derive(Debug, Clone, Copy)]
pub struct Guess {
    pub turn: f32,
    pub lift: f32,
}

impl Default for Guess {
    fn default() -> Self {
        Guess {
            turn: 0.002,
            lift: 1.0,
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// The view from `position` (a sample of the route), the rider on the middle line.
pub fn view(samples: &[Sample], position: usize, guess: Guess) -> Rgb {
    view_at(samples, position as f32, 0.0, guess)
}

/// Where a thing standing on the road shows in the 320x240 picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spot {
    /// Where its feet are, in the picture.
    pub x: f32,
    pub y: f32,
    /// How big it is compared with a thing one sample ahead of the camera (where the rider's
    /// own bike is): 1 there, a half at two samples, and so on.
    pub scale: f32,
}

/// Where a thing `ahead` samples in front of the camera, `offset` world units to the right of
/// the middle of the road, shows in the picture of [`view_at`]. `None` when it is too close or
/// the road ends before it.
pub fn project(
    samples: &[Sample],
    position: f32,
    lateral: f32,
    guess: Guess,
    ahead: f32,
    offset: f32,
) -> Option<Spot> {
    if ahead < 0.3 {
        return None;
    }
    let first = position.max(0.0).floor() as usize;
    let fraction = position.max(0.0) - first as f32;
    // the slice whose distance is the thing's, as a number between slices
    let wanted = (ahead * SAMPLE - 64.0 + fraction * SAMPLE) / 128.0;
    let whole = wanted.floor().max(0.0) as usize;
    let (mut x, mut dx, mut y) = (-lateral, 0.0f32, 0.0f32);
    let (mut before, mut after) = ((x, y), (x, y));
    for k in 0..=whole + 1 {
        let s = samples.get(first + k / 2)?;
        dx += s.bend as f32 * guess.turn * SAMPLE * 0.5;
        x += dx;
        y += s.rise as f32 * guess.lift * 0.5;
        if k == whole {
            before = (x, y);
        }
        after = (x, y);
    }
    let t = (wanted - whole as f32).clamp(0.0, 1.0);
    let (px, py) = (lerp(before.0, after.0, t), lerp(before.1, after.1, t));
    let z = ahead * SAMPLE;
    let scale = FOCAL / z;
    Some(Spot {
        x: WIDTH as f32 / 2.0 + (px + offset) * scale,
        y: HORIZON + (CAMERA_HEIGHT - py) * scale,
        scale: SAMPLE / z,
    })
}

/// The view from a place between samples, `lateral` world units to the right of the middle.
pub fn view_at(samples: &[Sample], position: f32, lateral: f32, guess: Guess) -> Rgb {
    view_over(samples, position, lateral, guess, None, None)
}

/// Like [`view_at`], over a picture of 320x240 for the sky and the far ground (the horizon),
/// with the hills or the buildings at the side of the road when `ground` has them (a slice for
/// each sample).
pub fn view_over(
    samples: &[Sample],
    position: f32,
    lateral: f32,
    guess: Guess,
    backdrop: Option<&Rgb>,
    ground: Option<&Ground>,
) -> Rgb {
    let first = position.max(0.0).floor() as usize;
    let fraction = position.max(0.0) - first as f32;
    let given = backdrop.filter(|b| b.pixels.len() == WIDTH * HEIGHT * 3);
    let mut pixels = given.map_or_else(|| vec![0u8; WIDTH * HEIGHT * 3], |b| b.pixels.clone());
    if given.is_none() {
        // the sky, then the ground under the horizon
        for row in 0..HEIGHT {
            let t = row as f32 / HORIZON;
            let colour = if (row as f32) < HORIZON {
                [(90.0 + 90.0 * t) as u8, (140.0 + 60.0 * t) as u8, 235]
            } else {
                [40, 120, 40]
            };
            put_row(&mut pixels, row, 0.0, WIDTH as f32, colour);
        }
    }

    // the slices of road ahead, near to far
    let mut slices: Vec<Slice> = Vec::new();
    let (mut x, mut dx, mut y) = (-lateral, 0.0f32, 0.0f32);
    for k in 0..SLICES {
        let z = (64.0 + k as f32 * 128.0 - fraction * SAMPLE).max(32.0);
        let Some(s) = samples.get(first + k / 2) else {
            break;
        };
        dx += s.bend as f32 * guess.turn * SAMPLE * 0.5;
        x += dx;
        y += s.rise as f32 * guess.lift * 0.5;
        let scale = FOCAL / z;
        let row = HORIZON + (CAMERA_HEIGHT - y) * scale;
        slices.push(Slice {
            row,
            left: WIDTH as f32 / 2.0 + (x - s.lanes[0] * LANE) * scale,
            right: WIDTH as f32 / 2.0 + (x + s.lanes[1] * LANE) * scale,
            centre: WIDTH as f32 / 2.0 + x * scale,
            scale,
            x,
            lanes: s.lanes,
            hills: ground.and_then(|g| g.slices.get(first + k / 2).copied().flatten()),
        });
    }

    // which rows each slice paints: from the row where the one before it ended, up to its own,
    // unless a nearer hill crest hides it
    struct Span {
        k: usize,
        first: usize,
        last: usize,
    }
    let mut spans = Vec::new();
    let mut near_row = HORIZON + CAMERA_HEIGHT * FOCAL / 64.0;
    let mut top = HEIGHT as f32;
    for (k, s) in slices.iter().enumerate() {
        if s.row < near_row && s.row < top {
            // up to the row where the slice before ended, rounded up so that no row is left out
            spans.push(Span {
                k,
                first: s.row.max(0.0).ceil() as usize,
                last: (near_row.min(top).min(HEIGHT as f32)).ceil() as usize,
            });
            top = top.min(s.row);
        }
        near_row = s.row;
    }
    let before = |k: usize| {
        slices.get(k.wrapping_sub(1)).copied().unwrap_or(Slice {
            row: HORIZON + CAMERA_HEIGHT * FOCAL / 64.0,
            left: 0.0,
            right: 0.0,
            centre: 0.0,
            scale: 0.0,
            x: 0.0,
            lanes: [0.0; 2],
            hills: None,
        })
    };

    // pass 1: the ground in the colour of the stripe
    for sp in &spans {
        let grass = if (sp.k / 4) % 2 == 0 {
            [45, 130, 45]
        } else {
            [35, 110, 35]
        };
        let far = slices[sp.k];
        for r in sp.first..sp.last {
            match far.hills {
                // the shoulder takes the colour of the face of the hill next to it
                Some(h) => {
                    put_row(&mut pixels, r, 0.0, far.centre, h.colour[0][0]);
                    put_row(&mut pixels, r, far.centre, WIDTH as f32, h.colour[1][0]);
                }
                None => put_row(&mut pixels, r, 0.0, WIDTH as f32, grass),
            }
        }
    }

    // pass 2: the hills and the buildings, the far faces first so that the near ones cover them
    let textures: &[Texture] = ground.map_or(&[], |g| &g.textures);
    let roof = ground.map_or([120, 120, 120], |g| g.roof);
    for k in (1..slices.len()).rev() {
        let (near, far) = (slices[k - 1], slices[k]);
        let (Some(hn), Some(hf)) = (near.hills, far.hills) else {
            continue;
        };
        for side in 0..2 {
            let sign = if side == 0 { -1.0 } else { 1.0 };
            if hn.hills && hf.hills {
                let chain = |s: &Slice, h: &HillSlice| -> [(f32, f32); 5] {
                    let mut c = [(0.0, 0.0); 5];
                    c[0] = point(s, s.x + h.edge[side], 0.0);
                    for i in 0..3 {
                        c[i + 1] = point(
                            s,
                            s.x + h.edge[side] + sign * h.out[side][i],
                            h.height[side][i],
                        );
                    }
                    c[4] = point(
                        s,
                        s.x + h.edge[side] + sign * (h.out[side][2] + terrain::FAR),
                        h.height[side][2],
                    );
                    c
                };
                let (cn, cf) = (chain(&near, &hn), chain(&far, &hf));
                for f in 0..4 {
                    let colour = hn.colour[side][f.min(2)];
                    let quad = [cn[f], cn[f + 1], cf[f + 1], cf[f]];
                    fill_triangle(&mut pixels, [quad[0], quad[1], quad[2]], colour);
                    fill_triangle(&mut pixels, [quad[0], quad[2], quad[3]], colour);
                }
            }
            if let (Some(fn_), Some(ff)) = (hn.facades[side], hf.facades[side]) {
                // the roof, as deep as a building is
                let at = |s: &Slice, h: &HillSlice, depth: f32| {
                    point(
                        s,
                        s.x + h.edge[side] + sign * depth,
                        fn_.height.min(ff.height),
                    )
                };
                let roof_quad = [
                    at(&far, &hf, 0.0),
                    at(&near, &hn, 0.0),
                    at(&near, &hn, 900.0),
                    at(&far, &hf, 900.0),
                ];
                fill_triangle(
                    &mut pixels,
                    [roof_quad[0], roof_quad[1], roof_quad[2]],
                    roof,
                );
                fill_triangle(
                    &mut pixels,
                    [roof_quad[0], roof_quad[2], roof_quad[3]],
                    roof,
                );
                // the front, whose picture runs once along each sample
                if let Some(tex) = textures.get(fn_.texture) {
                    let wall = |s: &Slice, h: &HillSlice, up: f32| point(s, s.x + h.edge[side], up);
                    let (u_far, u_near) = if side == 0 { (0.0, 1.0) } else { (1.0, 0.0) };
                    let p = [
                        wall(&far, &hf, ff.height),
                        wall(&near, &hn, fn_.height),
                        wall(&near, &hn, 0.0),
                        wall(&far, &hf, 0.0),
                    ];
                    let uv = [(u_far, 0.0), (u_near, 0.0), (u_near, 1.0), (u_far, 1.0)];
                    fill_textured(&mut pixels, [p[0], p[1], p[2]], [uv[0], uv[1], uv[2]], tex);
                    fill_textured(&mut pixels, [p[0], p[2], p[3]], [uv[0], uv[2], uv[3]], tex);
                }
            }
        }
    }

    // pass 3: the road, its edges and its lines
    for sp in &spans {
        let (far, near) = (slices[sp.k], before(sp.k));
        let stripe = (sp.k / 4) % 2 == 0;
        let (road, edge) = if stripe {
            ([110, 110, 115], [235, 235, 235])
        } else {
            ([100, 100, 105], [200, 40, 40])
        };
        for r in sp.first..sp.last {
            let t = (near.row - r as f32) / (near.row - far.row).max(0.001);
            let (l, rt) = (lerp(near.left, far.left, t), lerp(near.right, far.right, t));
            let edge_w = (3.0 * far.scale * FOCAL / 128.0).max(1.0);
            put_row(&mut pixels, r, l - edge_w, l, edge);
            put_row(&mut pixels, r, rt, rt + edge_w, edge);
            put_row(&mut pixels, r, l, rt, road);
            if stripe {
                // the lines between lanes: yellow in the middle, white elsewhere, in dashes
                let lanes = (far.lanes[0].floor() as i32, far.lanes[1].floor() as i32);
                for j in -(lanes.0 - 1).max(0)..=(lanes.1 - 1).max(0) {
                    let at = lerp(
                        near.centre + j as f32 * LANE * near.scale,
                        far.centre + j as f32 * LANE * far.scale,
                        t,
                    );
                    let w = (2.0 * far.scale).max(1.0);
                    let c = if j == 0 {
                        [230, 200, 40]
                    } else {
                        [230, 230, 230]
                    };
                    put_row(&mut pixels, r, at - w / 2.0, at + w / 2.0, c);
                }
            }
        }
    }
    Rgb {
        width: WIDTH as u32,
        height: HEIGHT as u32,
        pixels,
    }
}

/// Paint the part of a row of the picture from one column to another.
fn put_row(pixels: &mut [u8], row: usize, from: f32, to: f32, colour: [u8; 3]) {
    let (a, b) = (
        from.max(0.0) as usize,
        (to.min(WIDTH as f32) as usize).min(WIDTH),
    );
    for x in a..b.max(a) {
        let at = (row * WIDTH + x) * 3;
        pixels[at..at + 3].copy_from_slice(&colour);
    }
}

/// A slice of road as it shows in the picture.
#[derive(Clone, Copy)]
struct Slice {
    row: f32,
    left: f32,
    right: f32,
    centre: f32,
    scale: f32,
    /// How far the road's middle has gone sideways, in world units.
    x: f32,
    lanes: [f32; 2],
    hills: Option<HillSlice>,
}

/// Where a point at the world position `x` (sideways) and `height` over the road of `slice`
/// shows in the picture.
fn point(slice: &Slice, x: f32, height: f32) -> (f32, f32) {
    (
        WIDTH as f32 / 2.0 + x * slice.scale,
        slice.row - height * slice.scale,
    )
}

/// Fill a triangle with a picture, whose corners `uv` (0 to 1 across and down) go with the
/// corners of the triangle. Where the picture is see-through the picture behind shows.
fn fill_textured(pixels: &mut [u8], p: [(f32, f32); 3], uv: [(f32, f32); 3], tex: &Texture) {
    let clamp = |v: f32| v.clamp(-1.0e6, 1.0e6);
    let p = p.map(|(x, y)| (clamp(x), clamp(y)));
    let area = (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[2].0 - p[0].0) * (p[1].1 - p[0].1);
    if area.abs() < 1.0e-6 || tex.width == 0 || tex.height == 0 {
        return;
    }
    let (min_x, max_x) = (
        p.iter().map(|v| v.0).fold(f32::MAX, f32::min),
        p.iter().map(|v| v.0).fold(f32::MIN, f32::max),
    );
    let (min_y, max_y) = (
        p.iter().map(|v| v.1).fold(f32::MAX, f32::min),
        p.iter().map(|v| v.1).fold(f32::MIN, f32::max),
    );
    let (x0, x1) = (
        min_x.max(0.0).floor() as usize,
        (max_x.min(WIDTH as f32 - 1.0)).ceil().max(0.0) as usize,
    );
    let (y0, y1) = (
        min_y.max(0.0).floor() as usize,
        (max_y.min(HEIGHT as f32 - 1.0)).ceil().max(0.0) as usize,
    );
    for y in y0..=y1.min(HEIGHT - 1) {
        for x in x0..=x1.min(WIDTH - 1) {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let w1 = ((px - p[0].0) * (p[2].1 - p[0].1) - (p[2].0 - p[0].0) * (py - p[0].1)) / area;
            let w2 = ((p[1].0 - p[0].0) * (py - p[0].1) - (px - p[0].0) * (p[1].1 - p[0].1)) / area;
            let w0 = 1.0 - w1 - w2;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }
            let u = w0 * uv[0].0 + w1 * uv[1].0 + w2 * uv[2].0;
            let v = w0 * uv[0].1 + w1 * uv[1].1 + w2 * uv[2].1;
            let tx = ((u * tex.width as f32) as usize).min(tex.width - 1);
            let ty = ((v * tex.height as f32) as usize).min(tex.height - 1);
            let c = tex.pixels[ty * tex.width + tx];
            if c[3] != 0 {
                let at = (y * WIDTH + x) * 3;
                pixels[at..at + 3].copy_from_slice(&c[..3]);
            }
        }
    }
}

/// Fill a triangle with a colour (rows and columns outside the picture are left out).
fn fill_triangle(pixels: &mut [u8], p: [(f32, f32); 3], colour: [u8; 3]) {
    let clamp = |v: f32| v.clamp(-1.0e6, 1.0e6);
    let p = p.map(|(x, y)| (clamp(x), clamp(y)));
    let (min_y, max_y) = (
        p.iter().map(|v| v.1).fold(f32::MAX, f32::min),
        p.iter().map(|v| v.1).fold(f32::MIN, f32::max),
    );
    let (from, to) = (
        min_y.max(0.0).ceil() as usize,
        (max_y.min(HEIGHT as f32)).ceil() as usize,
    );
    for row in from..to.min(HEIGHT) {
        let y = row as f32;
        let (mut lo, mut hi) = (f32::MAX, f32::MIN);
        for i in 0..3 {
            let (a, b) = (p[i], p[(i + 1) % 3]);
            if (a.1 <= y && y < b.1) || (b.1 <= y && y < a.1) {
                let x = a.0 + (b.0 - a.0) * (y - a.1) / (b.1 - a.1);
                lo = lo.min(x);
                hi = hi.max(x);
            }
        }
        if lo > hi {
            continue;
        }
        let (a, b) = (
            lo.max(0.0).ceil() as usize,
            hi.min(WIDTH as f32).ceil() as usize,
        );
        for x in a..b.min(WIDTH) {
            let at = (row * WIDTH + x) * 3;
            pixels[at..at + 3].copy_from_slice(&colour);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn straight(n: usize) -> Vec<Sample> {
        vec![
            Sample {
                bend: 0,
                rise: 0,
                lanes: [2.0, 2.0]
            };
            n
        ]
    }
    fn pixel(p: &Rgb, x: usize, y: usize) -> [u8; 3] {
        let at = (y * WIDTH + x) * 3;
        [p.pixels[at], p.pixels[at + 1], p.pixels[at + 2]]
    }

    fn is_road(p: [u8; 3]) -> bool {
        (p[0] == 110 || p[0] == 100) && p[0].abs_diff(p[2]) < 10
    }

    #[test]
    fn a_straight_flat_road_is_grey_in_the_middle_with_grass_and_sky_around() {
        let picture = view(&straight(400), 0, Guess::default());
        assert_eq!((picture.width, picture.height), (320, 240));
        assert!(
            is_road(pixel(&picture, 160, 230)),
            "{:?}",
            pixel(&picture, 160, 230)
        );
        assert!(pixel(&picture, 160, 10)[2] > 200, "sky is blue");
        let grass = pixel(&picture, 3, 120);
        assert!(grass[1] > grass[0] + 40, "grass: {grass:?}");
        assert!(!is_road(grass));
    }

    #[test]
    fn a_bend_moves_the_far_road_to_its_side() {
        let first_road = |p: &Rgb| (0..WIDTH).find(|&x| is_road(pixel(p, x, 120)));
        let flat = view(&straight(400), 0, Guess::default());
        let mut bent = straight(400);
        for s in &mut bent {
            s.bend = 10;
        }
        let (flat, bent) = (
            first_road(&flat).unwrap(),
            first_road(&view(&bent, 0, Guess::default())).unwrap(),
        );
        assert!(
            bent > flat,
            "a positive bend goes right: {flat} then {bent}"
        );
    }

    #[test]
    fn a_climb_lifts_the_far_road_up_the_picture() {
        let top_of_road = |p: &Rgb| (0..HEIGHT).find(|&y| is_road(pixel(p, 160, y)));
        let flat = top_of_road(&view(&straight(400), 0, Guess::default())).unwrap();
        let mut hill = straight(400);
        for s in &mut hill {
            s.rise = 20;
        }
        let hill = top_of_road(&view(&hill, 0, Guess::default())).unwrap();
        assert!(
            hill < flat,
            "the road climbs above where the flat one ends: {hill} < {flat}"
        );
    }

    #[test]
    fn a_thing_further_ahead_is_smaller_and_nearer_the_horizon() {
        let road = straight(400);
        let near = project(&road, 0.0, 0.0, Guess::default(), 2.0, 0.0).unwrap();
        let far = project(&road, 0.0, 0.0, Guess::default(), 20.0, 0.0).unwrap();
        assert!(far.scale < near.scale && near.scale <= 0.5 + 1e-3);
        assert!(far.y < near.y && far.y > 90.0, "{far:?} {near:?}");
        assert!(
            (far.x - 160.0).abs() < 1.0,
            "straight ahead on a straight road"
        );
        let right = project(&road, 0.0, 0.0, Guess::default(), 5.0, 250.0).unwrap();
        assert!(right.x > 160.0);
    }

    #[test]
    fn a_thing_too_close_or_past_the_end_of_the_road_is_not_placed() {
        let road = straight(50);
        assert!(project(&road, 0.0, 0.0, Guess::default(), 0.1, 0.0).is_none());
        assert!(project(&road, 0.0, 0.0, Guess::default(), 100.0, 0.0).is_none());
    }

    #[test]
    fn a_picture_is_stretched_over_a_triangle_and_see_through_is_left_alone() {
        let tex = Texture {
            width: 2,
            height: 1,
            pixels: vec![[255, 0, 0, 255], [0, 0, 0, 0]],
        };
        let mut pixels = vec![9u8; WIDTH * HEIGHT * 3];
        // the whole of the left half of a square, as two triangles, left to right
        let quad = [(10.0, 10.0), (30.0, 10.0), (30.0, 30.0), (10.0, 30.0)];
        let uv = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        fill_textured(
            &mut pixels,
            [quad[0], quad[1], quad[2]],
            [uv[0], uv[1], uv[2]],
            &tex,
        );
        fill_textured(
            &mut pixels,
            [quad[0], quad[2], quad[3]],
            [uv[0], uv[2], uv[3]],
            &tex,
        );
        let at = |x: usize, y: usize| pixels[(y * WIDTH + x) * 3];
        assert_eq!(at(12, 20), 255, "the left half of the picture is red");
        assert_eq!(
            at(28, 20),
            9,
            "the right half is see-through: the picture behind stays"
        );
        assert_eq!(at(5, 20), 9, "outside the square nothing is drawn");
    }

    #[test]
    fn a_triangle_with_no_area_or_far_off_the_picture_does_not_panic() {
        let tex = Texture {
            width: 1,
            height: 1,
            pixels: vec![[1, 2, 3, 255]],
        };
        let mut pixels = vec![0u8; WIDTH * HEIGHT * 3];
        fill_textured(&mut pixels, [(5.0, 5.0); 3], [(0.0, 0.0); 3], &tex);
        fill_textured(
            &mut pixels,
            [(-1e9, -1e9), (1e9, -1e9), (0.0, 1e9)],
            [(0.0, 0.0), (1.0, 0.0), (0.5, 1.0)],
            &tex,
        );
        fill_triangle(
            &mut pixels,
            [(-1e9, 0.0), (1e9, 0.0), (0.0, 1e9)],
            [1, 2, 3],
        );
    }

    #[test]
    fn running_out_of_road_does_not_panic() {
        let _ = view(&straight(3), 2, Guess::default());
        let _ = view(&[], 0, Guess::default());
        let _ = view(&straight(5), 99, Guess::default());
    }
}
