//! `ENVIRO.EXE` as a file: the MZ header, the load image behind it, and the
//! kernel word tables in that image.
//!
//! The 16-bit engine is a plain real-mode DOS executable. Its header says how
//! long it is in paragraphs — 800, so the load image begins at file offset
//! `0x3200` — and a far pointer `segment:offset` inside the image is file
//! offset `0x3200 + segment * 16 + offset`. That is how the two kernel
//! tables are read: each is a NUL-terminated array of 8-byte entries
//!
//! ```text
//! u16 name offset, u16 name segment, u16 handler offset, u16 handler segment
//! ```
//!
//! terminated by an entry whose name points at an empty string and whose
//! handler is `0000:0000`. Measured on ENVIRO.EXE (167 430 bytes): the table
//! of 151 domain words is at file `0x1f9f6`, the table of 82 core words at
//! `0x2064e`, their name strings together at `0x1fec5`–`0x20aae`.
//!
//! The tables are found by scanning, the way the 32-bit engine's are
//! ([`crate::m32::le::kernel_words`]): a run of at least six entries whose
//! name pointer resolves to a printable string and whose handler pointer lies
//! inside the image is a table. No other run in the file passes that test.
//!
//! **Ordinals.** A kernel cell is `0x8000 | ordinal`, and the ordinal is the
//! entry's index plus a per-table base. The core table is 1-based: `##` is
//! cell `0x8001` at the end of every colon definition. The domain table's
//! base is the build's, and it is read from the build — see
//! [`placeholder_count`]. The player hands ordinals out in registration
//! order, and it registers three runs: the core table, then a run of
//! `DUMMY#F0R3i` placeholders, then the domain table. So the first domain
//! word binds at `core + placeholders + 1` — 105 in `ENVIRO.EXE`,
//! `HPPLAY.EXE` and `BMZ.EXE`, whose core tables hold 82 words and which
//! register 22 placeholders, and 102 in `LL.EXE`, whose core table holds 80
//! and which registers 21.
//!
//! The bases are confirmed against the modules: every one of the 24 282
//! kernel cells in the 65 modules of Die Enviro-Kids greifen ein names an
//! entry under this rule, `TOGFX` is `0x8069` where its `RUN` enters graphics
//! mode, and all 18 911 cells of Victor Loomes' 36 modules name one under its
//! own base — where the 105 of the later builds would leave three cells
//! naming nothing and would put `SDTXT`, `FADEIN` and `FADEOUT` out of reach
//! of a game that calls them 149, 11 and 15 times.

use crate::error::{Error, Result};
use crate::kernel::{Binding, Inline, KernelWord};
use crate::{Record, bytes};

/// The bit that marks a cell as a kernel word.
pub const KERNEL_BIT: u16 = 0x8000;
/// What is left of a kernel cell once the bit is taken off.
pub const ORDINAL_MASK: u16 = 0x7fff;
/// The ordinal of `##`, the word that returns from a colon definition.
pub const RETURN_ORDINAL: u32 = 1;

/// Ordinal of the core table's first entry, `##`.
const CORE_BASE: u32 = 1;

/// The name the player registers its ordinal placeholders under.
const PLACEHOLDER_NAME: &[u8] = b"DUMMY#F0R3i";

/// The executable, read whole.
pub struct Image {
    data: Vec<u8>,
    header_len: usize,
}

impl std::fmt::Debug for Image {
    /// Sizes, not bytes: an executable is several hundred kilobytes.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Image")
            .field("len", &self.data.len())
            .field("header_len", &self.header_len)
            .finish()
    }
}

