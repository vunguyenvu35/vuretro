//! A bike on the road: how it speeds up, steers and is thrown wide by a bend, and how it is
//! drawn. The numbers are mine, chosen so the ride feels sensible; they are not the game's.

use crate::bob::Frame;
use crate::course::Sample;
use crate::exe::Remap;
use crate::hud;
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

/// Another rider on the road, going its own way at its own speed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rival {
    pub position: f32,
    pub lateral: f32,
    pub speed: f32,
    /// Which of the rider colour tables it wears.
    pub colour: usize,
}

/// Move the rivals on. They start again from the front at the end of the route, like the bike.
pub fn step_rivals(rivals: &mut [Rival], samples: &[Sample], dt: f32) {
    for r in rivals {
        r.position += r.speed * dt;
        if r.position >= samples.len() as f32 - 130.0 {
            r.position = 0.0;
        }
    }
}

/// Run the bike into a rival if they are on top of each other: the bike is slowed to a little
/// under the rival's speed and pushed aside. True when it happened.
pub fn collide(bike: &mut Bike, rivals: &[Rival]) -> bool {
    let mut hit = false;
    for r in rivals {
        let ahead = r.position - bike.position;
        let side = bike.lateral - r.lateral;
        if (-0.4..0.7).contains(&ahead) && side.abs() < 110.0 {
            bike.speed = bike.speed.min(r.speed * 0.7);
            bike.lateral += if side >= 0.0 { 40.0 } else { -40.0 };
            hit = true;
        }
    }
    hit
}

/// What is needed to draw riders: their sprites, the palette and the colour tables.
pub struct Look<'a> {
    pub riders: &'a [Frame],
    pub palette: &'a Palette,
    pub remaps: &'a [Remap],
    /// The instrument panel, when there is one to draw.
    pub dash: Option<&'a Frame>,
}

/// Draw a sprite so that the wheel touches the picture at (`x`, `bottom`), `scale` times its
/// size (nearest pixel), coloured by `remap`.
fn draw_sprite(
    picture: &mut Rgb,
    sprite: &Frame,
    look: &Look,
    remap: &Remap,
    x: f32,
    bottom: f32,
    scale: f32,
) {
    let (width, height) = (picture.width as i64, picture.height as i64);
    let (dw, dh) = (
        (sprite.width as f32 * scale).ceil() as i64,
        (sprite.height as f32 * scale).ceil() as i64,
    );
    let left = (x - anchor(sprite) as f32 * scale) as i64;
    let top = (bottom - sprite.height as f32 * scale) as i64;
    for dy in 0..dh {
        for dx in 0..dw {
            let (sx, sy) = ((dx as f32 / scale) as usize, (dy as f32 / scale) as usize);
            if sx >= sprite.width as usize || sy >= sprite.height as usize {
                continue;
            }
            let index = sprite.indices[sy * sprite.width as usize + sx];
            let (px, py) = (left + dx, top + dy);
            if index != 0 && px >= 0 && py >= 0 && px < width && py < height {
                let at = ((py * width + px) * 3) as usize;
                picture.pixels[at..at + 3]
                    .copy_from_slice(&look.palette.0[remap[index as usize] as usize]);
            }
        }
    }
}

