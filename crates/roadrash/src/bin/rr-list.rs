//! Lists the resources of a `.RSC` container. Usage: rr-list <file.RSC>
use vuretro_roadrash::rsc;

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("usage: rr-list <file.RSC>")?;
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    for e in rsc::entries(&bytes)? {
        println!(
            "{:<5} {:>5} offset {:>9} size {:>9}",
            e.tag, e.id, e.offset, e.size
        );
    }
    Ok(())
}
