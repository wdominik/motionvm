//! Fonts as the 32-bit engine stores them (`Kind::Font`, and the loose
//! `000.FNT`): the glyph table of [`crate::font::Font`], LZW-compressed with
//! the very same codec as the sprites — see [`crate::lzw`] — behind a
//! header that states the codec parameters outright rather than leaving them
//! implicit:
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
//! What comes out is the glyph table the crate root decodes. The 16-bit
//! engine stores that table bare ([`crate::m16::font`]).

use crate::error::{Error, Result};
use crate::font::Font;
use crate::lzw;
use crate::past_end;

/// Bytes of font header before the LZW stream.
pub const HEADER_LEN: usize = 10;

/// Reads one font resource: unpacks the stream and decodes the glyph table.
pub fn parse(item: &[u8]) -> Result<Font> {
    let (head, stream) = item
        .split_first_chunk::<HEADER_LEN>()
        .ok_or_else(|| past_end(item, 0, HEADER_LEN))?;
    let unpacked = usize::from(u16::from_le_bytes([head[0], head[1]]));
    let dictionary_limit = u32::from(u16::from_le_bytes([head[6], head[7]]));
    let initial_width = u32::from(u16::from_le_bytes([head[8], head[9]]));
    // The header carries the codec parameters, so derive the code width from
    // the dictionary limit rather than assuming the sprites' 11 bits.
    let max_bits = dictionary_limit.max(2).ilog2();
    if initial_width != 9 {
        return Err(Error::Corrupt {
            what: "font",
            detail: format!("font declares an initial code width of {initial_width}, expected 9"),
        });
    }

    let raw = lzw::decode(stream, max_bits, unpacked)?;
    Font::from_glyph_table(&raw)
}
