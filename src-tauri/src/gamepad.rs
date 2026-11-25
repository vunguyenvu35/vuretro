//! Gamepad watcher: keeps the shared pad list current and tells the UI about connects
//! and button presses so it can identify pads and capture bindings.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Emitter;
use vuretro_controls::padnames::{self, PadNames};
use vuretro_controls::PadInfo;
use vuretro_procon::ProCons;

#[derive(Serialize, Clone)]
struct PadButton {
    id: String,
    name: String,
    button: String,
}

fn uuid_hex(u: [u8; 16]) -> String {
    u.iter().map(|b| format!("{b:02x}")).collect()
}

/// Runs on its own thread (gilrs must live where it was created).
pub fn spawn(
    app: tauri::AppHandle,
    pads: Arc<Mutex<Vec<PadInfo>>>,
    names: Arc<Mutex<PadNames>>,
    dongle_waiting: Arc<AtomicBool>,
) {
    std::thread::spawn(move || {
        let Ok(mut g) = gilrs::Gilrs::new() else {
            return;
        };
        // On macOS a Pro Controller gives gilrs no buttons: we read it ourselves, and the copy
        // gilrs lists is left out.
        let pro = ProCons::new();
        let snapshot = |g: &gilrs::Gilrs, pro_pads: &[vuretro_procon::ProPad]| -> Vec<PadInfo> {
            let mut list: Vec<PadInfo> = g
                .gamepads()
                .filter(|(_, p)| p.is_connected())
                .filter(|(_, p)| pro_pads.is_empty() || !vuretro_procon::is_procon_uuid(p.uuid()))
                .map(|(_, p)| PadInfo {
                    id: uuid_hex(p.uuid()),
                    name: p.name().to_string(),
                })
                .collect();
            list.extend(pro_pads.iter().map(|p| PadInfo {
                id: p.id.clone(),
                name: p.name.clone(),
            }));
            list
        };
        let mut pro_pads = pro.pads();
        *pads.lock().unwrap() = snapshot(&g, &pro_pads);
        // what each Pro Controller had down at the last look, to report new presses only
        let mut was_down: Vec<Vec<gilrs::Button>> = Vec::new();
        let mut dongle_was = false;
        loop {
            let mut changed = false;
            while let Some(ev) = g.next_event() {
                match ev.event {
                    gilrs::EventType::Connected | gilrs::EventType::Disconnected => changed = true,
                    gilrs::EventType::ButtonPressed(btn, _) => {
                        let pad = g.gamepad(ev.id);
                        if !pro_pads.is_empty() && vuretro_procon::is_procon_uuid(pad.uuid()) {
                            continue; // it is the Pro Controller we read ourselves
                        }
                        let _ = app.emit(
                            "pad-button",
                            PadButton {
                                id: uuid_hex(pad.uuid()),
                                name: pad.name().to_string(),
                                button: format!("{btn:?}"),
                            },
                        );
                    }
                    _ => {}
                }
            }
            let now = pro.pads();
            if now.len() != pro_pads.len() {
                changed = true;
            }
            for (i, p) in now.iter().enumerate() {
                let down = p.state.pressed();
                let before = was_down.get(i).cloned().unwrap_or_default();
                for btn in down.iter().filter(|b| !before.contains(b)) {
                    let _ = app.emit(
                        "pad-button",
                        PadButton {
                            id: p.id.clone(),
                            name: p.name.clone(),
                            button: format!("{btn:?}"),
                        },
                    );
                }
            }
            was_down = now.iter().map(|p| p.state.pressed()).collect();
            pro_pads = now;
            // a receiver with no awake pad behind it: the screen says to press Home
            let dongle = pro.dongle_waiting();
            if dongle != dongle_was {
                dongle_was = dongle;
                dongle_waiting.store(dongle, Ordering::Relaxed);
                let _ = app.emit("dongle-waiting", dongle);
            }
            if changed {
                let list = snapshot(&g, &pro_pads);
                *pads.lock().unwrap() = list.clone();
                // the UI is told the pads by the names the user gave them
                let _ = app.emit(
                    "pads-changed",
                    padnames::apply(&list, &names.lock().unwrap()),
                );
            }
            std::thread::sleep(Duration::from_millis(8));
        }
    });
}
