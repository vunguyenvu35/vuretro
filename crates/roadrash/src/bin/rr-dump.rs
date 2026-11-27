//! Writes pictures from a Road Rash disc folder as PNG files, to look at what the readers make.
//! Usage: rr-dump <ROADRASH folder> <output folder>
use std::path::Path;
use vuretro_roadrash::image::{colour, decode_bob, sheet, write_png};
use vuretro_roadrash::palette::{Order, Palette};
use vuretro_roadrash::{bob, cel, rsc};

fn read(path: std::path::PathBuf) -> Result<Vec<u8>, String> {
    std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (Some(disc), Some(out)) = (args.next(), args.next()) else {
        return Err("usage: rr-dump <ROADRASH folder> <output folder>".into());
    };
    let (disc, out) = (Path::new(&disc), Path::new(&out));
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let palette = Palette::parse(&read(disc.join("DATA/PALETTE.RAW"))?, Order::Rgbx)?;

    // a whole picture
    for (file, width) in [("MATTE", 640), ("BIGCLOUD", 512)] {
        let bytes = read(disc.join(format!("IMAGES/{file}.BOB")))?;
        write_png(
            &decode_bob(&bytes, width, &palette)?,
            &out.join(format!("{file}.png")),
        )?;
    }

    // the sprites of the riders
    for set in ["HIBOB", "HISHADOW", "HICOP"] {
        let Ok(bytes) = std::fs::read(disc.join(format!("DATA/BIKERS/{set}.DAT"))) else {
            continue;
        };
        let frames = bob::read_frames(&bytes)?;
        println!("{set}: {} frames", frames.len());
        let shown: Vec<_> = frames
            .iter()
            .take(48)
            .map(|f| colour(&f.indices, f.width, &palette))
            .filter(|p| p.height > 0)
            .collect();
        write_png(&sheet(&shown, 8), &out.join(format!("{set}.png")))?;
    }

    // the pictures of a course
    for course in ["NAPA"] {
        let file = read(disc.join(format!("DATA/COURSES/{course}.CRS")))?;
        let mut textures = Vec::new();
        for e in rsc::entries(&file)?.iter().filter(|e| e.tag == "CEL") {
            if let Some(t) = cel::read_texture(e.bytes(&file))? {
                textures.push(colour(&t.indices, t.width, &palette));
            }
        }
        println!("{course}: {} textures", textures.len());
        if !textures.is_empty() {
            write_png(
                &sheet(&textures, 8),
                &out.join(format!("{course}_textures.png")),
            )?;
        }
    }
    println!("written to {}", out.display());
    Ok(())
}
