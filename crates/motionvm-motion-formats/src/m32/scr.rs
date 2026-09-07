//! Compiled Forth modules ("scriptor files", `Kind::Script`).
//!
//! ```text
//! 0x00    "USERDEF\0#F0\0"
//! 0x10    u32  module number (matches the resource id)
//! 0x14    u32  \  addresses of the two regions below, in the writer's address
//! 0x18    u32  /  space; replaced when the module is loaded
//! 0x1c    u32  size of the module memory region
//! 0x20    u32  size of the second region (16004 in every module seen so far)
//! 0x24    u32  DP: the end of module memory, in cells from 0x30
//! 0x28    u32  LAST: the dictionary's link field
//! 0x30    u32  entry point, packed as (module << 16 | offset)
//! 0x50    ...  dictionary
//! ```
//!
//! **Two regions follow the header, not one.** `DP * 4` bytes of module memory
//! from 0x30 on — code, dictionary and data, everything an address can reach —
//! and then exactly 16004 further bytes that the engine allocates as a separate
//! block. `=>GET` (0x64999) allocates the first region at precisely `DP * 4`
//! and overwrites the field at 0x1c with that size, which is why the copies in
//! `001.RSC` can carry 0xFFF0 there: it is the authoring tool's capacity, and
//! the runtime never believes it.
//!
//! Measured across all 86 shipped modules: 0x20 is 16004 in every one, 0x1c is
//! 0xFFF0 in every one, and `0x30 + DP*4 + 16004` never exceeds the item. An
//! empty module has `DP == 8` — `create_module` bumps it by eight on creation —
//! so its memory ends at exactly 0x50 and the whole module is 16084 bytes,
//! which is what the original compiler emits for a source file with nothing in
//! it. No word in any module reaches past `0x30 + DP*4`, which is the
//! measurement that settles where the split lies. Getting it wrong is not
//! harmless: a walk that runs past DP carries the last word of 38 modules on
//! into the second region and hands back cells that are not code.
//!
//! The dictionary is a flat run of word definitions, each one a 16-byte header
//! followed by its body:
//!
//! ```text
//! +0   u8      name length before truncation
//! +1   u8[11]  name, NUL-padded
//! +12  u32     flags: 11 on the first word of a module, 6 on the rest
//! +16  ...     body, a sequence of 32-bit cells
//! ```
//!
//! Bodies are threaded code. A cell is either a kernel word (tagged `0x4000` in
//! the high half, see [`crate::m32::le`]) or a reference to a word in some module as
//! `(module << 16) | offset`. Cells that follow `_PutLit`, `_PutAdr` or
//! `_PutConst` are inline data rather than code.
//!
//! So `VAR X` compiles to `_PutAdr` plus a zeroed cell, and `5 CONST Y` to
//! `_PutConst` plus the literal 5 — which is exactly what the shipped inventory
//! module contains for `STIFT`, `ZETTEL` and the rest.
//!
//! The layout was settled by having the original compiler emit modules under
//! controlled conditions rather than by inference: an empty module comes out at
//! exactly `0x50 + 16004` bytes, and each construct added to the source shows up
//! as a known number of cells at a known offset.
//!
//! Header offsets 0x14/0x18 hold absolute addresses from the authoring tool's
//! address space and mean nothing at runtime.
//!
//! Items inside `001.RSC` may carry padding past the second region — nothing in
//! 60 of them, up to 4208 bytes in the other 26. It is zero except in module
//! 10, which has no words at all and keeps 4208 bytes of data there. So the
//! item's own length, not the sum of the two regions, is what a reader must
//! use.

use crate::error::{Error, Result};
use crate::{Record, bytes, cp437_char, slice};

/// The twelve bytes every script module starts with.
pub const MAGIC: &[u8; 12] = b"USERDEF\0#F0\0";
/// Where the dictionary begins; the length at 0x20 counts from here.
pub const DICT_OFF: usize = 0x50;
const HEADER_LEN: usize = 16;
/// Longest name the compiler stores; longer source names are truncated to this.
const NAME_CAP: usize = 11;
/// The cell that closes a colon definition.
const EXIT_CELL: u32 = 0;
/// What a call offset counts cells from. Also where the size field at 0x1c is
/// measured from, so it is presumably where the module image starts in memory.
const ADDRESS_BASE: usize = 0x30;

#[derive(Debug, Clone)]
/// One word of a module's dictionary: its header and its threaded body.
pub struct Entry {
    /// Name as stored. Truncated to 11 bytes, so `TELEFONKARTE` is kept as
    /// `TELEFONKART` even though `declared_len` still says 12.
    pub name: String,
    /// Name length before truncation.
    pub declared_len: usize,
    /// The field at +12. Behaves like a hash-chain link, not a flag set: it is
    /// 11 on the first word of a fresh module and 6 on the next ones, but takes
    /// over a hundred distinct values once a dictionary grows large.
    pub flags: u32,
    /// Offset of the header within the module.
    pub offset: usize,
    /// The word's threaded body.
    pub body: Vec<u32>,
}

