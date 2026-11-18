//! vuretro-scan <rom_dir> [system_dir]  → prints games and BIOS status as JSON.

use vuretro_core::registry;
use vuretro_library::{bios, scan};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let root = args
        .next()
        .ok_or_else(|| anyhow::anyhow!("usage: vuretro-scan <rom_dir> [system_dir]"))?;
    let games = scan::scan(root.as_ref());
    let bios: Vec<_> = match args.next() {
        Some(sys_dir) => registry::systems()
            .iter()
            .filter(|s| s.enabled)
            .flat_map(|s| bios::check(s, sys_dir.as_ref()))
            .collect(),
        None => Vec::new(),
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({ "games": games, "bios": bios }))?
    );
    Ok(())
}
