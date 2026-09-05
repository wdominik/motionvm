//! PSM 2 music modules: the tags, and the `PLX` section the Ad Lib music is.
//!
//! The 16-bit engine plays PSM 2 through `MUSADL.DRV`, not the HMI middleware
//! of the 32-bit engine. A module begins with the ASCII tag
//! `MTCVTS PSM 2.00\0` and **nine ascending `u32` section offsets** at
//! `+0x10`; the loader in `ENVIRO.EXE` (`1696:017e`) relocates exactly those
//! nine in place and hands the first section to the music driver, which
//! checks it for the tag `PLX\0` (`MUSADL.DRV` offset `0xe3b`) — so section 0
//! is the whole of the Ad Lib song. The `MDH\0` chunk at offset 56 and the
//! `SM8\0` sample sections belong to the digital driver, which nothing here
//! plays; a sample also ships as a block of its own, thirteen times in
//! Falsches Spiel mit Eddie M., and [`Sample`] reads that form's header.
//!
//! The `PLX` section, as the driver reads it (`MUSADL.DRV` `0xcb4`):
//!
//! ```text
//! +0  "PLX\0"
//! +4  u8   speed — ticks per row
//! +5  u16  tempo — the tick period in PIT cycles (scaled by the driver's
//!          speed setting, 0x100 = as written)
//! +7  9 × u16  per-channel stream offset, relative to the section; 0 = unused
//! +25 the instrument records and the event streams
//! ```
//!
//! Measured on every one of the ten songs of Die Enviro-Kids greifen ein
//! (blocks 1–5 and 7–11).

use crate::error::{Error, Result};
use crate::u16le;
use crate::{dwords, words};

/// The sixteen bytes every PSM 2 module starts with.
pub const MAGIC: &[u8; 16] = b"MTCVTS PSM 2.00\0";

/// The tag the Ad Lib section carries, module or not.
pub const TAG: &[u8; 4] = b"PLX\0";

/// The tag a digital sample carries, as a section of a module or as a block
/// of its own.
pub const SAMPLE_TAG: &[u8; 4] = b"SM8\0";

/// The PC's timer clock in Hz — the unit both PSM 2 drivers count in.
/// `MUSADL.DRV` takes its tempo word as a period in these cycles, and the
/// digital driver takes a sample's rate word the same way.
pub const PIT_HZ: u32 = 1_193_182;

/// Whether a block is a digital sample on its own — the form `PLAYSAMPLE`
/// asks for by block number.
pub fn is_sample(item: &[u8]) -> bool {
    item.starts_with(SAMPLE_TAG)
}

/// A digital sample as its block stores it: ten bytes of header, then the
/// PCM.
///
/// Read off the thirteen sample blocks of Falsches Spiel mit Eddie M. —
/// blocks 1, 9, 10, 12–15, 17, 18 and 20–23, 2305 to 31 960 bytes — and off
/// the driver that plays them. The header is `SM8\0`, `u16 0x0100`, `u16
/// length`, `u16 period`, and the length is the block's less ten in every
/// one. `DMABLAST.DRV`'s play path (`0x0611`) skips the tag and the version,
/// takes the length as the DMA count, and turns the last word into the DSP's
/// time constant through a 256-entry table at `0x60` — `256 − round(w · 1000
/// / 1193)`, the word in whole microseconds at 1.193 cycles each, `table[59]`
/// being `0xcf` — so the word is the **sample period in PIT cycles**, the
/// same unit `MUSADL.DRV`'s tempo is in, and what the DAC plays is the
/// driver's rounding of it, which the audio crate's voice reproduces. Its
/// multi-channel mixer (`0x196b`) reads the word at `+8` out of the header
/// the same way, as the step against the mixer's own period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sample {
    /// The header's second word — `0x0100` in every shipped block. The
    /// mixer path reads it as the channel's volume, taking `0x0100` as "the
    /// default" (`0x2090`); the direct path the games run never reads it.
    pub version: u16,
    /// The sample period in PIT cycles: 56 to 179 across the shipped blocks,
    /// which is 21.3 kHz down to 6.7 kHz — see [`Sample::rate`].
    pub period: u16,
    /// Unsigned 8-bit PCM, as many bytes as the header's length word says.
    pub pcm: Vec<u8>,
}

impl Sample {
    /// Reads a sample block: the tag, the three header words, and exactly
    /// the PCM the length word claims.
    pub fn parse(item: &[u8]) -> Result<Self> {
        if !is_sample(item) {
            return Err(Error::Corrupt {
                what: "PSM 2 sample",
                detail: "the SM8 tag is missing".into(),
            });
        }
        let [version, length, period] = words::<3>(item, 4)?;
        let start = 10;
        let end = start + usize::from(length);
        let pcm = item
            .get(start..end)
            .ok_or_else(|| Error::Truncated {
                off: start,
                need: usize::from(length),
                have: item.len().saturating_sub(start),
            })?
            .to_vec();
        Ok(Self {
            version,
            period,
            pcm,
        })
    }

