//! Prints what the Pro Controllers report, to check a pad by hand:
//! `cargo run -p vuretro-procon --example watch`
use std::time::Duration;

fn main() {
    let pads = vuretro_procon::ProCons::new();
    for _ in 0..80 {
        for p in pads.pads() {
            println!(
                "{} pressed={:?} left=({:.2},{:.2})",
                p.name,
                p.state.pressed(),
                p.state.left.0,
                p.state.left.1
            );
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
