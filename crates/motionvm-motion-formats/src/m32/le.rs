//! Loader for the Linear Executable (LE) inside `ENGINE.EXE`.
//!
//! `ENGINE.EXE` is a Watcom-built DOS/4GW program: a DOS stub followed by an LE
//! image. Standard tools on a Mac cannot read the format, and it matters here
//! for one reason — the Forth kernel's word table lives in the data segment as
//! `{char* name, void* handler}` pairs, and in the file those pointers are not
//! stored at all. They only come into existence when the loader applies the
//! fixup records. So the table is invisible until the image is relocated.
//!
//! [`Image`] does exactly what the DOS/4GW loader would: map every page to its
//! object's virtual address and apply the 32-bit fixups. What comes out is a
//! flat address space that can be read the way the running program sees it.

use crate::cursor::Cursor;
use crate::error::{Error, Result};
use crate::{Record, bytes, records, u32at, wide};

/// One object (segment) of the executable.
#[derive(Debug, Clone, Copy)]
pub struct Object {
    /// How much address space it occupies once loaded.
    pub virtual_size: u32,
    /// Its lowest virtual address.
    pub base: u32,
    /// Object flags; bit 1 is writable, bit 2 executable.
    pub flags: u32,
    /// 1-based index of this object's first page in the page map.
    pub first_page: u32,
    /// How many pages belong to it.
    pub page_count: u32,
}

impl Object {
    /// Whether the loader marks this object as code.
    pub fn executable(&self) -> bool {
        self.flags & 0x4 != 0
    }
    /// Whether the loader marks it as data that may be written.
    pub fn writable(&self) -> bool {
        self.flags & 0x2 != 0
    }
    /// Whether a virtual address falls inside it.
    pub fn contains(&self, addr: u32) -> bool {
        addr.checked_sub(self.base)
            .is_some_and(|into| into < self.virtual_size)
    }
}

/// The relocated image: a flat span of address space with fixups applied.
pub struct Image {
    /// Lowest object base; index 0 of `bytes`.
    low: u32,
    bytes: Vec<u8>,
    objects: Vec<Object>,
    fixups_applied: usize,
    pages: usize,
}

const LE_SIG: &[u8; 2] = b"LE";

impl std::fmt::Debug for Image {
    /// The span and the object table, not the megabyte of relocated bytes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field("low", &self.low)
            .field("len", &self.bytes.len())
            .field("objects", &self.objects)
            .field("fixups_applied", &self.fixups_applied)
            .field("pages", &self.pages)
            .finish()
    }
}

impl Image {
    /// Bytes of LE header the loader reads: the data pages offset at `+0x80`
    /// is the last field.
    const HEADER_BYTES: usize = 0x84;
    /// The most address space an image may span. `ENGINE.EXE` spans under two
    /// megabytes, and no DOS extender's image comes near this; a header that
    /// asks for more is damaged, and is refused before anything is allocated
    /// for it.
    const SPAN_CAP: usize = 64 << 20;