impl Image {
    /// Reads an MZ executable from disk.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        Self::parse(std::fs::read(path)?)
    }

    /// The same from memory.
    pub fn parse(data: Vec<u8>) -> Result<Self> {
        let head = Record(bytes::<0x1c>(&data, 0)?);
        let signature = head.bytes::<0, 2>();
        if &signature != b"MZ" && &signature != b"ZM" {
            return Err(Error::Corrupt {
                what: "MZ executable",
                detail: format!("signature is {signature:02x?}, expected MZ"),
            });
        }
        let header_len = usize::from(head.u16::<8>()).saturating_mul(16);
        if header_len > data.len() {
            return Err(Error::Corrupt {
                what: "MZ executable",
                detail: format!("header of {header_len} bytes in a {}-byte file", data.len()),
            });
        }
        Ok(Self { data, header_len })
    }

    /// Bytes of header before the load image — `0x3200` in ENVIRO.EXE.
    pub fn header_len(&self) -> usize {
        self.header_len
    }

    /// The whole file.
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }

    /// The load image: everything after the header.
    pub fn load_image(&self) -> &[u8] {
        self.data.get(self.header_len..).unwrap_or_default()
    }

    /// The file offset a far pointer names.
    pub fn file_offset(&self, segment: u16, offset: u16) -> usize {
        let linear = usize::from(segment)
            .saturating_mul(16)
            .saturating_add(usize::from(offset));
        self.header_len.saturating_add(linear)
    }

    /// The NUL-terminated string at a file offset, if it is one of at most
    /// `max` printable ASCII bytes.
    fn cstr_at(&self, off: usize, max: usize) -> Option<&str> {
        let bytes = self.data.get(off..)?;
        let end = bytes
            .iter()
            .take(max.saturating_add(1))
            .position(|&b| b == 0)?;
        let s = bytes.get(..end)?;
        if s.is_empty() || !s.iter().all(|&b| (0x21..0x7f).contains(&b)) {
            return None;
        }
        std::str::from_utf8(s).ok()
    }
}

/// Finds the kernel's word tables in the executable.
///
/// The tables are not aligned and not pointed at by anything the loader
/// reads, so the scan walks the image byte by byte. `table` in the answer is
/// the table's number in file order, `index` the entry's position in it,
/// `handler` and `entry` file offsets.
pub fn kernel_words(img: &Image) -> Vec<KernelWord> {
    const ENTRY: usize = 8;
    const MIN_RUN: usize = 6;
    let data = img.bytes();
    // An entry that fits, read as a name and a handler: `{name_off, name_seg,
    // fn_off, fn_seg}`, two far pointers.
    let plausible = |at: usize| -> Option<(String, u32)> {
        let entry = Record(bytes::<ENTRY>(data, at).ok()?);
        let name = img.cstr_at(img.file_offset(entry.u16::<2>(), entry.u16::<0>()), 24)?;
        let handler = img.file_offset(entry.u16::<6>(), entry.u16::<4>());
        (img.header_len()..data.len())
            .contains(&handler)
            .then(|| (name.to_string(), crate::narrow(handler)))
    };

    let mut words = Vec::new();
    let mut table = 0usize;
    let mut at = img.header_len();
    while at < data.len() {
        if plausible(at).is_none() {
            at = at.saturating_add(1);
            continue;
        }
        let start = at;
        let mut run = Vec::new();
        while let Some((name, handler)) = plausible(at) {
            run.push((at, name, handler));
            at = at.saturating_add(ENTRY);
        }
        if run.len() >= MIN_RUN {
            for (index, (entry, name, handler)) in run.into_iter().enumerate() {
                words.push(KernelWord {
                    name,
                    handler,
                    entry: crate::narrow(entry),
                    table,
                    index,
                });
            }
            table = table.saturating_add(1);
        } else {
            at = start.saturating_add(1);
        }
    }
    words
}

/// How many `DUMMY#F0R3i` placeholders the build registers between its two
/// tables, or `None` where the run that registers them is not found.
///
/// The placeholders are what separates the core table's last ordinal from the
/// domain table's first, so this number decides the domain base. It is read
/// rather than assumed: the loop that registers them loads the name's data
/// offset and is bounded by a `cmp %si, imm8` (`0afe:009a` in `LL.EXE`,
/// `140e:002f` in `ENVIRO.EXE`, `13d9:002f` in `HPPLAY.EXE`, `140a:0039` in
/// `BMZ.EXE`), so this looks for that comparison behind the one place in the
/// image that mentions the name's offset. Exactly one place does, in each of
/// the five builds.
///
/// `ds` is the data segment, which is the segment every kernel word's *name*
/// pointer is relative to — the scan takes it from the words it already found.
pub fn placeholder_count(img: &Image, ds: u16) -> Option<u8> {
    // `cmp %si, imm8`, the bound of the loop that does the registering.
    const CMP_SI: [u8; 2] = [0x83, 0xfe];
    // Far enough to cover the loop body, which registers one placeholder.
    const REACH: usize = 96;

    let data = img.bytes();
    let name = data
        .windows(PLACEHOLDER_NAME.len())
        .position(|w| w == PLACEHOLDER_NAME)?;
    let offset = name
        .checked_sub(img.header_len())?
        .checked_sub(usize::from(ds).checked_mul(16)?)?;
    let mentions = u16::try_from(offset).ok()?.to_le_bytes();

    let mut found = None;
    let mut at = img.header_len();
    while let Some(hit) = data.get(at..).and_then(|d| {
        d.windows(2)
            .position(|w| w == mentions)
            .and_then(|p| at.checked_add(p))
    }) {
        at = hit.saturating_add(1);
        let window = reach(data, hit, REACH)?;
        if let Some(p) = window.windows(2).position(|w| w == CMP_SI) {
            // Two mentions with a bound behind them would leave which loop is
            // meant to a guess, and a guessed ordinal base is a wrong one.
            if found.is_some() {
                return None;
            }
            found = p.checked_add(2).and_then(|i| window.get(i)).copied();
        }
    }
    found
}

