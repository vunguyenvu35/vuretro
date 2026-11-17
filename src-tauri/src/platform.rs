//! Asking the operating system to open things.

use std::path::Path;
use std::process::Command;

/// Opens a folder or web page with the system's default handler.
pub fn open_external(target: &str) {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    let _ = Command::new(program).arg(target).spawn();
}

/// Show a file in Finder / Explorer / the file manager, selected where the system can.
pub fn reveal(path: &str) {
    let p = Path::new(path);
    if cfg!(target_os = "macos") {
        let _ = Command::new("open").arg("-R").arg(p).spawn();
    } else if cfg!(windows) {
        let _ = Command::new("explorer")
            .arg(format!("/select,{path}"))
            .spawn();
    } else {
        open_external(&p.parent().unwrap_or(p).to_string_lossy());
    }
}
