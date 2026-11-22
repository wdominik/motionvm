//! PSM 2 music modules: the tags, and the `PLX` section the Ad Lib music is.
//!
//! A module begins with the ASCII tag `MTCVTS PSM 2.00\0` and **nine
//! ascending `u32` section offsets** at `+0x10`; a host's loader relocates
//! exactly those nine in place and hands the first section to the music
//! driver, which checks it for the tag `PLX\0` (`MUSADL.DRV` offset `0xe3b`)
//! — so section 0 is the whole of the Ad Lib song. The `MDH\0` chunk at
//! offset 56 and the `SM8\0` sample sections belong to the digital driver:
//! [`Mdh`] reads the chunk, the digital voices of the song, and [`Sample`]
//! a sample's header — a sample also ships on its own, outside any module,
//! with the same header.
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

use crate::error::{Error, Result};
use crate::read::{dwords, u16le, wide, words};

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

/// Whether a block is a digital sample on its own, outside any module.
pub fn is_sample(item: &[u8]) -> bool {
    item.starts_with(SAMPLE_TAG)
}

/// A digital sample as its block stores it: ten bytes of header, then the
/// PCM.
///
/// Read off the driver that plays it, and held to the sample blocks
/// measured. The header is `SM8\0`, `u16 volume`, `u16 length`, `u16
/// period`, and the length was the block's less ten in every one.
/// `DMABLAST.DRV`'s direct path (`0x0611`) skips the tag and the volume,
/// takes the length as the DMA count, and turns the last word into the DSP's
/// time constant through a 256-entry table at `0x60` — `256 − round(w · 1000
/// / 1193)`, the word in whole microseconds at 1.193 cycles each, `table[59]`
/// being `0xcf` — so the word is the **sample period in PIT cycles**, the
/// same unit `MUSADL.DRV`'s tempo is in, and what the DAC plays is the
/// driver's rounding of it, which [`Voice`](crate::Voice) reproduces. Its
/// mixer (`0x196b`) reads the word at `+8` out of the header the same way,
/// as the step against the mixer's own period, and the word at `+4` to
/// choose the channel's volume ([`Mixer`](crate::Mixer)).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sample {
    /// The header's second word, the volume — `0x0100` in every block
    /// measured. The play entry reads it signed: at zero or below it takes
    /// the direct path (`0x2012`), which plays at full scale and reads it no
    /// further. On the mixer path `0x0100` gives the channel the default the
    /// host set with entry 22, and **every other value gives it `0x0100`**:
    /// the entry loads `0x100` (`0x2090`), replaces it with the default
    /// only where the word equals it (`0x2093`–`0x2098`), and stores that
    /// (`0x209d`) — the word itself never becomes a channel's volume.
    pub volume: u16,
    /// The sample period in PIT cycles — 56 to 179 across the blocks
    /// measured, which is 21.3 kHz down to 6.7 kHz; see [`Sample::rate`].
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
        let [volume, length, period] = words::<3>(item, 4)?;
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
            volume,
            period,
            pcm,
        })
    }

    /// The sample rate the header names, in Hz: the PIT clock over the
    /// period. The DAC plays the driver's rounding of it — the period in
    /// whole microseconds, so 8000 Hz where this says 8008 — which is
    /// [`Voice`](crate::Voice)'s to reproduce; this is what the block says. A period of
    /// zero, which no shipped block carries, answers the clock itself rather
    /// than dividing by nothing.
    pub fn rate(&self) -> u32 {
        PIT_HZ.checked_div(u32::from(self.period)).unwrap_or(PIT_HZ)
    }
}

/// The tag of a song's digital part.
pub const MDH_TAG: &[u8; 4] = b"MDH\0";

/// The most digital voices a song has: `DMABLAST.DRV`'s entry 21 refuses a
/// count outside 1 to 8 (`0x2612`–`0x2619`), and its voice tables hold eight.
pub const MDH_VOICES: usize = 8;

/// One digital voice of a song, as its eight-byte record holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MdhVoice {
    /// The channel's priority (`+1`, stored at `0x2555`). The driver reads it
    /// only where a mixer that fell behind gives up a channel (`0x1b58`),
    /// which the rebuild never reaches.
    pub priority: u8,
    /// Which entry of the song's sample table the voice plays (`+2`, read at
    /// `0x2579`).
    pub sample: u16,
    /// The four levels an event chooses among (`+4`, copied at
    /// `0x255d`/`0x2565`).
    pub levels: [u8; 4],
}

/// A song's digital part: the `MDH\0` chunk, as `DMABLAST.DRV`'s entry 21
/// (`0x25ea`) checks it and its start callback (`0x2506`) reads it.
///
/// ```text
/// +00  "MDH\0"           compared as "MD" and "H\0" (0x2606, 0x260c)
/// +04  u16                not read
/// +06  u16                offset of the event stream from the tag (0x25bb)
/// +08  u16   nv           the voice count, 1 to 8 (0x2612)
/// +0A  8-byte records, one per voice:
///        +0  u8   not read
///        +1  u8   priority
///        +2  u16  index into the song's sample table
///        +4  4×u8 levels
/// ```
///
/// The event stream, one byte at a time (`0x2342`–`0x2402`):
///
/// | Byte | What it is |
/// |---|---|
/// | `000vvvll` | voice `v`'s volume from its level `l`, then its sample from the start |
/// | `20 00` | the end of the stream |
/// | `01xxxxxx` | a pause of one row, then the event `xxxxxx` |
/// | `10dddddd` | a pause of `2·d` rows |
/// | `11dddddd e` | a pause of `(d << 8) + e` rows |
///
/// Any other byte where an event is due — `20` followed by anything but `00`,
/// or `21` to `3F` — runs the end of the stream (`0x2402` → `0x2500`) from a
/// stack the end does not return through, so the driver does not survive it;
/// a stream that reaches one, or runs past its block without reaching `20
/// 00`, is refused here. So is a voice record past the block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mdh {
    /// The voices, as many as the count says.
    pub voices: Vec<MdhVoice>,
    /// The event stream, from its offset to the end of the block.
    pub events: Vec<u8>,
}

