//! The things at the side of the road: trees, signs, lamp posts, houses...
//!
//! Each stretch of roadside is a word of the section's table (`BORR`), read as follows (all
//! bits as numbered from 0 at the low end):
//!
//! - bits 24-27 how long the stretch lasts, in groups of (period + 1) samples, and bits 12-15
//!   the period (both only when bits 24-27 are not 0; a period is 0 otherwise);
//! - bits 16-23 the selector of the thing: the low six bits say which of the family's things
//!   and the high two which of the four family slots it is taken from (63 means nothing);
//! - bit 28 the side (set: left), flipped at every second sample when bit 31 is set;
//! - bits 8-10 how many things in a row (one more than the number), bit 11 whether the gaps are
//!   random, bits 4-7 the size, bit 30 whether the picture is mirrored;
//! - the gap from the edge of the road and between the things from bits 0-3 (and, when the
//!   stretch has no duration, bits 12-15), in units that bit 29 changes.
//!
//! Which family a slot stands for comes from the section's family events (`MSRR`), applied a
//! little before the camera needs them. The random numbers are those of the game's
//! generator (the one of the Microsoft C library), started from 1.

use crate::course::{Course, Section};
use crate::disc::Disc;
use crate::family::{Family, Sprite};
use crate::rsc;
use std::collections::{BTreeSet, HashMap};

/// Slices ahead of the camera at which a family must be there.
const WINDOW: i32 = 33;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prop {
    /// Where on the route (a sample of [`Course::route_samples`]).
    pub sample: usize,
    /// Sideways from the middle of the road, in world units.
    pub lateral: f32,
    pub family: u16,
    pub selector: u8,
    pub size: u8,
    pub mirror: bool,
}

struct Random(u32);

impl Random {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(214013).wrapping_add(2531011);
        (self.0 >> 16) & 0x7fff
    }
}

fn period(word: u32) -> i32 {
    if word & 0x0f00_0000 != 0 {
        ((word >> 12) & 15) as i32
    } else {
        0
    }
}

fn duration(word: u32) -> i32 {
    (period(word) + 1) * ((word >> 24) & 15) as i32
}

/// The things of one section, for the samples `0..length`, put at `base + sample`.
fn roadside(
    section: &Section,
    length: usize,
    base: usize,
    random: &mut Random,
    out: &mut Vec<Prop>,
) {
    let words = &section.roadside;
    if words.is_empty() {
        return;
    }
    let (mut record, mut start) = (0usize, 0i32);
    let mut end = duration(words[0]);
    let mut slots = [0u16; 4];
    let mut event = 0;
    for j in 0..length {
        if j > 0 && j as i64 > end as i64 {
            if record + 1 >= words.len() {
                break;
            }
            record += 1;
            start = j as i32;
            end = start + duration(words[record]);
        }
        // the families of the slots, as they stand a little ahead of this sample
        let cursor = (j as i32 + WINDOW + section.family_bias).max(0);
        while event < section.family_events.len() && section.family_events[event].sample <= cursor {
            let e = section.family_events[event];
            if e.kind == 1 || e.kind == 3 {
                slots[e.slot as usize] = e.family;
            }
            event += 1;
        }
        let word = words[record];
        let selector = (word >> 16) as u8;
        let stride = period(word) + 1;
        if selector & 63 == 63 || (j as i32 - start) % stride != 0 {
            continue;
        }
        let mut left = word & 0x1000_0000 != 0;
        if word & 0x8000_0000 != 0 && (j as i32 - start) & 1 == 0 {
            left = !left;
        }
        let spacing = if word & 0x0f00_0000 != 0 {
            if word & 0x2000_0000 != 0 {
                ((word & 15) as i32 + 4) * 256
            } else {
                (word & 15) as i32 * 64
            }
        } else {
            let amount = ((word & 0xf000) >> 8) as i32 + (word & 15) as i32;
            if word & 0x2000_0000 != 0 {
                (amount + 32) * 32
            } else {
                amount * 8
            }
        };
        let lanes = section.lanes[j];
        let mut x = if left {
            -lanes[0] * 250.0 + 224.0 - spacing as f32
        } else {
            lanes[1] * 250.0 - 224.0 + spacing as f32
        };
        let family = slots[(selector >> 6) as usize];
        for _ in 0..((word >> 8) & 7) + 1 {
            if family != 0 {
                out.push(Prop {
                    sample: base + j,
                    lateral: x,
                    family,
                    selector,
                    size: ((word >> 4) & 15) as u8,
                    mirror: word & 0x4000_0000 != 0,
                });
            }
            let step = if word & 0x800 != 0 {
                spacing / 2 + (random.next() % (spacing - spacing / 2 + 1) as u32) as i32
            } else {
                spacing
            };
            x += if left { -(step as f32) } else { step as f32 };
        }
    }
}

