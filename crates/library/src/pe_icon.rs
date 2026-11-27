//! Changing the icon of a Windows program (a PE file) to one that looks like a Mac icon.
//!
//! Wine shows the icon inside a program as the program's Dock icon, and the icons of programs
//! from the 90s are small pictures of 256 colours that come out garbled there. This reads the
//! biggest icon of the program, draws it on a rounded tile with a soft shadow at 256x256 in 32
//! bits, and writes that as the program's icon: the picture goes in a new section at the end of
//! the file and the icon resource is pointed at it, so nothing else in the file moves.

use std::convert::TryInto;

const SIDE: usize = 256;

fn u16_at(d: &[u8], o: usize) -> Result<u16, String> {
    d.get(o..o + 2)
        .map(|b| u16::from_le_bytes(b.try_into().unwrap()))
        .ok_or_else(|| "the file ends too early".to_string())
}

fn u32_at(d: &[u8], o: usize) -> Result<u32, String> {
    d.get(o..o + 4)
        .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
        .ok_or_else(|| "the file ends too early".to_string())
}

fn put_u16(d: &mut [u8], o: usize, v: u16) -> Result<(), String> {
    d.get_mut(o..o + 2)
        .ok_or("the file ends too early")?
        .copy_from_slice(&v.to_le_bytes());
    Ok(())
}

fn put_u32(d: &mut [u8], o: usize, v: u32) -> Result<(), String> {
    d.get_mut(o..o + 4)
        .ok_or("the file ends too early")?
        .copy_from_slice(&v.to_le_bytes());
    Ok(())
}

fn align(n: usize, to: usize) -> usize {
    n.div_ceil(to) * to
}

struct Section {
    va: usize,
    vsize: usize,
    raw_size: usize,
    raw_ptr: usize,
}

/// The parts of the headers that are needed.
struct Pe {
    /// File offset of the number of sections.
    count_at: usize,
    /// File offset of the section table.
    table_at: usize,
    sections: Vec<Section>,
    /// File offset of the optional header.
    opt: usize,
    /// The data directory entry of the resources: its RVA.
    resource_rva: usize,
    section_align: usize,
    file_align: usize,
    headers_size: usize,
}

impl Pe {
    fn parse(d: &[u8]) -> Result<Pe, String> {
        if d.get(..2) != Some(b"MZ") {
            return Err("not a Windows program".into());
        }
        let pe = u32_at(d, 0x3c)? as usize;
        if d.get(pe..pe + 4) != Some(b"PE\0\0") {
            return Err("not a Windows program".into());
        }
        let count_at = pe + 6;
        let count = u16_at(d, count_at)? as usize;
        let opt_size = u16_at(d, pe + 20)? as usize;
        let opt = pe + 24;
        let dirs = match u16_at(d, opt)? {
            0x10B => 96,
            0x20B => 112,
            _ => return Err("not a Windows program".into()),
        };
        let table_at = opt + opt_size;
        let mut sections = Vec::new();
        for i in 0..count {
            let o = table_at + 40 * i;
            sections.push(Section {
                vsize: u32_at(d, o + 8)? as usize,
                va: u32_at(d, o + 12)? as usize,
                raw_size: u32_at(d, o + 16)? as usize,
                raw_ptr: u32_at(d, o + 20)? as usize,
            });
        }
        Ok(Pe {
            count_at,
            table_at,
            sections,
            opt,
            resource_rva: u32_at(d, opt + dirs + 16)? as usize,
            section_align: u32_at(d, opt + 32)? as usize,
            file_align: u32_at(d, opt + 36)? as usize,
            headers_size: u32_at(d, opt + 60)? as usize,
        })
    }

    /// The place in the file of a relative virtual address.
    fn offset(&self, rva: usize) -> Result<usize, String> {
        self.sections
            .iter()
            .find(|s| rva >= s.va && rva < s.va + s.vsize.max(s.raw_size))
            .map(|s| rva - s.va + s.raw_ptr)
            .ok_or_else(|| "an address in the program is outside its sections".to_string())
    }
}

/// A leaf of the resource tree: where its entry (RVA of the data and size) is in the file.
struct Leaf {
    entry_at: usize,
}

