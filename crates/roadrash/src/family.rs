//! A family of `FAMILIES.RSC`: the pictures of one kind of place (a roadside, a town...). Its
//! resource is packed (see [`crate::explode`]) and, unpacked, a tree of nodes whose offsets
//! are counted from the node that holds them:
//!
//! - an array: a number below 0x1000 (how many) and then that many offsets;
//! - an `OFST` (0x4F465354): one offset, to the node it stands for;
//! - a leaf, of which the animation `RRAN` is the one used for the roadside.
//!
//! The root is an array of categories; a category is an array indexed by selector. The
//! roadside objects are category 0 and billboards category 4.

use crate::animation;
use crate::cel::Indexed;
use crate::explode;

const OFST: u32 = 0x4F46_5354;
const CLGP: u32 = 0x434C_4750;

pub struct Family {
    bytes: Vec<u8>,
}

/// A picture to draw: its pixels, where its feet are, and the box it is drawn in.
#[derive(Debug, Clone, PartialEq)]
pub struct Sprite {
    pub texture: Indexed,
    pub hotspot: [i32; 2],
}

impl Family {
    /// A family from the bytes of its resource in `FAMILIES.RSC`.
    pub fn parse(resource: &[u8]) -> Result<Family, String> {
        let bytes = explode::unpack(resource)?;
        if bytes.len() < 4 {
            return Err("the family is empty".into());
        }
        Ok(Family { bytes })
    }

    fn word(&self, at: usize) -> Option<u32> {
        self.bytes
            .get(at..at + 4)
            .map(|w| u32::from_le_bytes([w[0], w[1], w[2], w[3]]))
    }

    /// Go through any `OFST` to the node that matters.
    fn follow(&self, mut at: usize) -> Option<usize> {
        for _ in 0..32 {
            if self.word(at)? != OFST {
                return Some(at);
            }
            at += self.word(at + 4)? as usize;
        }
        None
    }

    /// The `index`th child of the array at `at`.
    fn child(&self, at: usize, index: usize) -> Option<usize> {
        let at = self.follow(at)?;
        let count = self.word(at)? as usize;
        if count >= 0x1000 || index >= count {
            return None;
        }
        self.follow(at + self.word(at + 4 + index * 4)? as usize)
    }

    /// The node for a `category` and a `selector`.
    pub fn resolve(&self, category: usize, selector: usize) -> Option<usize> {
        self.child(self.child(0, category)?, selector)
    }

    /// The first picture of a cel group (`CLGP`) for a `category`, a `selector` and, when the
    /// category has them, a `frame` (the three faces of a hill are frames). A group is a
    /// set of textures of one thing at different sizes or in pieces: at +20 its flags (the two low
    /// bits are the mode), at +21 where its table of textures is (in words from the group), at
    /// +24 and +25 the last column and row. Mode 0 holds rows of offsets to the cells; mode 1
    /// the offsets themselves. Offsets are counted from byte 8 of the group.
    pub fn cel_texture(
        &self,
        category: usize,
        selector: usize,
        frame: Option<usize>,
    ) -> Option<Indexed> {
        let mut at = self.resolve(category, selector)?;
        if let Some(f) = frame {
            at = self.child(at, f)?;
        }
        if self.word(at)? != CLGP {
            return None;
        }
        let flags = *self.bytes.get(at + 20)?;
        let table = at + *self.bytes.get(at + 21)? as usize * 4;
        let first = match flags & 3 {
            // the first cell of the first row
            0 => at + self.word(table)? as usize,
            1 => table,
            _ => return None,
        };
        let cell = at + 8 + self.word(first)? as usize;
        crate::cel::read_indexed(self.bytes.get(cell..)?).ok()
    }

