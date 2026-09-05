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

use crate::cursor::Cursor;
use crate::error::Result;
use crate::text::TextTable;
use crate::{cp437_to_string, nul_terminated, past_end};

/// Reads one TXT item.
pub fn parse(item: &[u8]) -> Result<TextTable> {
    let mut c = Cursor::new(item, 0);
    let count = usize::from(c.u16()?);
    let offsets = c.records::<2>(count)?;
    let area_at = c.position();
    let area = c.remaining();
    let mut strings = Vec::with_capacity(count);
    for rel in offsets.iter().map(|o| usize::from(u16::from_le_bytes(*o))) {
        let bytes = area
            .get(rel..)
            .ok_or_else(|| past_end(item, area_at.saturating_add(rel), 1))?;
        strings.push(cp437_to_string(nul_terminated(bytes)));
    }
    Ok(TextTable { strings })
}