/// The entries of a directory of resources: `(name or id, offset)`, the offset from the start of
/// the resources; its top bit says "a directory".
fn entries(d: &[u8], dir: usize) -> Result<Vec<(u32, u32)>, String> {
    let n = u16_at(d, dir + 12)? as usize + u16_at(d, dir + 14)? as usize;
    (0..n)
        .map(|i| Ok((u32_at(d, dir + 16 + 8 * i)?, u32_at(d, dir + 20 + 8 * i)?)))
        .collect()
}

/// The first leaf under resource `kind` with id `id` (any id when `None`): the first language.
fn find(d: &[u8], pe: &Pe, kind: u32, id: Option<u32>) -> Result<Option<(u32, Leaf)>, String> {
    let root = pe.offset(pe.resource_rva)?;
    let Some((_, kind_dir)) = entries(d, root)?.into_iter().find(|(n, _)| *n == kind) else {
        return Ok(None);
    };
    for (n, dir) in entries(d, root + (kind_dir & 0x7fff_ffff) as usize)? {
        if id.is_some_and(|want| want != n) {
            continue;
        }
        if dir & 0x8000_0000 == 0 {
            continue;
        }
        let langs = entries(d, root + (dir & 0x7fff_ffff) as usize)?;
        let Some((_, leaf)) = langs.first() else {
            continue;
        };
        return Ok(Some((
            n,
            Leaf {
                entry_at: root + *leaf as usize,
            },
        )));
    }
    Ok(None)
}

fn leaf_data<'a>(d: &'a [u8], pe: &Pe, leaf: &Leaf) -> Result<&'a [u8], String> {
    let rva = u32_at(d, leaf.entry_at)? as usize;
    let size = u32_at(d, leaf.entry_at + 4)? as usize;
    let at = pe.offset(rva)?;
    d.get(at..at + size)
        .ok_or_else(|| "a resource ends outside the file".to_string())
}

/// A picture of red, green, blue and alpha bytes, top row first.
struct Image {
    w: usize,
    h: usize,
    rgba: Vec<u8>,
}

/// An icon as stored in a program: a bitmap with no file header, twice as high as the picture
/// (the lower half is the mask of what is see-through).
fn decode_icon(data: &[u8]) -> Result<Image, String> {
    let head = u32_at(data, 0)? as usize;
    let w = u32_at(data, 4)? as usize;
    let h2 = u32_at(data, 8)? as usize;
    let bits = u16_at(data, 14)? as usize;
    let used = u32_at(data, 32)? as usize;
    if head < 40 || !(1..=512).contains(&w) || !(2..=1024).contains(&h2) || u32_at(data, 16)? != 0 {
        return Err("an icon in a format that is not handled".into());
    }
    let h = h2 / 2;
    let colors = if bits <= 8 {
        if used == 0 {
            1 << bits
        } else {
            used
        }
    } else {
        0
    };
    let palette = head;
    let pixels = palette + colors * 4;
    let stride = align(w * bits, 32) / 8;
    let mask_at = pixels + stride * h;
    let mask_stride = align(w, 32) / 8;
    let at = |o: usize| {
        data.get(o)
            .copied()
            .ok_or_else(|| "an icon ends too early".to_string())
    };
    let mut rgba = vec![0u8; w * h * 4];
    for y in 0..h {
        let row = pixels + stride * (h - 1 - y);
        for x in 0..w {
            let (b, g, r, a) = match bits {
                1 | 4 | 8 => {
                    let bit = x * bits;
                    let byte = at(row + bit / 8)? as usize;
                    let idx = (byte >> (8 - bits - bit % 8)) & ((1 << bits) - 1);
                    let p = palette + 4 * idx;
                    (at(p)?, at(p + 1)?, at(p + 2)?, 255)
                }
                24 => (
                    at(row + 3 * x)?,
                    at(row + 3 * x + 1)?,
                    at(row + 3 * x + 2)?,
                    255,
                ),
                32 => (
                    at(row + 4 * x)?,
                    at(row + 4 * x + 1)?,
                    at(row + 4 * x + 2)?,
                    at(row + 4 * x + 3)?,
                ),
                _ => return Err("an icon in a format that is not handled".into()),
            };
            let hole = at(mask_at + mask_stride * (h - 1 - y) + x / 8)? & (0x80 >> (x % 8)) != 0;
            let alpha = if bits == 32 && a != 0 {
                a
            } else if hole {
                0
            } else {
                255
            };
            let o = (y * w + x) * 4;
            rgba[o..o + 4].copy_from_slice(&[r, g, b, alpha]);
        }
    }
    Ok(Image { w, h, rgba })
}

