//! `DATA/COURSES/*.CRS`: a course, as a resource container (see [`crate::rsc`]) holding
//! `SGS ` (the road in sections) and `NOD ` (how the sections are joined), both number 1.
//!
//! `SGS `: a chunk `SGSR` with a 16-byte header (the section count at +8) and one 52-byte row
//! per section. A row holds, at +4, the offset of the section's path chunk `HTPR`: 16 bytes
//! of header (the sample count at +8, the elevation at the start as an `i16` at +0xC and at
//! the end at +0xE) and then two signed bytes per sample, the *bend* and then the *rise*. The
//! rise is a change of height from one sample to the next; the bend is a curvature. A row
//! also holds, at +0x18, the offset of the lane table `NALR`: a 16-byte header (the record
//! count at +8) and 20-byte records `{start, end: u32; lanes left and right at the start and
//! then at the end: 4 bytes; two i32 slopes in 1/65536 of a lane per sample}`. The lanes
//! change from the start values to the end values between the two samples, and a lane is 250
//! world units wide while a sample is 256 long.
//!
//! `NOD `: `DONR`, the size at +4 (the resource's own size), the node count at +8 and the
//! root node's offset at +16. A node starts with its kind: 0 road (+8 next, +12 previous,
//! +16 section, +20 how many of its samples are used), 1 split (+8 and +12 the two roads
//! ahead, +16 the road behind), 2 merge (+8 and +12 the two roads behind, +16 the road
//! ahead), 3 and 4 the ends. Offsets are from the start of the `NOD ` resource.

use crate::rsc;
use std::collections::BTreeMap;

/// One section of road.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    pub start_elevation: i16,
    pub end_elevation: i16,
    pub bend: Vec<i8>,
    pub rise: Vec<i8>,
    /// The number of lanes to the left and to the right of the middle, at every sample.
    pub lanes: Vec<[f32; 2]>,
}

impl Section {
    /// The elevation at the end as the rises add up from the start.
    pub fn summed_elevation(&self) -> i32 {
        self.start_elevation as i32 + self.rise.iter().map(|&r| r as i32).sum::<i32>()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Road {
        next: u32,
        previous: u32,
        section: usize,
        length: usize,
    },
    Split {
        ahead: [u32; 2],
        behind: u32,
    },
    Merge {
        behind: [u32; 2],
        ahead: u32,
    },
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Course {
    pub sections: Vec<Section>,
    pub nodes: BTreeMap<u32, Node>,
    pub root: u32,
}

struct Bytes<'a>(&'a [u8]);

impl Bytes<'_> {
    fn u32(&self, at: usize) -> Result<u32, String> {
        self.0
            .get(at..at + 4)
            .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
            .ok_or_else(|| format!("the course is cut before byte {at}"))
    }
    fn i16(&self, at: usize) -> Result<i16, String> {
        self.0
            .get(at..at + 2)
            .map(|w| i16::from_le_bytes([w[0], w[1]]))
            .ok_or_else(|| format!("the course is cut before byte {at}"))
    }
    fn tag(&self, at: usize, tag: &[u8; 4]) -> Result<(), String> {
        if self.0.get(at..at + 4) == Some(&tag[..]) {
            Ok(())
        } else {
            Err(format!("no {} at byte {at}", String::from_utf8_lossy(tag)))
        }
    }
}

/// The lanes at each of `samples` samples, from the table at `at` (none: two on each side).
fn read_lanes(sgs: &Bytes, at: usize, samples: usize) -> Result<Vec<[f32; 2]>, String> {
    if at == 0 {
        return Ok(vec![[2.0, 2.0]; samples]);
    }
    sgs.tag(at, b"NALR")?;
    let count = sgs.u32(at + 8)? as usize;
    let mut records = Vec::new();
    for i in 0..count {
        let o = at + 16 + i * 20;
        let bytes = sgs
            .0
            .get(o + 8..o + 12)
            .ok_or("the lane table is cut short")?;
        records.push((
            sgs.u32(o)? as usize,
            sgs.u32(o + 4)? as usize,
            [bytes[0] as f32, bytes[1] as f32],
            [bytes[2] as f32, bytes[3] as f32],
        ));
    }
    // between records the lanes stay as the last one left them; inside one they move from
    // its start values to its end values
    let mut lanes = Vec::with_capacity(samples);
    let mut now = records.first().map_or([2.0, 2.0], |r| r.2);
    for j in 0..samples {
        for &(start, end, from, to) in &records {
            if j >= start && j <= end {
                let t = if end > start {
                    (j - start) as f32 / (end - start) as f32
                } else {
                    1.0
                };
                now = [
                    from[0] + (to[0] - from[0]) * t,
                    from[1] + (to[1] - from[1]) * t,
                ];
            }
        }
        lanes.push(now);
    }
    Ok(lanes)
}

