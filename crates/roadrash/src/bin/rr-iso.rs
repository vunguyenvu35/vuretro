//! Lists the files of a disc image; with a folder that holds the same files unpacked it also
//! checks that every file reads the same from both. Usage: rr-iso <disc.iso> [folder]
use std::path::Path;
use vuretro_roadrash::disc::Disc;
use vuretro_roadrash::iso::Iso;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("usage: rr-iso <disc.iso> [folder]")?;
    let iso = Iso::open(Path::new(&path))?;
    let files = iso.files()?;
    println!("{} files", files.len());
    let mut tops: Vec<&str> = files
        .iter()
        .map(|f| f.split('/').next().unwrap_or(""))
        .collect();
    tops.dedup();
    println!("at the top: {}", tops.join(" "));
    if let Some(folder) = args.next() {
        let (mut same, mut different, mut missing) = (0, 0, 0);
        // the game's own files are the ones under ROADRASH/ (the disc also holds an installer)
        let disc = Disc::open(Path::new(&path))?;
        for f in files.iter().filter_map(|f| f.strip_prefix("ROADRASH/")) {
            let from_iso = disc.read(f)?;
            match std::fs::read(Path::new(&folder).join(f)) {
                Ok(unpacked) if unpacked == from_iso => same += 1,
                Ok(_) => {
                    different += 1;
                    println!("differs: {f}");
                }
                Err(_) => missing += 1,
            }
        }
        println!("same {same}, different {different}, not in the folder {missing}");
    }
    Ok(())
}
