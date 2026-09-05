//! Compiled Forth modules of the 16-bit engine (the SCR segment).
//!
//! ```text
//! 0x00  u16  firstID   lowest global word id defined here
//! 0x02  u16  lastID    highest
//! 0x04  u16  nwords
//! 0x06  u16  firstID, lastID, nwords once more
//! 0x0c  u8[20]  zero
//! 0x20  u16  module number — the SCR slot minus 3500
//! 0x22  u16[nwords]  body offsets, in cells from the start of the dictionary
//!       ...  the dictionary: for every word, immediately before its body,
//!            u8      name length before truncation
//!            u8[11]  name, NUL-padded, CP437
//!            u16     global word id
//!            u16     link
//!            u16[]   the body — threaded code, or a VAR/CONST data cell
//! ```
//!
//! A body offset points at the **body**; the 16-byte header is the sixteen
//! bytes before it, so the first offset is at least 8 cells. Headers are not
//! gathered in a table — each travels with its body — and the last body runs
//! to the end of the item. Measured over all 65 modules of
//! Die Enviro-Kids greifen ein: the
//! repeated triple equals the first, the twenty bytes are zero, the module
//! number equals the slot, the first word's id is `firstID`, the last word's
//! is `lastID`, and there are `nwords` words.
//!
//! What is different from the 32-bit module ([`crate::m32::scr`]): there is
//! no magic, no `DP`/`LAST`, no second region; a cell is 16 bits; and a word
//! is named by a **global id**, not by `(module << 16) | offset`. Ids are
//! allocated per module at authoring time and reused across modules that are
//! never loaded together — every one of the sixteen location macros of
//! Die Enviro-Kids greifen ein
//! defines id 549 — so which word an id names depends on what is resident.
//! This reader keeps the ids as stored and leaves the binding to the machine.
//!
//! A body's first cell says what kind of word it is: `0x8000 | 37`
//! (`_PutAdr`) opens a variable, whose data cell and any `ALLOT` cells
//! follow; `0x8000 | 38` (`_PutConst`) opens a constant; anything else is a
//! colon definition. The 1774 words of Die Enviro-Kids greifen ein are 792
//! variables, 310 constants and
//! 672 colon definitions.

use crate::cursor::Cursor;
use crate::error::{Error, Result};
use crate::{Record, bytes, cp437_char, slice};

/// Bytes of fixed header before the body-offset table.
pub const HEADER_LEN: usize = 0x22;
/// Bytes of a word header: length byte, eleven name bytes, id, link.
pub const WORD_HEADER_LEN: usize = 16;
/// Longest name the compiler stores; longer names are cut to this and keep
/// their original length in the length byte.
pub const NAME_CAP: usize = 11;

#[derive(Debug, Clone)]
/// One word of a module's dictionary: its header and its cells.
pub struct Entry {
    /// Name as stored, at most eleven characters.
    pub name: String,
    /// Name length before truncation.
    pub declared_len: usize,
    /// The global word id a call cell names this word by.
    pub id: u16,
    /// The field at +14. Read and kept; what it links is open.
    pub link: u16,
    /// Offset of the header within the item.
    pub offset: usize,
    /// The body, one `u16` per cell.
    pub body: Vec<u16>,
}

impl Entry {
    /// Offset of the first body cell within the item.
    pub fn body_offset(&self) -> usize {
        self.offset.saturating_add(WORD_HEADER_LEN)
    }

    /// Whether the body opens a variable (`_PutAdr`, ordinal 37): its first
    /// cell pushes the address of the next one and returns, so everything
    /// after it is data.
    pub fn is_variable(&self) -> bool {
        self.body.first() == Some(&(0x8000 | 37))
    }

    /// Whether the body opens a constant (`_PutConst`, ordinal 38).
    pub fn is_constant(&self) -> bool {
        self.body.first() == Some(&(0x8000 | 38))
    }
}