/// The colour of `img` at a place given in pixels (between the pixels: blended), premultiplied.
fn sample(img: &Image, x: f32, y: f32) -> [f32; 4] {
    let (x, y) = (x - 0.5, y - 0.5);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let px = |ix: f32, iy: f32| {
        let ix = (ix.max(0.0) as usize).min(img.w - 1);
        let iy = (iy.max(0.0) as usize).min(img.h - 1);
        let o = (iy * img.w + ix) * 4;
        let a = img.rgba[o + 3] as f32 / 255.0;
        [
            img.rgba[o] as f32 * a,
            img.rgba[o + 1] as f32 * a,
            img.rgba[o + 2] as f32 * a,
            a * 255.0,
        ]
    };
    let (a, b, c, d) = (
        px(x0, y0),
        px(x0 + 1.0, y0),
        px(x0, y0 + 1.0),
        px(x0 + 1.0, y0 + 1.0),
    );
    let mut out = [0.0; 4];
    for i in 0..4 {
        let top = a[i] * (1.0 - fx) + b[i] * fx;
        let bottom = c[i] * (1.0 - fx) + d[i] * fx;
        out[i] = top * (1.0 - fy) + bottom * fy;
    }
    out
}

/// How far `(x, y)` is outside a rounded square (negative: inside), in pixels.
fn rounded_distance(x: f32, y: f32, half: f32, radius: f32) -> f32 {
    let (qx, qy) = (x.abs() - (half - radius), y.abs() - (half - radius));
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - radius
}

/// The icon drawn as a Mac icon: a rounded dark tile with a soft shadow, the picture on it,
/// 256x256, blue/green/red/alpha bytes with the top row first.
fn mac_tile(src: &Image) -> Vec<u8> {
    let c = SIDE as f32 / 2.0;
    let (half, radius) = (c * 0.805, c * 0.805 * 0.45);
    let art = c * 1.2;
    let mut out = vec![0u8; SIDE * SIDE * 4];
    for py in 0..SIDE {
        for px in 0..SIDE {
            let (x, y) = (px as f32 + 0.5 - c, py as f32 + 0.5 - c);
            // the shadow: the tile moved down, spread out
            let shadow_d = rounded_distance(x, y - c * 0.035, half, radius);
            let shadow = (1.0 - (shadow_d / (c * 0.07)).clamp(0.0, 1.0)).powi(2) * 0.38;
            // the tile
            let d = rounded_distance(x, y, half, radius);
            let cover = (0.5 - d).clamp(0.0, 1.0);
            let t = ((y + half) / (2.0 * half)).clamp(0.0, 1.0);
            let tile = [
                0x4a as f32 * (1.0 - t) + 0x15 as f32 * t,
                0x4e as f32 * (1.0 - t) + 0x17 as f32 * t,
                0x5a as f32 * (1.0 - t) + 0x1d as f32 * t,
            ];
            // the picture, centred, smoothed
            let (u, v) = (
                (x + art / 2.0) / art * src.w as f32,
                (y + art / 2.0) / art * src.h as f32,
            );
            let inside = u >= 0.0 && v >= 0.0 && u < src.w as f32 && v < src.h as f32;
            let s = if inside { sample(src, u, v) } else { [0.0; 4] };
            let sa = s[3] / 255.0;
            // premultiplied: picture over tile over shadow
            let mut rgb = [0.0f32; 3];
            for i in 0..3 {
                let over_tile = s[i] + tile[i] * (1.0 - sa);
                rgb[i] = over_tile * cover;
            }
            let alpha = cover + shadow * (1.0 - cover);
            // the shadow is black, so it only lowers the colours that show through
            let o = (py * SIDE + px) * 4;
            let a = alpha.clamp(0.0, 1.0);
            if a > 0.0 {
                out[o] = (rgb[2] / a).clamp(0.0, 255.0) as u8;
                out[o + 1] = (rgb[1] / a).clamp(0.0, 255.0) as u8;
                out[o + 2] = (rgb[0] / a).clamp(0.0, 255.0) as u8;
            }
            out[o + 3] = (a * 255.0).round() as u8;
        }
    }
    out
}