impl Entry {
    /// Offset of the first body cell within the module.
    pub fn body_offset(&self) -> usize {
        self.offset.saturating_add(HEADER_LEN)
    }

    /// The address other words use to call this one.
    ///
    /// Threaded code addresses a word as `(module << 16) | offset`, where the
    /// offset counts **cells from file offset 0x30** — not bytes, and not from
    /// the start of the dictionary. Compiling four words and reading how they
    /// refer to each other settled it: bodies at 0x60, 0x7c and 0x98 came out
    /// as 0x0c, 0x13 and 0x1a, seven apart for words 28 bytes apart.
    pub fn call_offset(&self) -> u32 {
        crate::narrow(self.body_offset().saturating_sub(ADDRESS_BASE) / 4)
    }
}

#[derive(Debug, Clone)]
/// One parsed `.SCR`: a module number, an entry point, and its dictionary.
pub struct ScrModule {
    /// The module number, as the container and every call cell name it.
    pub module: u32,
    /// Entry point as stored: `(module << 16) | offset`.
    pub entry: u32,
    /// The dictionary, in file order.
    pub entries: Vec<Entry>,
    /// The field at 0x24: where module memory ends, in cells from 0x30.
    ///
    /// This is the dictionary pointer the compiler writes and `=>INIT`
    /// republishes. It is the only field that says how the item's body splits,
    /// because 0x1c is a capacity in the container copies.
    pub dp_cells: usize,
    /// Size of the second region, from the field at 0x20. Always 16004.
    pub second_area_len: usize,
    /// The field at 0x1c. Measures from 0x30. The runtime overwrites it with
    /// `dp_cells * 4`; in the copies embedded in `001.RSC` it is 0xFFF0, the
    /// authoring tool's capacity.
    pub declared_mem_len: usize,
    /// Bytes past the second region — container padding, and in module 10 data.
    pub tail_len: usize,
}

impl ScrModule {
    /// Reads a script module.
    pub fn parse(item: &[u8]) -> Result<Self> {
        let head = Record(bytes::<DICT_OFF>(item, 0)?);
        let magic = head.bytes::<0, { MAGIC.len() }>();
        if &magic != MAGIC {
            return Err(Error::Corrupt {
                what: "script module",
                detail: format!("script magic is {magic:02x?}, expected USERDEF"),
            });
        }

        let module = head.u32::<0x10>();
        let declared_mem_len = head.u32at::<0x1c>();
        let second_area_len = head.u32at::<0x20>();
        let dp_cells = head.u32at::<0x24>();
        let entry = head.u32::<0x30>();
        // Words run to the end of *module memory*, which is `DP * 4` bytes from
        // 0x30 — not to 0x50 + 16004. Stopping at the latter hid most of every
        // module: 65 words live in module 202's 45832 bytes, and the two the
        // park's macro calls — `GABY_BIG` at 0x5538 and `DEF_GABY` at 0x5944 —
        // both sit past it. The virtual machine never noticed, because it calls
        // by address; only the disassembler was blind, and with it every search
        // for where a word is used.
        //
        // The upper bound is not cosmetic: past DP lies the second region,
        // which is not code and must not be walked as if it were. Measured over
        // all 86 modules, no word begins at or past it.
        let mem_end = ADDRESS_BASE.saturating_add(dp_cells.saturating_mul(4));
        let dict_end = mem_end.min(item.len());

        let mut entries = Vec::new();
        let mut off = DICT_OFF;
        while let Some(body_start) = off.checked_add(HEADER_LEN).filter(|&b| b <= dict_end) {
            // The dictionary is not one unbroken run: modules typically hold a
            // first group of words, then a stretch of padding, then more. So a
            // position that is not a header means skip ahead, not stop —
            // stopping there would silently drop most of a module.
            let Some(header) = read_header(item, off, dict_end) else {
                off = off.saturating_add(4);
                continue;
            };
            let end = body_end(item, body_start, dict_end);
            let body = slice(item, body_start, end.saturating_sub(body_start))?
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| u32::from_le_bytes(*c))
                .collect();
            entries.push(Entry {
                name: header.0,
                declared_len: header.1,
                flags: header.2,
                offset: off,
                body,
            });
            off = end;
        }

