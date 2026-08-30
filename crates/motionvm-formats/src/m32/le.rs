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

use crate::error::{Error, Result};
use crate::{reserve, u8at, u16le, u32le};

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
        addr >= self.base && (addr - self.base) < self.virtual_size
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

impl Image {
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
        let le = u32le(file, 0x3c)? as usize;
        if file.get(le..le + 2) != Some(LE_SIG) {
            return Err(Error::Corrupt {
                what: "LE image",
                detail: format!("no LE signature at {le:#x}"),
            });
        }
        let h = |off: usize| u32le(file, le + off);

        let page_count = h(0x14)? as usize;
        let page_size = h(0x28)? as usize;
        let object_table = le + h(0x40)? as usize;
        let object_count = h(0x44)? as usize;
        let fixup_page_table = le + h(0x68)? as usize;
        let fixup_record_table = le + h(0x6c)? as usize;
        let data_pages = h(0x80)? as usize;

        if page_size == 0 || page_count == 0 || object_count == 0 {
            return Err(Error::Corrupt {
                what: "LE image",
                detail: "degenerate header".into(),
            });
        }

        // Twenty-four bytes per object table entry.
        let mut objects = reserve(object_count, file.len(), 24);
        for i in 0..object_count {
            let o = object_table + i * 24;
            objects.push(Object {
                virtual_size: u32le(file, o)?,
                base: u32le(file, o + 4)?,
                flags: u32le(file, o + 8)?,
                first_page: u32le(file, o + 12)?,
                page_count: u32le(file, o + 16)?,
            });
        }

        let low = objects
            .iter()
            .map(|o| o.base)
            .min()
            .expect("object_count > 0");
        let high = objects
            .iter()
            .map(|o| o.base + o.virtual_size)
            .max()
            .expect("object_count > 0");
        let mut bytes = vec![0u8; (high - low) as usize];

        // Page N of the file belongs to whichever object claims it. Keep the
        // buffer length fixed: the last page is short, and a plain slice
        // assignment would resize the vector.
        let page_base = |page_1based: usize| -> Option<u32> {
            objects.iter().find_map(|o| {
                let idx = page_1based as u32;
                (idx >= o.first_page && idx < o.first_page + o.page_count)
                    .then(|| o.base + (idx - o.first_page) * page_size as u32)
            })
        };
        for p in 0..page_count {
            let Some(base) = page_base(p + 1) else {
                continue;
            };
            let src = data_pages + p * page_size;
            let chunk = file
                .get(src..(src + page_size).min(file.len()))
                .unwrap_or(&[]);
            let dst = (base - low) as usize;
            let n = chunk.len().min(bytes.len().saturating_sub(dst));
            bytes[dst..dst + n].copy_from_slice(&chunk[..n]);
        }

        let mut fixups_applied = 0;
        for p in 0..page_count {
            let start = u32le(file, fixup_page_table + p * 4)? as usize;
            let end = u32le(file, fixup_page_table + (p + 1) * 4)? as usize;
            let Some(page_base_addr) = page_base(p + 1) else {
                continue;
            };
            let mut cur = fixup_record_table + start;
            let stop = fixup_record_table + end;
            while cur < stop {
                let src_type = u8at(file, cur)?;
                let flags = u8at(file, cur + 1)?;
                cur += 2;

                // A source list packs several patch sites under one target.
                let mut sources = Vec::new();
                if src_type & 0x20 != 0 {
                    let n = u8at(file, cur)? as usize;
                    cur += 1;
                    for i in 0..n {
                        sources.push(u16le(file, cur + 2 * i)? as i16);
                    }
                    cur += 2 * n;
                } else {
                    sources.push(u16le(file, cur)? as i16);
                    cur += 2;
                }

                // Only internal references occur in this file; the parse would
                // desynchronize on anything else, and the page-boundary check
                // below would catch it.
                let object = if flags & 0x40 != 0 {
                    let v = u16le(file, cur)? as usize;
                    cur += 2;
                    v
                } else {
                    let v = u8at(file, cur)? as usize;
                    cur += 1;
                    v
                };
                let target_off = match src_type & 0x0f {
                    2 => 0,
                    _ if flags & 0x10 != 0 => {
                        let v = u32le(file, cur)?;
                        cur += 4;
                        v
                    }
                    _ => {
                        let v = u16le(file, cur)? as u32;
                        cur += 2;
                        v
                    }
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
                    // Negative offsets reach back into the previous page.
                    let addr = page_base_addr as i64 + s as i64;
                    if addr < low as i64 || addr + 4 > high as i64 {
                        continue;
                    }
                    let at = (addr - low as i64) as usize;
                    bytes[at..at + 4].copy_from_slice(&target.to_le_bytes());
                    fixups_applied += 1;
                }
            }
            if cur != stop {
                return Err(Error::Corrupt {
                    what: "LE image",
                    detail: format!(
                        "fixup records for page {p} ended at {cur:#x}, expected {stop:#x}"
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

    /// Bytes at a virtual address, or `None` if it is outside the image.
    pub fn slice(&self, addr: u32, len: usize) -> Option<&[u8]> {
        let off = addr.checked_sub(self.low)? as usize;
        self.bytes.get(off..off + len)
    }

    /// The 32-bit word at a virtual address, already relocated.
    pub fn u32_at(&self, addr: u32) -> Option<u32> {
        let b = self.slice(addr, 4)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// A NUL-terminated string at a virtual address, if it looks like one.
    ///
    /// `max` bounds the search; anything longer, empty, or containing a
    /// non-printable byte is rejected, which is what keeps the table scan below
    /// from latching onto arbitrary data.
    pub fn cstr_at(&self, addr: u32, max: usize) -> Option<&str> {
        let bytes = self.slice(addr, max.min(self.remaining(addr)?))?;
        let end = bytes.iter().position(|&b| b == 0)?;
        if end == 0 {
            return None;
        }
        let s = &bytes[..end];
        s.iter()
            .all(|&b| (0x20..0x7f).contains(&b))
            .then(|| std::str::from_utf8(s).expect("ASCII range checked above"))
    }

    fn remaining(&self, addr: u32) -> Option<usize> {
        let off = addr.checked_sub(self.low)? as usize;
        self.bytes.len().checked_sub(off)
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
    let code_hi = code.base + code.virtual_size;

    let plausible = |addr: u32| -> Option<String> {
        let name = img.cstr_at(img.u32_at(addr)?, 24)?;
        let handler = img.u32_at(addr + 4)?;
        (code_lo..code_hi)
            .contains(&handler)
            .then(|| name.to_string())
    };

    let mut words = Vec::new();
    let mut table = 0;
    let mut addr = data.base;
    let end = data.base + data.virtual_size;
    while addr + ENTRY <= end {
        if plausible(addr).is_none() {
            addr += 1;
            continue;
        }
        // Walk the whole run before deciding whether it is a real table.
        let start = addr;
        let mut run = Vec::new();
        while addr + ENTRY <= end {
            let Some(name) = plausible(addr) else { break };
            run.push((
                addr,
                name,
                img.u32_at(addr + 4).expect("checked by plausible"),
            ));
            addr += ENTRY;
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
            table += 1;
        } else {
            // Not a table after all; resume just past where it started.
            addr = start + 1;
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
    let base = *TABLE_BASE.get(word.table)?;
    Some(base? + ORDINAL_STRIDE * word.index as u32)
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
