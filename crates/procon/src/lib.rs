//! Nintendo Switch Pro Controller on macOS.
//!
//! macOS does not turn this pad's input reports into buttons, so `gilrs` sees the pad but never
//! an event. The pad does send full reports (id `0x30`, about 60 a second) over Bluetooth, so
//! this crate reads them itself and presents the state with `gilrs::Button` names, which is what
//! the rest of the app already speaks.
//!
//! [`parse`] is pure and tested everywhere. [`ProCons`] reads the hardware and exists on macOS
//! only; elsewhere it is an empty list.

use gilrs::Button;

pub const VENDOR_ID: u16 = 0x057e;
pub const PRODUCT_ID: u16 = 0x2009;
/// The id `gilrs` gives this pad (`03000000 7e05 0000 0920 0000 01000000`). Using the same text
/// keeps a "this player uses this pad" choice that was saved earlier working.
pub const PAD_ID: &str = "030000007e0500000920000001000000";
pub const NAME: &str = "Nintendo Switch Pro Controller";

/// Whether a `gilrs` uuid is this pad (so the caller can leave that copy out).
pub fn is_procon_uuid(uuid: [u8; 16]) -> bool {
    uuid[4..6] == VENDOR_ID.to_le_bytes() && uuid[8..10] == PRODUCT_ID.to_le_bytes()
}

/// One sampled state of a pad.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ProState {
    /// Bit per button in `BUTTONS` order.
    bits: u32,
    /// Left stick, -1.0..=1.0; Y is positive upwards like `gilrs::Axis`.
    pub left: (f32, f32),
    pub right: (f32, f32),
}

/// Which byte and bit of a `0x30` report each button is in.
const BUTTONS: [(Button, usize, u8); 18] = [
    (Button::West, 3, 0x01),          // Y
    (Button::North, 3, 0x02),         // X
    (Button::South, 3, 0x04),         // B
    (Button::East, 3, 0x08),          // A
    (Button::RightTrigger, 3, 0x40),  // R
    (Button::RightTrigger2, 3, 0x80), // ZR
    (Button::Select, 4, 0x01),        // -
    (Button::Start, 4, 0x02),         // +
    (Button::RightThumb, 4, 0x04),
    (Button::LeftThumb, 4, 0x08),
    (Button::Mode, 4, 0x10), // Home
    (Button::DPadDown, 5, 0x01),
    (Button::DPadUp, 5, 0x02),
    (Button::DPadRight, 5, 0x04),
    (Button::DPadLeft, 5, 0x08),
    (Button::LeftTrigger, 5, 0x40),  // L
    (Button::LeftTrigger2, 5, 0x80), // ZL
    (Button::Z, 4, 0x20),            // Capture
];

impl ProState {
    pub fn is_pressed(&self, b: Button) -> bool {
        BUTTONS
            .iter()
            .position(|(x, _, _)| *x == b)
            .is_some_and(|i| self.bits & (1 << i) != 0)
    }

    /// Every button that is down, by `gilrs` name.
    pub fn pressed(&self) -> Vec<Button> {
        BUTTONS
            .iter()
            .enumerate()
            .filter(|(i, _)| self.bits & (1 << i) != 0)
            .map(|(_, (b, _, _))| *b)
            .collect()
    }
}

/// A stick is two 12-bit numbers packed in three bytes; the middle is about 2048.
fn stick(b: &[u8]) -> (f32, f32) {
    let x = b[0] as u16 | ((b[1] as u16 & 0x0f) << 8);
    let y = (b[1] as u16 >> 4) | ((b[2] as u16) << 4);
    let f = |v: u16| ((v as f32 - 2048.0) / 2048.0).clamp(-1.0, 1.0);
    (f(x), f(y))
}

/// Read one input report. `None` for any other kind of report (the pad also answers our own
/// commands with `0x21` reports, which carry no new button state).
pub fn parse(report: &[u8]) -> Option<ProState> {
    if report.len() < 12 || (report[0] != 0x30 && report[0] != 0x31) {
        return None;
    }
    let mut s = ProState::default();
    for (i, (_, byte, mask)) in BUTTONS.iter().enumerate() {
        if report[*byte] & mask != 0 {
            s.bits |= 1 << i;
        }
    }
    s.left = stick(&report[6..9]);
    s.right = stick(&report[9..12]);
    Some(s)
}

/// The four bytes that make one motor buzz (or stay still), as the pad takes them in an output
/// report. The pad treats any non-zero rumble as "on", so this is on or off, not a strength.
/// The values are the ones SDL uses (`rumble_data_table.md` in the Switch reverse-engineering
/// notes explains the encoding).
pub fn motor_bytes(on: bool) -> [u8; 4] {
    const HIGH_FREQ: u16 = 0x0074;
    const HIGH_FREQ_AMP: u8 = 0xBE;
    const LOW_FREQ: u8 = 0x3D;
    const LOW_FREQ_AMP: u16 = 0x806F;
    if on {
        [
            (HIGH_FREQ & 0xff) as u8,
            HIGH_FREQ_AMP | ((HIGH_FREQ >> 8) & 0x01) as u8,
            LOW_FREQ | ((LOW_FREQ_AMP >> 8) & 0x80) as u8,
            (LOW_FREQ_AMP & 0xff) as u8,
        ]
    } else {
        [0x00, 0x01, 0x40, 0x40]
    }
}

