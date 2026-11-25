//! The hardware side (macOS): one watcher thread that finds pads and one reader thread per pad.

use crate::{parse, ProPad, ProState, NAME, PAD_ID, PRODUCT_ID, VENDOR_ID};
use hidapi::{BusType, HidApi, HidDevice};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct Slot {
    state: Arc<Mutex<ProState>>,
    alive: Arc<AtomicBool>,
}

type Slots = Arc<Mutex<HashMap<String, Slot>>>;

/// Every Pro Controller that is connected right now, kept up to date in the background.
pub struct ProCons {
    slots: Slots,
}

impl Default for ProCons {
    fn default() -> Self {
        Self::new()
    }
}

impl ProCons {
    pub fn new() -> Self {
        let slots: Slots = Arc::default();
        let watch = slots.clone();
        let spawned = std::thread::Builder::new()
            .name("procon-watch".into())
            .spawn(move || watch_loop(watch));
        if let Err(e) = spawned {
            log::warn!("procon: cannot start the watcher: {e}");
        }
        ProCons { slots }
    }

    pub fn pads(&self) -> Vec<ProPad> {
        let slots = self.slots.lock().unwrap();
        let mut paths: Vec<&String> = slots.keys().collect();
        paths.sort(); // a stable order, so player 1 stays player 1
        paths
            .into_iter()
            .filter(|p| slots[*p].alive.load(Ordering::Relaxed))
            .map(|p| ProPad {
                id: PAD_ID.to_string(),
                name: NAME.to_string(),
                state: *slots[p].state.lock().unwrap(),
            })
            .collect()
    }
}

fn watch_loop(slots: Slots) {
    let mut api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            log::warn!("procon: HID is not available: {e}");
            return;
        }
    };
    loop {
        if api.refresh_devices().is_ok() {
            let found: Vec<(String, std::ffi::CString, bool)> = api
                .device_list()
                .filter(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PRODUCT_ID)
                .map(|d| {
                    let usb = matches!(d.bus_type(), BusType::Usb);
                    (
                        d.path().to_string_lossy().into_owned(),
                        d.path().to_owned(),
                        usb,
                    )
                })
                .collect();
            let mut map = slots.lock().unwrap();
            map.retain(|_, s| s.alive.load(Ordering::Relaxed)); // forget unplugged pads
            for (key, path, usb) in found {
                if map.contains_key(&key) {
                    continue;
                }
                match api.open_path(&path) {
                    Ok(dev) => {
                        log::info!(
                            "procon: reading {NAME} over {}",
                            if usb {
                                "USB (2.4G dongle or cable)"
                            } else {
                                "Bluetooth"
                            }
                        );
                        map.insert(key, start_reader(dev, usb));
                    }
                    Err(e) => log::debug!("procon: cannot open the pad: {e}"),
                }
            }
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

/// Over USB (a cable, or a 2.4G dongle) the pad stays silent until it has been greeted with
/// the USB handshake; it also stops after a while unless told not to time out.
fn usb_handshake(dev: &HidDevice) {
    for cmd in [0x02u8, 0x03, 0x02, 0x04] {
        // 0x02 hand-shake, 0x03 switch to 3 Mbaud, 0x04 no timeout
        if let Err(e) = dev.write(&[0x80, cmd]) {
            log::debug!("procon: USB command {cmd:#04x} not sent: {e}");
        }
        std::thread::sleep(Duration::from_millis(30));
    }
}

/// Asks the pad to send full reports (a pad that already does ignores it).
fn say_hello(dev: &HidDevice) {
    let mut out = [0u8; 49];
    out[0] = 0x01; // output report: rumble + subcommand
    out[2..10].copy_from_slice(&[0x00, 0x01, 0x40, 0x40, 0x00, 0x01, 0x40, 0x40]); // no rumble
    out[10] = 0x03; // set the input report mode
    out[11] = 0x30; // to: standard full
    if let Err(e) = dev.write(&out) {
        log::debug!("procon: hello not sent: {e}");
    }
}

fn start_reader(dev: HidDevice, usb: bool) -> Slot {
    let state = Arc::new(Mutex::new(ProState::default()));
    let alive = Arc::new(AtomicBool::new(true));
    let (st, al) = (state.clone(), alive.clone());
    let spawned = std::thread::Builder::new()
        .name("procon-read".into())
        .spawn(move || {
            if usb {
                usb_handshake(&dev);
            }
            say_hello(&dev);
            let mut buf = [0u8; 128];
            let mut silent = 0;
            loop {
                match dev.read_timeout(&mut buf, 200) {
                    Ok(0) => {
                        silent += 1;
                        if silent == 10 {
                            if usb {
                                usb_handshake(&dev);
                            }
                            say_hello(&dev); // 2 s of nothing: ask again
                            silent = 0;
                        }
                    }
                    Ok(n) => {
                        silent = 0;
                        if let Some(s) = parse(&buf[..n]) {
                            *st.lock().unwrap() = s;
                        }
                    }
                    Err(e) => {
                        log::info!("procon: pad gone: {e}");
                        break;
                    }
                }
            }
            al.store(false, Ordering::Relaxed);
        });
    if spawned.is_err() {
        alive.store(false, Ordering::Relaxed);
    }
    Slot { state, alive }
}
