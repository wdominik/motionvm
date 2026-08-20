//! Bitmap fonts (`Kind::Font`, and the standalone `000.FNT`) and the character
//! reference table `000.FRT`.
//!
//! A font is LZW-compressed with the very same codec as the sprites — see
//! [`crate::lzw`] — and its header states the codec parameters outright rather
//! than leaving them implicit:
//!
//! ```text
//! +0   u16  unpacked size
//! +2   u16  the same size again
//! +4   u16  packed size (file length minus this 10-byte header)
//! +6   u16  dictionary limit, 2048, i.e. codes grow to 11 bits
//! +8   u16  initial code width, 9
//! +10  ...  LZW stream
//! ```
//!
//! What comes out is a glyph table followed by the bitmaps:
//!
//! ```text
//! +0   u16  glyph count
//! +2   u16  height, shared by every glyph
//! +4   n x { u16 offset, u16 width }
//! ...  bitmaps: ceil(width / 8) bytes per row, `height` rows, 1 bit per pixel
//! ```
//!
//! Pixels within a byte are **least significant bit first**, which is the one
//! part that cannot be read off the structure: taking them the other way round
//! produces shapes that look vaguely glyph-like but are not letters. Rendering
//! the first glyphs of `008.FNT` settles it — they spell out A B C D E F G H,
//! matching `000.FRT`, where `'A'` maps to glyph 0.

use crate::error::{Error, Result};
use crate::{lzw, u16le};

/// `000.FRT` — maps a CP437 character code to a glyph index.
///
/// ```text
/// u16       number of character slots (256)
/// u16       number of glyphs
/// u16[256]  glyph index per character, 0xFFFF where the font has no glyph
/// ```
#[derive(Debug, Clone)]
pub struct FontRefTable {
    /// How many glyphs the fonts this table serves are expected to hold.
    pub glyph_count: usize,
    /// One entry per character code; `None` where the font has no glyph.
    pub map: Vec<Option<u16>>,
}

impl FontRefTable {
    /// Reads `000.FRT`.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let slots = u16le(data, 0)? as usize;
        let glyph_count = u16le(data, 2)? as usize;
        let mut map = Vec::with_capacity(slots);
        for i in 0..slots {
            let v = u16le(data, 4 + i * 2)?;
            map.push(if v == 0xffff { None } else { Some(v) });
        }
        Ok(Self { glyph_count, map })
    }

    /// The glyph index for one character code, or `None` where the font has
    /// none. Codes are CP437 bytes, one byte to one character.
    pub fn glyph_for(&self, ch: u8) -> Option<u16> {
        self.map.get(ch as usize).copied().flatten()
    }
}

/// A single glyph, one bit per pixel.
#[derive(Debug, Clone)]
pub struct Glyph {
    /// Pixels across.
    pub width: u16,
    /// Pixels down.
    pub height: u16,
    /// `ceil(width / 8) * height` bytes, row by row.
    pub bits: Vec<u8>,
}

impl Glyph {
    /// Bytes per row of [`Glyph::bits`], rounded up.
    pub fn stride(&self) -> usize {
        (self.width as usize).div_ceil(8)
    }

    /// Whether the pixel at `(x, y)` is set. Out-of-range reads as unset.
    pub fn pixel(&self, x: u16, y: u16) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let byte = y as usize * self.stride() + x as usize / 8;
        // Least significant bit first, established by rendering known letters.
        self.bits.get(byte).is_some_and(|b| b >> (x % 8) & 1 == 1)
    }
}

/// Bytes of font header before the first glyph.
pub const HEADER_LEN: usize = 10;

#[derive(Debug, Clone)]
/// One `.FNT` resource: a common line height and the glyphs themselves.
pub struct Font {
    /// The height every glyph is laid out on.
    pub height: u16,
    /// The glyphs, indexed as [`FontRefTable::glyph_for`] answers.
    pub glyphs: Vec<Glyph>,
}

impl Font {
    /// Reads a font resource.
    pub fn parse(item: &[u8]) -> Result<Self> {
        if item.len() < HEADER_LEN {
            return Err(Error::Truncated {
                off: 0,
                need: HEADER_LEN,
                have: item.len(),
            });
        }
        let unpacked = u16le(item, 0)? as usize;
        let dictionary_limit = u16le(item, 6)? as u32;
        let initial_width = u16le(item, 8)? as u32;
        // The header carries the codec parameters, so derive the code width from
        // the dictionary limit rather than assuming the sprites' 11 bits.
        let max_bits = dictionary_limit.max(2).ilog2();
        if initial_width != 9 {
            return Err(Error::Corrupt {
                what: "font",
                detail: format!(
                    "font declares an initial code width of {initial_width}, expected 9"
                ),
            });
        }

        let raw = lzw::decode(&item[HEADER_LEN..], max_bits, unpacked)?;

        let count = u16le(&raw, 0)? as usize;
        let height = u16le(&raw, 2)?;
        let table_end = 4 + count * 4;
        if table_end > raw.len() {
            return Err(Error::Truncated {
                off: 4,
                need: count * 4,
                have: raw.len(),
            });
        }

        let mut glyphs = Vec::with_capacity(count);
        for i in 0..count {
            let offset = u16le(&raw, 4 + i * 4)? as usize;
            let width = u16le(&raw, 6 + i * 4)?;
            let stride = (width as usize).div_ceil(8);
            let len = stride * height as usize;
            let bits = raw
                .get(offset..offset + len)
                .ok_or(Error::Truncated {
                    off: offset,
                    need: len,
                    have: raw.len(),
                })?
                .to_vec();
            glyphs.push(Glyph {
                width,
                height,
                bits,
            });
        }

        Ok(Self { height, glyphs })
    }

    /// Where the glyph bitmaps begin, which is also where the table ends.
    ///
    /// The two must coincide; that they do in every shipped font is what
    /// confirms the table has exactly `glyph_count` entries and no header
    /// fields were missed.
    pub fn table_end(&self) -> usize {
        4 + self.glyphs.len() * 4
    }

    /// Total width of `text` on screen, for laying out a line.
    ///
    /// A pixel of spacing follows every glyph but the last — see the engine's
    /// text drawing for where that number comes from.
    pub fn text_width(&self, text: &str, refs: &FontRefTable) -> u32 {
        let total: u32 = text
            .bytes()
            .filter_map(|b| refs.glyph_for(b))
            .filter_map(|g| self.glyphs.get(g as usize))
            .map(|g| g.width as u32 + 1)
            .sum();
        total.saturating_sub(1)
    }
}
