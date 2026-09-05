//! 8-bit sprites (`Kind::Gfx8`).
//!
//! ```text
//! +0     u8[2]        header; byte 1 mirrors the green channel of color 0
//! +2     u8[768]      VGA palette, 256 x RGB, 6 bits per channel
//! +770   "32BITGFX"   marker
//! +778   u32          unpacked length (6 + width*height)
//! +782   u32          packed length of the first stream
//! +786   u32          maximum LZW code width (11 or 12)
//! +790   ...          LZW stream
//! ```
//!
//! The decompressed stream starts with its own six-byte header — `u16 width`,
//! `u16 height`, `u16 color count` (always 256) — followed by one byte per
//! pixel, top row first.
//!
//! The declared packed length is only reliable for streams that fit in a single
//! block; for larger sprites it undercounts, so decoding is driven purely by the
//! unpacked length and simply reads as far into the item as it needs.

use crate::error::Error;
use crate::error::Result;
use crate::{bytes, lzw, past_end, tail, u32at, u32le};

/// The eight bytes every sprite resource starts with.
pub const MAGIC: &[u8; 8] = b"32BITGFX";
const PALETTE_OFF: usize = 2;
const MAGIC_OFF: usize = 770;
const STREAM_OFF: usize = 790;
/// Size of the header embedded in the decompressed stream.
const INNER_HEADER: usize = 6;

#[derive(Debug, Clone)]
/// One decoded picture: indices into a palette, row-major.
pub struct Sprite {
    /// Pixels per row.
    pub width: u16,
    /// Rows.
    pub height: u16,
    /// The sprite's own palette: 768 bytes of 6-bit VGA RGB, exactly as
    /// stored. Each sprite carries a full 256-color table; turning it into
    /// colors is the caller's step, because what this reader reports is what
    /// the file holds.
    pub palette: [u8; 768],
    /// One byte per pixel, `width * height` of them, top row first.
    pub pixels: Vec<u8>,
}

impl Sprite {
    /// Parses and decompresses one `Kind::Gfx8` item.
    pub fn parse(item: &[u8]) -> Result<Self> {
        let magic = item.get(MAGIC_OFF..MAGIC_OFF + 8).ok_or(Error::Truncated {
            off: MAGIC_OFF,
            need: 8,
            have: item.len(),
        })?;
        if magic != MAGIC {
            let mut found = [0u8; 8];
            found.copy_from_slice(magic);
            return Err(Error::MissingGfxMagic { found });
        }

        let palette = *bytes::<768>(item, PALETTE_OFF)?;
        let unpacked = u32at(item, 778)?;
        let max_bits = u32le(item, 786)?;
        let stream = tail(item, STREAM_OFF)?;

        let raw = lzw::decode(stream, max_bits, unpacked)?;
        let (inner, pixels) = raw
            .split_first_chunk::<INNER_HEADER>()
            .ok_or_else(|| past_end(&raw, 0, INNER_HEADER))?;
        let width = u16::from_le_bytes([inner[0], inner[1]]);
        let height = u16::from_le_bytes([inner[2], inner[3]]);
        // The last two bytes are the color count, always 256; nothing reads it.

        if usize::from(width).checked_mul(usize::from(height)) != Some(pixels.len()) {
            return Err(Error::GfxSizeMismatch {
                width,
                height,
                declared: unpacked,
            });
        }

        Ok(Self {
            width,
            height,
            palette,
            pixels: pixels.to_vec(),
        })
    }

    /// Reads the pixel count without decompressing the stream.
    ///
    /// Only the total is stored outside the stream, so this cannot recover
    /// width and height separately.
    pub fn pixel_count(item: &[u8]) -> Result<usize> {
        // `checked_sub` rather than `-`: the declared size comes out of the
        // file, and one below six wrapped to about four billion in release
        // builds and panicked in debug ones.
        u32at(item, 778)?
            .checked_sub(INNER_HEADER)
            .ok_or(Error::OutOfRange {
                field: "sprite unpacked size",
                value: u64::from(u32le(item, 778)?),
                allowed: "at least the 6-byte inner header",
            })
    }
}