/// Every thing at the side of the route of a course, in the order of the samples.
pub fn props(course: &Course) -> Vec<Prop> {
    let mut random = Random(1);
    let mut out = Vec::new();
    let mut base = 0;
    for (section, length) in course.route() {
        let s = &course.sections[section];
        roadside(s, length.min(s.lanes.len()), base, &mut random, &mut out);
        base += length;
    }
    out
}

/// The things of a course with the pictures they need.
pub struct Scenery {
    pub props: Vec<Prop>,
    sprites: HashMap<(u16, u8), Sprite>,
}

impl Scenery {
    /// Work out the things of a route and load the pictures of the ones that have one.
    pub fn load(disc: &Disc, course: &Course) -> Result<Scenery, String> {
        let props = props(course);
        let wanted: BTreeSet<(u16, u8)> =
            props.iter().map(|p| (p.family, p.selector & 63)).collect();
        let file = disc.read("DATA/FAMILIES.RSC")?;
        let entries = rsc::entries(&file)?;
        let mut sprites = HashMap::new();
        let ids: BTreeSet<u16> = wanted.iter().map(|w| w.0).collect();
        for id in ids {
            let Some(entry) = entries.iter().find(|e| e.tag == "FAM" && e.id == id as u32) else {
                continue;
            };
            let Ok(family) = Family::parse(entry.bytes(&file)) else {
                continue;
            };
            for &(f, selector) in wanted.iter().filter(|w| w.0 == id) {
                if let Some(sprite) = family.sprite(0, selector as usize, 2) {
                    sprites.insert((f, selector), sprite);
                }
            }
        }
        // only the things that have a picture are kept
        let props: Vec<Prop> = props
            .into_iter()
            .filter(|p| sprites.contains_key(&(p.family, p.selector & 63)))
            .collect();
        Ok(Scenery { props, sprites })
    }

    pub fn sprite(&self, prop: &Prop) -> Option<&Sprite> {
        self.sprites.get(&(prop.family, prop.selector & 63))
    }

