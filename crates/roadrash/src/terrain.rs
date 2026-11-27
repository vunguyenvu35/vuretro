//! The ground at the side of the road. Most of every course is hills: at each sample three
//! points on each side give the shape of the ground out from the road, and the faces between
//! them are lit according to how steep they are.
//!
//! Every point lies between limits the section gives ([`crate::course::HillPoint`]) and drifts
//! between them: for a stretch of a few samples it moves a steady amount a sample, then it
//! chooses a new stretch. The choices are random in the game; here they come from a generator of
//! the same kind, started anew for every section, so a course always looks the same.
//!
//! The brightness of a face is a band from 0 (dark) to 31, worked out from the horizontal and
//! the vertical distance it covers (`shade`). Its colour is the average colour of the picture
//! that the family of the section holds for that face (category 1 of the family: a group of
//! pictures for the face next to the road, the middle one and the outer one), made brighter or
//! darker by the band, and a plain green when the family has none.

use crate::course::{Course, HillRecord, Section};
use crate::disc::Disc;
use crate::family::Family;
use crate::palette::Palette;
use crate::rsc;
use std::collections::HashMap;

/// How far beyond the lanes the ground begins: the shoulder (224 on most of the road).
const SHOULDER: f32 = 224.0;
/// How far out, past the last point of a hill, the ground is drawn level.
pub const FAR: f32 = 6000.0;

/// The ground at one sample: for each side (0 left, 1 right) three points, how far out from the
/// edge of the shoulder and how high, and the colour of the face that ends at each.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HillSlice {
    pub out: [[f32; 3]; 2],
    pub height: [[f32; 3]; 2],
    pub colour: [[[u8; 3]; 3]; 2],
    /// Where the shoulder ends on each side, in world units from the middle of the road.
    pub edge: [f32; 2],
}

struct Random(u32);

impl Random {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(214013).wrapping_add(2531011);
        (self.0 >> 16) & 0x7fff
    }
}

#[derive(Clone, Copy, Default)]
struct Point {
    x: i32,
    h: i32,
    dx: i32,
    dh: i32,
    end: i32,
}

/// The brightness band of a face that goes `dx` sideways and `dy` up (both as they come).
pub fn shade(dx: f32, dy: f32) -> u8 {
    let (dx, dy) = (dx.abs() as i64, dy.abs() as i64);
    let denominator = dx.min(dy) / 2 - dy - dx;
    if denominator == 0 {
        return 31;
    }
    let shade = ((dy << 16) / denominator + 65536).max(0);
    ((shade * 32) / 65536).min(31) as u8
}

/// The shape of the hills at each of the samples `0..length` of a section.
pub fn profiles(section: &Section, length: usize, seed: u32) -> Vec<Option<[[(f32, f32); 3]; 2]>> {
    let records = &section.hills;
    if records.is_empty() {
        return vec![None; length];
    }
    let mut random = Random(seed);
    let mut record = 0usize;
    let mut points = [Point::default(); 8];
    for (i, p) in points.iter_mut().enumerate() {
        let l = records[0].points[i];
        p.x = (random.next() % ((l.max_x as u32 - l.min_x as u32) * 32 + 1)) as i32
            + l.min_x as i32 * 32;
        p.h = l.min_height as i32 * 16
            + (random.next() % ((l.max_height as i32 - l.min_height as i32) as u32 * 16 + 1))
                as i32;
    }
    let mut out = Vec::with_capacity(length);
    for j in 0..length as i32 {
        if j > 0 {
            if record + 1 < records.len() && records[record + 1].start <= j {
                record += 1;
            }
            let rec: &HillRecord = &records[record];
            for (i, p) in points.iter_mut().enumerate() {
                let l = rec.points[i];
                if p.end <= j {
                    let mut len = l.min_length as i32
                        + (random.next() % (l.max_length as u32 - l.min_length as u32 + 1)) as i32;
                    if j + len > rec.end {
                        len = (rec.end - j).max(1);
                    }
                    p.end = j + len;
                    let slope = (random.next() % (l.x_variation as u32 * 16 + 1)) as i32
                        - l.x_variation as i32 * 8;
                    let target = p.x + slope * len;
                    p.dx = if target < l.min_x as i32 * 32 {
                        (l.min_x as i32 * 32 - p.x) / len
                    } else if target > l.max_x as i32 * 32 {
                        (l.max_x as i32 * 32 - p.x) / len
                    } else {
                        slope
                    };
                    let slope = (random.next() % (l.height_variation as u32 * 16 + 1)) as i32
                        - l.height_variation as i32 * 8;
                    let target = p.h + slope * len;
                    p.dh = if target < l.min_height as i32 * 16 {
                        (l.min_height as i32 * 16 - p.h) / len
                    } else if target > l.max_height as i32 * 16 {
                        (l.max_height as i32 * 16 - p.h) / len
                    } else {
                        slope
                    };
                }
                p.x += p.dx;
                p.h += p.dh;
            }
        }
        let mut slice = [[(0.0, 0.0); 3]; 2];
        for (side, s) in slice.iter_mut().enumerate() {
            for (i, v) in s.iter_mut().enumerate() {
                let p = points[side * 4 + i];
                *v = (p.x as f32, p.h as f32);
            }
        }
        out.push(Some(slice));
    }
    out
}

