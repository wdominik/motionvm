//! `II` dialogue records, and the [`Bank`] that holds them.
//!
//! A dialogue resource — the first two of `DIALOG.DAT`, resource 3 of a
//! `DSROOM.DAT` slot — is a [`Bank`]: a directory of `II` blobs. Tag `II`
//! at offset 0:
//! ```text
//! "II"
//! u16 count
//! u16 mode          # record 0's flags
//! u16 id            # record 0's id
//! # then count × 8 bytes:
//! u16 textOff
//! u16 replyOff      # 0 = leaf; else a string *or* a next-list
//! u16 flag          # the *next* record's flags
//! u16 next id       # the *next* record's id
//! # CP437 string pool / next-lists (offsets from start of this II blob)
//! ```
//!
//! The player draws the boundary four bytes earlier than the layout above
//! and reads records of `{flags, id, textOff, replyOff}` starting at `+4`,
//! so that the mode word and the first node are one record. Both
//! descriptions cover the same bytes —
//! `textOff` lands at `8+8i` either way. What this reader calls `flag` is
//! the next record's `flags`.
//!
//! A next-list is `u16 n` then `n` node indices, `n` in 1..=32 and every
//! index `< count`; a reply that does not read as one is a C string.
//!
//! Walking the tree as data is what this reader does; picking a node is the
//! engine's.

use crate::error::{Error, Result};
use crate::rib::directory;
use crate::{NULLPTR, bytes, c_string, cp437_to_string, records, slice, u16le};

/// One node of an `II` tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// The prompt / question, decoded from CP437. Control bytes (`\x01`,
    /// `\x02`, `\x03`) that the original substitutes at run time are left in
    /// place: they are part of the record, not damage.
    pub text: String,
    /// Mode-1 answer string. Empty when `replyOff` was 0 or a next-list.
    pub reply: String,
    /// Next-list from `replyOff`: `u16 n` + `n` node indices. Empty on a
    /// leaf or a mode-1 string reply.
    pub next: Vec<u16>,
    /// Word at node `+4`: the flags of the following record
    /// (`blob+4+(i+1)*8`).
    pub flag: u16,
    /// Word at node `+6`: the id of the following record, the one the
    /// parsec1994 generation's player hands its hook with code 4
    /// (`0x0DA2C`).
    pub next_id: u16,
}

/// One record the way the parsec1994 generation's player reads it: eight
/// bytes from the blob plus four, `{flags, id, textOff, replyOff}`
/// (`0x0D9FE`, `0x0DA1C`, `0x0DAAC`, `0x0DA70`).
///
/// The header's mode word and the first node are one record under this
/// reading, so record `i`'s flags are what [`Line::flag`] of node `i - 1`
/// holds, and record 0's are the header's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record<'a> {
    /// `+0`: the low nibble is the speaker plus one, or 0 for a choice
    /// list; bit 12 asks a flag, bit 13 sets one, bits 4–11 name it, bit
    /// 15 marks the record spent.
    pub flags: u16,
    /// `+2`: an id the hook is told about.
    pub id: u16,
    /// The text `+4` points at.
    pub text: &'a str,
    /// The reply `+6` points at, when it is a string.
    pub reply: &'a str,
    /// The reply, when it is a next-list.
    pub next: &'a [u16],
}

/// One `II` blob: a count of lines and the mode word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dialogue {
    /// `mode` as stored: record 0's flags word under the parsec1994 reading.
    pub mode: u16,
    /// The header's fourth word, record 0's id under the parsec1994 reading.
    pub first_id: u16,
    /// Nodes in table order.
    pub lines: Vec<Line>,
}