    /// The sample rate the header names, in Hz: the PIT clock over the
    /// period. The DAC plays the driver's rounding of it — the period in
    /// whole microseconds, so 8000 Hz where this says 8008 — which is the
    /// audio crate's to reproduce; this is what the block says. A period of
    /// zero, which no shipped block carries, answers the clock itself rather
    /// than dividing by nothing.
    pub fn rate(&self) -> u32 {
        PIT_HZ.checked_div(u32::from(self.period)).unwrap_or(PIT_HZ)
    }
}

/// Where the tags of one module sit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tags {
    /// Offset of `MDH\0`, if present — 56 in every song of
    /// Die Enviro-Kids greifen ein.
    pub mdh: Option<usize>,
    /// Offset of the first `SM8\0`, if present.
    pub sm8: Option<usize>,
}

/// Whether a block begins with the PSM 2 tag.
pub fn is_song(item: &[u8]) -> bool {
    is_module(item) || item.starts_with(TAG)
}

/// Whether the item is an `MTCVTS` module, as opposed to a bare section.
pub fn is_module(item: &[u8]) -> bool {
    item.starts_with(MAGIC)
}

/// The tags of a module, or `None` if the block is not one.
pub fn tags(item: &[u8]) -> Option<Tags> {
    if !is_module(item) {
        return None;
    }
    let find = |tag: &[u8]| item.windows(tag.len()).position(|w| w == tag);
    Some(Tags {
        mdh: find(b"MDH\0"),
        sm8: find(b"SM8\0"),
    })
}

/// The nine section offsets the loader relocates, as the header holds them.
pub fn sections(item: &[u8]) -> Result<[u32; 9]> {
    if !is_module(item) {
        return Err(Error::Corrupt {
            what: "PSM 2 module",
            detail: "the MTCVTS tag is missing".into(),
        });
    }
    dwords::<9>(item, 0x10)
}

/// The Ad Lib song: section 0, parsed the way the driver reads it.
#[derive(Debug, Clone)]
pub struct Plx {
    /// Ticks per row.
    pub speed: u8,
    /// The tick period in PIT cycles, before the driver's speed scale.
    pub tempo: u16,
    /// Byte offset of each channel's event stream, relative to the section;
    /// 0 for a channel the song does not use.
    pub channels: [u16; 9],
    /// The whole section — instrument records and event streams address
    /// into it, so it travels as one slice.
    pub bytes: Vec<u8>,
}

impl Plx {
    /// Cuts section 0 out of a module and reads its head.
    ///
    /// A game in the earlier framing stores the section on its own, with no module
    /// around it: all fourteen songs of Victor Loomes are a bare `PLX` where
    /// the later games' ten, four and nine are the first section of an
    /// `MTCVTS` module. Both reach the same reader.
    pub fn parse(item: &[u8]) -> Result<Self> {
        if item.starts_with(TAG) {
            return Self::from_section(item.to_vec());
        }
        let sections = sections(item)?;
        let start = crate::wide(sections[0]);
        let end = sections
            .iter()
            .map(|&o| crate::wide(o))
            .filter(|&o| o > start)
            .min()
            .unwrap_or(item.len())
            .min(item.len());
        let bytes = item
            .get(start..end)
            .ok_or_else(|| Error::Corrupt {
                what: "PSM 2 module",
                detail: format!(
                    "section 0 at {start:#x} lies outside the {} bytes",
                    item.len()
                ),
            })?
            .to_vec();
        Self::from_section(bytes)
    }

    /// Reads a `PLX` section that has already been cut out, or that was
    /// never in a module to begin with.
    pub fn from_section(bytes: Vec<u8>) -> Result<Self> {
        if !bytes.starts_with(TAG) {
            return Err(Error::Corrupt {
                what: "PSM 2 module",
                detail: "section 0 does not carry the PLX tag".into(),
            });
        }
        let speed = *bytes.get(4).ok_or(Error::Truncated {
            off: 4,
            need: 1,
            have: bytes.len(),
        })?;
        let tempo = u16le(&bytes, 5)?;
        let channels = words::<9>(&bytes, 7)?;
        Ok(Self {
            speed,
            tempo,
            channels,
            bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Sample, is_sample};

    /// The header as the shipped blocks have it, over four bytes of PCM.
    #[test]
    fn a_sample_block_reads_its_header_and_its_pcm() {
        let block = [
            b'S', b'M', b'8', 0, 0x00, 0x01, 4, 0, 59, 0, 0x80, 0x84, 0x7f, 0x7a,
        ];
        assert!(is_sample(&block));
        let s = Sample::parse(&block).expect("a sample");
        assert_eq!(s.version, 0x0100);
        assert_eq!(s.period, 59);
        assert_eq!(s.pcm, [0x80, 0x84, 0x7f, 0x7a]);
        assert_eq!(s.rate(), 20223, "1 193 182 / 59");
    }

    /// A length word past the block is refused, and so is the wrong tag.
    #[test]
    fn a_short_block_and_a_wrong_tag_are_refused() {
        let short = [b'S', b'M', b'8', 0, 0x00, 0x01, 9, 0, 59, 0, 0x80];
        assert!(Sample::parse(&short).is_err());
        let plx = [b'P', b'L', b'X', 0, 13, 0, 0, 0, 0, 0, 0];
        assert!(!is_sample(&plx));
        assert!(Sample::parse(&plx).is_err());
    }
}
