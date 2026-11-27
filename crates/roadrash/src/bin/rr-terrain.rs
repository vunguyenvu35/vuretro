//! Says how much of a course's ground is hills and how many of the colours of the hills come
//! from the pictures of the disc. Usage: rr-terrain <disc.iso or folder> [course]
use std::path::Path;
use vuretro_roadrash::course;
use vuretro_roadrash::disc::Disc;
use vuretro_roadrash::terrain;

fn main() -> Result<(), String> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let disc = Disc::open(Path::new(
        a.first().ok_or("usage: rr-terrain <disc> [course]")?,
    ))?;
    let name = a.get(1).map_or("NAPA", String::as_str).to_uppercase();
    let course = course::parse(&disc.read(&format!("DATA/COURSES/{name}.CRS"))?)?;
    if a.get(2).map(String::as_str) == Some("debug") {
        use vuretro_roadrash::{family::Family, rsc};
        let file = disc.read("DATA/FAMILIES.RSC")?;
        let entries = rsc::entries(&file)?;
        let mut shown = std::collections::BTreeSet::new();
        for (n, s) in course
            .sections
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.hills.is_empty())
            .take(4)
        {
            println!("section {n}: events {:?}", s.family_events);
            for r in s.hills.iter().take(2) {
                for sel in r.selectors {
                    for e in s
                        .family_events
                        .iter()
                        .filter(|e| e.kind == 1 || e.kind == 3)
                    {
                        if e.slot != sel >> 6 || !shown.insert((e.family, sel & 63)) {
                            continue;
                        }
                        let Some(entry) = entries
                            .iter()
                            .find(|x| x.tag == "FAM" && x.id == e.family as u32)
                        else {
                            continue;
                        };
                        let Ok(f) = Family::parse(entry.bytes(&file)) else {
                            continue;
                        };
                        println!(
                            "  family {} selector {}: category 1 -> {}",
                            e.family,
                            sel & 63,
                            f.describe(1, (sel & 63) as usize)
                        );
                    }
                }
            }
        }
        return Ok(());
    }
    let palette = vuretro_roadrash::palette::Palette::parse(
        &disc.read("DATA/PALETTE.RAW")?,
        vuretro_roadrash::palette::Order::Rgbx,
    )?;
    let ground = terrain::load(&disc, &course, &palette)?;
    let hills = ground.slices.iter().flatten().count();
    println!(
        "{name}: {hills} of {} samples are hills; {} kinds of face coloured from pictures of the disc, {} plain green",
        ground.slices.len(),
        ground.from_pictures,
        ground.plain
    );
    Ok(())
}