    /// The things between two samples (the first included, the second not).
    pub fn between(&self, from: usize, to: usize) -> &[Prop] {
        let a = self.props.partition_point(|p| p.sample < from);
        let b = self.props.partition_point(|p| p.sample < to);
        &self.props[a..b.max(a)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::course::FamilyEvent;

    fn section(words: Vec<u32>, events: Vec<FamilyEvent>, samples: usize) -> Section {
        Section {
            start_elevation: 0,
            end_elevation: 0,
            bend: vec![0; samples],
            rise: vec![0; samples],
            lanes: vec![[2.0, 2.0]; samples],
            roadside: words,
            family_events: events,
            family_bias: 0,
            hills: vec![],
            buildings: vec![],
        }
    }
    fn event(family: u16) -> Vec<FamilyEvent> {
        vec![FamilyEvent {
            sample: 0,
            kind: 1,
            slot: 0,
            family,
        }]
    }
    fn run(s: &Section, length: usize) -> Vec<Prop> {
        let mut out = Vec::new();
        roadside(s, length, 100, &mut Random(1), &mut out);
        out
    }

    /// A word for a stretch of 8 samples (period 0, 8 groups), a thing every sample on the right
    /// at spacing 3, selector 5, size 6.
    const SIMPLE: u32 = 0x0800_0000 | (5 << 16) | (6 << 4) | 3;

    #[test]
    fn a_thing_stands_on_the_right_beside_the_road_at_every_sample() {
        let props = run(&section(vec![SIMPLE], event(7), 8), 8);
        assert_eq!(props.len(), 8);
        assert_eq!(props[0].sample, 100);
        assert_eq!(props[7].sample, 107);
        // the edge of the road is 500 units out, and the gap 3 * 64 inside the 224
        assert_eq!(props[0].lateral, 500.0 - 224.0 + 192.0);
        assert_eq!(
            (props[0].family, props[0].selector, props[0].size),
            (7, 5, 6)
        );
        assert!(!props[0].mirror);
    }

    #[test]
    fn the_left_side_the_mirror_and_the_count_come_from_the_word() {
        let word = SIMPLE | 0x1000_0000 | 0x4000_0000 | (2 << 8); // left, mirrored, three in a row
        let props = run(&section(vec![word], event(7), 1), 1);
        assert_eq!(props.len(), 3);
        assert!(props.iter().all(|p| p.mirror));
        assert_eq!(props[0].lateral, -500.0 + 224.0 - 192.0);
        assert_eq!(
            props[1].lateral,
            props[0].lateral - 192.0,
            "the next is further out"
        );
    }

    #[test]
    fn nothing_is_drawn_for_selector_63_or_a_slot_that_has_no_family() {
        let none = (SIMPLE & !(0xff << 16)) | (63 << 16);
        assert!(run(&section(vec![none], event(7), 8), 8).is_empty());
        assert!(
            run(&section(vec![SIMPLE], vec![], 8), 8).is_empty(),
            "no family event"
        );
        // selector 70 is slot 1, which nothing has set
        let slot1 = (SIMPLE & !(0xff << 16)) | (70 << 16);
        assert!(run(&section(vec![slot1], event(7), 8), 8).is_empty());
    }

    #[test]
    fn a_stretch_that_ends_hands_over_to_the_next_word() {
        let first = 0x0200_0000 | (5 << 16) | 3; // two samples
        let second = 0x0800_0000 | (9 << 16) | 3;
        let props = run(&section(vec![first, second], event(7), 6), 6);
        let selectors: Vec<u8> = props.iter().map(|p| p.selector).collect();
        assert_eq!(
            selectors,
            vec![5, 5, 5, 9, 9, 9],
            "samples 0..=2 use the first (it lasts 2 and the change is after it)"
        );
    }

    #[test]
    fn a_period_leaves_gaps_and_a_random_gap_varies_with_the_generator() {
        let spaced = 0x0400_0000 | (2 << 12) | (5 << 16) | 3; // every third sample
        let props = run(&section(vec![spaced], event(7), 12), 12);
        let at: Vec<usize> = props.iter().map(|p| p.sample - 100).collect();
        assert_eq!(at, vec![0, 3, 6, 9]);
        let rough = SIMPLE | 0x800 | (3 << 8);
        let a = run(&section(vec![rough], event(7), 1), 1);
        assert_eq!(a.len(), 4);
        assert_eq!(
            a,
            run(&section(vec![rough], event(7), 1), 1),
            "the same every time"
        );
        assert_ne!(
            a[1].lateral - a[0].lateral,
            a[2].lateral - a[1].lateral,
            "gaps differ"
        );
    }

    #[test]
    fn the_random_numbers_are_those_of_the_c_library() {
        let mut r = Random(1);
        assert_eq!([r.next(), r.next(), r.next()], [41, 18467, 6334]);
    }

    #[test]
    fn a_family_event_ahead_of_the_window_is_not_used_yet() {
        let late = vec![FamilyEvent {
            sample: 100,
            kind: 1,
            slot: 0,
            family: 7,
        }];
        let props = run(&section(vec![SIMPLE], late, 8), 8);
        assert!(
            props.is_empty(),
            "the event is at sample 100, far beyond 8 + 33"
        );
    }

    #[test]
    fn things_are_found_between_two_samples() {
        let p = |sample| Prop {
            sample,
            lateral: 0.0,
            family: 1,
            selector: 0,
            size: 8,
            mirror: false,
        };
        let s = Scenery {
            props: vec![p(1), p(3), p(3), p(7)],
            sprites: HashMap::new(),
        };
        assert_eq!(s.between(3, 7).len(), 2);
        assert_eq!(s.between(0, 100).len(), 4);
        assert!(s.between(8, 20).is_empty());
        assert!(s.between(5, 2).is_empty());
    }
}