    /// Reads and relocates an LE executable from disk.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        Self::parse(&std::fs::read(path)?)
    }

    /// Relocates an LE executable already in memory.
    ///
    /// The result is one flat span of address space with the fixups applied,
    /// so a virtual address out of the disassembly indexes it directly.
    pub fn parse(file: &[u8]) -> Result<Self> {
        // The LE header sits where the DOS stub's e_lfanew points.
        let le = u32at(file, 0x3c)?;
        if bytes::<2>(file, le).ok() != Some(LE_SIG) {
            return Err(Error::Corrupt {
                what: "LE image",
                detail: format!("no LE signature at {le:#x}"),
            });
        }
        let head = Record(bytes::<{ Self::HEADER_BYTES }>(file, le)?);

        let page_count = head.u32at::<0x14>();
        let page_size = head.u32at::<0x28>();
        let object_table = le.saturating_add(head.u32at::<0x40>());
        let object_count = head.u32at::<0x44>();
        let fixup_page_table = le.saturating_add(head.u32at::<0x68>());
        let fixup_record_table = le.saturating_add(head.u32at::<0x6c>());
        let data_pages = head.u32at::<0x80>();

        if page_size == 0 || page_count == 0 || object_count == 0 {
            return Err(Error::Corrupt {
                what: "LE image",
                detail: "degenerate header".into(),
            });
        }

        // Twenty-four bytes per object table entry.
        let objects: Vec<Object> = records::<24>(file, object_table, object_count)?
            .iter()
            .map(|entry| {
                let entry = Record(entry);
                Object {
                    virtual_size: entry.u32::<0>(),
                    base: entry.u32::<4>(),
                    flags: entry.u32::<8>(),
                    first_page: entry.u32::<12>(),
                    page_count: entry.u32::<16>(),
                }
            })
            .collect();

        // An image with no objects has no bytes to lay out; the header may say
        // so, and a file that does is refused rather than unwrapped. So is one
        // whose objects reach past the end of the address space, or span more
        // of it than any DOS extender's image does.
        let ends = objects
            .iter()
            .map(|o| o.base.checked_add(o.virtual_size))
            .collect::<Option<Vec<u32>>>()
            .ok_or_else(|| Error::Corrupt {
                what: "LE image",
                detail: "an object reaches past the end of the address space".into(),
            })?;
        let (Some(low), Some(high)) =
            (objects.iter().map(|o| o.base).min(), ends.into_iter().max())
        else {
            return Err(Error::Corrupt {
                what: "LE image",
                detail: "the object table is empty".into(),
            });
        };
        let span = wide(high.saturating_sub(low));
        if span > Self::SPAN_CAP {
            return Err(Error::Corrupt {
                what: "LE image",
                detail: format!("the objects span {span} bytes of address space"),
            });
        }
        let mut bytes = vec![0u8; span];

        // Page N of the file belongs to whichever object claims it, and lands
        // at that object's base plus the page's position in it.
        let page_base = |page_1based: usize| -> Option<u32> {
            let page = crate::narrow(page_1based);
            objects.iter().find_map(|o| {
                page.checked_sub(o.first_page)
                    .filter(|&into| into < o.page_count)
                    .and_then(|into| into.checked_mul(crate::narrow(page_size)))
                    .and_then(|offset| o.base.checked_add(offset))
            })
        };
        // Keep the buffer length fixed: the last page is short, and a plain
        // slice assignment would resize the vector.
        let pages = (1usize..)
            .zip((data_pages..).step_by(page_size))
            .take(page_count);
        for (page, src) in pages {
            let Some(base) = page_base(page) else {
                continue;
            };
            let chunk = file
                .get(src..)
                .map(|rest| rest.get(..page_size).unwrap_or(rest))
                .unwrap_or_default();
            let dst = wide(base.saturating_sub(low));
            for (into, &byte) in bytes.iter_mut().skip(dst).zip(chunk) {
                *into = byte;
            }
        }

        // One bound per page and one past the last: where each page's fixup
        // records start and where they stop.
        let bounds: Vec<usize> =
            records::<4>(file, fixup_page_table, page_count.saturating_add(1))?
                .iter()
                .map(|entry| wide(u32::from_le_bytes(*entry)))
                .collect();
        let mut fixups_applied = 0usize;
        for (page, (&start, &end)) in (1usize..).zip(bounds.iter().zip(bounds.iter().skip(1))) {
            let Some(page_base_addr) = page_base(page) else {
                continue;
            };
            let mut c = Cursor::new(file, fixup_record_table.saturating_add(start));
            let stop = fixup_record_table.saturating_add(end);
            while c.position() < stop {
                let src_type = c.u8()?;
                let flags = c.u8()?;

                // A source list packs several patch sites under one target.
                let sources: Vec<i16> = if src_type & 0x20 != 0 {
                    let n = usize::from(c.u8()?);
                    c.records::<2>(n)?
                        .iter()
                        .map(|s| i16::from_le_bytes(*s))
                        .collect()
                } else {
                    vec![c.i16()?]
                };

                // Only internal references occur in this file; the parse would
                // desynchronize on anything else, and the page-boundary check
                // below would catch it.
                let object = if flags & 0x40 != 0 {
                    usize::from(c.u16()?)
                } else {
                    usize::from(c.u8()?)
                };
                let target_off = match src_type & 0x0f {
                    2 => 0,
                    _ if flags & 0x10 != 0 => c.u32()?,
                    _ => u32::from(c.u16()?),
                };

                // 32-bit offsets are the only kind that matter for pointers.
                if src_type & 0x0f != 7 {
                    continue;
                }
                let Some(obj) = objects.get(object.wrapping_sub(1)) else {
                    continue;
                };
                let target = obj.base.wrapping_add(target_off);
                for s in sources {
                    // Negative offsets reach back into the previous page; a
                    // site outside the image, or one whose four bytes are not
                    // all inside it, is skipped.
                    let site = page_base_addr
                        .checked_add_signed(i32::from(s))
                        .and_then(|addr| addr.checked_sub(low))
                        .and_then(|at| bytes.get_mut(wide(at)..))
                        .and_then(|rest| rest.first_chunk_mut::<4>());
                    let Some(site) = site else {
                        continue;
                    };
                    *site = target.to_le_bytes();
                    fixups_applied = fixups_applied.saturating_add(1);
                }
            }
            if c.position() != stop {
                return Err(Error::Corrupt {
                    what: "LE image",
                    detail: format!(
                        "fixup records for page {page} ended at {:#x}, expected {stop:#x}",
                        c.position()
                    ),
                });
            }
        }

        Ok(Self {
            low,
            bytes,
            objects,
            fixups_applied,
            pages: page_count,
        })
    }

    /// The objects, in file order.
    pub fn objects(&self) -> &[Object] {
        &self.objects
    }
    /// How many relocations were applied, for reporting.
    pub fn fixups_applied(&self) -> usize {
        self.fixups_applied
    }
    /// How many pages the image was assembled from.
    pub fn pages(&self) -> usize {
        self.pages
    }

    /// Everything from a virtual address on, or `None` if it is outside the
    /// image.
    fn from(&self, addr: u32) -> Option<&[u8]> {
        self.bytes.get(wide(addr.checked_sub(self.low)?)..)
    }

    /// Bytes at a virtual address, or `None` if they are not all inside the
    /// image.
    pub fn slice(&self, addr: u32, len: usize) -> Option<&[u8]> {
        self.from(addr)?.get(..len)
    }

    /// The 32-bit word at a virtual address, already relocated.
    pub fn u32_at(&self, addr: u32) -> Option<u32> {
        let word = self.from(addr)?.first_chunk::<4>()?;
        Some(u32::from_le_bytes(*word))
    }

    /// A NUL-terminated string at a virtual address, if it looks like one.
    ///
    /// `max` bounds the search; anything longer, empty, or containing a
    /// non-printable byte is rejected, which is what keeps the table scan below
    /// from latching onto arbitrary data.
    pub fn cstr_at(&self, addr: u32, max: usize) -> Option<&str> {
        let rest = self.from(addr)?;
        let bytes = rest.get(..max).unwrap_or(rest);
        let end = bytes.iter().position(|&b| b == 0)?;
        if end == 0 {
            return None;
        }
        let s = bytes.get(..end)?;
        // Printable ASCII is UTF-8 by construction, so the second check can
        // only agree with the first; it is a check and not an unwrap all the
        // same, because `?`-shaped code costs nothing here.
        s.iter()
            .all(|&b| (0x20..0x7f).contains(&b))
            .then(|| std::str::from_utf8(s).ok())
            .flatten()
    }

    fn writable_object(&self) -> Option<&Object> {
        self.objects
            .iter()
            .find(|o| o.writable() && o.virtual_size > 0x1000)
    }

    fn code_object(&self) -> Option<&Object> {
        self.objects.iter().find(|o| o.executable())
    }
}