        Ok(Self {
            module,
            entry,
            entries,
            dp_cells,
            second_area_len,
            declared_mem_len,
            tail_len: item
                .len()
                .saturating_sub(mem_end.saturating_add(second_area_len)),
        })
    }

    /// Bytes of module memory: everything an address can reach, from 0x30 on.
    pub fn mem_len(&self) -> usize {
        self.dp_cells.saturating_mul(4)
    }

    /// Module number and offset of the entry point.
    pub fn entry_point(&self) -> (u32, u32) {
        (self.entry >> 16, self.entry & 0xffff)
    }

    /// The word whose body covers `offset`, for resolving intra-module calls.
    pub fn entry_at(&self, offset: usize) -> Option<&Entry> {
        self.entries.iter().rev().find(|e| {
            e.offset <= offset
                && offset
                    .checked_sub(e.body_offset())
                    .is_none_or(|into_body| into_body / 4 < e.body.len())
        })
    }
}

/// Finds where a word's body ends, by walking it the way the interpreter would.
///
/// A purely structural scan does not work here. Threaded code embeds its
/// operands: the cell after `_PutLit` is a number, and `_PutString` is followed
/// by raw text — text that looks exactly like a counted name and would be
/// mistaken for the next word's header. Consuming operands as they come means
/// the header test only ever runs at a genuine cell boundary.
fn body_end(item: &[u8], start: usize, limit: usize) -> usize {
    let mut p = start;
    while let Some(next) = p.checked_add(4).filter(|&n| n <= limit) {
        if read_header(item, p, limit).is_some() {
            break;
        }
        // `limit` is at most the item's length, checked by the caller, so the
        // four bytes are there; a limit that lied ends the walk rather than
        // the process.
        let Some(&bytes) = item.get(p..).and_then(|rest| rest.first_chunk::<4>()) else {
            break;
        };
        let cell = u32::from_le_bytes(bytes);
        p = next;
        // A zero cell is the return that closes a colon definition, but it also
        // occurs mid-body as an early return, so it cannot end the walk on its
        // own. What does end it is a return with nothing but padding behind it:
        // that is the last word, and the rest of the dictionary area is zeros.
        if cell == EXIT_CELL && rest_is_padding(item, p, limit) {
            break;
        }
        if cell >> 16 != crate::m32::le::TAG_KERNEL >> 16 {
            continue;
        }
        let ordinal = cell & 0xffff;
        // The measured inline set, not a binding: a module is parsed before
        // any kernel is bound to it, and the words with an operand sit in the
        // first eighty-four entries of table 0, which both shipped builds of
        // the engine share entry for entry.
        if crate::m32::le::INLINE.takes_cell(ordinal) {
            p = p.saturating_add(4);
        } else if crate::m32::le::INLINE.takes_string(ordinal) {
            let text = crate::nul_terminated(item.get(p..limit).unwrap_or_default());
            // The terminator is included, and strings are padded out to the
            // next cell boundary.
            p = if text.len() == limit.saturating_sub(p) {
                limit
            } else {
                p.saturating_add(text.len())
                    .saturating_add(1)
                    .next_multiple_of(4)
            };
        }
    }
    p.min(limit)
}

/// Whether nothing but zero padding follows `p`.
///
/// One cell of lookahead is not enough to be sure, and scanning to the end of a
/// 16 KB area for every return would be wasteful, so this looks at a short run.
/// Real code never has this many zero cells in a row: a literal zero is an
/// operand, and consecutive returns do not occur.
fn rest_is_padding(item: &[u8], p: usize, limit: usize) -> bool {
    const LOOKAHEAD: usize = 4 * 4;
    item.get(p..limit)
        .unwrap_or_default()
        .iter()
        .take(LOOKAHEAD)
        .all(|&b| b == 0)
}

/// Reads a word header at `off`, or `None` if there is not one there.
///
/// Only the counted name is checked. The field at +12 looks like a hash-chain
/// link rather than a small set of flags — it takes over a hundred distinct
/// values across the shipped modules — so it is no use as a discriminator.
/// Since [`body_end`] only asks at real cell boundaries, the name test is
/// enough: a literal's bytes cannot pass it, because a plausible length byte
/// would have to be followed by printable characters and then NUL padding.
fn read_header(item: &[u8], off: usize, limit: usize) -> Option<(String, usize, u32)> {
    if off.checked_add(HEADER_LEN).is_none_or(|end| end > limit) {
        return None;
    }
    let head = Record(bytes::<HEADER_LEN>(item, off).ok()?);
    let declared_len = usize::from(head.u8::<0>());
    if declared_len == 0 || declared_len > 32 {
        return None;
    }
    let kept = declared_len.min(NAME_CAP);
    let name_field = head.bytes::<1, NAME_CAP>();
    // The kept part must be printable, and anything beyond it must be padding.
    if name_field.iter().take(kept).any(|&b| b < 0x20) {
        return None;
    }
    if name_field.iter().skip(kept).any(|&b| b != 0) {
        return None;
    }
    let name = name_field
        .iter()
        .take(kept)
        .map(|&b| cp437_char(b))
        .collect();
    Some((name, declared_len, head.u32::<12>()))
}