/// The 256x256 icon as a bitmap of an icon resource: the header, the rows from the bottom up,
/// and a mask that hides nothing (the alpha says what is see-through).
fn icon_bitmap(bgra: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(40 + bgra.len() + SIDE * SIDE / 8);
    for v in [40u32, SIDE as u32, (SIDE * 2) as u32] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&[0; 24]);
    for y in (0..SIDE).rev() {
        out.extend_from_slice(&bgra[y * SIDE * 4..(y + 1) * SIDE * 4]);
    }
    out.extend_from_slice(&vec![0u8; SIDE * SIDE / 8]);
    out
}

/// Add a section with `data` at the end of the file (and of the image), readable and nothing
/// else. Gives its relative virtual address. Nothing that is already in the file moves.
fn append_section(exe: &mut Vec<u8>, pe: &Pe, name: &[u8], data: &[u8]) -> Result<usize, String> {
    let last = pe.sections.last().ok_or("the program has no sections")?;
    if pe.table_at + 40 * (pe.sections.len() + 1)
        > pe.headers_size.min(
            pe.sections
                .iter()
                .map(|s| s.raw_ptr)
                .min()
                .unwrap_or(usize::MAX),
        )
    {
        return Err("there is no room for a section in the headers".into());
    }
    let va = align(last.va + last.vsize.max(last.raw_size), pe.section_align);
    let raw_ptr = align(exe.len(), pe.file_align);
    let raw_size = align(data.len(), pe.file_align);
    exe.resize(raw_ptr, 0);
    exe.extend_from_slice(data);
    exe.resize(raw_ptr + raw_size, 0);

    let header = pe.table_at + 40 * pe.sections.len();
    exe[header..header + 40].fill(0);
    exe[header..header + name.len().min(8)].copy_from_slice(&name[..name.len().min(8)]);
    put_u32(exe, header + 8, data.len() as u32)?;
    put_u32(exe, header + 12, va as u32)?;
    put_u32(exe, header + 16, raw_size as u32)?;
    put_u32(exe, header + 20, raw_ptr as u32)?;
    put_u32(exe, header + 36, 0x4000_0040)?; // initialized data, readable
    put_u16(exe, pe.count_at, pe.sections.len() as u16 + 1)?;
    put_u32(
        exe,
        pe.opt + 56,
        align(va + data.len(), pe.section_align) as u32,
    )?; // size of image
    put_u32(exe, pe.opt + 64, 0)?; // no checksum
    Ok(va)
}

/// Change the title of the window of a program that names it with a push of the address of a
/// text, such as `push "Road Rash"` before the call that makes its window: the text `old` (found
/// once as a whole text, and pushed from exactly one place) is replaced by `new` in a section of
/// its own, and the push points there. For programs of 32 bits whose address cannot change by
/// more than the image (the push is in the list of addresses that Windows corrects).
pub fn set_window_title(exe: &mut Vec<u8>, old: &str, new: &[u8]) -> Result<(), String> {
    let pe = Pe::parse(exe)?;
    let image_base = u32_at(exe, pe.opt + 28)? as usize;
    let mut needle = old.as_bytes().to_vec();
    needle.push(0);
    let mut pushes = Vec::new();
    for at in (1..exe.len().saturating_sub(needle.len()))
        .filter(|&i| exe[i - 1] == 0 && exe[i..i + needle.len()] == needle[..])
    {
        // where the text is in memory
        let Some(section) = pe
            .sections
            .iter()
            .find(|s| at >= s.raw_ptr && at < s.raw_ptr + s.raw_size)
        else {
            continue;
        };
        let address = (image_base + section.va + at - section.raw_ptr) as u32;
        let mut push = vec![0x68];
        push.extend_from_slice(&address.to_le_bytes());
        pushes.extend(
            (0..exe.len().saturating_sub(5))
                .filter(|&i| exe[i..i + 5] == push[..])
                .collect::<Vec<_>>(),
        );
    }
    let [at] = pushes[..] else {
        return Err(format!(
            "the title is pushed from {} places, not one",
            pushes.len()
        ));
    };
    let mut text = new.to_vec();
    text.push(0);
    let va = append_section(exe, &pe, b".title", &text)?;
    put_u32(exe, at + 1, (image_base + va) as u32)
}

