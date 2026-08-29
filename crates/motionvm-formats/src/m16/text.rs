//! Text tables (the TXT segment).
//!
//! ```text
//! u16     string count n
//! u16[n]  offset of each string, relative to the start of the string area
//! ...     the string area: n NUL-terminated CP437 strings
//! ```
//!
//! The string area begins right after the offset table, at `2 + 2 * n`, and
//! every offset counts from there — `off[0]` is 0 in every one of the 96
//! tables of Die Enviro-Kids greifen ein
//! tables. The 32-bit engine's table differs on both points: a `u32` count,
//! offsets from the item start, and no entry for string 0
//! ([`crate::m32::text`]). What comes out is the same [`crate::TextTable`].
//!
//! Line breaks are a single `0x0A`; 2199 of the 3508 strings of
//! Die Enviro-Kids greifen ein contain
//! one. `SDTB` selects the table, `SDTXT n` the string — entry `n - 1`, as
//! [`crate::TextTable::get`] counts.

use crate::error::{Error, Result};
use crate::text::TextTable;
use crate::{cp437_to_string, u16le};

/// Reads one TXT item.
pub fn parse(item: &[u8]) -> Result<TextTable> {
    let count = u16le(item, 0)? as usize;
    let area = 2 + 2 * count;
    if area > item.len() {
        return Err(Error::Truncated {
            off: 0,
            need: area,
            have: item.len(),
        });
    }
    let mut strings = Vec::with_capacity(count);
    for i in 0..count {
        let start = area + u16le(item, 2 + 2 * i)? as usize;
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