impl Dialogue {
    /// Parses one `II` blob of a parsec1994 [`Bank`], each record's text
    /// read the way that generation's player copies it with `13A7h:0023h`:
    /// a zero that is the argument of a `01`, `02` or `03` control does not
    /// end it.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let head = crate::Record(bytes::<8>(data, 0)?);
        let tag = head.bytes::<0, 2>();
        if tag != *b"II" {
            return Err(Error::Corrupt {
                what: "II dialogue",
                detail: format!("expected II, found {tag:02x?}"),
            });
        }
        let count = usize::from(head.u16::<2>());
        let lines = records::<8>(data, 8, count)?
            .iter()
            .map(|rec| {
                let rec = crate::Record(rec);
                let reply_off = usize::from(rec.u16::<2>());
                let next = next_list(data, reply_off, count);
                let reply = if next.is_empty() {
                    cstring(data, reply_off)
                } else {
                    String::new()
                };
                Line {
                    text: record_text(data, usize::from(rec.u16::<0>())),
                    reply,
                    next,
                    flag: rec.u16::<4>(),
                    next_id: rec.u16::<6>(),
                }
            })
            .collect();
        Ok(Dialogue {
            mode: head.u16::<4>(),
            first_id: head.u16::<6>(),
            lines,
        })
    }

    /// Record `i` under the parsec1994 reading, or `None` past the count.
    pub fn record(&self, i: usize) -> Option<Record<'_>> {
        let line = self.lines.get(i)?;
        let (flags, id) = match i.checked_sub(1).and_then(|p| self.lines.get(p)) {
            Some(prev) => (prev.flag, prev.next_id),
            None => (self.mode, self.first_id),
        };
        Some(Record {
            flags,
            id,
            text: &line.text,
            reply: &line.reply,
            next: &line.next,
        })
    }

    /// How many records the blob counts.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// `true` when the blob counts no records.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// `true` when `data` starts with `II`.
    pub fn looks_like(data: &[u8]) -> bool {
        data.starts_with(b"II")
    }
}

/// `replyOff` is a next-list when the first word `n` is in 1..=32 and
/// every following index is `< node_count`. Otherwise it is a C string.
fn next_list(data: &[u8], off: usize, node_count: usize) -> Vec<u16> {
    if off == 0 {
        return Vec::new();
    }
    let Ok(n) = u16le(data, off) else {
        return Vec::new();
    };
    if n == 0 || n > 32 {
        return Vec::new();
    }
    let Ok(indices) = records::<2>(data, off.saturating_add(2), usize::from(n)) else {
        return Vec::new();
    };
    let out: Vec<u16> = indices.iter().map(|i| u16::from_le_bytes(*i)).collect();
    if out.iter().any(|&i| usize::from(i) >= node_count) {
        return Vec::new();
    }
    out
}

/// A record's text as the parsec1994 generation's player reaches its end.
///
/// The player never measures a record's text on its own: `0CFDh:0932h`
/// hands the record's pointer to `13A7h:0023h` (`0x0D93D`; the menu does
/// the same at `0x0DC14`), and that copy decides where the text ends:
/// it copies a byte above 1Fh (`0x13AB3`), ends at a zero (`0x13AB7`), copies
/// one above 3 (`0x13ABB`), copies a 1 or a 3 together with the byte after
/// it (`0x13AC8`), and for a 2 takes the byte after it as an index, whatever
/// that byte is (`0x13ABF`, `0x13AD2`). It also stops where the room its
/// caller hands it runs out (`0x13AC3`, `0x13AEA`): `[1E4Ah]`, half of the
/// 2000-byte scratch `[1691h]` (`0x0DF3D`–`0x0DF45`), which no text Dunkle
/// Schatten ships reaches: the longest is 510 bytes.
///
/// The byte after `01`, `02` or `03` is an argument and is never tested
/// for zero, so `01 00` (the first ink again) and `02 00` (substitution 0)
/// are inside the text, and the text ends at the first zero that is not
/// such an argument.
fn record_text(data: &[u8], off: usize) -> String {
    if off == 0 {
        return String::new();
    }
    let rest = data.get(off..).unwrap_or_default();
    let mut end = rest.len();
    let mut walk = rest.iter().enumerate();
    while let Some((i, &b)) = walk.next() {
        match b {
            0 => {
                end = i;
                break;
            }
            1..=3 => {
                walk.next();
            }
            _ => {}
        }
    }
    cp437_to_string(rest.get(..end).unwrap_or(rest))
}