/// Whether this build's `?XINSIDE` passes over a hot area whose four corners
/// are all zero, or takes it as a rectangle at the origin like any other.
///
/// A property of the build and not of the format, and the two are three years
/// apart rather than one generation apart: `ENVIRO.EXE` (`0a40:1b37`) and
/// `BMZ.EXE` follow the four corner comparisons with four more asking whether
/// each corner is zero, and pass over the entry when they all are.
/// `HPPLAY.EXE` and `LL.EXE` stop after the comparisons — 71 instructions
/// against 101. So this reads the handler rather than the game: the later
/// pair holds four `cmpw $0, %es:(%bx)` in the 300 bytes behind its entry
/// point and the older pair none.
///
/// A build with no `?XINSIDE` at all answers `false`, which is what a handler
/// with no such test does.
pub fn skips_empty_areas(img: &Image, words: &[KernelWord]) -> bool {
    // `cmpw $0, %es:(%bx)`: the segment override, the group-1 opcode with an
    // 8-bit immediate, the `[bx]` mode with `/7` for `cmp`, and the zero.
    const HOLE_TEST: [u8; 4] = [0x26, 0x83, 0x3f, 0x00];
    // Past the end of the longer of the two handlers, and short of anything
    // that could hold a second one.
    const REACH: usize = 300;

    let Some(w) = words.iter().find(|w| w.name == "?XINSIDE") else {
        return false;
    };
    reach(img.bytes(), crate::wide(w.handler), REACH)
        .is_some_and(|body| body.windows(HOLE_TEST.len()).any(|w| w == HOLE_TEST))
}

/// Up to `len` bytes from `at` — fewer at the end of the data — or `None`
/// for a start past it.
fn reach(data: &[u8], at: usize, len: usize) -> Option<&[u8]> {
    let rest = data.get(at..)?;
    Some(rest.get(..len).unwrap_or(rest))
}

/// Whether a byte pattern — `None` a wildcard — lies within `reach` bytes of
/// a kernel word's handler. How a build's variant of a routine is told apart:
/// by what the routine holds, not by which game shipped it.
fn handler_holds(
    img: &Image,
    words: &[KernelWord],
    name: &str,
    reach: usize,
    pat: &[Option<u8>],
) -> bool {
    let Some(w) = words.iter().find(|w| w.name == name) else {
        return false;
    };
    self::reach(img.bytes(), crate::wide(w.handler), reach).is_some_and(|body| {
        body.windows(pat.len())
            .any(|w| w.iter().zip(pat).all(|(b, p)| p.is_none_or(|p| p == *b)))
    })
}

/// Whether this build's `NEWSETDESC` refuses the hundred-and-first descriptor
/// of a screen.
///
/// `ENVIRO.EXE` (`05f1:0ad4`), `HPPLAY.EXE` (file `0x9b2e`) and `BMZ.EXE`
/// (file `0x9bd5`) open the handler by comparing the screen's count against
/// a hundred and jump past every pop and the push when it is there; `LL.EXE`
/// (file `0x44ce`) takes the count and raises it without looking. Read off
/// the handler: the comparison itself, five bytes.
pub fn newsetdesc_capped(img: &Image, words: &[KernelWord]) -> bool {
    // `cmp word ptr es:[bx+0x16], 0x64`.
    const HUNDRED: [Option<u8>; 5] = [Some(0x26), Some(0x83), Some(0x7f), Some(0x16), Some(0x64)];
    handler_holds(img, words, "NEWSETDESC", 0x20, &HUNDRED)
}

/// How far the walk builder's body reaches from `CROUTE`'s entry: past the
/// longest of the five (`LL.EXE`'s, `0x8d4` bytes with its closing pass) and
/// short of the routines behind it.
const CROUTE_REACH: usize = 0xa00;

