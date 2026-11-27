//! `DATA/PALETTE.RAW`: 256 colours of 4 bytes each (1024 bytes).

/// An 8-bit palette: the colour of every index.
#[derive(Debug, Clone, PartialEq)]
pub struct Palette(pub [[u8; 3]; 256]);

/// How the 4 bytes of an entry are laid out. The disc does not say.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Order {
    Rgbx,
    Bgrx,
}

impl Palette {
    pub fn parse(bytes: &[u8], order: Order) -> Result<Palette, String> {
        if bytes.len() != 1024 {
            return Err(format!(
                "a palette is 1024 bytes, this one is {}",
                bytes.len()
            ));
        }
        let mut colours = [[0u8; 3]; 256];
        for (colour, entry) in colours.iter_mut().zip(bytes.chunks_exact(4)) {
            *colour = match order {
                Order::Rgbx => [entry[0], entry[1], entry[2]],
                Order::Bgrx => [entry[2], entry[1], entry[0]],
            };
        }
        Ok(Palette(colours))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_are_four_bytes_and_the_order_picks_the_channels() {
        let mut bytes = vec![0u8; 1024];
        bytes[4..8].copy_from_slice(&[1, 2, 3, 0]);
        assert_eq!(Palette::parse(&bytes, Order::Rgbx).unwrap().0[1], [1, 2, 3]);
        assert_eq!(Palette::parse(&bytes, Order::Bgrx).unwrap().0[1], [3, 2, 1]);
    }

    #[test]
    fn a_palette_of_the_wrong_size_is_refused() {
        assert!(Palette::parse(&[0; 100], Order::Rgbx).is_err());
    }
}