/// An output report `0x10` that sets only the rumble: the strong (left) and weak (right) motor.
pub fn rumble_report(counter: u8, strong: bool, weak: bool) -> [u8; 10] {
    let (l, r) = (motor_bytes(strong), motor_bytes(weak));
    [
        0x10,
        counter & 0x0f,
        l[0],
        l[1],
        l[2],
        l[3],
        r[0],
        r[1],
        r[2],
        r[3],
    ]
}

/// A connected pad as the rest of the app sees it.
#[derive(Clone, Debug)]
pub struct ProPad {
    pub id: String,
    pub name: String,
    pub state: ProState,
}

#[cfg(target_os = "macos")]
mod hid;
#[cfg(target_os = "macos")]
pub use hid::ProCons;

/// Not macOS: the system has a driver for this pad, so there is nothing to read here.
#[cfg(not(target_os = "macos"))]
#[derive(Default)]
pub struct ProCons;

#[cfg(not(target_os = "macos"))]
impl ProCons {
    pub fn new() -> Self {
        ProCons
    }
    pub fn pads(&self) -> Vec<ProPad> {
        Vec::new()
    }
    /// Nothing to vibrate here.
    pub fn rumble(&self, _index: usize, _strong: bool, _weak: bool) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A report as the pad sent it while A was held and the left stick pushed fully right.
    const A_AND_STICK_RIGHT: [u8; 14] = [
        0x30, 0x0a, 0x60, 0x08, 0x00, 0x00, 0xff, 0xbf, 0x7f, 0xff, 0xf7, 0x7f, 0x0b, 0x00,
    ];

    #[test]
    fn reads_a_button_and_a_stick() {
        let s = parse(&A_AND_STICK_RIGHT).unwrap();
        assert_eq!(s.pressed(), vec![Button::East]);
        assert!(s.is_pressed(Button::East) && !s.is_pressed(Button::South));
        assert!(s.left.0 > 0.99, "stick right: {:?}", s.left);
        assert!(s.left.1.abs() < 0.05, "stick centred in y: {:?}", s.left);
    }

    #[test]
    fn maps_every_button() {
        for (i, (button, byte, mask)) in BUTTONS.iter().enumerate() {
            let mut r = A_AND_STICK_RIGHT;
            r[3] = 0;
            r[*byte] = *mask;
            let s = parse(&r).unwrap();
            assert_eq!(s.pressed(), vec![*button], "button {i}");
        }
    }

    #[test]
    fn dpad_and_menu_buttons() {
        let mut r = A_AND_STICK_RIGHT;
        r[3] = 0;
        r[4] = 0x02; // +
        r[5] = 0x02; // up
        let s = parse(&r).unwrap();
        assert!(s.is_pressed(Button::Start) && s.is_pressed(Button::DPadUp));
        assert!(!s.is_pressed(Button::Select));
    }

    #[test]
    fn stick_up_is_positive_and_down_negative() {
        let mut r = A_AND_STICK_RIGHT;
        // y = 4095 (up): low nibble of byte 7 is x's high bits, high nibble is y's low bits
        r[6] = 0x00;
        r[7] = 0xf8;
        r[8] = 0xff;
        assert!(parse(&r).unwrap().left.1 > 0.99);
        r[7] = 0x08;
        r[8] = 0x00;
        assert!(parse(&r).unwrap().left.1 < -0.99);
    }

    #[test]
    fn ignores_other_reports_and_short_ones() {
        assert!(parse(&[0x21, 0, 0]).is_none());
        let mut r = A_AND_STICK_RIGHT;
        r[0] = 0x3f;
        assert!(parse(&r).is_none());
        assert!(parse(&A_AND_STICK_RIGHT[..8]).is_none());
    }

    #[test]
    fn rumble_report_is_on_or_neutral_per_motor() {
        let r = rumble_report(0x13, true, false);
        assert_eq!(r[0], 0x10);
        assert_eq!(r[1], 0x03, "the counter is four bits");
        assert_eq!(&r[2..6], &[0x74, 0xBE, 0xBD, 0x6F], "left motor on");
        assert_eq!(&r[6..10], &[0x00, 0x01, 0x40, 0x40], "right motor still");
        assert_eq!(&rumble_report(0, false, false)[2..6], &motor_bytes(false));
    }

    #[test]
    fn recognises_the_uuid_gilrs_gives() {
        let u: [u8; 16] = [3, 0, 0, 0, 0x7e, 5, 0, 0, 9, 0x20, 0, 0, 1, 0, 0, 0];
        assert!(is_procon_uuid(u));
        let hex: String = u.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, PAD_ID);
        let mut other = u;
        other[8] = 0x0a;
        assert!(!is_procon_uuid(other));
    }
}