/// Whether this build's `CROUTE` takes a shadow record's zero shrink as 1000
/// for the walk's first step.
///
/// `ENVIRO.EXE` does (`0a40:116f`–`0x1183`: `cmpw $0, %es:0x1c(%bx)`, then
/// `mov $0x3e8, %ax` on the zero branch), as the 32-bit routine does
/// (`0x778ca`). `HPPLAY.EXE` (`0a16:116c`), `BMZ.EXE` (`0a3f:1170`) and
/// `LL.EXE` (`0104:4ae4`) copy the field as it stands. Read off the handler:
/// the comparison and the immediate together, nine bytes with the branch
/// displacement left open.
pub fn croute_defaults_shrink(img: &Image, words: &[KernelWord]) -> bool {
    const TEST_AND_DEFAULT: [Option<u8>; 10] = [
        Some(0x26),
        Some(0x83),
        Some(0x7f),
        Some(0x1c),
        Some(0x00), // cmpw $0, %es:0x1c(%bx)
        Some(0x75),
        None, // jne
        Some(0xb8),
        Some(0xe8),
        Some(0x03), // mov $0x3e8, %ax
    ];
    handler_holds(img, words, "CROUTE", CROUTE_REACH, &TEST_AND_DEFAULT)
}

/// Whether this build's `CROUTE` ends with the pass that rewrites the heading
/// of a short run of steps between two longer runs — `LL.EXE`'s
/// `0104:516d`–`0x5318`, which no later build has.
///
/// Read off the handler by the pass's own tests: the two run headings held
/// against 2 and 3 on both sides (`0104:52b0`–`0x52c6`), four compare-and-
/// branch pairs on frame locals whose offsets are the compiler's and are
/// left open.
pub fn croute_smooths_headings(img: &Image, words: &[KernelWord]) -> bool {
    const SIDE_TESTS: [Option<u8>; 23] = [
        Some(0x83),
        Some(0x7e),
        None,
        Some(0x02),
        Some(0x7f),
        None, // cmpw $2, h2; jg
        Some(0x83),
        Some(0x7e),
        None,
        Some(0x03),
        Some(0x7d),
        None, // cmpw $3, h1; jge
        Some(0x83),
        Some(0x7e),
        None,
        Some(0x03),
        Some(0x7c),
        None, // cmpw $3, h2; jl
        Some(0x83),
        Some(0x7e),
        None,
        Some(0x02),
        Some(0x7f), // cmpw $2, h1; jg
    ];
    handler_holds(img, words, "CROUTE", CROUTE_REACH, &SIDE_TESTS)
}

/// The bytecode ordinal for a kernel word: its index plus its table's base.
///
/// `domain_base` is what [`placeholder_count`] settles; the core table is
/// always 1-based. In file order the domain table comes first, so table 0 is
/// the domain one and table 1 the core one.
pub fn ordinal_of(word: &KernelWord, domain_base: u32) -> Option<u32> {
    match word.table {
        0 => domain_base.checked_add(crate::narrow(word.index)),
        1 => CORE_BASE.checked_add(crate::narrow(word.index)),
        _ => None,
    }
}

/// The binding of a 16-bit kernel: every word at its ordinal, and the inline
/// set read off the names. Fails if the tables do not name one of the inline
/// words — which is what a scan that found the wrong runs looks like — or if
/// the build does not say how many placeholders it registers, because the
/// domain base then cannot be told from a guess.
pub fn binding_of(img: &Image, words: &[KernelWord]) -> Result<Binding> {
    // The data segment, which every entry's name pointer is relative to: the
    // segment half of the first entry's, at `+2` of the entry it was read from.
    let ds = match words.first() {
        Some(w) => Record(bytes::<8>(img.bytes(), crate::wide(w.entry))?).u16::<2>(),
        None => {
            return Err(Error::Corrupt {
                what: "kernel table",
                detail: "no kernel words at all".into(),
            });
        }
    };
    let placeholders = placeholder_count(img, ds).ok_or_else(|| Error::Corrupt {
        what: "kernel table",
        detail: "the run of DUMMY#F0R3i placeholders that sets the domain \
                 table's ordinal base is not in this build"
            .into(),
    })?;
    let core = crate::narrow(words.iter().filter(|w| w.table == 1).count());
    let domain_base = CORE_BASE
        .saturating_add(core)
        .saturating_add(u32::from(placeholders));

    let mut bound: Vec<(u32, String)> = words
        .iter()
        .filter_map(|w| ordinal_of(w, domain_base).map(|o| (o, w.name.clone())))
        .collect();
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