fn cstring(data: &[u8], off: usize) -> String {
    cp437_to_string(c_string(data, off))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_magic_is_refused() {
        assert!(Dialogue::parse(&[0, 0, 0, 0, 0, 0, 0, 0]).is_err());
    }

    /// A zero after `01`, `02` or `03` is the control's argument, and the
    /// text goes on past it.
    #[test]
    fn a_control_argument_of_zero_does_not_end_a_1994_text() {
        let mut blob = b"II\x01\x00\x01\x00\x00\x00".to_vec();
        blob.extend_from_slice(&[16, 0, 0, 0, 0, 0, 0, 0]);
        blob.extend_from_slice(b"\x01\x01HEAD\n\x01\x00body \x02\x00!\r\x00tail");
        let read = Dialogue::parse(&blob).unwrap();
        assert_eq!(read.lines[0].text, "\u{1}\u{1}HEAD\n\u{1}\0body \u{2}\0!\r");
    }

    #[test]
    fn an_empty_tree_parses() {
        let d = Dialogue::parse(b"II\x00\x00\x01\x00\x00\x00").unwrap();
        assert!(d.lines.is_empty());
        assert_eq!(d.mode, 1);
    }
}

/// A parsec1994 bank of `II` dialogues.
///
/// A bank is a second directory — the same `u32 off[n]`, `u32 0`,
/// `u32 size[n]` shape a RIB's own table uses — nested inside a resource,
/// and an entry is addressed by number.
///
/// `XP01` reaches a bank through one of three tables and says so when an
/// entry comes up empty: `NULLDLG: D%03d_%03d.DCS`, formatted with the bank's
/// number and the entry's. Those empty entries are in the file as the
/// literal `NULLPTR\r\n`, so a bank is not a list of dialogues but a list
/// of numbered places, some of which are deliberately unused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bank {
    /// Entries in table order. `None` is a `NULLPTR\r\n` place.
    pub entries: Vec<Option<Dialogue>>,
}

impl Bank {
    /// Parses one resource holding a directory of `II` blobs.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let ranges = directory(data, "dialogue bank")?.ok_or_else(|| Error::Corrupt {
            what: "dialogue bank",
            detail: "no directory table at offset 0".to_string(),
        })?;
        let mut entries = Vec::with_capacity(ranges.len());
        for range in ranges {
            let blob = slice(data, range.start, range.len())?;
            entries.push(if blob.starts_with(NULLPTR) {
                None
            } else {
                Some(Dialogue::parse(blob)?)
            });
        }
        Ok(Self { entries })
    }

    /// `true` when this resource is a directory whose every entry is either
    /// an `II` blob or the `NULLPTR` marker.
    ///
    /// Both halves are needed. The directory shape alone also fits a RIB's
    /// own table, and `II` alone fits a resource that *is* one dialogue.
    pub fn looks_like(data: &[u8]) -> bool {
        let Ok(Some(ranges)) = directory(data, "dialogue bank") else {
            return false;
        };
        !ranges.is_empty()
            && ranges.iter().all(|r| {
                data.get(r.clone())
                    .is_some_and(|blob| blob.starts_with(NULLPTR) || Dialogue::looks_like(blob))
            })
    }

    /// Entry `i`, or `None` when it is past the table or a `NULLPTR` place.
    pub fn get(&self, i: usize) -> Option<&Dialogue> {
        self.entries.get(i).and_then(|e| e.as_ref())
    }

    /// How many places the bank has, used and unused alike.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` when the bank has no places at all.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