pub use crate::kernel::KernelWord;
use crate::kernel::{Binding, Inline};

/// Finds the kernel's word tables in the relocated image.
///
/// The tables are plain C arrays of `{const char *name; void (*fn)();}`,
/// terminated by a `{"None", NULL}` sentinel, and they are not aligned — the
/// first one starts three bytes off a four-byte boundary — so the scan walks
/// byte by byte rather than by word.
pub fn kernel_words(img: &Image) -> Vec<KernelWord> {
    const ENTRY: u32 = 8;
    const MIN_RUN: usize = 6;

    let (Some(data), Some(code)) = (img.writable_object(), img.code_object()) else {
        return Vec::new();
    };
    let code_lo = code.base;
    let code_hi = code.base.saturating_add(code.virtual_size);

    // A plausible entry, as its name and its handler — the handler comes back
    // with the name so that a caller has both without reading it twice.
    let plausible = |addr: u32| -> Option<(String, u32)> {
        let name = img.cstr_at(img.u32_at(addr)?, 24)?;
        let handler = img.u32_at(addr.checked_add(4)?)?;
        (code_lo..code_hi)
            .contains(&handler)
            .then(|| (name.to_string(), handler))
    };

    let mut words = Vec::new();
    let mut table = 0usize;
    let mut addr = data.base;
    let end = data.base.saturating_add(data.virtual_size);
    // Whether a whole entry from `addr` lies inside the data object.
    let fits = |addr: u32| addr.checked_add(ENTRY).is_some_and(|past| past <= end);
    while fits(addr) {
        if plausible(addr).is_none() {
            addr = addr.saturating_add(1);
            continue;
        }
        // Walk the whole run before deciding whether it is a real table.
        let start = addr;
        let mut run = Vec::new();
        while fits(addr) {
            let Some((name, handler)) = plausible(addr) else {
                break;
            };
            run.push((addr, name, handler));
            addr = addr.saturating_add(ENTRY);
        }
        if run.len() >= MIN_RUN {
            for (index, (entry, name, handler)) in run.into_iter().enumerate() {
                words.push(KernelWord {
                    name,
                    handler,
                    entry,
                    table,
                    index,
                });
            }
            table = table.saturating_add(1);
        } else {
            // Not a table after all; resume just past where it started.
            addr = start.saturating_add(1);
        }
    }
    words
}

