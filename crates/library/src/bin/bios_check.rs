//! vuretro-bios-check <folder> [system_dir]
//!
//! Tells what each file of a folder is as a BIOS (which system and file name, and whether its
//! content is an accepted dump), without copying anything, then what every system would
//! report with those files in place. Use it on the BIOS files you keep (`docs/bios`) to see
//! that Vuretro recognises them before you add them to the app.

use std::path::{Path, PathBuf};
use vuretro_core::registry;
use vuretro_library::bios;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let dir = PathBuf::from(
        args.next()
            .ok_or_else(|| anyhow::anyhow!("usage: vuretro-bios-check <folder> [system_dir]"))?,
    );
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .filter(|p| {
            !p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        })
        .filter(|p| !p.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")))
        .collect();
    files.sort();

    println!("Files in {}:", dir.display());
    let mut good = 0;
    for f in &files {
        let name = f.file_name().unwrap().to_string_lossy().into_owned();
        let data = std::fs::read(f)?;
        match bios::identify(&data, &name) {
            Ok(i) if i.verified => {
                good += 1;
                let how = if i.spec.md5.is_empty() {
                    "by name"
                } else {
                    "MD5 matches"
                };
                println!(
                    "  ok       {name}  ->  {} / {} ({how})",
                    i.system.name, i.spec.file
                );
            }
            Ok(i) => println!(
                "  BAD MD5  {name}  looks like {} / {} but MD5 {} is not an accepted dump",
                i.system.name, i.spec.file, i.md5
            ),
            Err(_) => println!("  unknown  {name}  is not a BIOS file Vuretro looks for"),
        }
    }

    // what the systems would say with these files as their system folder
    let system_dir: &Path = &args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| dir.clone());
    println!("\nSystems with the BIOS files in {}:", system_dir.display());
    for s in registry::systems()
        .iter()
        .filter(|s| s.enabled && !s.bios.is_empty())
    {
        let st = bios::check(s, system_dir);
        let ok = st.iter().filter(|b| b.state == bios::BiosState::Ok).count();
        let ready = bios::ready(s, system_dir);
        println!(
            "  {:<26} {}  ({} of {} files found)",
            s.name,
            if ready { "ready" } else { "BIOS missing" },
            ok,
            st.len()
        );
        for b in &st {
            let mark = match &b.state {
                bios::BiosState::Ok => "ok ",
                bios::BiosState::Missing => {
                    if b.required {
                        "MISSING"
                    } else {
                        "-  "
                    }
                }
                bios::BiosState::BadHash { .. } => "BAD",
            };
            println!(
                "      {mark:<8} {}{}",
                b.file,
                if b.required { " (required)" } else { "" }
            );
        }
    }
    println!("\n{good} of {} files recognised.", files.len());
    Ok(())
}
