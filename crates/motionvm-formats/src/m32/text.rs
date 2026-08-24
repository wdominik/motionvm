//! Text tables as the 32-bit engine stores them (`Kind::Text`).
//!
//! ```text
//! u32       string count
//! u16[n-1]  start of strings 1..n-1, relative to the start of the string area
//! ...       NUL-terminated CP437 strings, the first one at offset 0
//! ```
//!
//! The offset table has one entry fewer than there are strings because string 0
//! always sits at the start of the area. Strings may contain `\n`; index 0 is
//! usually empty and used as a "no text" sentinel. The 16-bit engine's table
//! has a `u16` count and an offset for every string ([`crate::m16::text`]);
//! both decode to a [`crate::TextTable`].

use crate::error::{Error, Result};
use crate::text::TextTable;
use crate::{cp437_to_string, u16le, u32le};

/// Reads a text resource.
pub fn parse(item: &[u8]) -> Result<TextTable> {
    // Text slot 32 holds a two-byte stub instead of a table; treat anything
    // too short to carry a count as an empty table rather than an error.
    if item.len() < 4 {
        return Ok(TextTable::default());
    }
    let count = u32le(item, 0)? as usize;
    if count == 0 {
        return Ok(TextTable::default());
    }
    // Guard against a bogus count before allocating from it.
    let area = 4 + 2 * (count - 1);
    if area > item.len() {
        return Err(Error::Truncated {
            off: 0,
            need: area,
            have: item.len(),
        });
    }

    let mut strings = Vec::with_capacity(count);
    for i in 0..count {
        let rel = if i == 0 {
            0
        } else {
            u16le(item, 4 + 2 * (i - 1))? as usize
        };
        let start = area + rel;
        let bytes = item.get(start..).ok_or(Error::Truncated {
            off: start,
            need: 1,
            have: item.len(),
        })?;
        let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
        strings.push(cp437_to_string(&bytes[..end]));
    }
    Ok(TextTable { strings })
}
