//! 768-byte VGA palettes: 256 entries of RGB, six bits per channel, exactly
//! as they were written to the VGA DAC. Which of a game's files a palette
//! comes out of is its reader's business; the layout ends here either way.
//!
//! Widening to 8 bits replicates the top bits rather than shifting, so
//! full-scale 63 maps to 255 instead of 252.

#[derive(Debug, Clone, PartialEq, Eq)]
/// A 256-entry VGA palette, six bits per channel as the DAC stores them.
pub struct Palette {
    /// 6-bit values as stored on disk.
    pub raw: [u8; Self::BYTES],
}

impl Palette {
    /// Bytes in a whole palette: 256 entries of three channels.
    pub const BYTES: usize = 768;
    /// Entries in a palette.
    pub const COLORS: usize = 256;

    /// A palette from the 768 bytes as they sit on disk.
    ///
    /// A short input is padded with black and a long one is cut, rather than
    /// refused. That is deliberate and it is what the hardware did: the DAC
    /// gets however many triples a program hands it and the rest of the
    /// registers keep whatever they held. Refusing here would turn a palette
    /// a game displays into a startup failure.
    ///
    /// It is worth knowing the short case never happens on shipped data —
    /// every palette in the containers this project reads is exactly 768
    /// bytes, which each game's `every_palette_is_768_six_bit_bytes` style of
    /// test checks — so a short one is a sign of damage even though it is
    /// tolerated.
    pub fn from_6bit(bytes: &[u8]) -> Self {
        let mut raw = [0u8; Self::BYTES];
        let n = bytes.len().min(Self::BYTES);
        raw[..n].copy_from_slice(&bytes[..n]);
        Self { raw }
    }

    /// The palette entry for `index`, widened from 6 to 8 bits per channel.
    pub fn rgb8(&self, index: u8) -> [u8; 3] {
        let o = index as usize * 3;
        [
            widen(self.raw[o]),
            widen(self.raw[o + 1]),
            widen(self.raw[o + 2]),
        ]
    }

    /// The whole palette as 256 RGB triples, widened to 8 bits.
    pub fn to_rgb8(&self) -> Vec<u8> {
        self.raw.iter().map(|&v| widen(v)).collect()
    }

    /// Recovers a palette from 8-bit triples, as read back out of a PNG.
    ///
    /// The inverse of [`to_rgb8`](Self::to_rgb8) for any value that round-trips
    /// — which is every value this crate writes, since the widening is injective
    /// on six bits.
    pub fn from_rgb8(bytes: &[u8]) -> Self {
        let mut raw = [0u8; Self::BYTES];
        for (dst, &v) in raw.iter_mut().zip(bytes) {
            *dst = v >> 2;
        }
        Self { raw }
    }
}

/// Widens a 6-bit DAC value to 8 bits so that 63 becomes 255.
#[inline]
fn widen(v6: u8) -> u8 {
    let v = v6 & 0x3f;
    (v << 2) | (v >> 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widening_covers_full_range() {
        assert_eq!(widen(0), 0);
        assert_eq!(widen(63), 255);
        assert_eq!(widen(32), 130);
    }
}
