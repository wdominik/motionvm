//! Raw fonts (the FNT segment): the glyph table of [`crate::font::Font`] as
//! it lies in the container, with no compression wrapper.
//!
//! The 32-bit engine packs the same table with its LZW codec behind a 10-byte
//! header ([`crate::m32::font`]); the 16-bit engine stores it bare. ENVIRO's
//! three fonts — ids 0, 2 and 7 — all hold 116 glyphs, 12, 14 and 11 rows
//! high, their bitmaps starting at byte 468 and packed back to back. The bit
//! order is least significant bit first here too: rendered that way, glyph 0
//! of font 0 is an `A`, glyph 26 an `Ä`, glyph 52 an `x`, and the same glyphs
//! of font 7 are those letters in outline.

use crate::error::Result;
use crate::font::Font;

/// Reads one FNT item.
pub fn parse(item: &[u8]) -> Result<Font> {
    Font::from_glyph_table(item)
}