/// How the bytecode refers to a kernel word.
///
/// A cell in a compiled thread is either `(module << 16) | offset` pointing at a
/// word in another module, or a kernel reference tagged with `0x4000` in the top
/// half. The low half is not an index into the tables below but an offset into
/// the base module's dictionary, where every entry occupies five bytes:
///
/// ```text
/// ordinal = 5 * index + base(table)
/// ```
///
/// The bases were measured, not derived: compiling `: T DUP ;` and friends with
/// the original compiler and reading the cell it emitted. `DUP` (table 0 index
/// 7) came out as 139, `DROP` (index 10) as 154, `_PutLit` (14) as 174,
/// `_PutAdr` (15) as 179, `_PutConst` (16) as 184 — all of them `5 * index +
/// 104`. `TOGFX` (table 2 index 0) came out as 1039 and `NEWSCREEN` (index 21)
/// as 1144, again five apart per index.
pub const TAG_KERNEL: u32 = 0x4000_0000;

/// Ordinal of the first entry of each table, indexed by table number.
///
/// Table 1 holds the compiling words (`:`, `IF`, `DO`, …). Those run at compile
/// time and never appear as a cell in a thread, so their base was never needed
/// and is left unknown.
const TABLE_BASE: [Option<u32>; 3] = [Some(104), None, Some(1039)];
const ORDINAL_STRIDE: u32 = 5;

/// The bytecode ordinal for a kernel word, if its table's base is known.
pub fn ordinal_of(word: &KernelWord) -> Option<u32> {
    let base = (*TABLE_BASE.get(word.table)?)?;
    base.checked_add(ORDINAL_STRIDE.checked_mul(crate::narrow(word.index))?)
}

/// Resolves a bytecode ordinal back to a word.
pub fn word_by_ordinal(words: &[KernelWord], ordinal: u32) -> Option<&KernelWord> {
    words.iter().find(|w| ordinal_of(w) == Some(ordinal))
}

/// The 32-bit kernel's inline set: the measured constants of [`inline`],
/// gathered into the shape a machine or a disassembler takes.
pub const INLINE: Inline = Inline {
    put_lit: inline::PUT_LIT,
    put_adr: inline::PUT_ADR,
    put_const: inline::PUT_CONST,
    put_string: inline::PUT_STRING,
    put_string_adr: Some(inline::PUT_STRING_ADR),
    check_if: inline::CHECK_IF,
    check_eif: inline::CHECK_EIF,
    // `ELSEDUP` compiles to `_ChElseDup`; its ordinal was never measured, and
    // no shipped 32-bit module uses it.
    ch_else_dup: None,
    check_else: inline::CHECK_ELSE,
    until: inline::UNTIL,
    repeat: inline::REPEAT,
    loop_break: inline::LOOP_BREAK,
    loop_end: inline::LOOP_END,
    add_loop: inline::ADD_LOOP,
    u_loop_end: inline::U_LOOP_END,
};

