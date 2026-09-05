//! Bitmap fonts as both engine generations decode them, and the character
//! reference table `000.FRT` / FRT slot 0.
//!
//! A font is a glyph table followed by the bitmaps:
//!
//! ```text
//! +0   u16  glyph count
//! +2   u16  height, shared by every glyph
//! +4   n x { u16 offset, u16 width }
//! ...  bitmaps: ceil(width / 8) bytes per row, `height` rows, 1 bit per pixel
//! ```
//!
//! That table is what this module reads, with [`Font::from_glyph_table`]. How
//! it is stored differs by generation: the 32-bit engine packs it with its
//! LZW codec behind a 10-byte header ([`crate::m32::font`]), the 16-bit
//! engine stores it bare ([`crate::m16::font`]).
//!
//! Pixels within a byte are **least significant bit first**, which is the one
//! part that cannot be read off the structure: taking them the other way round
//! produces shapes that look vaguely glyph-like but are not letters. Rendering
//! settles it in both generations — the first glyphs of Dunkle Schatten 2's
//! `008.FNT` spell out A B C D E F G H, matching `000.FRT`, where `'A'` maps
//! to glyph 0; and glyph 0, 26 and 52 of Die Enviro-Kids greifen ein's font 0
//! are `A`, `Ä` and `x`.

use crate::cursor::Cursor;
use crate::error::Result;
use crate::slice;

/// `000.FRT` (32-bit) / FRT slot 0 (16-bit) — maps a CP437 character code
/// to a glyph index. The two are laid out identically, 516 bytes each.
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
    /// Reads a font reference table.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let mut c = Cursor::new(data, 0);
        let slots = usize::from(c.u16()?);
        let glyph_count = usize::from(c.u16()?);
        let map = c
            .records::<2>(slots)?
            .iter()
            .map(|e| {
                let v = u16::from_le_bytes(*e);
                (v != 0xffff).then_some(v)
            })
            .collect();
        Ok(Self { glyph_count, map })
    }

    /// The glyph index for one character code, or `None` where the font has
    /// none. Codes are CP437 bytes, one byte to one character.
    pub fn glyph_for(&self, ch: u8) -> Option<u16> {
        self.map.get(usize::from(ch)).copied().flatten()
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
        crate::wide(self.width.into()).div_ceil(8)
    }

    /// Whether the pixel at `(x, y)` is set. Out-of-range reads as unset.
    pub fn pixel(&self, x: u16, y: u16) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        let byte = usize::from(y)
            .checked_mul(self.stride())
            .and_then(|row| row.checked_add(usize::from(x) / 8));
        // Least significant bit first, established by rendering known letters.
        byte.and_then(|byte| self.bits.get(byte))
            .is_some_and(|b| ((b >> (x % 8)) & 1) == 1)
    }
}

#[derive(Debug, Clone)]
/// One decoded font: a common line height and the glyphs themselves.
pub struct Font {
    /// The height every glyph is laid out on.
    pub height: u16,
    /// The glyphs, indexed as [`FontRefTable::glyph_for`] answers.
    pub glyphs: Vec<Glyph>,
}

impl Font {
    /// Reads the decoded glyph table — the layout both generations share
    /// once any compression is undone.
    pub fn from_glyph_table(raw: &[u8]) -> Result<Self> {
        let mut c = Cursor::new(raw, 0);
        let count = usize::from(c.u16()?);
        let height = c.u16()?;
        let table = c.records::<4>(count)?;

        let mut glyphs = Vec::with_capacity(count);
        for entry in table {
            let offset = usize::from(u16::from_le_bytes([entry[0], entry[1]]));
            let width = u16::from_le_bytes([entry[2], entry[3]]);
            let stride = crate::wide(width.into()).div_ceil(8);
            // As many bytes as the glyph needs, which the table must hold.
            let len = stride.saturating_mul(usize::from(height));
            glyphs.push(Glyph {
                width,
                height,
                bits: slice(raw, offset, len)?.to_vec(),
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
        self.glyphs.len().saturating_mul(4).saturating_add(4)
    }

    /// Total width of `text` on screen, for laying out a line.
    ///
    /// A pixel of spacing follows every glyph but the last — see the engine's
    /// text drawing for where that number comes from.
    pub fn text_width(&self, text: &str, refs: &FontRefTable) -> u32 {
        let total: u32 = text
            .bytes()
            .filter_map(|b| refs.glyph_for(b))
            .filter_map(|g| self.glyphs.get(usize::from(g)))
            .map(|g| u32::from(g.width) + 1)
            .sum();
        total.saturating_sub(1)
    }
}