/// The picture of the ride, 640x480: the road, the rivals on it (the far ones first) and the
/// bike. `colour` is the bike's own rider colour.
pub fn render(
    samples: &[Sample],
    bike: &Bike,
    rivals: &[Rival],
    look: &Look,
    colour: usize,
) -> Rgb {
    let guess = Guess::default();
    let mut picture = double(&road::view_at(samples, bike.position, bike.lateral, guess));
    let remap_of = |c: usize| look.remaps.get(c).or(look.remaps.first());

    // the bike itself is one sample ahead of the camera
    let mut seen: Vec<(f32, &Rival)> = rivals
        .iter()
        .map(|r| (r.position - bike.position + 1.0, r))
        .filter(|(ahead, _)| *ahead > 0.3)
        .collect();
    seen.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (ahead, r) in seen {
        let Some(spot) = road::project(
            samples,
            bike.position,
            bike.lateral,
            guess,
            ahead,
            r.lateral,
        ) else {
            continue;
        };
        let bend = samples
            .get(r.position.max(0.0) as usize)
            .map_or(0.0, |s| s.bend as f32);
        let frame = lean_frame((-bend * 0.04).clamp(-1.0, 1.0));
        if let (Some(sprite), Some(remap)) = (look.riders.get(frame), remap_of(r.colour)) {
            draw_sprite(
                &mut picture,
                sprite,
                look,
                remap,
                spot.x * 2.0,
                spot.y * 2.0,
                spot.scale,
            );
        }
    }

    if let Some(dash) = look.dash {
        // the speed is shown in km/h: the top speed stands for 280
        let kmh = bike.speed / TOP_SPEED * 280.0;
        let place = 1 + rivals.iter().filter(|r| r.position > bike.position).count();
        hud::draw(
            &mut picture,
            dash,
            look.palette,
            kmh,
            (place, rivals.len() + 1),
        );
    }
    if let (Some(sprite), Some(remap)) = (look.riders.get(lean_frame(bike.lean)), remap_of(colour))
    {
        let (w, h) = (picture.width as f32, picture.height as f32);
        draw_sprite(&mut picture, sprite, look, remap, w / 2.0, h - 8.0, 1.0);
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
        let same: Remap = std::array::from_fn(|i| i as u8);
        let riders = vec![rider; 20];
        let look = Look {
            riders: &riders,
            palette: &palette,
            remaps: &[same],
            dash: None,
        };
        let with = render(&road, &Bike::start(), &[], &look, 0);
        assert_eq!((with.width, with.height), (640, 480));
        let at = ((480 - 8 - 2) * 640 + 320) * 3;
        assert_eq!(&with.pixels[at..at + 3], &[200, 0, 0]);
        let nobody = Look {
            riders: &[],
            palette: &palette,
            remaps: &[same],
            dash: None,
        };
        let without = render(&road, &Bike::start(), &[], &nobody, 0);
        assert_ne!(with.pixels, without.pixels);
    }

    #[test]
    fn rivals_move_on_and_start_again_at_the_end() {
        let road = straight(500);
        let mut rivals = [Rival {
            position: 100.0,
            lateral: 0.0,
            speed: 40.0,
            colour: 1,
        }];
        step_rivals(&mut rivals, &road, 0.5);
        assert_eq!(rivals[0].position, 120.0);
        rivals[0].position = 369.0;
        step_rivals(&mut rivals, &road, 0.5);
        assert!(rivals[0].position < 10.0);
    }

    #[test]
    fn running_into_a_rival_slows_and_pushes_the_bike() {
        let rival = Rival {
            position: 10.2,
            lateral: 20.0,
            speed: 40.0,
            colour: 0,
        };
        let mut bike = Bike {
            position: 10.0,
            speed: 70.0,
            lateral: 0.0,
            lean: 0.0,
        };
        assert!(collide(&mut bike, &[rival]));
        assert!(bike.speed <= 28.0 && bike.lateral < 0.0, "{bike:?}");
        let mut clear = Bike {
            position: 10.0,
            speed: 70.0,
            lateral: 400.0,
            lean: 0.0,
        };
        assert!(!collide(&mut clear, &[rival]), "in another lane");
        let mut behind = Bike {
            position: 20.0,
            speed: 70.0,
            lateral: 0.0,
            lean: 0.0,
        };
        assert!(!collide(&mut behind, &[rival]), "the rival is behind");
    }

    #[test]
    fn a_rival_ahead_is_drawn_smaller_than_the_bike() {
        let road = straight(400);
        let palette = Palette([[200, 0, 0]; 256]);
        let rider = Frame {
            width: 20,
            height: 40,
            indices: vec![1; 800],
        };
        let same: Remap = std::array::from_fn(|i| i as u8);
        let riders = vec![rider; 20];
        let look = Look {
            riders: &riders,
            palette: &palette,
            remaps: &[same],
            dash: None,
        };
        let red = |p: &Rgb| {
            p.pixels
                .chunks_exact(3)
                .filter(|c| c == &[200, 0, 0])
                .count()
        };
        let alone = red(&render(&road, &Bike::start(), &[], &look, 0));
        let rival = Rival {
            position: 8.0,
            lateral: 0.0,
            speed: 30.0,
            colour: 0,
        };
        let with = red(&render(&road, &Bike::start(), &[rival], &look, 0));
        assert!(with > alone, "the rival adds red pixels");
        assert!(
            with - alone < alone / 4,
            "and few of them, it is far away: {alone} {with}"
        );
    }
}