#[derive(Debug, Clone)]
/// One parsed 16-bit module: its number, its id range and its dictionary.
pub struct ScrModule {
    /// The module number, from the header — equal to the slot minus 3500 in
    /// every module of Die Enviro-Kids greifen ein.
    pub module: u16,
    /// The lowest id defined here, as the header states it.
    pub first_id: u16,
    /// The highest.
    pub last_id: u16,
    /// The dictionary, in file order.
    pub entries: Vec<Entry>,
}

impl ScrModule {
    /// Reads a script module.
    pub fn parse(item: &[u8]) -> Result<Self> {
        let head = Record(bytes::<HEADER_LEN>(item, 0)?);
        let first_id = head.u16::<0>();
        let last_id = head.u16::<2>();
        let nwords = usize::from(head.u16::<4>());
        let again = (
            head.u16::<6>(),
            head.u16::<8>(),
            usize::from(head.u16::<10>()),
        );
        if again != (first_id, last_id, nwords) {
            return Err(Error::Corrupt {
                what: "script module",
                detail: format!(
                    "header repeats ({}, {}, {}) as ({}, {}, {})",
                    first_id, last_id, nwords, again.0, again.1, again.2
                ),
            });
        }
        let module = head.u16::<0x20>();
        let mut c = Cursor::new(item, HEADER_LEN);
        let body_offsets: Vec<usize> = c
            .records::<2>(nwords)?
            .iter()
            .map(|o| usize::from(u16::from_le_bytes(*o)))
            .collect();
        let dict = c.position();
        // Where each body starts, in bytes from the dictionary start, and
        // where the next one does — the end of the item for the last.
        let starts = body_offsets
            .iter()
            .map(|&cells| cells.checked_mul(2).and_then(|b| dict.checked_add(b)));
        let nexts = starts
            .clone()
            .skip(1)
            .map(Some)
            .chain(std::iter::once(None));
        let mut entries = Vec::with_capacity(nwords);
        for (i, (start, next)) in starts.zip(nexts).enumerate() {
            // The header is the sixteen bytes before the body, so the first
            // body cannot sit closer than that to the dictionary start.
            let header = start
                .and_then(|s| s.checked_sub(WORD_HEADER_LEN))
                .filter(|&h| h >= dict);
            let Some((body_start, header)) = start.zip(header) else {
                return Err(Error::Corrupt {
                    what: "script module",
                    detail: format!("word {i} has its body before its own header"),
                });
            };
            let end = match next {
                Some(next) => next.and_then(|n| n.checked_sub(WORD_HEADER_LEN)),
                None => Some(item.len()),
            };
            let Some(len) = end
                .filter(|&end| end <= item.len())
                .and_then(|end| end.checked_sub(body_start))
            else {
                return Err(Error::Corrupt {
                    what: "script module",
                    detail: format!(
                        "word {i} runs from {body_start:#x} past the next word or the {}-byte item",
                        item.len()
                    ),
                });
            };
            if !len.is_multiple_of(2) {
                return Err(Error::Corrupt {
                    what: "script module",
                    detail: format!("word {i} is {len} bytes, not whole cells"),
                });
            }
            let head = Record(bytes::<WORD_HEADER_LEN>(item, header)?);
            let declared_len = usize::from(head.u8::<0>());
            let name_field = head.bytes::<1, NAME_CAP>();
            let kept = name_field
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(NAME_CAP)
                .min(declared_len.max(1));
            let name = name_field
                .iter()
                .take(kept)
                .map(|&b| cp437_char(b))
                .collect();
            let id = head.u16::<12>();
            let link = head.u16::<14>();
            let body = slice(item, body_start, len)?
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect();
            entries.push(Entry {
                name,
                declared_len,
                id,
                link,
                offset: header,
                body,
            });
        }
        Ok(Self {
            module,
            first_id,
            last_id,
            entries,
        })
    }

    /// The word with global id `id`, if this module defines it.
    pub fn entry(&self, id: u16) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// The first word of that name, if this module defines one.
    pub fn entry_named(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// Cells of code and data in all bodies together.
    pub fn cell_count(&self) -> usize {
        self.entries.iter().map(|e| e.body.len()).sum()
    }
}
