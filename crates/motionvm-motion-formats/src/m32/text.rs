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

use crate::cursor::Cursor;
use crate::error::Result;
use crate::text::TextTable;
use crate::{cp437_to_string, nul_terminated, past_end};

/// Reads a text resource.
pub fn parse(item: &[u8]) -> Result<TextTable> {
    // Text slot 32 holds a two-byte stub instead of a table; treat anything
    // too short to carry a count as an empty table rather than an error.
    if item.len() < 4 {
        return Ok(TextTable::default());
    }
    let mut c = Cursor::new(item, 0);
    let count = c.u32at()?;
    let Some(after_first) = count.checked_sub(1) else {
        return Ok(TextTable::default());
    };
    // The table, which a bogus count fails to cut before anything is
    // allocated from it.
    let offsets = c.records::<2>(after_first)?;
    let area_at = c.position();
    let area = c.remaining();

    let mut strings = Vec::with_capacity(count);
    let starts =
        std::iter::once(0).chain(offsets.iter().map(|o| usize::from(u16::from_le_bytes(*o))));
    for rel in starts {
        let bytes = area
            .get(rel..)
            .ok_or_else(|| past_end(item, area_at.saturating_add(rel), 1))?;
        strings.push(cp437_to_string(nul_terminated(bytes)));
    }
    Ok(TextTable { strings })
}
