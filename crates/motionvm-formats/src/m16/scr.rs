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
//! to the end of the item. Measured over all 65 modules of ENVIRO: the
//! repeated triple equals the first, the twenty bytes are zero, the module
//! number equals the slot, the first word's id is `firstID`, the last word's
//! is `lastID`, and there are `nwords` words.
//!
//! What is different from the 32-bit module ([`crate::m32::scr`]): there is
//! no magic, no `DP`/`LAST`, no second region; a cell is 16 bits; and a word
//! is named by a **global id**, not by `(module << 16) | offset`. Ids are
//! allocated per module at authoring time and reused across modules that are
//! never loaded together — every one of ENVIRO's sixteen location macros
//! defines id 549 — so which word an id names depends on what is resident.
//! This reader keeps the ids as stored and leaves the binding to the machine.
//!
//! A body's first cell says what kind of word it is: `0x8000 | 37`
//! (`_PutAdr`) opens a variable, whose data cell and any `ALLOT` cells
//! follow; `0x8000 | 38` (`_PutConst`) opens a constant; anything else is a
//! colon definition. ENVIRO's 1774 words are 792 variables, 310 constants and
//! 672 colon definitions.

use crate::error::{Error, Result};
use crate::{cp437_to_string, u16le};

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
        self.offset + WORD_HEADER_LEN
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
    /// every ENVIRO module.
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
        if item.len() < HEADER_LEN {
            return Err(Error::Truncated {
                off: 0,
                need: HEADER_LEN,
                have: item.len(),
            });
        }
        let first_id = u16le(item, 0)?;
        let last_id = u16le(item, 2)?;
        let nwords = u16le(item, 4)? as usize;
        let again = (u16le(item, 6)?, u16le(item, 8)?, u16le(item, 10)? as usize);
        if again != (first_id, last_id, nwords) {
            return Err(Error::Corrupt {
                what: "script module",
                detail: format!(
                    "header repeats ({}, {}, {}) as ({}, {}, {})",
                    first_id, last_id, nwords, again.0, again.1, again.2
                ),
            });
        }
        let module = u16le(item, 0x20)?;
        let dict = HEADER_LEN + 2 * nwords;
        if dict > item.len() {
            return Err(Error::Truncated {
                off: HEADER_LEN,
                need: 2 * nwords,
                have: item.len(),
            });
        }
        let mut body_offsets = Vec::with_capacity(nwords);
        for i in 0..nwords {
            body_offsets.push(u16le(item, HEADER_LEN + 2 * i)? as usize);
        }
        let mut entries = Vec::with_capacity(nwords);
        for (i, &cells) in body_offsets.iter().enumerate() {
            let body_start = dict + cells * 2;
            // The header is the sixteen bytes before the body, so the first
            // body cannot sit closer than that to the dictionary start.
            let header = body_start
                .checked_sub(WORD_HEADER_LEN)
                .filter(|&h| h >= dict);
            let Some(header) = header else {
                return Err(Error::Corrupt {
                    what: "script module",
                    detail: format!(
                        "word {i} has its body {cells} cells in, before its own header"
                    ),
                });
            };
            let end = match body_offsets.get(i + 1) {
                Some(&next) => dict + next * 2 - WORD_HEADER_LEN,
                None => item.len(),
            };
            if end < body_start || end > item.len() {
                return Err(Error::Corrupt {
                    what: "script module",
                    detail: format!(
                        "word {i} runs from {body_start:#x} to {end:#x} in a {}-byte item",
                        item.len()
                    ),
                });
            }
            if !(end - body_start).is_multiple_of(2) {
                return Err(Error::Corrupt {
                    what: "script module",
                    detail: format!("word {i} is {} bytes, not whole cells", end - body_start),
                });
            }
            let declared_len = item[header] as usize;
            let name_field = &item[header + 1..header + 1 + NAME_CAP];
            let kept = name_field
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(NAME_CAP)
                .min(declared_len.max(1));
            let name = cp437_to_string(&name_field[..kept]);
            let id = u16le(item, header + 12)?;
            let link = u16le(item, header + 14)?;
            let body = (body_start..end)
                .step_by(2)
                .map(|o| u16le(item, o))
                .collect::<Result<Vec<_>>>()?;
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
