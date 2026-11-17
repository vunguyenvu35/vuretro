//! Gamepad watcher: keeps the shared pad list current and tells the UI about connects
//! and button presses so it can identify pads and capture bindings.

use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::Emitter;
use vuretro_controls::PadInfo;

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
pub fn spawn(app: tauri::AppHandle, pads: Arc<Mutex<Vec<PadInfo>>>) {
    std::thread::spawn(move || {
        let Ok(mut g) = gilrs::Gilrs::new() else {
            return;
        };
        let snapshot = |g: &gilrs::Gilrs| -> Vec<PadInfo> {
            g.gamepads()
                .filter(|(_, p)| p.is_connected())
                .map(|(_, p)| PadInfo {
                    id: uuid_hex(p.uuid()),
                    name: p.name().to_string(),
                })
                .collect()
        };
        *pads.lock().unwrap() = snapshot(&g);
        loop {
            while let Some(ev) = g.next_event() {
                match ev.event {
                    gilrs::EventType::Connected | gilrs::EventType::Disconnected => {
                        let list = snapshot(&g);
                        *pads.lock().unwrap() = list.clone();
                        let _ = app.emit("pads-changed", list);
                    }
                    gilrs::EventType::ButtonPressed(btn, _) => {
                        let pad = g.gamepad(ev.id);
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
            std::thread::sleep(Duration::from_millis(8));
        }
    });
}
