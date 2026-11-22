//! VGA DAC palettes, six bits per channel.
//!
//! PARSEC does not load one 256-color dump. It writes **banks** into the DAC:
//!
//! | Bank | Where it is measured in Dunkle Schatten's files |
//! |---|---|
//! | 0..127 | Room picture (`RDV1.00` starts `00 80` + 128 RGB) |
//! | 128..159 | `PLAY00.DAT` resource 0 (`80 20` + 32 RGB) |
//! | 160..191 | Verb strip (`PANELS.DAT` resource 1 is `a0 20` + 32 RGB) |
//! | 224..255 | Mouse (`CMBMOU.DAT` resource 0 is `e0 20` + 32 RGB) |
//!
//! The on-disk encoding of a bank is `u8 start, u8 count` followed by
//! `count` RGB triples, **exactly** `2 + 3*count` bytes. That is the same
//! header as the 386-byte form `00 80` + 128 RGB (start 0, count 128).
//! `XP01`'s loader `0590h:0A1Fh` (`0x0631F`; five far calls and the near
//! one at `0x05D35`) reads those two bytes and nothing else and takes a
//! count of 0 for 256 (`0x0633E`–`0x06342`). `XP00` takes the count at
//! `+1` the same way, 0 for 256, and the triples from `+2`
//! (`0x33D9`–`0x33F2`). Neither has a shape without the header.
//!
//! Channels are 6-bit DAC values (0..=63). What a write does to the DAC,
//! and how a register is widened to 8 bits for a screen, is not this
//! reader's: it says which registers a blob writes and with what, and the
//! caller writes them into whatever palette it keeps.

use crate::Record;

/// Bytes in a 128-color RGB dump.
pub const RGB_128: usize = 128 * 3;
/// Bytes in the prefixed form (`00 80` + 128×RGB).
pub const PREFIXED: usize = 2 + RGB_128;
/// Registers in the DAC a bank is written into.
pub const REGISTERS: usize = 256;

/// One write to the DAC: a run of registers from `start`, as 6-bit triples.
///
/// What a palette resource *is*, as opposed to what it does to a palette:
/// the registers it names and the bytes it puts there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DacBank<'a> {
    /// The first register written.
    pub start: u8,
    /// Packed 6-bit RGB triples, three bytes a register.
    pub rgb: &'a [u8],
}

/// `true` when `bytes` is a DAC bank: `start, count` + exactly `count`
/// triples.
pub fn looks_like(bytes: &[u8]) -> bool {
    bank(bytes).is_some()
}

/// The write one on-disk palette blob makes, or `None` when it is not one.
///
/// Exactness is the check: `PANELS.DAT` resource 1 is 98 bytes with
/// `start=160, count=32`; `CMBMOU.DAT` resource 0 is 98 bytes with
/// `start=224, count=32`. A run that would reach past the last register is
/// not a bank either.
pub fn bank(bytes: &[u8]) -> Option<DacBank<'_>> {
    let (head, rgb) = bytes.split_first_chunk::<2>()?;
    let head = Record(head);
    let start = head.u8::<0>();
    let count = match head.u8::<1>() {
        0 => REGISTERS,
        n => usize::from(n),
    };
    if rgb.len() != count.saturating_mul(3) || usize::from(start).saturating_add(count) > REGISTERS
    {
        return None;
    }
    Some(DacBank { start, rgb })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_00_80_bank_writes_128_colors_at_zero() {
        let mut bytes = vec![0x00, 0x80];
        bytes.extend(std::iter::repeat_n(0x20u8, RGB_128));
        let b = bank(&bytes).unwrap();
        assert_eq!(b.start, 0);
        assert_eq!(b.rgb.len(), RGB_128);
        assert!(b.rgb.iter().all(|&v| v == 0x20));
    }

    #[test]
    fn a_start_count_bank_writes_at_the_named_register() {
        // 32 colors at 32.
        let mut bytes = vec![32, 32];
        bytes.extend(std::iter::repeat_n(0x15u8, 96));
        let b = bank(&bytes).unwrap();
        assert_eq!((b.start, b.rgb.len()), (32, 96));
        assert!(looks_like(&bytes));
    }

    /// A count of 0 is 256 registers, the way `0x0633E` reads it.
    #[test]
    fn a_count_of_zero_is_every_register() {
        let mut bytes = vec![0, 0];
        bytes.extend(std::iter::repeat_n(0x10u8, REGISTERS * 3));
        assert_eq!(bank(&bytes).map(|b| b.rgb.len()), Some(REGISTERS * 3));
        bytes[0] = 1;
        assert_eq!(bank(&bytes), None, "a run past the last register");
    }

    /// A header whose count does not match the length is not a bank, and
    /// neither are 96 bare bytes: the loader reads no such shape.
    #[test]
    fn an_inexact_header_is_not_a_bank() {
        let bytes = [32u8, 32, 1, 2, 3, 4, 5];
        assert_eq!(bank(&bytes), None);
        assert!(!looks_like(&bytes));
        assert!(!looks_like(&[0x10; 96]));
    }
}