impl Mdh {
    /// Reads the chunk at the start of `block`, which runs to the end of
    /// what the song's digital part may read.
    pub fn parse(block: &[u8]) -> Result<Self> {
        let corrupt = |detail: &str| Error::Corrupt {
            what: "PSM 2 MDH",
            detail: detail.into(),
        };
        if !block.starts_with(MDH_TAG) {
            return Err(corrupt("the MDH tag is missing"));
        }
        let [_, events, count] = words::<3>(block, 4)?;
        let count = usize::from(count);
        if !(1..=MDH_VOICES).contains(&count) {
            return Err(corrupt("the voice count is outside 1 to 8"));
        }
        let mut voices = Vec::with_capacity(count);
        for v in 0..count {
            let at = 0x0a + 8 * v;
            let rec = block.get(at..at + 8).ok_or(Error::Truncated {
                off: at,
                need: 8,
                have: block.len(),
            })?;
            voices.push(MdhVoice {
                priority: rec[1],
                sample: u16::from_le_bytes([rec[2], rec[3]]),
                levels: [rec[4], rec[5], rec[6], rec[7]],
            });
        }
        let events = block
            .get(usize::from(events)..)
            .ok_or_else(|| corrupt("the event stream starts past the block"))?
            .to_vec();
        if !stream_ends(&events) {
            return Err(corrupt(
                "the event stream does not reach 20 00 through events the driver reads",
            ));
        }
        Ok(Self { voices, events })
    }
}

/// Whether a stream, read the way the driver reads it, reaches `20 00`
/// through bytes the driver comes back from.
fn stream_ends(events: &[u8]) -> bool {
    let mut at = 0;
    while let Some(&b) = events.get(at) {
        at += 1;
        // A pause byte; the one-row pause hands its low six bits on as the
        // event after it.
        let event = match b {
            0x80..=0xbf => continue,
            0xc0..=0xff => {
                at += 1;
                continue;
            }
            0x40..=0x7f => b & 0x3f,
            _ => b,
        };
        match event {
            0x00..=0x1f => {}
            0x20 => return events.get(at) == Some(&0),
            _ => return false,
        }
    }
    false
}

/// Where the tags of one module sit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tags {
    /// Offset of `MDH\0`, if present.
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
    /// A section may also be stored on its own, a bare `PLX` with no module
    /// around it, and both forms reach the same reader.
    pub fn parse(item: &[u8]) -> Result<Self> {
        if item.starts_with(TAG) {
            return Self::from_section(item.to_vec());
        }
        let sections = sections(item)?;
        let start = wide(sections[0]);
        let end = sections
            .iter()
            .map(|&o| wide(o))
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
    use super::{Mdh, MdhVoice, Sample, is_sample};

    /// The header as the shipped blocks have it, over four bytes of PCM.
    #[test]
    fn a_sample_block_reads_its_header_and_its_pcm() {
        let block = [
            b'S', b'M', b'8', 0, 0x00, 0x01, 4, 0, 59, 0, 0x80, 0x84, 0x7f, 0x7a,
        ];
        assert!(is_sample(&block));
        let s = Sample::parse(&block).expect("a sample");
        assert_eq!(s.volume, 0x0100);
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

    /// A chunk of two voices whose stream holds every kind of byte the
    /// driver reads before its end.
    fn mdh(stream: &[u8]) -> Vec<u8> {
        let mut b = b"MDH\0".to_vec();
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(&0x1au16.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&[0, 0x40, 0, 0, 0x10, 0x20, 0x30, 0x40]);
        b.extend_from_slice(&[0, 0x80, 1, 0, 0x50, 0x60, 0x70, 0x80]);
        b.extend_from_slice(stream);
        b
    }

    #[test]
    fn an_mdh_reads_its_voices_and_its_stream() {
        let stream = [0x00, 0x86, 0x45, 0xc1, 0x02, 0x07, 0x20, 0x00];
        let m = Mdh::parse(&mdh(&stream)).expect("an MDH");
        assert_eq!(
            m.voices,
            [
                MdhVoice {
                    priority: 0x40,
                    sample: 0,
                    levels: [0x10, 0x20, 0x30, 0x40],
                },
                MdhVoice {
                    priority: 0x80,
                    sample: 1,
                    levels: [0x50, 0x60, 0x70, 0x80],
                },
            ]
        );
        assert_eq!(m.events, stream);
        // A one-row pause can carry the end as its event.
        assert!(Mdh::parse(&mdh(&[0x60, 0x00])).is_ok());
    }

    /// A stream the driver does not come back from, or one without its end,
    /// is refused; so is a count outside 1 to 8.
    #[test]
    fn an_mdh_the_driver_does_not_survive_is_refused() {
        for stream in [&[0x20, 0x01][..], &[0x21], &[0x61], &[0x00, 0x04], &[0xc0]] {
            assert!(Mdh::parse(&mdh(stream)).is_err(), "{stream:x?}");
        }
        let mut b = mdh(&[0x20, 0x00]);
        b[8] = 9;
        assert!(Mdh::parse(&b).is_err());
        b[8] = 0;
        assert!(Mdh::parse(&b).is_err());
    }
}