/// Give the program in `exe` the icon of its own biggest icon drawn as a Mac icon.
pub fn mac_style_icon(exe: &mut Vec<u8>) -> Result<(), String> {
    replace_icon(exe, |source| Ok(mac_tile(source)))
}

/// Give the program in `exe` the 256x256 picture `rgba` (red, green, blue, alpha bytes, top row
/// first) as its icon: an icon that is made for the purpose, such as the app's own with the badge
/// of a system.
pub fn set_icon(exe: &mut Vec<u8>, rgba: &[u8]) -> Result<(), String> {
    if rgba.len() != SIDE * SIDE * 4 {
        return Err("the icon must be 256 by 256 pixels".into());
    }
    let bgra: Vec<u8> = rgba
        .as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| [p[2], p[1], p[0], p[3]])
        .collect();
    replace_icon(exe, |_| Ok(bgra))
}

/// Write the icon that `draw` makes (from the program's biggest icon) as the program's icon.
fn replace_icon(
    exe: &mut Vec<u8>,
    draw: impl FnOnce(&Image) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    let pe = Pe::parse(exe)?;
    let (_, group) = find(exe, &pe, 14, None)?.ok_or("the program has no icon")?;
    let at = {
        let rva = u32_at(exe, group.entry_at)? as usize;
        pe.offset(rva)?
    };
    let count = u16_at(exe, at + 4)? as usize;
    if count == 0 {
        return Err("the program has no icon".into());
    }
    // the biggest of its icons, and the id of the first (the one that is replaced)
    let mut best: Option<(usize, u32)> = None;
    for i in 0..count {
        let e = at + 6 + 14 * i;
        let w = match exe.get(e) {
            Some(0) => 256,
            Some(w) => *w as usize,
            None => return Err("the icon list ends too early".into()),
        };
        let id = u16_at(exe, e + 12)? as u32;
        if best.is_none_or(|(bw, _)| w > bw) {
            best = Some((w, id));
        }
    }
    let first_id = u16_at(exe, at + 6 + 12)? as u32;
    let (_, biggest_id) = best.unwrap();
    let (_, biggest) = find(exe, &pe, 3, Some(biggest_id))?.ok_or("an icon is missing")?;
    let source = decode_icon(leaf_data(exe, &pe, &biggest)?)?;
    let (_, target) = find(exe, &pe, 3, Some(first_id))?.ok_or("an icon is missing")?;
    let bitmap = icon_bitmap(&draw(&source)?);

    // a new section at the end for the bitmap
    let va = append_section(exe, &pe, b".icon", &bitmap)?;

    // the icon resource points at it, and the list holds that one icon
    put_u32(exe, target.entry_at, va as u32)?;
    put_u32(exe, target.entry_at + 4, bitmap.len() as u32)?;
    put_u16(exe, at + 4, 1)?;
    let e = at + 6;
    exe[e..e + 4].copy_from_slice(&[0, 0, 0, 0]); // 256 x 256, no palette
    put_u16(exe, e + 4, 1)?; // planes
    put_u16(exe, e + 6, 32)?; // bits
    put_u32(exe, e + 8, bitmap.len() as u32)?;
    put_u16(exe, e + 12, first_id as u16)?;
    put_u32(exe, group.entry_at + 4, 20)?;
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A small program with one icon (2x2, 8 bits, a red pixel and three see-through ones) and
    /// the resource tree around it, laid out like a real one.
    pub(crate) fn program() -> Vec<u8> {
        let mut d = vec![0u8; 0x400];
        d[0..2].copy_from_slice(b"MZ");
        d[0x3c..0x40].copy_from_slice(&0x40u32.to_le_bytes());
        d[0x40..0x44].copy_from_slice(b"PE\0\0");
        d[0x46..0x48].copy_from_slice(&1u16.to_le_bytes()); // one section
        d[0x54..0x56].copy_from_slice(&0xE0u16.to_le_bytes());
        let opt = 0x58;
        d[opt..opt + 2].copy_from_slice(&0x10Bu16.to_le_bytes());
        d[opt + 32..opt + 36].copy_from_slice(&0x1000u32.to_le_bytes());
        d[opt + 36..opt + 40].copy_from_slice(&0x200u32.to_le_bytes());
        d[opt + 56..opt + 60].copy_from_slice(&0x2000u32.to_le_bytes());
        d[opt + 60..opt + 64].copy_from_slice(&0x200u32.to_le_bytes());
        d[opt + 96 + 16..opt + 96 + 20].copy_from_slice(&0x1000u32.to_le_bytes());
        let table = opt + 0xE0;
        d[table..table + 5].copy_from_slice(b".rsrc");
        for (o, v) in [(8, 0x200u32), (12, 0x1000), (16, 0x200), (20, 0x400)] {
            d[table + o..table + o + 4].copy_from_slice(&v.to_le_bytes());
        }
        let mut r = vec![0u8; 0x200];
        let dir = |r: &mut Vec<u8>, at: usize, items: &[(u32, u32)]| {
            r[at + 14..at + 16].copy_from_slice(&(items.len() as u16).to_le_bytes());
            for (i, (n, o)) in items.iter().enumerate() {
                r[at + 16 + 8 * i..at + 20 + 8 * i].copy_from_slice(&n.to_le_bytes());
                r[at + 20 + 8 * i..at + 24 + 8 * i].copy_from_slice(&o.to_le_bytes());
            }
        };
        let sub = 0x8000_0000u32;
        dir(&mut r, 0x00, &[(3, sub | 0x30), (14, sub | 0x48)]); // types
        dir(&mut r, 0x30, &[(1, sub | 0x60)]); // icon ids
        dir(&mut r, 0x48, &[(7, sub | 0x78)]); // group ids
        dir(&mut r, 0x60, &[(1033, 0x90)]); // icon languages
        dir(&mut r, 0x78, &[(1033, 0xa0)]); // group languages
                                            // the data entries: RVA, size
        let icon_data = [
            40u32,
            2,
            4,
            0x0008_0001,
            0,
            0,
            0,
            0,
            0,
            0, // header (2 wide, 4 = 2 x 2 high)
        ];
        let mut bitmap = Vec::new();
        for v in icon_data {
            bitmap.extend_from_slice(&v.to_le_bytes());
        }
        bitmap[12..14].copy_from_slice(&1u16.to_le_bytes());
        bitmap[14..16].copy_from_slice(&8u16.to_le_bytes());
        bitmap[32..36].copy_from_slice(&2u32.to_le_bytes()); // two colours used
        bitmap.extend_from_slice(&[0, 0, 0, 0, 0, 0, 255, 0]); // black, red (b g r pad)
        bitmap.extend_from_slice(&[1, 0, 0, 0]); // the bottom row: a red pixel, a black one
        bitmap.extend_from_slice(&[0, 0, 0, 0]); // the top row
        bitmap.extend_from_slice(&[0x40, 0, 0, 0, 0xC0, 0, 0, 0]); // the mask: bottom, top rows
        let group_at = 0x100;
        let icon_at = 0x140;
        r[0x90..0x94].copy_from_slice(&(0x1000 + icon_at as u32).to_le_bytes());
        r[0x94..0x98].copy_from_slice(&(bitmap.len() as u32).to_le_bytes());
        r[0xa0..0xa4].copy_from_slice(&(0x1000 + group_at as u32).to_le_bytes());
        r[0xa4..0xa8].copy_from_slice(&26u32.to_le_bytes());
        r[group_at + 2..group_at + 4].copy_from_slice(&1u16.to_le_bytes()); // an icon list
        r[group_at + 4..group_at + 6].copy_from_slice(&1u16.to_le_bytes()); // one icon
        r[group_at + 6] = 2; // width
        r[group_at + 7] = 2;
        r[group_at + 10..group_at + 12].copy_from_slice(&1u16.to_le_bytes());
        r[group_at + 12..group_at + 14].copy_from_slice(&8u16.to_le_bytes());
        r[group_at + 14..group_at + 18].copy_from_slice(&(bitmap.len() as u32).to_le_bytes());
        r[group_at + 18..group_at + 20].copy_from_slice(&1u16.to_le_bytes()); // id 1
        r[icon_at..icon_at + bitmap.len()].copy_from_slice(&bitmap);
        d.extend_from_slice(&r);
        d
    }

    #[test]
    fn an_icon_of_a_program_is_read_with_its_colours_and_its_see_through_parts() {
        let exe = program();
        let pe = Pe::parse(&exe).unwrap();
        let (_, leaf) = find(&exe, &pe, 3, Some(1)).unwrap().unwrap();
        let img = decode_icon(leaf_data(&exe, &pe, &leaf).unwrap()).unwrap();
        assert_eq!((img.w, img.h), (2, 2));
        // the bottom-left pixel is colour 1 (red), the others are black; the mask hides
        // the bottom-right pixel
        let at = |x: usize, y: usize| &img.rgba[(y * 2 + x) * 4..(y * 2 + x) * 4 + 4];
        assert_eq!(at(0, 1), [255, 0, 0, 255]);
        assert_eq!(at(0, 0)[3], 0, "the mask of the top row hides the pixel");
    }

    #[test]
    fn the_mac_icon_is_drawn_round_with_a_shadow_and_the_picture_in_the_middle() {
        let img = Image {
            w: 2,
            h: 2,
            rgba: [255, 0, 0, 255].repeat(4),
        };
        let t = mac_tile(&img);
        let px = |x: usize, y: usize| &t[(y * SIDE + x) * 4..(y * SIDE + x) * 4 + 4];
        assert_eq!(px(0, 0)[3], 0, "the corner is clear");
        assert_eq!(px(128, 40)[3], 255, "the tile is solid");
        assert!(
            px(128, 128)[2] > 200 && px(128, 128)[0] < 60,
            "red picture (b g r a)"
        );
        let shadow = px(128, 128 + 110);
        assert!(
            shadow[3] > 0 && shadow[3] < 255,
            "a soft shadow under the tile"
        );
    }

    #[test]
    fn the_new_icon_is_a_256_pixel_bitmap_with_room_for_the_mask() {
        let b = icon_bitmap(&vec![7u8; SIDE * SIDE * 4]);
        assert_eq!(b.len(), 40 + SIDE * SIDE * 4 + SIDE * SIDE / 8);
        assert_eq!(u32::from_le_bytes(b[8..12].try_into().unwrap()), 512);
        assert_eq!(u16::from_le_bytes(b[14..16].try_into().unwrap()), 32);
    }

    #[test]
    fn a_program_gets_one_new_icon_in_a_new_section_and_nothing_else_moves() {
        let mut exe = program();
        let before = exe.clone();
        mac_style_icon(&mut exe).unwrap();
        let pe = Pe::parse(&exe).unwrap();
        assert_eq!(pe.sections.len(), 2);
        let new = &pe.sections[1];
        assert_eq!(new.va, 0x2000);
        assert_eq!(new.raw_ptr % 0x200, 0);
        assert_eq!(exe.len(), new.raw_ptr + new.raw_size);
        // the old part of the file is as it was, but for the headers and the two entries
        assert_eq!(&exe[0x400..0x400 + 0x90], &before[0x400..0x400 + 0x90]);
        // the icon resource points to the new bitmap, which is the new picture
        let (_, leaf) = find(&exe, &pe, 3, Some(1)).unwrap().unwrap();
        assert_eq!(u32_at(&exe, leaf.entry_at).unwrap() as usize, new.va);
        let data = leaf_data(&exe, &pe, &leaf).unwrap();
        let img = decode_icon(data).unwrap();
        assert_eq!((img.w, img.h), (SIDE, SIDE));
        // and the list has the one icon, 256 by 256 at 32 bits
        let (_, group) = find(&exe, &pe, 14, None).unwrap().unwrap();
        let g = pe
            .offset(u32_at(&exe, group.entry_at).unwrap() as usize)
            .unwrap();
        assert_eq!(u16_at(&exe, g + 4).unwrap(), 1);
        assert_eq!(&exe[g + 6..g + 8], [0, 0]);
        assert_eq!(u16_at(&exe, g + 12).unwrap(), 32);
    }

    #[test]
    fn what_is_not_a_program_or_has_no_icon_is_refused() {
        assert!(mac_style_icon(&mut b"hello".to_vec()).is_err());
        let mut exe = program();
        // no resources: the directory points at nothing
        exe[0x58 + 96 + 16..0x58 + 96 + 20].copy_from_slice(&0u32.to_le_bytes());
        assert!(mac_style_icon(&mut exe).is_err());
    }

    #[test]
    fn a_made_icon_replaces_the_programs_own() {
        let mut exe = program();
        let mut rgba = vec![0u8; SIDE * SIDE * 4];
        for p in rgba.as_chunks_mut::<4>().0 {
            p.copy_from_slice(&[10, 20, 30, 255]);
        }
        set_icon(&mut exe, &rgba).unwrap();
        let pe = Pe::parse(&exe).unwrap();
        let (_, leaf) = find(&exe, &pe, 3, Some(1)).unwrap().unwrap();
        let img = decode_icon(leaf_data(&exe, &pe, &leaf).unwrap()).unwrap();
        assert_eq!(
            &img.rgba[..4],
            [10, 20, 30, 255],
            "the colours keep their order"
        );
        assert!(set_icon(&mut program(), &[0; 4]).is_err());
    }

    /// A program of one section that holds a text and the code that pushes its address.
    fn program_with_title() -> Vec<u8> {
        let mut d = program();
        // one more section's worth of room is already in the headers; the data goes in the
        // resource section's file space (nothing reads it as resources here)
        let section_va = 0x1000usize;
        let at = 0x400 + 0x1c0; // in the file
        d[at..at + 10].copy_from_slice(b"Road Rash\0");
        d[at - 1] = 0;
        let address = (0x40_0000 + section_va + 0x1c0) as u32;
        d[0x400 + 0x1f0] = 0x68;
        d[0x400 + 0x1f1..0x400 + 0x1f5].copy_from_slice(&address.to_le_bytes());
        d[0x58 + 28..0x58 + 32].copy_from_slice(&0x40_0000u32.to_le_bytes());
        d
    }

    #[test]
    fn the_title_a_program_pushes_is_replaced_in_a_section_of_its_own() {
        let mut exe = program_with_title();
        set_window_title(&mut exe, "Road Rash", b"Road Rash \x97 Wine").unwrap();
        let pe = Pe::parse(&exe).unwrap();
        let new = pe.sections.last().unwrap();
        assert_eq!(new.va, 0x2000);
        let pushed = u32_at(&exe, 0x400 + 0x1f1).unwrap() as usize;
        assert_eq!(pushed, 0x40_0000 + new.va);
        assert_eq!(&exe[new.raw_ptr..new.raw_ptr + 16], b"Road Rash \x97 Wine");
        assert_eq!(exe[new.raw_ptr + 16], 0, "the text ends");
    }

    #[test]
    fn a_title_that_is_not_there_or_not_pushed_once_is_refused() {
        let mut exe = program_with_title();
        assert!(set_window_title(&mut exe, "Other", b"x").is_err());
        let mut twice = program_with_title();
        let address = (0x40_0000 + 0x1000 + 0x1c0) as u32;
        twice[0x400 + 0x1a0] = 0x68;
        twice[0x400 + 0x1a1..0x400 + 0x1a5].copy_from_slice(&address.to_le_bytes());
        assert!(set_window_title(&mut twice, "Road Rash", b"x").is_err());
    }

    /// With the real program of the game (`VURETRO_ROADRASH_EXE`), writing the result where
    /// `VURETRO_ICON_OUT` says: `cargo test -p vuretro-library the_real_program -- --ignored`.
    #[test]
    #[ignore]
    fn the_real_program_gets_its_icon() {
        let mut exe = std::fs::read(std::env::var("VURETRO_ROADRASH_EXE").unwrap()).unwrap();
        mac_style_icon(&mut exe).unwrap();
        std::fs::write(std::env::var("VURETRO_ICON_OUT").unwrap(), exe).unwrap();
    }
}
