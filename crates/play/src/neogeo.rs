//! Which Neo Geo BIOS mode to ask FBNeo for. The core boots the mode named in its
//! `fbneo-neogeo-mode` option; when the BIOS for that mode is missing from `neogeo.zip` it
//! silently falls back to the arcade (MVS) BIOS, which starts with a green, garbled system
//! check. So the mode is chosen from what the zip really contains. Pure, so it is unit-tested.

/// True when `needle` occurs in `hay` (file names are stored as plain text in a zip).
fn contains(hay: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle)
}

/// The best mode for a `neogeo.zip` with these bytes:
/// - the European AES (console) BIOS is there: `AES_EUR`, which starts at the logo;
/// - otherwise the Universe BIOS: `UNIBIOS`, which starts with a clean logo instead of the
///   MVS system check;
/// - otherwise `AES_EUR` anyway, as before (the core then reports what is missing).
pub fn default_mode(zip: &[u8]) -> &'static str {
    if contains(zip, b"neo-epo.bin") {
        "AES_EUR"
    } else if contains(zip, b"uni-bios") {
        "UNIBIOS"
    } else {
        "AES_EUR"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zip_with_the_aes_bios_uses_it() {
        assert_eq!(
            default_mode(b"...sp-s2.sp1...neo-epo.bin...uni-bios_2_3.rom"),
            "AES_EUR"
        );
    }

    #[test]
    fn a_zip_with_only_mvs_and_universe_bioses_uses_the_universe_one() {
        // the set that gives a green screen with AES_EUR: no neo-epo.bin, but uni-bios files
        assert_eq!(
            default_mode(b"000-lo.lo sp-e.sp1 sp-s2.sp1 uni-bios_2_3.rom vs-bios.rom"),
            "UNIBIOS"
        );
    }

    #[test]
    fn a_zip_with_neither_keeps_the_old_default() {
        assert_eq!(default_mode(b"sp-s2.sp1 sp-e.sp1"), "AES_EUR");
        assert_eq!(default_mode(b""), "AES_EUR");
    }
}
