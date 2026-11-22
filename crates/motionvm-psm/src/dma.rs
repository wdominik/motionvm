//! What the mixer of `DMABLAST.DRV` carries in its file, read out of the
//! shipped driver.
//!
//! Read rather than embedded, as [`Driver`](crate::Driver) reads
//! `MUSADL.DRV`: the file is part of every install that plays through it,
//! and the bytes stay the original's. File offsets in this module are
//! `DMABLAST.DRV`'s, which are also its code-segment offsets — the entry
//! table at `+0x18` holds code-segment offsets that point into the file.

use crate::error::{Error, Result};

/// The trailer offset of the one `DMABLAST.DRV` build that is read.
const TRAILER: u16 = 0x2656;

/// The tables and settings the mixer of `DMABLAST.DRV` reads, checked out of
/// the file.
#[derive(Debug, Clone)]
pub struct DmaDriver {
    /// The DSP's time constant for the mixer's output (`0x4a0`): `0xd0` in
    /// the file, so a byte every `256 − 0xd0` = 48 microseconds. The
    /// percentage entry (24, `0x0ffd`) that would set another is not built;
    /// a host that never calls it plays at the file's.
    pub time_constant: u8,
    /// The mixer's period in PIT cycles, 16.16 (`0x49e`:`0x49c`):
    /// `0x0039:0x4120` in the file, 48 µs at 1.1928 cycles each, the
    /// arithmetic `0x0530` does for a time constant of `0xd0`. A sample's
    /// period is stepped against it (`0x196b`).
    pub period: u32,
    /// The default volume (`0x1888`) a sample whose header says `0x0100`
    /// plays at, until entry 22 sets another; `0x40` in the file.
    pub volume: u16,
    /// The scale shift (`0x284`): every volume entry 22 takes, and every
    /// volume a digital voice gets, is shifted left by it. 0 in the file,
    /// and no code of the driver writes it.
    pub shift: u8,
    /// The channel the first play tries (`0x1180`, a word index): 2 in the
    /// file, channel 1.
    pub cursor: u16,
    /// The curve a digital song's volume runs through below `0x100`
    /// (`0x2212`, 256 bytes rising from 0 to `0xff`, `0x240d`).
    pub curve: [u8; 256],
}

impl DmaDriver {
    /// Reads the mixer's tables out of `DMABLAST.DRV`, verifying the header
    /// the way a host's driver manager does before it installs the file:
    /// the `DMA\0` tag, version 1.02, 25 entries and 4 imports, and the `NS`
    /// trailer at the offset the header names.
    ///
    /// The four `DMA*.DRV` drivers share that header, and the trailer's
    /// offset tells them apart: `0x2656` is `DMABLAST.DRV`, the 9840-byte
    /// build every install measured ships. The other three — and the further
    /// builds of two of them — are refused, because their mixers sit at other
    /// offsets and are not read.
    pub fn parse(file: &[u8]) -> Result<Self> {
        let word = |at: usize| {
            file.get(at..at + 2)
                .map(|w| u16::from_le_bytes([w[0], w[1]]))
                .unwrap_or(0)
        };
        let head_ok = file.starts_with(b"DMA\0")
            && file.get(4..10) == Some([0x02, 0x01, 0x19, 0x00, 0x04, 0x00].as_ref());
        if !head_ok {
            return Err(Error::Psm(
                "not a DMA 1.02 driver with 25 entries and 4 imports",
            ));
        }
        if word(0x0a) != TRAILER {
            return Err(Error::Psm(
                "a DMA driver that is not read: its trailer is not at 0x2656",
            ));
        }
        let trailer = usize::from(TRAILER);
        if file.get(trailer..trailer + 2) != Some(b"NS".as_ref()) {
            return Err(Error::Psm("the NS trailer is not where the header says"));
        }
        let mut curve = [0u8; 256];
        curve.copy_from_slice(&file[0x2212..0x2312]);
        Ok(Self {
            time_constant: file[0x4a0],
            period: (u32::from(word(0x49e)) << 16) | u32::from(word(0x49c)),
            volume: word(0x1888),
            shift: file[0x284],
            cursor: word(0x1180),
            curve,
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A driver file with the header and trailer of the build that is read,
    /// and the file's values where the mixer reads them; the curve is the
    /// identity.
    pub(crate) fn file() -> Vec<u8> {
        let mut f = vec![0u8; 9840];
        f[..4].copy_from_slice(b"DMA\0");
        f[4..10].copy_from_slice(&[0x02, 0x01, 0x19, 0x00, 0x04, 0x00]);
        f[0x0a..0x0c].copy_from_slice(&TRAILER.to_le_bytes());
        f[0x2656..0x2658].copy_from_slice(b"NS");
        f[0x49c..0x4a1].copy_from_slice(&[0x20, 0x41, 0x39, 0x00, 0xd0]);
        f[0x1180..0x1182].copy_from_slice(&2u16.to_le_bytes());
        f[0x1888..0x188a].copy_from_slice(&0x40u16.to_le_bytes());
        for (i, b) in f[0x2212..0x2312].iter_mut().enumerate() {
            *b = u8::try_from(i).unwrap_or(u8::MAX);
        }
        f
    }

    #[test]
    fn the_build_that_is_read_parses() {
        let d = DmaDriver::parse(&file()).expect("the driver");
        assert_eq!(d.time_constant, 0xd0);
        assert_eq!(d.period, 0x0039_4120);
        assert_eq!(d.volume, 0x40);
        assert_eq!(d.cursor, 2);
        assert_eq!(d.curve[0x80], 0x80);
    }

    /// The same header with another trailer is another driver's mixer.
    #[test]
    fn another_build_is_refused() {
        let mut f = file();
        f[0x0a..0x0c].copy_from_slice(&0x2776u16.to_le_bytes());
        let e = DmaDriver::parse(&f).unwrap_err();
        assert!(e.to_string().contains("not read"), "{e}");
        assert!(DmaDriver::parse(b"MUS\0").is_err());
    }
}
