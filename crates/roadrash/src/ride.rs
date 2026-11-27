//! A bike on the road: how it speeds up, steers and is thrown wide by a bend, and how it is
//! drawn. The numbers are mine, chosen so the ride feels sensible; they are not the game's.

use crate::bob::Frame;
use crate::course::Sample;
use crate::image::Rgb;
use crate::palette::Palette;
use crate::road::{self, Guess};

/// Samples per second at the top speed (a sample is 256 world units).
pub const TOP_SPEED: f32 = 80.0;
const LANE: f32 = 250.0;
/// How far past the edge of the road the bike can go before it is on the grass.
const SHOULDER: f32 = 120.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bike {
    /// Where on the route, in samples.
    pub position: f32,
    /// Samples per second.
    pub speed: f32,
    /// World units to the right of the middle of the road.
    pub lateral: f32,
    /// -1 (leaning left) to 1 (leaning right).
    pub lean: f32,
}

impl Bike {
    pub fn start() -> Bike {
        Bike {
            position: 0.0,
            speed: 0.0,
            lateral: 0.0,
            lean: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Input {
    pub throttle: bool,
    pub brake: bool,
    /// -1 left, 0 straight, 1 right.
    pub steer: f32,
}

/// Move the bike on for `dt` seconds. At the end of the route it starts again from the front.
pub fn step(bike: &mut Bike, input: Input, samples: &[Sample], dt: f32) {
    if samples.is_empty() {
        return;
    }
    let here = samples[(bike.position.max(0.0) as usize).min(samples.len() - 1)];
    let (left, right) = (
        here.lanes[0] * LANE + SHOULDER,
        here.lanes[1] * LANE + SHOULDER,
    );
    let on_road = bike.lateral > -left && bike.lateral < right;

    let mut speed = bike.speed;
    if input.throttle {
        speed += 25.0 * dt;
    }
    if input.brake {
        speed -= 60.0 * dt;
    }
    speed -= 4.0 * dt; // the air and the road
    let limit = if on_road { TOP_SPEED } else { 30.0 };
    speed = speed.clamp(0.0, TOP_SPEED);
    if speed > limit {
        speed = (speed - 60.0 * dt).max(limit); // the grass slows the bike down
    }
    bike.speed = speed;

    let fraction = speed / TOP_SPEED;
    // steering moves it sideways; a bend to the right throws it to the left, and the other way
    let steer = input.steer.clamp(-1.0, 1.0);
    bike.lateral += (steer * 700.0 - here.bend as f32 * 20.0) * fraction * dt;
    bike.lateral = bike.lateral.clamp(-left - 500.0, right + 500.0);
    bike.lean += (steer - bike.lean) * (8.0 * dt).min(1.0);

    bike.position += speed * dt;
    if bike.position >= samples.len() as f32 - 130.0 {
        bike.position = 0.0;
    }
}

/// The lean of each frame of the rider set (`HIBOB.DAT`) that is used, measured as how far the
/// head is to the right of the rear wheel (as a part of the width), and the frame's number.
/// Frame 3 is the bike upright; the others are spread to either side.
pub const LEAN_FRAMES: [(f32, usize); 11] = [
    (-0.437, 0),
    (-0.380, 1),
    (-0.307, 5),
    (-0.236, 6),
    (-0.079, 17),
    (-0.030, 3),
    (0.097, 7),
    (0.150, 13),
    (0.269, 8),
    (0.424, 9),
    (0.515, 15),
];

/// The frame to draw for a lean between -1 and 1.
pub fn lean_frame(lean: f32) -> usize {
    let target = lean.clamp(-1.0, 1.0) * 0.5;
    LEAN_FRAMES
        .iter()
        .min_by(|a, b| (a.0 - target).abs().total_cmp(&(b.0 - target).abs()))
        .map_or(3, |f| f.1)
}

/// Where the wheel touches the road in a sprite: the middle of its lowest rows.
fn anchor(frame: &Frame) -> usize {
    let rows = (frame.height as usize / 4).max(1);
    let (mut sum, mut count) = (0usize, 0usize);
    for y in frame.height as usize - rows..frame.height as usize {
        for x in 0..frame.width as usize {
            if frame.indices[y * frame.width as usize + x] != 0 {
                sum += x;
                count += 1;
            }
        }
    }
    sum.checked_div(count).unwrap_or(frame.width as usize / 2)
}

/// A picture twice as big, each pixel made into four.
fn double(picture: &Rgb) -> Rgb {
    let (w, h) = (picture.width as usize, picture.height as usize);
    let mut pixels = vec![0u8; w * h * 4 * 3];
    for y in 0..h * 2 {
        for x in 0..w * 2 {
            let from = ((y / 2) * w + x / 2) * 3;
            let to = (y * w * 2 + x) * 3;
            pixels[to..to + 3].copy_from_slice(&picture.pixels[from..from + 3]);
        }
    }
    Rgb {
        width: picture.width * 2,
        height: picture.height * 2,
        pixels,
    }
}

/// The picture of the ride, 640x480: the road and the rider on it.
pub fn render(samples: &[Sample], bike: &Bike, riders: &[Frame], palette: &Palette) -> Rgb {
    let mut picture = double(&road::view_at(
        samples,
        bike.position,
        bike.lateral,
        Guess::default(),
    ));
    if let Some(sprite) = riders.get(lean_frame(bike.lean)) {
        let (width, height) = (picture.width as i64, picture.height as i64);
        let ax = anchor(sprite) as i64;
        let (left, top) = (width / 2 - ax, height - 8 - sprite.height as i64);
        for y in 0..sprite.height as i64 {
            for x in 0..sprite.width as i64 {
                let index = sprite.indices[(y * sprite.width as i64 + x) as usize];
                let (px, py) = (left + x, top + y);
                if index != 0 && px >= 0 && py >= 0 && px < width && py < height {
                    let at = ((py * width + px) * 3) as usize;
                    picture.pixels[at..at + 3].copy_from_slice(&palette.0[index as usize]);
                }
            }
        }
    }
    picture
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

    #[test]
    fn the_throttle_speeds_up_and_the_brake_slows_down() {
        let road = straight(2000);
        let mut bike = Bike::start();
        for _ in 0..60 {
            step(
                &mut bike,
                Input {
                    throttle: true,
                    ..Input::default()
                },
                &road,
                1.0 / 30.0,
            );
        }
        let fast = bike.speed;
        assert!(fast > 20.0 && bike.position > 10.0, "{bike:?}");
        for _ in 0..60 {
            step(
                &mut bike,
                Input {
                    brake: true,
                    ..Input::default()
                },
                &road,
                1.0 / 30.0,
            );
        }
        assert!(bike.speed < fast / 2.0);
    }

    #[test]
    fn steering_moves_the_bike_sideways_and_leans_it() {
        let road = straight(2000);
        let mut bike = Bike {
            speed: 60.0,
            ..Bike::start()
        };
        for _ in 0..10 {
            step(
                &mut bike,
                Input {
                    steer: 1.0,
                    ..Input::default()
                },
                &road,
                1.0 / 30.0,
            );
        }
        assert!(bike.lateral > 100.0 && bike.lean > 0.5, "{bike:?}");
    }

    #[test]
    fn a_bend_to_the_right_throws_the_bike_to_the_left() {
        let mut road = straight(2000);
        for s in &mut road {
            s.bend = 20;
        }
        let mut bike = Bike {
            speed: 70.0,
            ..Bike::start()
        };
        for _ in 0..10 {
            step(&mut bike, Input::default(), &road, 1.0 / 30.0);
        }
        assert!(bike.lateral < -50.0, "{bike:?}");
    }

    #[test]
    fn the_grass_slows_the_bike_down() {
        let road = straight(2000);
        let mut bike = Bike {
            speed: 80.0,
            lateral: 900.0,
            ..Bike::start()
        };
        for _ in 0..60 {
            step(
                &mut bike,
                Input {
                    throttle: true,
                    ..Input::default()
                },
                &road,
                1.0 / 30.0,
            );
        }
        assert!(bike.speed <= 30.5, "{bike:?}");
    }

    #[test]
    fn the_route_starts_again_at_its_end() {
        let road = straight(500);
        let mut bike = Bike {
            speed: 80.0,
            position: 369.0,
            ..Bike::start()
        };
        step(&mut bike, Input::default(), &road, 0.5);
        assert!(bike.position < 10.0, "{bike:?}");
    }

    #[test]
    fn the_frame_follows_the_lean() {
        assert_eq!(lean_frame(0.0), 3, "upright");
        assert_ne!(lean_frame(-1.0), lean_frame(1.0));
        assert_eq!(
            lean_frame(-9.0),
            lean_frame(-1.0),
            "out of range is kept in range"
        );
    }

    #[test]
    fn the_picture_is_640_by_480_and_the_rider_is_on_it() {
        let road = straight(400);
        let palette = Palette([[200, 0, 0]; 256]);
        let rider = Frame {
            width: 4,
            height: 8,
            indices: vec![1; 32],
        };
        let with = render(&road, &Bike::start(), &vec![rider; 20], &palette);
        assert_eq!((with.width, with.height), (640, 480));
        let at = ((480 - 8 - 2) * 640 + 320) * 3;
        assert_eq!(&with.pixels[at..at + 3], &[200, 0, 0]);
        let without = render(&road, &Bike::start(), &[], &palette);
        assert_ne!(with.pixels, without.pixels);
    }
}
