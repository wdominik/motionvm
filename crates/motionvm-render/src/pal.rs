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

    /// Writes `rgb` — 6-bit triples, as a program hands them to the DAC — into
    /// the registers from `start` onward, leaving every other register as it
    /// was.
    ///
    /// A palette is not always written whole: a program may load a run of
    /// registers and keep the rest, and this is that write. What would run
    /// past the last register is cut, the way [`from_6bit`](Self::from_6bit)
    /// cuts a long input, rather than refused or wrapped.
    pub fn load(&mut self, start: u8, rgb: &[u8]) {
        let offset = usize::from(start) * 3;
        let n = rgb.len().min(Self::BYTES - offset);
        self.raw[offset..offset + n].copy_from_slice(&rgb[..n]);
    }

    /// The palette entry for `index`, widened from 6 to 8 bits per channel.
    pub fn rgb8(&self, index: u8) -> [u8; 3] {
        let o = usize::from(index) * 3;
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

/// Every register black: what a palette holds before anything is written to
/// it, and where a picture faded in from black starts.
impl Default for Palette {
    fn default() -> Self {
        Self {
            raw: [0; Self::BYTES],
        }
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
    fn the_default_palette_is_black() {
        let p = Palette::default();
        assert!(p.raw.iter().all(|&v| v == 0));
        assert_eq!(p, Palette::from_6bit(&[]), "the same as an empty load");
    }

    /// A run of registers lands where it is aimed and nowhere else.
    #[test]
    fn a_load_writes_only_its_own_registers() {
        let mut p = Palette::from_6bit(&[7; Palette::BYTES]);
        p.load(2, &[1, 2, 3, 4, 5, 6]);
        assert_eq!(&p.raw[..6], &[7; 6], "registers 0 and 1 kept");
        assert_eq!(
            &p.raw[6..12],
            &[1, 2, 3, 4, 5, 6],
            "registers 2 and 3 written"
        );
        assert!(p.raw[12..].iter().all(|&v| v == 7), "the rest kept");
    }

    /// A run past the last register is cut there, not wrapped to register 0.
    #[test]
    fn a_load_past_the_end_is_cut() {
        let mut p = Palette::default();
        p.load(255, &[9; 9]);
        assert_eq!(&p.raw[765..], &[9, 9, 9], "the last register written");
        assert!(
            p.raw[..765].iter().all(|&v| v == 0),
            "nothing wrapped round"
        );
    }

    #[test]
    fn widening_covers_full_range() {
        assert_eq!(widen(0), 0);
        assert_eq!(widen(63), 255);
        assert_eq!(widen(32), 130);
    }

    /// Every one of the sixty-four values a channel can hold, checked rather
    /// than three of them.
    ///
    /// A property over an exhaustive domain, which is what makes it a proof
    /// and not a sample: a 6-bit channel has sixty-four values, so "for all"
    /// is a loop. What it holds to is what widening is *for* — the bit pattern
    /// repeated into the low bits, so that the range ends land on 0 and 255
    /// and nothing in between jumps by more than the step below it.
    #[test]
    fn widening_is_monotone_and_reaches_both_ends() {
        let widened: Vec<u8> = (0..64).map(widen).collect();
        assert_eq!(widened[0], 0, "the darkest value is black");
        assert_eq!(widened[63], 255, "the brightest is white");
        for pair in widened.windows(2) {
            assert!(
                pair[1] > pair[0],
                "widening should be strictly increasing, got {pair:?}"
            );
            // Four or five: 63 steps spanning 255 cannot all be equal, and a
            // jump of six would mean a value the widening skips over twice.
            let step = pair[1] - pair[0];
            assert!((4..=5).contains(&step), "a step of {step} between {pair:?}");
        }
    }

    /// Narrowing a widened value gives it back.
    ///
    /// The direction that matters for a comparison against the original: a
    /// capture arrives as 8-bit colour and has to be reduced to the 6-bit
    /// values the game's palette holds, and a reduction that did not invert
    /// the widening would put a picture one shade off everywhere.
    #[test]
    fn narrowing_a_widened_value_gives_it_back() {
        for v6 in 0..64u8 {
            assert_eq!(widen(v6) >> 2, v6, "{v6} did not survive the round trip");
        }
    }
}