    /// What is at a category and selector, for looking into a family (the node's first word, how
    /// many children it has when it is an array, and the same for its first children).
    pub fn describe(&self, category: usize, selector: usize) -> String {
        let Some(at) = self.resolve(category, selector) else {
            return "nothing".into();
        };
        let word = self.word(at).unwrap_or(0);
        let mut text = format!("node at {at}, word {word:#x}");
        if word < 0x1000 {
            text += &format!(", {word} children:");
            for i in 0..(word as usize).min(4) {
                match self.child(at, i) {
                    Some(c) => {
                        text += &format!(" [{i}: {c} word {:#x}]", self.word(c).unwrap_or(0))
                    }
                    None => text += &format!(" [{i}: none]"),
                }
            }
        }
        text
    }

    /// The building of category 1 for a selector: its height in world units and the picture of
    /// its front. The group is a grid of pictures (mode 0) whose last cell is the front, and
    /// byte +22 of the group is the height in units of 16.
    pub fn building(&self, selector: usize) -> Option<(f32, Indexed)> {
        let at = self.resolve(1, selector)?;
        if self.word(at)? != CLGP || *self.bytes.get(at + 20)? & 3 != 0 {
            return None;
        }
        let height = *self.bytes.get(at + 22)? as f32 * 16.0;
        let (last_col, last_row) = (
            *self.bytes.get(at + 24)? as usize,
            *self.bytes.get(at + 25)? as usize,
        );
        let table = at + *self.bytes.get(at + 21)? as usize * 4;
        let row = at + self.word(table + 4 * last_row)? as usize;
        let cell = at + 8 + self.word(row + 4 * last_col)? as usize;
        let texture = crate::cel::read_indexed(self.bytes.get(cell..)?).ok()?;
        Some((height, texture))
    }

    /// The picture of frame `frame` of the animation for `category` and `selector`.
    pub fn sprite(&self, category: usize, selector: usize, frame: usize) -> Option<Sprite> {
        let at = self.resolve(category, selector)?;
        let frames = animation::read(&self.bytes, at).ok()?;
        let f = frames.get(frame)?;
        Some(Sprite {
            texture: (**f.texture.as_ref()?).clone(),
            hotspot: f.hotspot,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(b: &mut Vec<u8>, at: usize, v: u32) {
        if b.len() < at + 4 {
            b.resize(at + 4, 0);
        }
        b[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }

    /// A family whose bytes are not packed: root of 2 categories; category 1 is an OFST to an
    /// array of one selector, which is a marker word.
    fn sample() -> Family {
        let mut b = vec![0u8; 0];
        put(&mut b, 0, 2); // root: two categories
        put(&mut b, 4, 100); // category 0 at 100
        put(&mut b, 8, 200); // category 1 at 200 (an OFST)
        put(&mut b, 100, 0); // category 0 is an empty array
        put(&mut b, 200, OFST);
        put(&mut b, 204, 100); // to 300
        put(&mut b, 300, 1); // an array of one
        put(&mut b, 304, 50); // its child at 350
        put(&mut b, 350, 0x1234);
        Family { bytes: b }
    }

    #[test]
    fn categories_and_selectors_are_found_through_arrays_and_ofst_nodes() {
        let f = sample();
        assert_eq!(f.resolve(1, 0), Some(350));
        assert_eq!(f.resolve(0, 0), None, "an empty category");
        assert_eq!(f.resolve(2, 0), None, "no such category");
        assert_eq!(f.resolve(1, 1), None, "no such selector");
        assert!(f.sprite(1, 0, 0).is_none(), "a marker is not an animation");
    }

    #[test]
    fn a_node_that_loops_or_lies_outside_is_not_found_and_does_not_hang() {
        let mut b = vec![0u8; 0];
        put(&mut b, 0, 1);
        put(&mut b, 4, 8);
        put(&mut b, 8, OFST);
        put(&mut b, 12, 0); // an OFST to itself
        let f = Family { bytes: b };
        assert_eq!(f.resolve(0, 0), None);
        assert!(Family {
            bytes: vec![1, 0, 0]
        }
        .resolve(0, 0)
        .is_none());
    }

    #[test]
    fn a_resource_that_does_not_unpack_is_an_error() {
        assert!(Family::parse(&[1, 2, 3]).is_err());
    }
}
