//! Unpacks the families of `DATA/FAMILIES.RSC` and checks every one against its CRC.
//! Usage: rr-families <disc.iso or folder> [id to show the chunks of]
//!        rr-families <disc.iso or folder> sprites <id> <category> <picture.png>
//! The second form draws the pictures of a family's category (frame 2 of each selector).
use std::path::Path;
use vuretro_roadrash::disc::Disc;
use vuretro_roadrash::{explode, rsc};

fn sprites(disc: &Disc, args: &[String]) -> Result<(), String> {
    use vuretro_roadrash::family::Family;
    use vuretro_roadrash::image::{colour, sheet, write_png};
    use vuretro_roadrash::palette::{Order, Palette};
    if args.first().map(String::as_str) == Some("census") {
        // how many pictures every family has in each category
        let file = disc.read("DATA/FAMILIES.RSC")?;
        for e in rsc::entries(&file)?.iter().filter(|e| e.tag == "FAM") {
            let Ok(family) = Family::parse(e.bytes(&file)) else {
                continue;
            };
            let counts: Vec<usize> = (0..8)
                .map(|c| {
                    (0..64)
                        .filter(|&s| family.sprite(c, s, 2).is_some())
                        .count()
                })
                .collect();
            if counts.iter().any(|&n| n > 0) {
                println!("family {:>3}: {counts:?}", e.id);
            }
        }
        return Ok(());
    }
    let [id, category, out] = args else {
        return Err("usage: rr-families <disc> sprites <id> <category> <picture.png>".into());
    };
    let (id, category): (u32, usize) = (
        id.parse().map_err(|_| "id is a number")?,
        category.parse().map_err(|_| "category is a number")?,
    );
    let palette = Palette::parse(&disc.read("DATA/PALETTE.RAW")?, Order::Rgbx)?;
    let file = disc.read("DATA/FAMILIES.RSC")?;
    let entry = rsc::entries(&file)?
        .into_iter()
        .find(|e| e.tag == "FAM" && e.id == id)
        .ok_or("no such family")?;
    let family = Family::parse(entry.bytes(&file))?;
    let mut pictures = Vec::new();
    for selector in 0..64 {
        if let Some(s) = family.sprite(category, selector, 2) {
            pictures.push(colour(&s.texture.pixels, s.texture.width, &palette));
        }
    }
    println!(
        "family {id} category {category}: {} pictures",
        pictures.len()
    );
    if pictures.is_empty() {
        return Ok(());
    }
    write_png(&sheet(&pictures, 8), Path::new(out))
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let disc = Disc::open(Path::new(
        &args.next().ok_or("usage: rr-families <disc> [id]")?,
    ))?;
    let rest: Vec<String> = args.collect();
    if rest.first().map(String::as_str) == Some("sprites") {
        return sprites(&disc, &rest[1..]);
    }
    let show: Option<u32> = rest.first().and_then(|a| a.parse().ok());
    let file = disc.read("DATA/FAMILIES.RSC")?;
    let (mut ok, mut empty, mut bad, mut bytes) = (0, 0, 0, 0usize);
    for e in rsc::entries(&file)?.iter().filter(|e| e.tag == "FAM") {
        if e.size == 0 {
            empty += 1;
            continue;
        }
        match explode::unpack(e.bytes(&file)) {
            Ok(data) => {
                ok += 1;
                bytes += data.len();
                if show == Some(e.id) {
                    let count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
                    println!("family {}: {} bytes, {count} chunks", e.id, data.len());
                    for i in 0..count.min(40) {
                        let at = u32::from_le_bytes(data[4 + i * 4..8 + i * 4].try_into().unwrap())
                            as usize;
                        let tag: String =
                            data[at..at + 4].iter().rev().map(|&b| b as char).collect();
                        let len = u32::from_le_bytes(data[at + 4..at + 8].try_into().unwrap());
                        println!(
                            "  chunk {i}: at {at}, tag {tag:?} (disk {:?}), length {len}",
                            String::from_utf8_lossy(&data[at..at + 4])
                        );
                    }
                }
            }
            Err(error) => {
                bad += 1;
                println!("family {}: {error}", e.id);
            }
        }
    }
    println!("{ok} unpacked ({bytes} bytes), {empty} empty, {bad} failed");
    Ok(())
}
