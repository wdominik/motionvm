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

/// The group number [`kernel_words`] gives the shell's words — the ones the
/// engine registers one at a time rather than out of a table, see
/// `Layout`. The three tables are groups 0, 1 and 2, in address order.
pub const SHELL_GROUP: usize = 3;

/// Every word the kernel registers, in the relocated image: the three tables,
/// and the shell's words after them.
///
/// The tables are plain C arrays of `{const char *name; void (*fn)();}`,
/// terminated by a `{"None", NULL}` sentinel, and they are not aligned — the
/// first one starts three bytes off a four-byte boundary — so the scan walks
/// byte by byte rather than by word. The shell's words are not in a table at
/// all: the shell's init registers each with its own call, and they are read
/// out of that code (`Layout::read`) and appended as group
/// [`SHELL_GROUP`], in the order they are registered, with the call site as
/// the entry address. A build whose init the reader cannot follow yields its
/// tables and no shell words; [`binding_of`] then refuses it by name, because
/// without the shell's count the domain table's ordinals cannot be placed.
pub fn kernel_words(img: &Image) -> Vec<KernelWord> {
    let mut words = tables(img);
    if let Ok(layout) = Layout::read(img, &words) {
        for (index, (name, handler, site)) in layout.shell.into_iter().enumerate() {
            words.push(KernelWord {
                name,
                handler,
                entry: site,
                table: SHELL_GROUP,
                index,
            });
        }
    }
    words
}

