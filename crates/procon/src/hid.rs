//! The hardware side (macOS): one watcher thread that finds pads and one reader thread per pad.

use crate::{
    is_idle_receiver, parse, rumble_report, ProPad, ProState, NAME, PAD_ID, PRODUCT_ID, VENDOR_ID,
};
use hidapi::{BusType, HidApi, HidDevice};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Slot {
    state: Arc<Mutex<ProState>>,
    alive: Arc<AtomicBool>,
    /// Tells the reader thread to vibrate: (strong motor, weak motor).
    rumble: Sender<(bool, bool)>,
}

type Slots = Arc<Mutex<HashMap<String, Slot>>>;

/// Every Pro Controller that is connected right now, kept up to date in the background.
pub struct ProCons {
    slots: Slots,
    /// An 8BitDo receiver is plugged in but no pad is paired with it.
    dongle: Arc<AtomicBool>,
}

impl Default for ProCons {
    fn default() -> Self {
        Self::new()
    }
}

impl ProCons {
    pub fn new() -> Self {
        let slots: Slots = Arc::default();
        let dongle = Arc::new(AtomicBool::new(false));
        let (watch, flag) = (slots.clone(), dongle.clone());
        let spawned = std::thread::Builder::new()
            .name("procon-watch".into())
            .spawn(move || watch_loop(watch, flag));
        if let Err(e) = spawned {
            log::warn!("procon: cannot start the watcher: {e}");
        }
        ProCons { slots, dongle }
    }

    /// A 2.4G receiver is plugged in and no pad answers through it: the pad is off or asleep and
    /// pressing its Home button brings it back.
    pub fn dongle_waiting(&self) -> bool {
        self.dongle.load(Ordering::Relaxed)
    }

    /// Vibrate the pad at `index` of [`ProCons::pads`] (both motors off stops it).
    pub fn rumble(&self, index: usize, strong: bool, weak: bool) {
        let slots = self.slots.lock().unwrap();
        let mut paths: Vec<&String> = slots.keys().collect();
        paths.sort();
        let alive: Vec<&&String> = paths
            .iter()
            .filter(|p| slots[**p].alive.load(Ordering::Relaxed))
            .collect();
        if let Some(p) = alive.get(index) {
            let _ = slots[**p].rumble.send((strong, weak));
        }
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

/// Stops every pad's vibration when this goes away (a game closing must not leave one buzzing).
impl Drop for ProCons {
    fn drop(&mut self) {
        let slots = self.slots.lock().unwrap();
        for s in slots.values() {
            let _ = s.rumble.send((false, false));
        }
        if !slots.is_empty() {
            std::thread::sleep(Duration::from_millis(60)); // let the reader send it
        }
    }
}

fn watch_loop(slots: Slots, dongle: Arc<AtomicBool>) {
    let mut api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            log::warn!("procon: HID is not available: {e}");
            return;
        }
    };
    loop {
        if api.refresh_devices().is_ok() {
            dongle.store(
                api.device_list()
                    .any(|d| is_idle_receiver(d.vendor_id(), d.product_string())),
                Ordering::Relaxed,
            );
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
    let (tx, rx) = channel::<(bool, bool)>();
    let (st, al) = (state.clone(), alive.clone());
    let spawned = std::thread::Builder::new()
        .name("procon-read".into())
        .spawn(move || {
            if usb {
                usb_handshake(&dev);
            }
            say_hello(&dev);
            let mut buf = [0u8; 128];
            let mut quiet = Instant::now();
            let (mut motors, mut counter, mut sent) = ((false, false), 0u8, Instant::now());
            loop {
                match dev.read_timeout(&mut buf, 10) {
                    Ok(0) => {
                        if quiet.elapsed() > Duration::from_secs(2) {
                            if usb {
                                usb_handshake(&dev);
                            }
                            say_hello(&dev); // 2 s of nothing: ask again
                            quiet = Instant::now();
                        }
                    }
                    Ok(n) => {
                        quiet = Instant::now();
                        if let Some(s) = parse(&buf[..n]) {
                            *st.lock().unwrap() = s;
                        }
                    }
                    Err(e) => {
                        log::info!("procon: pad gone: {e}");
                        break;
                    }
                }
                // rumble: send a change at once, and keep sending while it is on (the pad lets
                // go of a vibration it is not reminded of)
                let mut changed = false;
                while let Ok(m) = rx.try_recv() {
                    changed |= m != motors;
                    motors = m;
                }
                let on = motors.0 || motors.1;
                if changed || (on && sent.elapsed() > Duration::from_millis(50)) {
                    counter = counter.wrapping_add(1);
                    if let Err(e) = dev.write(&rumble_report(counter, motors.0, motors.1)) {
                        log::debug!("procon: rumble not sent: {e}");
                    }
                    sent = Instant::now();
                }
            }
            al.store(false, Ordering::Relaxed);
        });
    if spawned.is_err() {
        alive.store(false, Ordering::Relaxed);
    }
    Slot {
        state,
        alive,
        rumble: tx,
    }
}
