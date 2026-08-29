//! PSM 2 music modules: the tags, and the `PLX` section the Ad Lib music is.
//!
//! The 16-bit engine plays PSM 2 through `MUSADL.DRV`, not the HMI middleware
//! of the 32-bit engine. A module begins with the ASCII tag
//! `MTCVTS PSM 2.00\0` and **nine ascending `u32` section offsets** at
//! `+0x10`; the loader in `ENVIRO.EXE` (`1696:017e`) relocates exactly those
//! nine in place and hands the first section to the music driver, which
//! checks it for the tag `PLX\0` (`MUSADL.DRV` offset `0xe3b`) — so section 0
//! is the whole of the Ad Lib song. The `MDH\0` chunk at offset 56 and the
//! `SM8\0` sample sections belong to the digital sound-effect driver, which
//! nothing here plays.
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
use crate::{u16le, u32le};

/// The sixteen bytes every PSM 2 module starts with.
pub const MAGIC: &[u8; 16] = b"MTCVTS PSM 2.00\0";

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
    let mut out = [0u32; 9];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = u32le(item, 0x10 + 4 * i)?;
    }
    Ok(out)
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
    pub fn parse(item: &[u8]) -> Result<Self> {
        let sections = sections(item)?;
        let start = sections[0] as usize;
        let end = sections
            .iter()
            .map(|&s| s as usize)
            .filter(|&s| s > start)
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
        if !bytes.starts_with(b"PLX\0") {
            return Err(Error::Corrupt {
                what: "PSM 2 module",
                detail: "section 0 does not carry the PLX tag".into(),
            });
        }
        let speed = bytes[4];
        let tempo = u16le(&bytes, 5)?;
        let mut channels = [0u16; 9];
        for (i, slot) in channels.iter_mut().enumerate() {
            *slot = u16le(&bytes, 7 + 2 * i)?;
        }
        Ok(Self {
            speed,
            tempo,
            channels,
            bytes,
        })
    }
}