/// The binding of a 32-bit kernel: every word whose table has a known base,
/// at `5 * index + base`, and the measured inline set.
pub fn binding_of(words: &[KernelWord]) -> Binding {
    let mut bound: Vec<(u32, String)> = words
        .iter()
        .filter_map(|w| ordinal_of(w).map(|o| (o, w.name.clone())))
        .collect();
    bound.sort_by_key(|&(o, _)| o);
    Binding {
        words: bound,
        inline: INLINE,
    }
}

/// Ordinals of the kernel words whose operand follows them inline.
///
/// Measured by compiling the constructs and reading what came out:
/// `: T 5 ;` emitted `_PutLit` then the cell 5, `VAR X` emitted `_PutAdr` then a
/// zeroed cell, `5 CONST Y` emitted `_PutConst` then 5, and `: T ." Hallo" ;`
/// emitted `_PutString` followed by the raw bytes `Hallo\0` padded out to a cell
/// boundary. Walking a body without honoring these would read the operand as
/// code — and for the string case would mistake the text for a word header.
pub mod inline {
    /// Followed by one cell of data.
    pub const PUT_LIT: u32 = 174;
    /// A variable: pushes the address of the cell that follows, then returns.
    pub const PUT_ADR: u32 = 179;
    /// A constant: pushes the following cell's value, then returns.
    pub const PUT_CONST: u32 = 184;
    /// Followed by a NUL-terminated string, padded to a 4-byte boundary.
    pub const PUT_STRING: u32 = 499;
    /// The same, but pushes the string's address and carries on.
    pub const PUT_STRING_ADR: u32 = 504;

    /// Branch words, each followed by one cell holding its jump distance.
    /// Every one of these was produced on purpose and read back:
    ///
    /// ```text
    /// : T 1 IF 2 ELSE 3 ENDIF ;      -> _CheckIf 5 ... _CheckElse 3
    /// : T 1 =IF 2 ENDIF ;            -> _CheckEIf 3
    /// : T BEGIN 1 UNTIL ;            -> _Until 3
    /// : T BEGIN 1 WHILE 2 REPEAT ;   -> _LoopBreak 5 ... _Repeat 7
    /// : T 0 10 DO 1 LOOP ;           -> _LoopStart (no operand) ... _LoopEnd 3
    /// : T 0 10 DO 1 2 +LOOP ;        -> _AddLoop 5
    /// : T 0 10 DO 1 2 /LOOP ;        -> _ULoopEnd 5
    /// ```
    ///
    /// `_LoopStart` is the one that takes nothing, which is why guessing by
    /// family would have gone wrong.
    pub const CHECK_IF: u32 = 314;
    /// `=IF` — compare-and-branch.
    pub const CHECK_EIF: u32 = 319;
    /// `ELSE`.
    pub const CHECK_ELSE: u32 = 329;
    /// `+LOOP`.
    pub const ADD_LOOP: u32 = 359;
    /// `/LOOP`.
    pub const U_LOOP_END: u32 = 364;
    /// `UNTIL`.
    pub const UNTIL: u32 = 374;
    /// `WHILE` — which leaves the loop when the flag is **true**.
    pub const LOOP_BREAK: u32 = 379;
    /// `REPEAT`.
    pub const REPEAT: u32 = 384;
    /// `LOOP`.
    pub const LOOP_END: u32 = 394;

    /// Whether the word is followed by one cell of operand.
    pub fn takes_cell(ordinal: u32) -> bool {
        matches!(
            ordinal,
            PUT_LIT
                | PUT_ADR
                | PUT_CONST
                | CHECK_IF
                | CHECK_EIF
                | CHECK_ELSE
                | ADD_LOOP
                | U_LOOP_END
                | UNTIL
                | LOOP_BREAK
                | REPEAT
                | LOOP_END
        )
    }
    /// Whether the word is followed by a NUL-terminated string.
    pub fn takes_string(ordinal: u32) -> bool {
        matches!(ordinal, PUT_STRING | PUT_STRING_ADR)
    }
}
