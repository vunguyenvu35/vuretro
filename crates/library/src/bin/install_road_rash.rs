//! vuretro-install-road-rash <disc image or unpacked folder>
//! Does what the Install button of the Native games page does: fetches the source of the native
//! port, builds it and reads the game's data from the disc, into the data folder of the app.
//! Everything the build prints goes to the terminal.

use vuretro_core::ports::NativeInstaller;
use vuretro_library::adapters::{system_paths, RoadRashInstaller};

fn main() -> anyhow::Result<()> {
    let disc = std::env::args().nth(1).ok_or_else(|| {
        anyhow::anyhow!("usage: vuretro-install-road-rash <disc image or folder>")
    })?;
    let paths = system_paths().ok_or_else(|| anyhow::anyhow!("no data folder for this user"))?;
    let installer = RoadRashInstaller::new(paths);
    let missing = installer.missing_tools();
    if !missing.is_empty() {
        println!("missing first: {}", missing.join(", "));
    }
    installer
        .install_road_rash(disc.as_ref(), &|line| println!("{line}"))
        .map_err(|e| anyhow::anyhow!(e))
}