/// The colour a picture of the family stands for: the average of its colours that show.
fn average(indices: &[u8], palette: &Palette) -> Option<[u8; 3]> {
    let (mut sum, mut n) = ([0u64; 3], 0u64);
    for &i in indices.iter().filter(|&&i| i != 0) {
        for (s, c) in sum.iter_mut().zip(palette.0[i as usize]) {
            *s += c as u64;
        }
        n += 1;
    }
    (n > 0).then(|| [(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8])
}

fn fallback(band: u8) -> [u8; 3] {
    let b = band as u32;
    [(25 + b * 2) as u8, (80 + b * 3) as u8, (25 + b) as u8]
}

/// The ground along a route and how its colours were found.
pub struct Ground {
    /// A slice for each sample of the route (none where the terrain is not hills).
    pub slices: Vec<Option<HillSlice>>,
    /// How many of the kinds of face got their colour from a picture of the disc, and how many
    /// had to be plain green.
    pub from_pictures: usize,
    pub plain: usize,
}

/// A face darker (band 0) or brighter (band 31).
fn lit(colour: [u8; 3], band: u8) -> [u8; 3] {
    let factor = 0.55 + 0.45 * band as f32 / 31.0;
    colour.map(|c| (c as f32 * factor).min(255.0) as u8)
}

/// The ground along the route of a course.
pub fn load(disc: &Disc, course: &Course, palette: &Palette) -> Result<Ground, String> {
    let file = disc.read("DATA/FAMILIES.RSC")?;
    let entries = rsc::entries(&file)?;
    let mut families: HashMap<u16, Option<Family>> = HashMap::new();
    let mut colours: HashMap<(u16, u8, usize), Option<[u8; 3]>> = HashMap::new();
    let mut slices = Vec::new();
    for (index, (section_no, length)) in course.route().into_iter().enumerate() {
        let section = &course.sections[section_no];
        let length = length.min(section.lanes.len());
        let shapes = profiles(section, length, 1 + section_no as u32 * 7919 + index as u32);
        let mut slots = [0u16; 4];
        let mut event = 0;
        for (j, shape) in shapes.iter().enumerate() {
            let cursor = (j as i32 + 33 + section.family_bias).max(0);
            while event < section.family_events.len()
                && section.family_events[event].sample <= cursor
            {
                let e = section.family_events[event];
                if e.kind == 1 || e.kind == 3 {
                    slots[e.slot as usize] = e.family;
                }
                event += 1;
            }
            let Some(shape) = shape else {
                slices.push(None);
                continue;
            };
            let record = section
                .hills
                .iter()
                .rev()
                .find(|r| r.start <= j as i32)
                .unwrap_or(&section.hills[0]);
            let lanes = section.lanes[j];
            let mut slice = HillSlice {
                out: [[0.0; 3]; 2],
                height: [[0.0; 3]; 2],
                colour: [[[0; 3]; 3]; 2],
                edge: [-(lanes[0] * 250.0) - SHOULDER, lanes[1] * 250.0 + SHOULDER],
            };
            for side in 0..2 {
                let selector = record.selectors[side];
                let (mut prev_out, mut prev_h) = (0.0f32, 0.0f32);
                for face in 0..3 {
                    let (px, ph) = shape[side][face];
                    slice.out[side][face] = px;
                    slice.height[side][face] = ph;
                    let band = shade(px - prev_out, ph - prev_h);
                    (prev_out, prev_h) = (px, ph);
                    let family_id = slots[(selector >> 6) as usize];
                    let base = *colours
                        .entry((family_id, selector & 63, face))
                        .or_insert_with(|| {
                            let family = families.entry(family_id).or_insert_with(|| {
                                let e = entries
                                    .iter()
                                    .find(|e| e.tag == "FAM" && e.id == family_id as u32)?;
                                Family::parse(e.bytes(&file)).ok()
                            });
                            family
                                .as_ref()
                                .and_then(|f| {
                                    f.cel_texture(1, (selector & 63) as usize, Some(face))
                                })
                                .and_then(|t| average(&t.pixels, palette))
                        });
                    slice.colour[side][face] = lit(base.unwrap_or_else(|| fallback(31)), band);
                }
            }
            slices.push(Some(slice));
        }
    }
    let from_pictures = colours.values().filter(|c| c.is_some()).count();
    let plain = colours.len() - from_pictures;
    Ok(Ground {
        slices,
        from_pictures,
        plain,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::HillPoint;

    fn point(min_x: u8, max_x: u8, min_h: i8, max_h: i8) -> HillPoint {
        HillPoint {
            min_x,
            max_x,
            x_variation: 4,
            min_height: min_h,
            max_height: max_h,
            height_variation: 4,
            min_length: 2,
            max_length: 5,
        }
    }
    fn section(samples: usize) -> Section {
        Section {
            start_elevation: 0,
            end_elevation: 0,
            bend: vec![0; samples],
            rise: vec![0; samples],
            lanes: vec![[2.0, 2.0]; samples],
            roadside: vec![],
            family_events: vec![],
            family_bias: 0,
            hills: vec![HillRecord {
                start: 0,
                end: samples as i32,
                selectors: [2, 2],
                points: [point(10, 20, -3, 8); 8],
            }],
        }
    }

    #[test]
    fn a_flat_face_is_bright_and_a_steep_one_dark() {
        assert_eq!(shade(100.0, 0.0), 31);
        assert!(shade(100.0, 100.0) < 31);
        assert!(
            shade(10.0, 400.0) < shade(100.0, 100.0),
            "steeper is darker"
        );
        assert_eq!(shade(0.0, 0.0), 31, "a face of no size is not dark");
        assert!(shade(-100.0, -50.0) <= 31);
    }

    #[test]
    fn the_points_stay_between_their_limits_and_the_hills_are_always_the_same() {
        let s = section(300);
        let a = profiles(&s, 300, 5);
        assert_eq!(a, profiles(&s, 300, 5), "the same every time");
        assert_ne!(a, profiles(&s, 300, 6), "another seed, other hills");
        for slice in a.iter().flatten() {
            for side in slice {
                for &(x, h) in side {
                    assert!(
                        (320.0..=640.0).contains(&x),
                        "x {x} is between 10 and 20 times 32"
                    );
                    assert!(
                        (-48.0..=128.0).contains(&h),
                        "height {h} is between -3 and 8 times 16"
                    );
                }
            }
        }
        let moved = a.windows(2).filter(|w| w[0] != w[1]).count();
        assert!(moved > 100, "the hills change along the road: {moved}");
    }

    #[test]
    fn a_section_without_hills_has_none() {
        let mut s = section(10);
        s.hills.clear();
        assert!(profiles(&s, 10, 1).iter().all(Option::is_none));
    }

    #[test]
    fn a_brighter_band_makes_a_brighter_colour() {
        let c = [100, 200, 50];
        assert!(lit(c, 31)[1] > lit(c, 0)[1]);
        assert_eq!(lit(c, 31), c, "the brightest band keeps the colour");
        assert!(lit([250, 250, 250], 31)[0] >= 250);
    }

    #[test]
    fn the_colour_of_a_picture_is_the_average_of_what_shows() {
        let mut colours = [[0u8; 3]; 256];
        colours[1] = [100, 0, 0];
        colours[2] = [200, 100, 0];
        let palette = Palette(colours);
        assert_eq!(average(&[0, 1, 2, 0], &palette), Some([150, 50, 0]));
        assert_eq!(average(&[0, 0], &palette), None);
        assert!(
            fallback(31)[1] > fallback(0)[1],
            "brighter bands are brighter"
        );
    }
}
