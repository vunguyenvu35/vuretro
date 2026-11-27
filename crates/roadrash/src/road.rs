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

/// The view from a place between samples, `lateral` world units to the right of the middle.
pub fn view_at(samples: &[Sample], position: f32, lateral: f32, guess: Guess) -> Rgb {
    let first = position.max(0.0).floor() as usize;
    let fraction = position.max(0.0) - first as f32;
    let mut pixels = vec![0u8; WIDTH * HEIGHT * 3];
    let mut put_row = |row: usize, from: f32, to: f32, colour: [u8; 3]| {
        let (a, b) = (
            from.max(0.0) as usize,
            (to.min(WIDTH as f32) as usize).min(WIDTH),
        );
        for x in a..b.max(a) {
            let at = (row * WIDTH + x) * 3;
            pixels[at..at + 3].copy_from_slice(&colour);
        }
    };
    // the sky, then the ground under the horizon
    for row in 0..HEIGHT {
        let t = row as f32 / HORIZON;
        let colour = if (row as f32) < HORIZON {
            [(90.0 + 90.0 * t) as u8, (140.0 + 60.0 * t) as u8, 235]
        } else {
            [40, 120, 40]
        };
        put_row(row, 0.0, WIDTH as f32, colour);
    }

    let (mut x, mut dx, mut y) = (-lateral, 0.0f32, 0.0f32);
    // the slice before the first one is far below the picture
    let mut near = (HORIZON + CAMERA_HEIGHT * FOCAL / 64.0, 0.0f32, 0.0f32);
    let mut top = HEIGHT as f32;
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
        let (left, right) = (
            WIDTH as f32 / 2.0 + (x - s.lanes[0] * LANE) * scale,
            WIDTH as f32 / 2.0 + (x + s.lanes[1] * LANE) * scale,
        );
        let far = (row, left, right);
        if row < near.0 && row < top {
            let stripe = (k / 4) % 2 == 0;
            let (road, grass, edge) = if stripe {
                ([110, 110, 115], [45, 130, 45], [235, 235, 235])
            } else {
                ([100, 100, 105], [35, 110, 35], [200, 40, 40])
            };
            let first = row.max(0.0).ceil() as usize;
            let last = (near.0.min(top).min(HEIGHT as f32)).floor() as usize;
            for r in first..last {
                let t = (near.0 - r as f32) / (near.0 - row).max(0.001);
                let (l, rt) = (lerp(near.1, far.1, t), lerp(near.2, far.2, t));
                let edge_w = (3.0 * scale * FOCAL / 128.0).max(1.0);
                put_row(r, 0.0, WIDTH as f32, grass);
                put_row(r, l - edge_w, l, edge);
                put_row(r, rt, rt + edge_w, edge);
                put_row(r, l, rt, road);
            }
            top = top.min(row);
        }
        near = far;
    }
    Rgb {
        width: WIDTH as u32,
        height: HEIGHT as u32,
        pixels,
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
    fn running_out_of_road_does_not_panic() {
        let _ = view(&straight(3), 2, Guess::default());
        let _ = view(&[], 0, Guess::default());
        let _ = view(&straight(5), 99, Guess::default());
    }
}