pub fn parse(file: &[u8]) -> Result<Course, String> {
    let entries = rsc::entries(file)?;
    let find = |tag: &str| {
        entries
            .iter()
            .find(|e| e.tag == tag && e.id == 1)
            .map(|e| e.bytes(file))
            .ok_or_else(|| format!("the course has no {tag} resource"))
    };
    let (sgs, nod) = (Bytes(find("SGS")?), Bytes(find("NOD")?));

    sgs.tag(0, b"SGSR")?;
    let count = sgs.u32(8)? as usize;
    let mut sections = Vec::new();
    for i in 0..count {
        let row = 16 + i * 52;
        let path = sgs.u32(row + 4)? as usize;
        sgs.tag(path, b"HTPR")?;
        let samples = sgs.u32(path + 8)? as usize;
        let data = sgs
            .0
            .get(path + 16..path + 16 + samples * 2)
            .ok_or_else(|| format!("section {i} has fewer samples than it says"))?;
        sections.push(Section {
            start_elevation: sgs.i16(path + 0xC)?,
            end_elevation: sgs.i16(path + 0xE)?,
            bend: data.chunks(2).map(|p| p[0] as i8).collect(),
            rise: data.chunks(2).map(|p| p[1] as i8).collect(),
            lanes: read_lanes(&sgs, sgs.u32(row + 0x18)? as usize, samples)?,
        });
    }

    nod.tag(0, b"DONR")?;
    if nod.u32(4)? as usize != nod.0.len() {
        return Err("the node resource does not have the size it says".into());
    }
    let (total, root) = (nod.u32(8)? as usize, nod.u32(16)?);
    let mut nodes = BTreeMap::new();
    let mut pending = vec![root];
    while let Some(at) = pending.pop() {
        if nodes.contains_key(&at) {
            continue;
        }
        if at < 20 || at % 4 != 0 {
            return Err(format!("{at} is not the place of a node"));
        }
        if nodes.len() >= total {
            return Err("more nodes are reachable than the header counts".into());
        }
        let at_ = at as usize;
        let node = match nod.u32(at_)? {
            0 => {
                let (next, previous) = (nod.u32(at_ + 8)?, nod.u32(at_ + 12)?);
                let (section, length) = (nod.u32(at_ + 16)? as usize, nod.u32(at_ + 20)? as usize);
                if section >= sections.len() || length == 0 || length > sections[section].bend.len()
                {
                    return Err(format!("the road node at {at} points at a bad section"));
                }
                pending.extend([next, previous]);
                Node::Road {
                    next,
                    previous,
                    section,
                    length,
                }
            }
            1 => {
                let (a, b, behind) = (nod.u32(at_ + 8)?, nod.u32(at_ + 12)?, nod.u32(at_ + 16)?);
                pending.extend([a, b, behind]);
                Node::Split {
                    ahead: [a, b],
                    behind,
                }
            }
            2 => {
                let (a, b, ahead) = (nod.u32(at_ + 8)?, nod.u32(at_ + 12)?, nod.u32(at_ + 16)?);
                pending.extend([a, b, ahead]);
                Node::Merge {
                    behind: [a, b],
                    ahead,
                }
            }
            3 | 4 => Node::End,
            k => return Err(format!("node kind {k} at {at} is not known")),
        };
        nodes.insert(at, node);
    }
    Ok(Course {
        sections,
        nodes,
        root,
    })
}

impl Course {
    /// The way along the course from its root, taking the first road at every split: the
    /// sections to drive and how many samples of each. Stops at an end, or when it would go
    /// round in a circle.
    pub fn route(&self) -> Vec<(usize, usize)> {
        let mut route = Vec::new();
        let mut at = self.root;
        let mut seen = std::collections::BTreeSet::new();
        while seen.insert(at) {
            match self.nodes.get(&at) {
                Some(Node::Road {
                    next,
                    section,
                    length,
                    ..
                }) => {
                    route.push((*section, *length));
                    at = *next;
                }
                Some(Node::Split { ahead, .. }) => at = ahead[0],
                Some(Node::Merge { ahead, .. }) => at = *ahead,
                _ => break,
            }
        }
        route
    }
}

/// One sample of the way along a route, ready to be drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub bend: i8,
    pub rise: i8,
    pub lanes: [f32; 2],
}