/// The kernel's three word tables, scanned out of the data object.
fn tables(img: &Image) -> Vec<KernelWord> {
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
/// half. The low half is not an index into the tables but an offset into the
/// base module's dictionary, which the engine fills at start-up by registering
/// every kernel word in turn, five bytes apiece — see `Layout` for the
/// order, and [`binding_of`] for the ordinal each word ends up with.
///
/// The anchors were measured, not derived: compiling `: T DUP ;` and friends
/// with the original compiler and reading the cell it emitted. `DUP` (table 0
/// index 7) came out as 139, `DROP` (index 10) as 154, `_PutLit` (14) as 174,
/// `_PutAdr` (15) as 179, `_PutConst` (16) as 184 — all of them `5 * index +
/// 104`. `TOGFX` (table 2 index 0) came out as 1039 and `NEWSCREEN` (index 21)
/// as 1144, again five apart per index. The reading of the init below
/// reproduces every one of them, on both builds.
pub const TAG_KERNEL: u32 = 0x4000_0000;

/// Bytes one word takes in the base module's dictionary: the registration
/// routine advances the dictionary pointer by four and then by one
/// (`ENGINE.EXE` V0.06.06/R109 `0x5f8a3` and `0x5f8d4`; V0.04.15/R78
/// `0x4ecbb` and the `incl` after it).
const ENTRY_BYTES: u32 = 5;

/// What a word's ordinal is past the dictionary pointer it was registered
/// at: the routine hands the dictionary insert the pointer plus four
/// (`0x5f85f` `add $4,%ebx` in R109, `0x4ec75` in R78).
const ORDINAL_SKEW: u32 = 4;

/// How the kernel init lays the base module's dictionary out, read out of the
/// image rather than assumed — which is what lets a second build bind without
/// a constant of its own.
///
/// Both builds fill module 0 in the same order, from the same three places in
/// the binary, and every number below is read from them:
///
/// 1. **The kernel init** sets the dictionary pointer (`movl $100,0x24(%eax)`
///    — R109 `0x5f5bd`, R78 `0x4ea0b`), registers table 0 in a loop (R109
///    `0x5f607`–`0x5f653`, R78 `0x4ea55`–`0x4eaad`), registers `_FNAME` on its
///    own (R109 `0x5f655`–`0x5f677`, R78 `0x4eaaf`–`0x4ead1`), skips twenty
///    bytes (`addl $20,0x24(%eax)` — R109 `0x5f681`, R78 `0x4eadb`) and
///    registers table 1 in a second loop, flagged `0x8000` as compiling words
///    (R109 `0x5f68c`–`0x5f6c3`, R78 `0x4eae6`–`0x4eb29`).
/// 2. **The shell init** registers its words one call at a time — `TEST`,
///    `RH`, `LOAD`, `->LOAD`, `DIR`, … `->RSCPATH`, `RSCRESCAN`,
///    `STARTSCRIPT` … — through a one-line wrapper around the same
///    registration routine (R109 `0x36f42`–`0x3726c`, 54 words, wrapper
///    `0x5f768`; R78 `0x2faab`–`0x2fd30`, 43 words, wrapper `0x4eb70`).
///    `LOAD` alone goes to the routine directly, flagged `0x8000`.
/// 3. **The graphics init** registers table 2 through the same wrapper (R109
///    `0x684ba`–`0x684ec`, R78 `0x56d60`–`0x56d9e`).
///
/// So the domain table binds where the count of everything before it puts
/// it: at `104 + 5·(|table 0| + 1 + 4 + |table 1| + |shell|)`, which is 1039
/// for R109 and 934 for R78. That the three inits run in this order is not
/// in the code that is read here; it is what the measured anchors on
/// [`TAG_KERNEL`] and the decoding of every shipped module of both games
/// confirm.
struct Layout {
    /// Where the dictionary pointer starts: 100 in both builds.
    first: u32,
    /// The words the kernel init registers itself between table 0 and table
    /// 1, as `(name, handler)`: `_FNAME` in both builds.
    between: Vec<(String, u32)>,
    /// The bytes it skips after them: 20 in both builds.
    gap: u32,
    /// The shell's words in registration order, as `(name, handler, call
    /// site)`.
    shell: Vec<(String, u32, u32)>,
}

/// Whether this build's `SDINSERT` takes a kind beside the value and the slot.
///
/// The text record's five insert slots (`+0x20`) are what `SDINSERT` writes,
/// and the two builds disagree on what it pops. R78 (`0x611c0`) pops two —
/// the slot and the value — and stores the value; R109 (`0x75cfe`) pops three
/// — the slot, a kind and the value — and files the kind in a second array at
/// `+0x34`, which its text layout reads back to decide what each slot is
/// (see the engine's text layout). Read off the handler: it opens with one
/// `mov $"SDINSERT",%eax; call pop` per argument, each naming the word for the
/// underflow message, and nothing else in it loads that name. Two pops answer
/// `false`, three `true`; any other count is a build this reading does not
/// cover, refused rather than guessed at.
pub fn sdinsert_takes_kind(img: &Image, words: &[KernelWord]) -> Result<bool> {
    const WORD: &str = "SDINSERT";
    let refuse = |detail: String| Error::Corrupt {
        what: "SDINSERT",
        detail,
    };
    let word = words
        .iter()
        .find(|w| w.name == WORD)
        .ok_or_else(|| refuse("no such kernel word".into()))?;
    // The prologue — six pushes, `mov %esp,%ebp`, `sub $n,%esp` — and the
    // pops fit in the first 0x40 bytes of both handlers, and the first
    // instruction after the pops loads a global, not the name.
    const REACH: usize = 0x40;
    let bytes = img
        .slice(word.handler, REACH)
        .ok_or_else(|| refuse(format!("handler {:#x} runs off the image", word.handler)))?;
    let pops = bytes
        .windows(6)
        .filter(|w| {
            let &[0xb8, a, b, c, d, 0xe8] = *w else {
                return false;
            };
            img.cstr_at(u32::from_le_bytes([a, b, c, d]), 24) == Some(WORD)
        })
        .count();
    match pops {
        2 => Ok(false),
        3 => Ok(true),
        n => Err(refuse(format!(
            "the handler at {:#x} opens with {n} pops; the two builds read take two or three",
            word.handler
        ))),
    }
}

/// Whether this build's curtains wait between bands.
///
/// `FADEIN` and `FADEOUT` close and open the picture in bands of eight rows
/// from both edges, marking each band for the presenter and presenting.
/// R109 (`0x74a42`, `0x74c79`) works out a delay first — the duration
/// argument divided by the band count, `idivl -0x14(%ebp)` at `0x74ac5`
/// and `0x74cf6` — and spins on the timer for that many ticks after every
/// band. R78 (`0x60360`, `0x60560`) has no division and no timer in either
/// handler: its loops mark and present and nothing else, so a curtain takes
/// the presenter's time and no more, and the duration argument goes
/// unread. Read off the handlers: the division is the one `idivl` with an
/// `%ebp`-relative operand in the mode-1 branch of each, `F7 7D disp8`;
/// both handlers waiting answers `true`, neither `false`, and one without
/// the other is a build this reading does not cover, refused rather than
/// guessed at.
pub fn fades_wait(img: &Image, words: &[KernelWord]) -> Result<bool> {
    let refuse = |what: &'static str, detail: String| Error::Corrupt { what, detail };
    let mut answers = Vec::new();
    for name in ["FADEIN", "FADEOUT"] {
        let word = words
            .iter()
            .find(|w| w.name == name)
            .ok_or_else(|| refuse(name, "no such kernel word".into()))?;
        // R109's division sits within the first 0x100 bytes of each
        // handler, past the three pops and the screen lookup; R78's
        // handlers are 0x200 bytes and hold none anywhere.
        const REACH: usize = 0x100;
        let bytes = img.slice(word.handler, REACH).ok_or_else(|| {
            refuse(
                name,
                format!("handler {:#x} runs off the image", word.handler),
            )
        })?;
        answers.push(bytes.windows(2).any(|w| w == [0xf7, 0x7d]));
    }
    match answers[..] {
        [true, true] => Ok(true),
        [false, false] => Ok(false),
        _ => Err(refuse(
            "FADEIN/FADEOUT",
            "one fade handler divides for its delay and the other does not; the two builds \
             read do both or neither"
                .into(),
        )),
    }
}

/// The one `E8` near call's target, or `None` for another opcode or a target
/// outside the code object.
fn call_target(img: &Image, at: u32) -> Option<u32> {
    let code = img.code_object()?;
    let bytes = img.slice(at, 5)?;
    if bytes.first() != Some(&0xe8) {
        return None;
    }
    let rel = i32::from_le_bytes(*bytes.get(1..)?.first_chunk::<4>()?);
    let target = at.checked_add(5)?.checked_add_signed(rel)?;
    code.contains(target).then_some(target)
}

/// Every address in the code object where a near call to `target` starts.
fn calls_to(img: &Image, target: u32) -> Vec<u32> {
    let Some(code) = img.code_object() else {
        return Vec::new();
    };
    (code.base..code.base.saturating_add(code.virtual_size))
        .filter(|&at| call_target(img, at) == Some(target))
        .collect()
}

/// Every address in the code object holding `value` as a 32-bit word.
fn code_sites(img: &Image, value: u32) -> Vec<u32> {
    let Some(code) = img.code_object() else {
        return Vec::new();
    };
    (code.base..code.base.saturating_add(code.virtual_size))
        .filter(|&at| img.u32_at(at) == Some(value))
        .collect()
}

/// A registration by two immediates and a call — `mov $handler,%edx; mov
/// $name,%eax; call` — read back from the call site: the name the string
/// points at and the handler, or `None` if the ten bytes before the call are
/// not that shape.
fn registered_at(img: &Image, call: u32) -> Option<(String, u32)> {
    let code = img.code_object()?;
    let bytes = img.slice(call.checked_sub(10)?, 10)?;
    let &[0xba, h0, h1, h2, h3, 0xb8, n0, n1, n2, n3] = bytes else {
        return None;
    };
    let handler = u32::from_le_bytes([h0, h1, h2, h3]);
    let name = img.cstr_at(u32::from_le_bytes([n0, n1, n2, n3]), 24)?;
    code.contains(handler).then(|| (name.to_string(), handler))
}

/// A registration through two locals — `movl $name,-0x30(%ebp); movl
/// $handler,-0x2c(%ebp); … call` — read back from the call site, as the kernel
/// init registers `_FNAME`: the last two `movl $imm32,disp8(%ebp)` before
/// the call, the first a string and the second in the code object.
fn registered_through_locals(img: &Image, call: u32) -> Option<(String, u32)> {
    let code = img.code_object()?;
    let start = call.checked_sub(0x40)?;
    let bytes = img.slice(start, 0x40)?;
    let mut stores = Vec::new();
    for (i, w) in bytes.windows(7).enumerate() {
        if let &[0xc7, 0x45, _, a, b, c, d] = w {
            stores.push((
                start.checked_add(crate::narrow(i))?,
                u32::from_le_bytes([a, b, c, d]),
            ));
        }
    }
    let (&(_, name), &(_, handler)) = (stores.iter().nth_back(1)?, stores.last()?);
    let name = img.cstr_at(name, 24)?;
    code.contains(handler).then(|| (name.to_string(), handler))
}

/// The first `pattern` in the code object between `from` and `to`, with the
/// address of the byte after it.
fn find_between(img: &Image, from: u32, to: u32, pattern: &[u8]) -> Option<u32> {
    let bytes = img.slice(from, wide(to.checked_sub(from)?))?;
    let at = bytes.windows(pattern.len()).position(|w| w == pattern)?;
    from.checked_add(crate::narrow(at))?
        .checked_add(crate::narrow(pattern.len()))
}

impl Layout {
    /// Reads the registration out of the image, given the three tables the
    /// scan found. Every failure names what was not where the two builds have
    /// it, because a build whose init differs is a build this reading does not
    /// cover — and guessing its domain base would mis-decode every module
    /// silently.
    fn read(img: &Image, words: &[KernelWord]) -> Result<Self> {
        let refuse = |detail: String| Error::Corrupt {
            what: "kernel registration",
            detail,
        };
        // The three tables' first entries, and the one instruction in the
        // code that reads each: `mov TABLE(%eax),%eax`, whose displacement is
        // the table's address and occurs nowhere else.
        let mut refs = [0u32; 3];
        for (table, r) in refs.iter_mut().enumerate() {
            let entry = words
                .iter()
                .find(|w| w.table == table && w.index == 0)
                .map(|w| w.entry)
                .ok_or_else(|| refuse(format!("kernel table {table} was not found")))?;
            *r = match code_sites(img, entry).as_slice() {
                [one] => *one,
                sites => {
                    return Err(refuse(format!(
                        "table {table} at {entry:#x} is read from {} places in the code, not one",
                        sites.len()
                    )));
                }
            };
        }
        let [ref0, ref1, ref2] = refs;

        // In the loops over tables 1 and 2 the call follows the load at once,
        // so the registration routine and its wrapper are the two targets.
        let register = call_target(img, ref1.saturating_add(4))
            .ok_or_else(|| refuse("no call follows the table 1 loop's load".into()))?;
        let wrapper = call_target(img, ref2.saturating_add(4))
            .ok_or_else(|| refuse("no call follows the table 2 loop's load".into()))?;
        if !(wrapper..wrapper.saturating_add(0x30)).any(|at| call_target(img, at) == Some(register))
        {
            return Err(refuse(format!(
                "the table 2 loop's {wrapper:#x} does not call the table 1 loop's {register:#x}"
            )));
        }

        // The dictionary pointer's start, set just before the table 0 loop,
        // and the gap skipped between `_FNAME` and table 1.
        let first = find_between(img, ref0.saturating_sub(0x100), ref0, &[0xc7, 0x40, 0x24])
            .and_then(|after| img.u32_at(after))
            .ok_or_else(|| refuse("no `movl $n,0x24(%eax)` before the table 0 loop".into()))?;
        let gap = find_between(img, ref0, ref1, &[0x83, 0x40, 0x24])
            .and_then(|after| img.slice(after, 1))
            .and_then(|b| b.first().copied())
            .map(u32::from)
            .ok_or_else(|| refuse("no `addl $n,0x24(%eax)` between the two loops".into()))?;

        // The direct calls between the two loops' loads: the first is table
        // 0's own, the rest register single words through locals.
        let direct = calls_to(img, register);
        let between = direct
            .iter()
            .filter(|&&at| at > ref0 && at < ref1)
            .skip(1)
            .map(|&at| {
                registered_through_locals(img, at).ok_or_else(|| {
                    refuse(format!(
                        "the registration at {at:#x} is not `_FNAME`'s shape"
                    ))
                })
            })
            .collect::<Result<Vec<_>>>()?;

        // The shell's words: every call to the wrapper but the table 2 loop's,
        // and every direct call outside the kernel init and the wrapper.
        let in_init = |at: u32| at >= ref0 && at <= ref1.saturating_add(8);
        let in_wrapper = |at: u32| at >= wrapper && at < wrapper.saturating_add(0x30);
        let mut sites: Vec<u32> = calls_to(img, wrapper)
            .into_iter()
            .filter(|&at| at != ref2.saturating_add(4))
            .chain(
                direct
                    .iter()
                    .copied()
                    .filter(|&at| !in_init(at) && !in_wrapper(at)),
            )
            .collect();
        sites.sort_unstable();
        let shell = sites
            .into_iter()
            .map(|site| {
                registered_at(img, site)
                    .map(|(name, handler)| (name, handler, site))
                    .ok_or_else(|| {
                        refuse(format!(
                            "the registration at {site:#x} is not `mov $handler,%edx; mov $name,%eax; call`"
                        ))
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            first,
            between,
            gap,
            shell,
        })
    }
}

/// The binding of a 32-bit kernel: every word at the ordinal the init gives
/// it, and the inline set read off the names.
///
/// The ordinals follow `Layout`: the dictionary pointer starts where the
/// init sets it and every registered word takes `ENTRY_BYTES`, so a word's
/// ordinal is the pointer at its registration plus `ORDINAL_SKEW` — table
/// 0 first, then `_FNAME`, the gap, table 1, the shell's words and table 2.
/// `words` is what [`kernel_words`] answered; a shell group in it is ignored
/// in favor of the image's own, which is the same list.
///
/// Fails, by name, where the image does not have the registration the two
/// builds have, or where the tables do not name one of the inline words.
pub fn binding_of(img: &Image, words: &[KernelWord]) -> Result<Binding> {
    let layout = Layout::read(img, words)?;
    let mut bound: Vec<(u32, String)> = Vec::new();
    let mut dp = layout.first;
    let mut register = |name: &str, dp: &mut u32| {
        bound.push((dp.saturating_add(ORDINAL_SKEW), name.to_string()));
        *dp = dp.saturating_add(ENTRY_BYTES);
    };
    let table = |t: usize| words.iter().filter(move |w| w.table == t);
    for w in table(0) {
        register(&w.name, &mut dp);
    }
    for (name, _) in &layout.between {
        register(name, &mut dp);
    }
    dp = dp.saturating_add(layout.gap);
    for w in table(1) {
        register(&w.name, &mut dp);
    }
    for (name, _, _) in &layout.shell {
        register(name, &mut dp);
    }
    for w in table(2) {
        register(&w.name, &mut dp);
    }
    bound.sort_by_key(|&(o, _)| o);
    let inline = Inline::by_name(&bound).ok_or_else(|| Error::Corrupt {
        what: "kernel table",
        detail: format!(
            "{} words found, but not every inline word among them",
            bound.len()
        ),
    })?;
    Ok(Binding {
        words: bound,
        inline,
    })
}

/// The 32-bit kernel's inline set as **measured** on `ENGINE.EXE`
/// V0.06.06/R109: the constants of [`inline`], gathered into the shape a
/// machine or a disassembler takes.
///
/// The machine reads its set off the binding a game's own binary yields
/// ([`binding_of`]); this is the measurement that reading is held to, and
/// both shipped builds agree with it word for word, because table 0 differs
/// between them only past index 83.
pub const INLINE: Inline = Inline {
    put_lit: inline::PUT_LIT,
    put_adr: inline::PUT_ADR,
    put_const: inline::PUT_CONST,
    put_string: inline::PUT_STRING,
    put_string_adr: Some(inline::PUT_STRING_ADR),
    check_if: inline::CHECK_IF,
    check_eif: inline::CHECK_EIF,
    // `ELSEDUP` compiles to `_ChElseDup`, and this is the one entry of the set
    // that is read off the table rather than measured on the compiler's
    // output: table 0 index 44 in both builds, and no shipped 32-bit module
    // uses it.
    ch_else_dup: Some(inline::CH_ELSE_DUP),
    check_else: inline::CHECK_ELSE,
    until: inline::UNTIL,
    repeat: inline::REPEAT,
    loop_break: inline::LOOP_BREAK,
    loop_end: inline::LOOP_END,
    add_loop: inline::ADD_LOOP,
    u_loop_end: inline::U_LOOP_END,
};

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
    /// `ELSEDUP` — read off the table (index 44), not measured: no shipped
    /// module compiles one.
    pub const CH_ELSE_DUP: u32 = 324;
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
}
