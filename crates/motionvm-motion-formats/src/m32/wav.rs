//! The WAV files the 32-bit engine's sample words play: the 39 speech and
//! effect blocks in Checker 2000's `004.RSC` and the 72 files in its `WAVS/`
//! directory, all canonical RIFF/WAVE with a 44-byte header, 22 050 Hz,
//! 16-bit, mono.
//!
//! The engine reads the header **blind**, at fixed offsets, as the stream
//! path of `->STARTSAMPLE` does (`ENGINE.EXE` V0.04.15/R78 `0x6aa11`–
//! `0x6aa39`): the sample rate at `+0x18`, the bits per sample at `+0x22`,
//! the channels at `+0x16`, and the PCM from `+0x2c` to the end of the file —
//! no chunk is walked and no tag is compared. The block path hands the whole
//! block to the sound layer and reads only the rate, at the same `+0x18`
//! (`0x6ae5b`). So a file whose `fmt ` chunk is not sixteen bytes, or that
//! carries a `LIST` chunk before its data, would play its own header as sound
//! in the original; no shipped file does, and this reader refuses one rather
//! than reproduce that, which is the one place it is stricter than the code
//! it reads for.

use crate::error::{Error, Result};
use crate::{Record, bytes};

/// Where the PCM begins: the canonical header's length.
pub const HEADER_LEN: usize = 0x2c;

/// One WAV file as the engine plays it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wav {
    /// Channels, `+0x16`: 1 in every shipped file.
    pub channels: u16,
    /// Frames per second, `+0x18`: 22 050 in every shipped file, 11 025 in
    /// the sound setup's `TEST.WAV`.
    pub rate: u32,
    /// Bits per sample, `+0x22`: 16 in every shipped file, 8 in `TEST.WAV`.
    pub bits: u16,
    /// The PCM as the file holds it, little-endian signed where 16-bit and
    /// unsigned where 8-bit, interleaved by channel — everything from
    /// [`HEADER_LEN`] to the end.
    pub pcm: Vec<u8>,
}

impl Wav {
    /// Reads a file the way the engine does, refusing what the engine would
    /// play as noise.
    pub fn parse(file: &[u8]) -> Result<Self> {
        let head = Record(bytes::<HEADER_LEN>(file, 0).map_err(|_| Error::Corrupt {
            what: "WAV file",
            detail: format!("{} bytes, shorter than the 44-byte header", file.len()),
        })?);
        if &head.bytes::<0, 4>() != b"RIFF" || &head.bytes::<8, 4>() != b"WAVE" {
            return Err(Error::Corrupt {
                what: "WAV file",
                detail: "no RIFF/WAVE tags".into(),
            });
        }
        // The canonical header and nothing else: a `fmt ` chunk of sixteen
        // bytes, PCM, and the data chunk at `+0x24`. The engine assumes all
        // of it, and a file that departs would be misread by it.
        if &head.bytes::<0xc, 4>() != b"fmt " || head.u32::<0x10>() != 16 || head.u16::<0x14>() != 1
        {
            return Err(Error::Corrupt {
                what: "WAV file",
                detail: "not the canonical 16-byte PCM format chunk the engine reads".into(),
            });
        }
        if &head.bytes::<0x24, 4>() != b"data" {
            return Err(Error::Corrupt {
                what: "WAV file",
                detail: "no data chunk at +0x24, where the engine takes the PCM to begin".into(),
            });
        }
        let channels = head.u16::<0x16>();
        let rate = head.u32::<0x18>();
        let bits = head.u16::<0x22>();
        // A rate of zero divides the engine's duration arithmetic by zero
        // (`0x6aa79`), and a sample of no bits or channels is nothing to play.
        if rate == 0 || !matches!(bits, 8 | 16) || channels == 0 {
            return Err(Error::Corrupt {
                what: "WAV file",
                detail: format!("{channels} channel(s), {rate} Hz, {bits} bits"),
            });
        }
        let pcm = file.get(HEADER_LEN..).unwrap_or_default().to_vec();
        Ok(Self {
            channels,
            rate,
            bits,
            pcm,
        })
    }

    /// Whether `item` begins as a WAV file does — how a block that is a
    /// sample is told from a block that is a song or a table.
    pub fn is_wav(item: &[u8]) -> bool {
        item.len() >= HEADER_LEN && item.starts_with(b"RIFF") && item.get(8..12) == Some(b"WAVE")
    }

    /// Bytes per frame: the channels times the bytes a sample takes.
    pub fn frame_bytes(&self) -> usize {
        usize::from(self.channels)
            .saturating_mul(usize::from(self.bits / 8))
            .max(1)
    }

    /// How many frames the PCM holds. `frame_bytes` is never nought, so the
    /// checked division is the lint's, not a case.
    pub fn frames(&self) -> usize {
        self.pcm.len().checked_div(self.frame_bytes()).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::{HEADER_LEN, Wav};

    /// A canonical header over `pcm`.
    fn file(channels: u16, rate: u32, bits: u16, pcm: &[u8]) -> Vec<u8> {
        let mut f = Vec::new();
        f.extend_from_slice(b"RIFF");
        f.extend_from_slice(&u32::try_from(36 + pcm.len()).unwrap().to_le_bytes());
        f.extend_from_slice(b"WAVEfmt ");
        f.extend_from_slice(&16u32.to_le_bytes());
        f.extend_from_slice(&1u16.to_le_bytes());
        f.extend_from_slice(&channels.to_le_bytes());
        f.extend_from_slice(&rate.to_le_bytes());
        let block = u32::from(channels) * u32::from(bits / 8);
        f.extend_from_slice(&(rate * block).to_le_bytes());
        f.extend_from_slice(&u16::try_from(block).unwrap().to_le_bytes());
        f.extend_from_slice(&bits.to_le_bytes());
        f.extend_from_slice(b"data");
        f.extend_from_slice(&u32::try_from(pcm.len()).unwrap().to_le_bytes());
        assert_eq!(f.len(), HEADER_LEN);
        f.extend_from_slice(pcm);
        f
    }

    #[test]
    fn the_fields_come_off_the_offsets_the_engine_reads() {
        let w = Wav::parse(&file(1, 22050, 16, &[1, 2, 3, 4])).unwrap();
        assert_eq!((w.channels, w.rate, w.bits), (1, 22050, 16));
        assert_eq!(w.pcm, [1, 2, 3, 4]);
        assert_eq!(w.frame_bytes(), 2);
        assert_eq!(w.frames(), 2);
        assert!(Wav::is_wav(&file(2, 11025, 8, &[])));
        assert!(!Wav::is_wav(b"HMI-MIDISONG061595"));
    }

    #[test]
    fn what_the_engine_would_misread_is_refused() {
        assert!(Wav::parse(&file(1, 22050, 16, &[])[..40]).is_err(), "short");
        let mut f = file(1, 22050, 16, &[0; 4]);
        f[0x18..0x1c].copy_from_slice(&0u32.to_le_bytes());
        assert!(
            Wav::parse(&f).is_err(),
            "a zero rate divides by zero in the original"
        );
        let mut f = file(1, 22050, 16, &[0; 4]);
        f[0x24..0x28].copy_from_slice(b"LIST");
        assert!(
            Wav::parse(&f).is_err(),
            "a chunk before the data would play as sound"
        );
        let mut f = file(1, 22050, 16, &[0; 4]);
        f[0x22..0x24].copy_from_slice(&24u16.to_le_bytes());
        assert!(
            Wav::parse(&f).is_err(),
            "24 bits is nothing the engine plays"
        );
    }
}
