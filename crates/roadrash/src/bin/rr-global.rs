//! Draws the pictures (`CEL ` resources) of `DATA/GLOBAL.RSC` that are indexed textures.
//! Usage: rr-global <disc.iso or folder> <picture.png>
use std::path::Path;
use vuretro_roadrash::disc::Disc;
use vuretro_roadrash::image::{colour, sheet, write_png};
use vuretro_roadrash::palette::{Order, Palette};
use vuretro_roadrash::{cel, rsc};

fn main() -> Result<(), String> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let [disc, out] = a.as_slice() else {
        return Err("usage: rr-global <disc> <picture.png>".into());
    };
    let disc = Disc::open(Path::new(disc))?;
    let palette = Palette::parse(&disc.read("DATA/PALETTE.RAW")?, Order::Rgbx)?;
    let file = disc.read("DATA/GLOBAL.RSC")?;
    let mut pictures = Vec::new();
    for e in rsc::entries(&file)?.iter().filter(|e| e.tag == "CEL") {
        match cel::read_texture(e.bytes(&file)) {
            Ok(Some(t)) => {
                println!("CEL {}: {}x{}", e.id, t.width, t.height);
                pictures.push(colour(&t.indices, t.width, &palette));
            }
            Ok(None) => println!("CEL {}: not indexed", e.id),
            Err(error) => println!("CEL {}: {error}", e.id),
        }
    }
    if pictures.is_empty() {
        return Ok(());
    }
    write_png(&sheet(&pictures, 8), Path::new(out))
}