impl Course {
    /// The samples of [`route`](Course::route), one after the other.
    pub fn route_samples(&self) -> Vec<Sample> {
        let mut out = Vec::new();
        for (section, length) in self.route() {
            let s = &self.sections[section];
            for i in 0..length {
                out.push(Sample {
                    bend: s.bend[i],
                    rise: s.rise[i],
                    lanes: s.lanes[i],
                });
            }
        }
        out
    }
}

/// A picture of a route: the road seen from above in the top part and its height below.
///
/// What the bend means in angle is not known yet, so `turn` (radians of turning per unit of
/// bend, per sample) is a guess the caller gives; the rise is drawn as it is.
pub fn draw_route(course: &Course, turn: f64) -> crate::image::Rgb {
    const SIZE: usize = 900;
    const PROFILE: usize = 160;
    let mut points = Vec::new(); // x, y, elevation
    let (mut x, mut y, mut heading) = (0.0f64, 0.0f64, 0.0f64);
    for (section, length) in course.route() {
        let s = &course.sections[section];
        let mut height = s.start_elevation as f64;
        for i in 0..length {
            heading += s.bend[i] as f64 * turn;
            x += heading.cos();
            y += heading.sin();
            height += s.rise[i] as f64;
            points.push((x, y, height));
        }
    }
    let mut pixels = vec![24u8; SIZE * (SIZE + PROFILE) * 3];
    if points.is_empty() {
        return crate::image::Rgb {
            width: SIZE as u32,
            height: (SIZE + PROFILE) as u32,
            pixels,
        };
    }
    let (min_x, max_x) = points
        .iter()
        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.0), a.1.max(p.0)));
    let (min_y, max_y) = points
        .iter()
        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
    let (min_h, max_h) = points
        .iter()
        .fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.2), a.1.max(p.2)));
    let scale = (SIZE as f64 - 40.0) / (max_x - min_x).max(max_y - min_y).max(1.0);
    let mut put = |px: i64, py: i64, c: [u8; 3]| {
        if px >= 0 && py >= 0 && (px as usize) < SIZE && (py as usize) < SIZE + PROFILE {
            let at = (py as usize * SIZE + px as usize) * 3;
            pixels[at..at + 3].copy_from_slice(&c);
        }
    };
    let n = points.len() as f64;
    for (i, p) in points.iter().enumerate() {
        // colour runs from blue at the start to red at the end of the route
        let t = i as f64 / n;
        let c = [(255.0 * t) as u8, 90, (255.0 * (1.0 - t)) as u8];
        put(
            ((p.0 - min_x) * scale) as i64 + 20,
            ((p.1 - min_y) * scale) as i64 + 20,
            c,
        );
        let px = (i as f64 / n * (SIZE as f64 - 40.0)) as i64 + 20;
        let py = SIZE as i64 + PROFILE as i64
            - 15
            - ((p.2 - min_h) / (max_h - min_h).max(1.0) * (PROFILE as f64 - 30.0)) as i64;
        put(px, py, c);
    }
    crate::image::Rgb {
        width: SIZE as u32,
        height: (SIZE + PROFILE) as u32,
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_elevation_is_the_start_plus_the_rises() {
        let s = Section {
            start_elevation: 100,
            end_elevation: 90,
            bend: vec![0, 0, 0],
            rise: vec![5, -20, 5],
            lanes: vec![[2.0, 2.0]; 3],
        };
        assert_eq!(s.summed_elevation(), 90);
    }

    #[test]
    fn a_route_follows_the_first_road_of_a_split_and_stops_at_an_end() {
        let road = |next, section| Node::Road {
            next,
            previous: 0,
            section,
            length: 2,
        };
        let nodes = BTreeMap::from([
            (20, road(40, 0)),
            (
                40,
                Node::Split {
                    ahead: [60, 80],
                    behind: 20,
                },
            ),
            (60, road(100, 1)),
            (80, road(100, 2)),
            (
                100,
                Node::Merge {
                    behind: [60, 80],
                    ahead: 120,
                },
            ),
            (120, road(140, 3)),
            (140, Node::End),
        ]);
        let course = Course {
            sections: Vec::new(),
            nodes,
            root: 20,
        };
        assert_eq!(course.route(), vec![(0, 2), (1, 2), (3, 2)]);
    }

    #[test]
    fn a_route_that_goes_round_stops() {
        let nodes = BTreeMap::from([(
            20,
            Node::Road {
                next: 20,
                previous: 20,
                section: 0,
                length: 1,
            },
        )]);
        let course = Course {
            sections: Vec::new(),
            nodes,
            root: 20,
        };
        assert_eq!(course.route().len(), 1);
    }

    #[test]
    fn other_files_are_errors_not_panics() {
        assert!(parse(b"nope").is_err());
        assert!(parse(&[0; 100]).is_err());
    }
}
