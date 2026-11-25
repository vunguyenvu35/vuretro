//! Vibrates the first Pro Controller, to check by hand:
//! `cargo run -p vuretro-procon --example rumble`
use std::time::Duration;

fn main() {
    let pads = vuretro_procon::ProCons::new();
    std::thread::sleep(Duration::from_secs(2)); // let the pad be found
    if pads.pads().is_empty() {
        println!("no Pro Controller found");
        return;
    }
    for (label, strong, weak) in [
        ("both motors", true, true),
        ("strong (left) motor only", true, false),
        ("weak (right) motor only", false, true),
    ] {
        println!("{label}: on");
        pads.rumble(0, strong, weak);
        std::thread::sleep(Duration::from_secs(1));
        pads.rumble(0, false, false);
        println!("  off");
        std::thread::sleep(Duration::from_secs(1));
    }
}
