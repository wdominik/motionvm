//! The four data tables `MUSADL.DRV` carries, read out of the shipped file.
//!
//! Read rather than embedded: the file is part of every install that plays
//! through it, and the bytes stay the original's.
//!
//! File offsets in the comments are `MUSADL.DRV`'s unless marked otherwise.

use crate::error::{Error, Result};

/// The PC timer's rate in Hz; the driver counts its tick periods in PIT
/// cycles (`0x4a9` = 1193 of them to a millisecond in the fade arithmetic),
/// and the digital driver counts a sample's period the same way — one
/// constant, the format's.
pub use crate::format::PIT_HZ;

/// The four tables `MUSADL.DRV` carries, checked out of the file.
///
/// Read from the shipped driver rather than embedded: the file is part of
/// every install that plays through it, and the bytes stay the original's.
#[derive(Debug)]
pub struct Driver {
    /// One default per OPL register (`0x5c4`); `0xFF` marks a register the
    /// init leaves untouched. The init writes every other one, in ascending
    /// register order, which is the first thing a capture of the original
    /// holds.
    pub defaults: [u8; 256],
    /// The modulator operator offsets per channel (`0x6c4`).
    pub mod_ops: [u8; 9],
    /// The carrier operator offsets per channel (`0x6cd`).
    pub car_ops: [u8; 9],
    /// The note table (`0x6d6`): 96 words of `block << 10 | fnum`, twelve
    /// F-numbers repeated over eight octaves.
    pub notes: [u16; 96],
}

impl Driver {
    /// Reads the tables out of `MUSADL.DRV`, verifying the header the way a
    /// host's driver manager does before it installs the file: the `MUS\0`
    /// tag, version 1.00, the entry and import counts, and the `NS` trailer
    /// at the offset the header names.
    ///
    /// Two builds are read: a 4480-byte one with fifteen entries and its
    /// trailer at `0x1165`, and a 3915-byte one with fourteen and its trailer
    /// at `0xf49`, whose four tables sit `0xd0` earlier and whose contents
    /// are byte-for-byte the same — the defaults, both operator-offset tables
    /// and all 96 notes compare equal between the two files. So the build
    /// decides where to read, not what the tables mean.
    ///
    /// The build is told by both words the file says of itself, the entry
    /// count and the trailer's offset, because the count alone does not
    /// tell it: further 4480-byte builds with fifteen entries carry the same
    /// tables at the same offsets around code that differs, and what that
    /// code does is unread. Such a build is refused rather than played as
    /// the one that is read.
    pub fn parse(file: &[u8]) -> Result<Self> {
        let word = |at: usize| {
            file.get(at..at + 2)
                .map(|w| u16::from_le_bytes([w[0], w[1]]))
                .unwrap_or(0)
        };
        let (entries, trailer) = (word(6), word(0x0a));
        // Where the defaults begin; everything else follows it.
        let base = match (entries, trailer) {
            (15, 0x1165) => 0x5c4,
            (14, 0x0f49) => 0x4f4,
            (14 | 15, _) => {
                return Err(Error::Psm(
                    "a build of the MUS driver that is not read: its trailer is at neither 0x1165 nor 0xf49",
                ));
            }
            _ => return Err(Error::Psm("not a MUS driver with 14 or 15 entries")),
        };
        let head_ok = file.len() >= base + 0x1d2
            && &file[..4] == b"MUS\0"
            && file[4..6] == [0x00, 0x01]
            && file[8..10] == [0x07, 0x00];
        if !head_ok {
            return Err(Error::Psm("not a MUS 1.00 driver with 7 imports"));
        }
        let trailer = usize::from(trailer);
        if file.get(trailer..trailer + 2) != Some(b"NS".as_ref()) {
            return Err(Error::Psm("the NS trailer is not where the header says"));
        }
        let mut defaults = [0u8; 256];
        defaults.copy_from_slice(&file[base..base + 0x100]);
        let mut mod_ops = [0u8; 9];
        mod_ops.copy_from_slice(&file[base + 0x100..base + 0x109]);
        let mut car_ops = [0u8; 9];
        car_ops.copy_from_slice(&file[base + 0x109..base + 0x112]);
        // An operator offset names one of the chip's operator slots, the
        // last of which is 0x15. Every register the sequencer forms from one
        // is a base of at most 0xE0 plus the offset, so past 0x15 the file is
        // damaged, and a sum would leave the byte.
        if mod_ops.iter().chain(&car_ops).any(|&op| op > 0x15) {
            return Err(Error::Psm("an operator offset past the chip's last, 0x15"));
        }
        let mut notes = [0u16; 96];
        for (i, n) in notes.iter_mut().enumerate() {
            *n = u16::from_le_bytes([file[base + 0x112 + 2 * i], file[base + 0x113 + 2 * i]]);
        }
        Ok(Self {
            defaults,
            mod_ops,
            car_ops,
            notes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A driver file of `len` bytes with the header a build of `entries`
    /// entries carries, its trailer at `trailer`, and zeroed tables.
    fn driver(len: usize, entries: u8, trailer: u16) -> Vec<u8> {
        let mut f = vec![0u8; len];
        f[..4].copy_from_slice(b"MUS\0");
        f[4..10].copy_from_slice(&[0x00, 0x01, entries, 0x00, 0x07, 0x00]);
        f[0x0a..0x0c].copy_from_slice(&trailer.to_le_bytes());
        let t = usize::from(trailer);
        f[t..t + 2].copy_from_slice(b"NS");
        f
    }

    #[test]
    fn both_builds_that_are_read_parse() {
        assert!(Driver::parse(&driver(4480, 15, 0x1165)).is_ok());
        assert!(Driver::parse(&driver(3915, 14, 0x0f49)).is_ok());
    }

    /// Fifteen entries and the same length, but the trailer elsewhere: the
    /// tables would read, and the code around them is not the code read.
    #[test]
    fn a_build_that_is_not_read_is_refused() {
        for trailer in [0x1166, 0x1168] {
            let e = Driver::parse(&driver(4480, 15, trailer)).unwrap_err();
            assert!(e.to_string().contains("not read"), "{e}");
        }
    }
}
