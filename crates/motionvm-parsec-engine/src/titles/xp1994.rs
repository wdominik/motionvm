//! The location dispatch of the parsec1994 generation.
//!
//! The dispatch is five instructions, seventeen bytes, that occur exactly
//! once in Dunkle Schatten's `XP01.EXE` (`0x4FC0`): the location word read
//! into BX, BX multiplied by the record's stride, the table's segment
//! moved into ES through AX, and a far call through the record's handler
//! pointer. The location word, the stride, the segment and the handler's
//! offset are the immediates of those instructions, and every other figure
//! this module needs is an immediate inside an instruction found the same
//! way — the constants below spell the opcodes it looks for.
//!
//! What that comes out as:
//!
//! | | location | stride | table | handler | name | rows |
//! |---|---|---:|---|---:|---:|---:|
//! | Dunkle Schatten | `1F4Ah` | 52 | `204Bh` | `+23h` | `+27h` | 61 |
//!
//! Nothing in that table is written down here; it is read out of the
//! program.
//!
//! The three globals sit together: the entry id at `location − 2`, the
//! location at `location`, and the one the handler is compared against at
//! `location + 2`.
//!
//! The word at `+7` of a record says what kind of location the row is.
//! The main loop reads it before it dispatches and treats zero as "not a
//! place":
//! it bounds `[1F4Ah]` against 3Dh at `0x04F8E`, compares the kind word
//! `es:[bx+7]` with zero at `0x04FA1`, and for a zero kind calls `0x053B2`
//! instead of the row's handler.
//!
//! Where that call lands, Dunkle Schatten walks forward to the next row
//! whose kind is neither zero nor 4 and enters it with entry `7Fh`.

use crate::exe::Image;
use crate::num;

use crate::error::Result;

/// `mov bx, [imm16]`, the dispatch read.
const MOV_BX_MEM: [u8; 2] = [0x8B, 0x1E];
/// `imul bx, bx, imm8`, the record stride.
const IMUL_BX_IMM8: [u8; 2] = [0x6B, 0xDB];
/// `mov ax, imm16` / `mov es, ax`, the record table's segment.
const MOV_AX_IMM: u8 = 0xB8;
/// `mov es, ax`.
const MOV_ES_AX: [u8; 2] = [0x8E, 0xC0];
/// `call far es:[bx+imm16]`, the handler.
const CALL_FAR_ES_BX: [u8; 3] = [0x26, 0xFF, 0x9F];
/// `cmp word [imm16], imm8`, how the program bounds the location.
const CMP_MEM_IMM8: [u8; 2] = [0x83, 0x3E];
/// `jge short`, which is what follows that bound.
const JGE: u8 = 0x7D;
/// `mov ax, es:[bx+imm16]` — the high half of a far pointer in a record.
const MOV_AX_ES_BX: [u8; 3] = [0x26, 0x8B, 0x87];
/// `cmp word es:[bx+imm16], imm8` — how the main loop asks a row what kind
/// of location it is.
const CMP_ES_BX_IMM8: [u8; 3] = [0x26, 0x83, 0xBF];
/// `mov al, es:[bx+imm16]` — how a byte list inside a record is read.
const MOV_AL_ES_BX: [u8; 3] = [0x26, 0x8A, 0x87];
/// `mov al, es:[bx+imm8]` — the same with a short displacement, which is
/// what the entry function's walk over the object list uses.
const MOV_AL_ES_BX8: [u8; 3] = [0x26, 0x8A, 0x47];
/// The seven bytes the entry function adds to a record's address before it
/// walks the object list. It is an `add ax, 7`, and the
/// displacements below are relative to that.
const ENTRY_BIAS: usize = 7;
/// `mov ah, 0`, which widens it.
const MOV_AH_0: [u8; 2] = [0xB4, 0x00];
/// `cmp ax, cx`, the comparison against the id being looked for.
const CMP_AX_CX: [u8; 2] = [0x3B, 0xC1];
/// `mov [imm16], ax`.
const MOV_MEM_AX: u8 = 0xA3;
/// `mov dx, es:[bx+imm16]` — its low half.
const MOV_DX_ES_BX: [u8; 3] = [0x26, 0x8B, 0x97];

/// The location table of the program.
pub(crate) struct Locations {
    /// Image offset of the dispatch, so an error can name it.
    pub at: usize,
    /// The global the main loop dispatches on.
    pub location: u16,
    /// Segment of the record table.
    pub table: u16,
    /// Bytes per record.
    pub stride: usize,
    /// Offset of the handler far pointer inside a record.
    pub handler: usize,
    /// Offset of the name far pointer inside a record.
    pub name: usize,
    /// How many locations there are, from the bound the program checks
    /// before it dispatches.
    pub count: usize,
    /// One entry per location. `None` where the record carries no pointer,
    /// which is not the same as a name that is the empty string.
    pub names: Vec<Option<String>>,
    /// Offset of the kind word inside a record. It comes out as 7, and is
    /// read anyway.
    pub kind: usize,
    /// Offset of the list of exit ids inside a record.
    pub exits: usize,
    /// Offset of the list of destinations beside it. The two lists are
    /// parallel and the gap between them is how long each is.
    pub destinations: usize,
    /// Offset of the list of object ids, and how many slots it has —
    /// both from the loop the entry function walks it with.
    pub objects: usize,
    /// How many object slots a record has.
    pub object_slots: usize,
    /// Offset of the far pointer to the location's own interaction table,
    /// the one the entry function installs before anything else.
    pub rules: usize,
}

/// The row of item names the program carries.
///
/// `0x0F5AC` bounds an id against the row's length and takes the name out
/// of a far-pointer row, four bytes to an entry, **counting from one**:
/// the bound is `cmp si, 17h` at `0x0F5AC`, so ids below 23, and the far
/// pointers are read from `[bx+1000h]` and `[bx+1002h]` with `bx = si · 4`
/// (`0x0F5B1`–`0x0F5BA`): the row is at `DS:1000h`.
///
/// So entry 0 is never named and the names are ids 1 upwards: 22 of them
/// in Dunkle Schatten.
pub(crate) fn item_names(img: &Image) -> Result<Vec<String>> {
    let b = img.bytes();
    let (bound, row) = (0..b.len().saturating_sub(20))
        .find_map(|i| {
            (b[i..i + 2] == [0x83, 0xFE]
                && b[i + 3] == 0x7D
                && b[i + 5..i + 7] == [0x8B, 0xDE]
                && b[i + 7..i + 10] == [0xC1, 0xE3, 0x02]
                && b[i + 10..i + 12] == [0x8B, 0x87]
                && b[i + 14..i + 16] == [0x8B, 0x97])
            .then(|| Some((usize::from(b[i + 2]), usize::from(img.word(i + 16).ok()?))))
            .flatten()
        })
        .ok_or("nothing names an item the way the hover routine does")?;
    (1..bound)
        .map(|n| img.c_string(img.far_ptr(img.at(row) + 4 * n)?))
        .collect()
}

/// Panel resources indexed by item id, excluding unused id zero.
///
/// `XP01.EXE`'s `0x0C283` reads a byte mapping, indexed with `[si+..]` (`8A 84`),
/// before fetching the panel RIB pointer at `8+4*n`. It maps id 1
/// onwards, and a pair of ids may share one cel.
pub(crate) fn item_resources(img: &Image, count: usize) -> Result<Vec<usize>> {
    let b = img.bytes();
    let sites: Vec<_> = b
        .windows(23)
        .enumerate()
        .filter_map(|(i, w)| {
            (w[0] == 0x8A
                && w[1] == 0x84
                && w[4..11] == [0xB4, 0, 0xC1, 0xE0, 2, 0xC4, 0x1E]
                && w[13..23] == [0x03, 0xD8, 0x26, 0x8B, 0x47, 0x0A, 0x26, 0x8B, 0x57, 8])
            .then_some(i)
        })
        .collect();
    let [site] = sites.as_slice() else {
        return Err(format!("expected one inventory cel mapping, found {}", sites.len()).into());
    };
    let first = img.at(usize::from(img.word(site + 2)?)) + 1;
    let row = b
        .get(first..first + count)
        .ok_or("inventory cel mapping out of range")?;
    Ok(row.iter().map(|&n| usize::from(n) + 2).collect())
}

/// The cue exclusion word read by `XP01.EXE`'s `0x0D67F`. The first comparison is
/// a separate global, not the cursor mode applied by the delayed setter.
/// The routine holds the comparisons, the pending-mode store and the
/// three-pass countdown.
pub(crate) fn cursor_cue_guard(img: &Image) -> Result<u16> {
    let sites: Vec<_> = img
        .bytes()
        .windows(30)
        .enumerate()
        .filter_map(|(i, w)| {
            (w[..8] == [0x55, 0x8B, 0xEC, 0x8B, 0x56, 4, 0x39, 0x16]
                && w[10..14] == [0x74, 0x10, 0x39, 0x16]
                && w[16..20] == [0x74, 0x0A, 0x89, 0x16]
                && w[14..16] == w[20..22]
                && w[22..24] == [0xC7, 6]
                && w[26..30] == [3, 0, 0x5D, 0xC3])
            .then_some(i)
        })
        .collect();
    let [site] = sites.as_slice() else {
        return Err(format!("expected one cursor cue routine, found {}", sites.len()).into());
    };
    img.word(img.at(usize::from(img.word(site + 8)?)))
}

/// One speaker style a caption can be drawn in.
///
/// Three bytes, picked by an index the task carries: `0x0D95A` computes
/// `di * 3 + 9` into the row and keeps the far pointer in the task's own
/// record, and the painter reads `+0` for the ink and `+2` for the panel.
/// Dunkle Schatten has eleven of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Style {
    /// The color the glyphs are drawn in.
    pub ink: u8,
    /// The byte between the two. What it is for is not read.
    pub middle: u8,
    /// The color a panel is filled with, when the style asks for one.
    pub panel: u8,
}

impl Style {
    /// Three bytes each.
    pub(crate) const SIZE: usize = 3;

    /// Byte `n` of the three, the way a `01 n` control in a caption picks
    /// its ink (`0x129B0`): 0 the ink, 1 the middle byte, 2 the panel.
    /// Past the three the original reads into the next style; this reads
    /// the ink instead.
    pub(crate) fn byte(self, n: u8) -> u8 {
        match n {
            0 => self.ink,
            1 => self.middle,
            2 => self.panel,
            _ => self.ink,
        }
    }

    /// The row of styles, found by the arithmetic that indexes it.
    ///
    /// `imul ax, 3` followed by `add ax, imm16` is the row's offset being
    /// built, and the `mov word [bx+field], imm16` a few bytes later is its
    /// segment going into the task. The row ends at its first all-zero
    /// entry, after eleven in Dunkle Schatten.
    pub(crate) fn read_row(img: &Image) -> Result<Vec<Style>> {
        let row = Self::row_address(img)?;
        let b = img.bytes();
        let mut out = Vec::new();
        while let Some(e) = b.get(row + out.len() * Self::SIZE..row + (out.len() + 1) * Self::SIZE)
        {
            if e == [0, 0, 0] || out.len() > 64 {
                break;
            }
            out.push(Style {
                ink: e[0],
                middle: e[1],
                panel: e[2],
            });
        }
        Ok(out)
    }

    /// The style a hover text is set in: the three bytes before the row,
    /// which `0CFDh:1810h` points the task at (`0x0E882`, `2139h:0006h`).
    pub(crate) fn read_hover(img: &Image) -> Result<Style> {
        let row = Self::row_address(img)?;
        let b = img.bytes();
        let e = b
            .get(row - Self::SIZE..row)
            .ok_or("the style row has nothing before it")?;
        Ok(Style {
            ink: e[0],
            middle: e[1],
            panel: e[2],
        })
    }

    /// Where the row is, in the image.
    fn row_address(img: &Image) -> Result<usize> {
        let b = img.bytes();
        // Several sites multiply by three; the style row's is the one whose
        // segment goes straight into a task record a few bytes later.
        let (base, segment) = (0..b.len().saturating_sub(24))
            .filter(|&i| b[i] == 0x6B && b[i + 1] == 0xC0 && b[i + 2] == 3 && b[i + 3] == 0x05)
            .find_map(|i| {
                let seg = (i + 6..i + 24)
                    .find(|&j| b[j] == 0xC7 && b[j + 1] == 0x87)
                    .and_then(|j| img.word(j + 4).ok())?;
                Some((usize::from(img.word(i + 4).ok()?), seg))
            })
            .ok_or("nothing indexes a style row the way a task takes one")?;
        Ok(usize::from(segment) * 16 + base)
    }
}

/// How the caption painter flattens what is under a box.
///
/// Its middle pass walks every pixel of the rectangle through a 256-byte
/// table in its own code segment. The table is built at run time by
/// `0724h:01A6h`, which fills all of it with one color and then writes a
/// run that maps to itself:
/// `0x073EE` points `di` at the table (`0A6h` in that segment), `0x073FB`
/// fills all 256 bytes, and from `0x07405` a run starting at `first` is stored
/// mapping each color to itself.
///
/// Dunkle Schatten calls it with `(0C0h, 10h)`: everything becomes color
/// **192** except colors 192–207, which stay as they are. This describes
/// only initialization: `0724h:01E1h` updates entries from loaded RGB
/// using weights 5:9:2; [`Flatten::enable_updates`] turns those on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Flatten {
    /// What every other color becomes.
    pub to: u8,
    /// The first color of the run that is left alone.
    pub first: u8,
    /// How many colors that run holds.
    pub count: u8,
    /// Mutable lookup after resource loads; absent until
    /// [`Self::enable_updates`].
    pub table: Option<[u8; 256]>,
}

impl Flatten {
    /// The table builder's initial fill (`0x073E6` in `XP01.EXE`), before
    /// any resource RGB updates. The destination run maps to itself.
    pub(crate) fn enable_updates(&mut self) {
        self.table = Some(std::array::from_fn(|i| self.map(num::byte(i))));
    }

    /// `0x07421` in `XP01.EXE`: a resource load updates only the given range.
    /// Raw DAC changes (panel variants, ramps) must not call this method.
    pub(crate) fn load_rgb(&mut self, start: u8, rgb: &[u8]) {
        let Some(table) = &mut self.table else {
            return;
        };
        for (dst, c) in table[usize::from(start)..]
            .iter_mut()
            .zip(rgb.as_chunks::<3>().0)
        {
            let weighted = 5 * u32::from(c[0]) + 9 * u32::from(c[1]) + 2 * u32::from(c[2]);
            *dst = self.first.wrapping_add(num::low8(num::low_word(
                (weighted * (u32::from(self.count) << 6)) >> 16,
            )));
        }
    }

    /// `0x07461` in `XP01.EXE`: replace a pixel index with its lookup entry.
    pub(crate) fn map(&self, pixel: u8) -> u8 {
        if let Some(table) = &self.table {
            return table[usize::from(pixel)];
        }
        if (u16::from(self.first)..u16::from(self.first) + u16::from(self.count))
            .contains(&u16::from(pixel))
        {
            pixel
        } else {
            self.to
        }
    }

    /// The prologue of the routine that builds the table: `push bp;`
    /// `mov bp,sp; push di; mov ax,cs; mov es,ax; mov di,0A6h`.
    const BUILDER: [u8; 11] = [
        0x55, 0x8B, 0xEC, 0x57, 0x8C, 0xC8, 0x8E, 0xC0, 0xBF, 0xA6, 0x00,
    ];

    /// The two numbers the one call to the builder hands it.
    ///
    /// The builder is entered far, and which segment the linker gave it is
    /// not derivable from where it sits, so the call is found by resolving
    /// every far call's target instead. In front of it stand the two
    /// pushes: `push imm8` for the count and `push imm16` for the color.
    pub(crate) fn read(img: &Image) -> Result<Self> {
        let b = img.bytes();
        let at = (0..b.len().saturating_sub(Self::BUILDER.len()))
            .find(|&i| b[i..i + Self::BUILDER.len()] == Self::BUILDER)
            .ok_or("nothing builds a flattening table in this program")?;
        let site = (6..b.len().saturating_sub(5))
            .find(|&i| {
                b[i] == 0x9A
                    && usize::from(img.word(i + 3).unwrap_or(0)) * 16
                        + usize::from(img.word(i + 1).unwrap_or(0))
                        == at
                    && b[i - 3] == 0x68
                    && b[i - 5] == 0x6A
            })
            .ok_or("nothing calls the flattening table's builder")?;
        let color = img.word(site - 2)?;
        Ok(Flatten {
            to: num::low8(color),
            first: num::low8(color),
            count: b[site - 4],
            table: None,
        })
    }
}

/// How long a page of a caption stays up, in the tick's own units.
///
/// `0x0D865` weighs the page's own size into a duration and clamps it
/// both ways. The four weights sit together in the data segment, and
/// they are read rather than written down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Timing {
    /// The starting value, and the floor.
    pub base: u16,
    /// Added per line.
    pub per_line: u16,
    /// Added per character, over 32.
    pub per_char: u16,
    /// The ceiling.
    pub max: u16,
    /// Where the four live in the data segment, in the same order: the
    /// options write them (`0x125E9`–`0x12616`), so a page is weighed with
    /// what the data segment holds when it goes up, not with these.
    pub sites: [u16; 4],
}

impl Timing {
    /// The four weights, found by the arithmetic that uses them.
    ///
    /// `0x0D871` multiplies the line count by `[167Fh]`, the time per line;
    /// `0x0D876` adds the base `[167Dh]`; `0x0D87C` multiplies the character
    /// count by `[1681h]` and `0x0D882` shifts that right by five, per
    /// character over 32.
    ///
    /// The ceiling is six bytes past the base, which is where `0x0D8A2`
    /// compares against it.
    pub(crate) fn read(img: &Image) -> Result<Self> {
        let b = img.bytes();
        let (base, per_line, per_char) = (0..b.len().saturating_sub(24))
            .find_map(|i| {
                (b[i] == 0xA1
                    && b[i + 3..i + 5] == [0xF7, 0xEF]
                    && b[i + 5..i + 7] == [0x8B, 0x3E]
                    && b[i + 11] == 0xA1
                    && b[i + 14..i + 16] == [0xF7, 0x6E]
                    && b[i + 17..i + 20] == [0xC1, 0xF8, 0x05])
                .then(|| {
                    Some((
                        img.word(i + 7).ok()?,
                        img.word(i + 1).ok()?,
                        img.word(i + 12).ok()?,
                    ))
                })
                .flatten()
            })
            .ok_or("nothing weighs a caption's duration the way the page walk does")?;
        Ok(Timing {
            base: img.word(img.at(usize::from(base)))?,
            per_line: img.word(img.at(usize::from(per_line)))?,
            per_char: img.word(img.at(usize::from(per_char)))?,
            max: img.word(img.at(usize::from(base) + 6))?,
            sites: [base, per_line, per_char, base.wrapping_add(6)],
        })
    }
}

/// One rule of the parsec1994 generation's interaction table.
///
/// What a verb does to an object is a table, not a switch. `0x13E41`
/// walks a row of these with three keys — the verb in `[288Dh]`, the
/// thing under the pointer in `[2891h]`, and a phase mask the caller
/// passes — and a row ends where the phase is zero.
///
/// A handler does not act by itself. It names a dialogue and hands the
/// number to the task machine's prepare call, which is why the table is
/// worth reading as data: the rules say *which* rule wins, and the
/// handlers say what it answers with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// The verb this rule answers for. **0 matches any verb.**
    pub verb: u16,
    /// Which phases of an interaction the rule takes part in: bit 0 while
    /// the pointer is only naming things, bit 2 for acting on one object,
    /// bit 5 for acting with two.
    pub phase: u16,
    /// The object, by the same 1-based number the item row uses. Negative
    /// values are classes, and `-1` is anything.
    pub object: i16,
    /// A second object, for the two-object verbs.
    pub object2: i16,
    /// The handler, as `(segment, offset)`. Code, so this reader stops
    /// here and the address says where it is.
    pub handler: (u16, u16),
}

impl Rule {
    /// Fourteen bytes, which is the stride `0x13E4F` adds.
    pub const SIZE: usize = 14;

    /// The prologue of the installer that is handed the global table:
    /// `push bp; mov bp,sp; push ds; push si; push di; lds si,[bp+6];`
    /// `mov cs:[0], si`. It keeps the table in its own code segment, which
    /// is what makes it findable at all.
    const INSTALLER: [u8; 14] = [
        0x55, 0x8B, 0xEC, 0x1E, 0x56, 0x57, 0xC5, 0x76, 0x06, 0x2E, 0x89, 0x36, 0x00, 0x00,
    ];

    /// The segment the interaction tables' installers live in.
    ///
    /// Found by the prologue above, which sits at offset `0Ch` of that
    /// segment. Everything else about the tables hangs off this.
    fn installer(img: &Image) -> Result<u16> {
        let bytes = img.bytes();
        let at = (0..bytes.len().saturating_sub(Self::INSTALLER.len()))
            .find(|&i| bytes[i..i + Self::INSTALLER.len()] == Self::INSTALLER)
            .ok_or("no interaction-table installer in this program")?;
        if at < 0x0C || (at - 0x0C) % 16 != 0 {
            return Err(
                format!("the installer at {at:#x} is not at offset 0Ch of a segment").into(),
            );
        }
        Ok(num::word((at - 0x0C) / 16))
    }

    /// Where a location record keeps the far pointer to its own table.
    ///
    /// The entry function builds `es:bx` as the record plus seven and then
    /// pushes two words off it into the second installer, `<seg>:0031h`:
    /// `0x0EDBF` multiplies the row by the stride 34h, `0x0EDC2` adds seven,
    /// and `0x0EDD0`–`0x0EDD8` push `es:[bx+26h]` and `es:[bx+24h]` and call
    /// `13D0h:0031h`.
    ///
    /// So the field is `7 + 24h` = `+2Bh` in Dunkle Schatten — read rather
    /// than written down.
    fn location_field(img: &Image) -> Result<usize> {
        let segment = Self::installer(img)?;
        let bytes = img.bytes();
        let [lo, hi] = segment.to_le_bytes();
        let call = [0x9A, 0x31, 0x00, lo, hi];
        let site = (40..bytes.len().saturating_sub(call.len()))
            .find(|&i| {
                bytes[i..i + call.len()] == call
                    && bytes[i - 4..i - 1] == [0x26, 0xFF, 0x77]
                    && bytes[i - 8..i - 5] == [0x26, 0xFF, 0x77]
            })
            .ok_or("nothing hands a location's own table to its installer")?;
        let disp = usize::from(bytes[site - 1]);
        // `imul ax, imm8` immediately followed by `add ax, imm16` is the
        // record address being built; the immediate of the add is where the
        // pushes count from.
        let base = (site - 40..site)
            .find(|&j| bytes[j] == 0x6B && bytes[j + 1] == 0xC0 && bytes[j + 3] == 0x05)
            .map(|j| usize::from(img.word(j + 4).unwrap_or(0)))
            .ok_or("the record address is not built the way the entry function builds it")?;
        Ok(base + disp)
    }

    /// The global table, found by shape rather than by address.
    ///
    /// The installer sits at offset `0Ch` of its own segment, so finding
    /// its prologue gives the segment; the one far call to it is preceded
    /// by the two `push imm16` that make up the table's far pointer.
    /// Nothing here is an address this build happens to have.
    pub(crate) fn read_table(img: &Image) -> Result<Vec<Rule>> {
        let segment = Self::installer(img)?;
        let bytes = img.bytes();
        let [lo, hi] = segment.to_le_bytes();
        let call = [0x9A, 0x0C, 0x00, lo, hi];
        let site = (6..bytes.len().saturating_sub(call.len()))
            .find(|&i| {
                bytes[i..i + call.len()] == call && bytes[i - 6] == 0x68 && bytes[i - 3] == 0x68
            })
            .ok_or("nothing hands the interaction table to its installer")?;
        let seg = usize::from(img.word(site - 5)?);
        let off = usize::from(img.word(site - 2)?);
        Self::read_row(img, seg * 16 + off)
    }

    /// Every rule of the row at image offset `at`.
    ///
    /// The row ends at the first zero phase, which is the same test
    /// `0x13E41` opens with. A row that runs past the image is an error
    /// rather than a truncated answer.
    pub(crate) fn read_row(img: &Image, at: usize) -> Result<Vec<Rule>> {
        let mut out = Vec::new();
        loop {
            let r = at + out.len() * Self::SIZE;
            let phase = img.word(r + 4)?;
            if phase == 0 {
                return Ok(out);
            }
            out.push(Rule {
                verb: img.word(r + 2)?,
                phase,
                object: num::signed(img.word(r + 6)?),
                object2: num::signed(img.word(r + 8)?),
                handler: (img.word(r + 12)?, img.word(r + 10)?),
            });
            if out.len() > 4096 {
                return Err(format!("{at:#x}: a rule row with no end").into());
            }
        }
    }
}

impl Locations {
    /// Reads the dispatch out of `img`.
    ///
    /// Every step insists on being unambiguous. The dispatch shape must
    /// occur exactly once; every bound the program checks on the location
    /// must name the same count; every record far pointer read with the
    /// `es:[bx+n]` pair must name the same offset. A program where any of
    /// those does not hold is refused, because the alternative is picking
    /// one of the candidates and calling it the answer.
    pub(crate) fn read(img: &Image) -> Result<Self> {
        let (at, location, stride, table, handler) = find_dispatch(img)?;
        let count = find_count(img, location)?;
        let name = find_name_field(img)?;
        let (exits, destinations) = find_exit_lists(img, location)?;
        let (objects, object_slots) = find_object_list(img)?;
        let mut me = Locations {
            at,
            location,
            table,
            stride,
            handler,
            name,
            count,
            names: Vec::new(),
            kind: find_kind_field(img, location, stride, table)?,
            exits,
            destinations,
            objects,
            object_slots,
            rules: Rule::location_field(img)?,
        };
        me.names = (0..count).map(|n| me.read_name(img, n)).collect();
        Ok(me)
    }

    /// The image offset of location `n`'s handler, or `None` when the row
    /// does not name one inside the image.
    pub(crate) fn handler(&self, img: &Image, n: usize) -> Option<usize> {
        self.far(img, n, self.handler)
            .filter(|&p| p != 0 && p < img.bytes().len())
    }

    /// Location `n`'s own interaction rules.
    ///
    /// The entry function installs these before the global ones, and the
    /// search reaches them first, so a location can answer for a verb the
    /// global table would otherwise have. A record that names no table
    /// answers with an empty list rather than an error: locations without
    /// one are ordinary.
    pub(crate) fn rules(&self, img: &Image, n: usize) -> Vec<Rule> {
        self.far(img, n, self.rules)
            .filter(|&p| p != 0 && p < img.bytes().len())
            .and_then(|p| Rule::read_row(img, p).ok())
            .unwrap_or_default()
    }

    /// What location `n` calls itself, when its record names anything.
    pub(crate) fn name(&self, n: usize) -> Option<&str> {
        self.names.get(n).and_then(Option::as_deref)
    }

    fn read_name(&self, img: &Image, n: usize) -> Option<String> {
        let ptr = self.far(img, n, self.name).filter(|&p| p != 0)?;
        img.c_string(ptr).ok()
    }

    /// What kind of location row `n` is. Zero is "not a place".
    pub(crate) fn kind(&self, img: &Image, n: usize) -> Option<u16> {
        let field = self.kind;
        img.word(usize::from(self.table) * 16 + n * self.stride + field)
            .ok()
    }

    /// The exits location `n` has, as `(id, destination)`.
    ///
    /// Both lists are bytes inside the record and they are parallel, so
    /// the gap between where each starts is how many slots each has. A
    /// zero id is an unused slot; a destination outside the table is a
    /// row that is not a location, and both are dropped.
    pub(crate) fn exits(&self, img: &Image, n: usize) -> Vec<(u8, u8)> {
        let width = self.destinations.saturating_sub(self.exits);
        let base = usize::from(self.table) * 16 + n * self.stride;
        let b = img.bytes();
        (0..width)
            .filter_map(|i| {
                let id = *b.get(base + self.exits + i)?;
                let dest = *b.get(base + self.destinations + i)?;
                (id != 0 && usize::from(dest) < self.count).then_some((id, dest))
            })
            .collect()
    }

    /// The objects location `n` lists. A zero slot is an unused one.
    pub(crate) fn objects(&self, img: &Image, n: usize) -> Vec<u8> {
        let base = usize::from(self.table) * 16 + n * self.stride + self.objects;
        let b = img.bytes();
        (0..self.object_slots)
            .filter_map(|i| b.get(base + i).copied().filter(|&v| v != 0))
            .collect()
    }

    /// The same list with its holes left in.
    ///
    /// The entry function's loop does not stop at a zero and does not
    /// close up the gap: it takes the "clear this slot" arm and carries on
    /// counting (`0x0EE0C` jumps to `0x0EE48`, which rejoins the loop at
    /// `0x0EE53`). So position `i` names slot `i + 1` whether or not the
    /// positions before it were filled, and a reader that compacted the
    /// list would put the objects in the wrong slots.
    pub(crate) fn object_ids(&self, img: &Image, n: usize) -> Vec<u8> {
        let base = usize::from(self.table) * 16 + n * self.stride + self.objects;
        let b = img.bytes();
        (0..self.object_slots)
            .map(|i| b.get(base + i).copied().unwrap_or(0))
            .collect()
    }

    /// The far pointer at `field` inside location `n`'s record.
    fn far(&self, img: &Image, n: usize, field: usize) -> Option<usize> {
        let at = usize::from(self.table) * 16 + n * self.stride + field;
        let off = usize::from(img.word(at).ok()?);
        let seg = usize::from(img.word(at + 2).ok()?);
        Some(seg * 16 + off)
    }
}

/// The one place the program dispatches on a location.
fn find_dispatch(img: &Image) -> Result<(usize, u16, usize, u16, usize)> {
    let b = img.bytes();
    let mut found = Vec::new();
    for i in 0..b.len().saturating_sub(17) {
        let shape = b[i..i + 2] == MOV_BX_MEM
            && b[i + 4..i + 6] == IMUL_BX_IMM8
            && b[i + 7] == MOV_AX_IMM
            && b[i + 10..i + 12] == MOV_ES_AX
            && b[i + 12..i + 15] == CALL_FAR_ES_BX;
        if shape {
            found.push((
                i,
                u16::from_le_bytes([b[i + 2], b[i + 3]]),
                usize::from(b[i + 6]),
                u16::from_le_bytes([b[i + 8], b[i + 9]]),
                usize::from(u16::from_le_bytes([b[i + 15], b[i + 16]])),
            ));
        }
    }
    match found.len() {
        1 => Ok(found[0]),
        0 => Err("no parsec1994 location dispatch in this program".into()),
        n => Err(format!("{n} places dispatch on a location; which one is the main loop is not decidable from the shape alone").into()),
    }
}

/// How many locations there are, from every bound the program checks.
///
/// The count is not stored anywhere as data — the compiler knew it. What
/// the program does say is `cmp word [location], n` followed by `jge`,
/// which is the guard before a dispatch, and it says it at every site that
/// dispatches. So the count is read from those, and disagreement between
/// them is an error rather than a vote.
///
/// `cmp word [location], 0` followed by `jge` is the same two instructions
/// and a different question — it is how the program tells a location from
/// one of the negative values it uses for a transition out of play. A
/// table of no locations is not a thing, so zero is skipped rather than
/// counted as a disagreement.
fn find_count(img: &Image, location: u16) -> Result<usize> {
    let b = img.bytes();
    let want = location.to_le_bytes();
    let mut seen: Option<usize> = None;
    for i in 0..b.len().saturating_sub(6) {
        if b[i..i + 2] != CMP_MEM_IMM8 || b[i + 2..i + 4] != want || b[i + 5] != JGE {
            continue;
        }
        let n = usize::from(b[i + 4]);
        if n == 0 {
            continue;
        }
        match seen {
            None => seen = Some(n),
            Some(m) if m == n => {}
            Some(m) => {
                return Err(format!(
                    "the program bounds its location at both {m} and {n}; \
                     which is the table's length is not decidable"
                )
                .into());
            }
        }
    }
    seen.ok_or_else(|| "the program never bounds its location, so the table has no length".into())
}

/// Where a record keeps the word that says what kind of location it is.
///
/// Asked for by the guard in front of the dispatch, which is the same
/// twelve bytes as the dispatch itself and then a different question:
/// `0x04F95`–`0x04F9F` load `[1F4Ah]`, multiply it by the stride 34h and point
/// `es` at the table segment `204Bh`, and `0x04FA1` compares the word
/// `es:[bx+7]` with zero.
///
/// Tying it to the stride and the table is what keeps it from matching
/// some other structure's field — `cmp word es:[bx+n], 0` on its own is a
/// common enough instruction to name several.
fn find_kind_field(img: &Image, location: u16, stride: usize, table: u16) -> Result<usize> {
    let b = img.bytes();
    let loc = location.to_le_bytes();
    let tab = table.to_le_bytes();
    let mut seen: Option<usize> = None;
    for i in 0..b.len().saturating_sub(18) {
        let shape = b[i..i + 2] == MOV_BX_MEM
            && b[i + 2..i + 4] == loc
            && b[i + 4..i + 6] == IMUL_BX_IMM8
            && usize::from(b[i + 6]) == stride
            && b[i + 7] == MOV_AX_IMM
            && b[i + 8..i + 10] == tab
            && b[i + 10..i + 12] == MOV_ES_AX
            && b[i + 12..i + 15] == CMP_ES_BX_IMM8
            && b[i + 17] == 0;
        if !shape {
            continue;
        }
        let n = usize::from(u16::from_le_bytes([b[i + 15], b[i + 16]]));
        match seen {
            None => seen = Some(n),
            Some(m) if m == n => {}
            Some(m) => {
                return Err(format!(
                    "the guard asks rows about both +{m:#x} and +{n:#x}; \
                     which one is the kind is not decidable"
                )
                .into());
            }
        }
    }
    seen.ok_or_else(|| "the program never asks a row what kind of location it is".into())
}

/// Where a record keeps its exits, as `(ids, destinations)`.
///
/// A location's exits are not compiled into its handler — they are two
/// parallel byte lists in the record, and the
/// main loop resolves them itself. When the handler leaves the location
/// alone, the loop looks the entry id up in the current row's list and
/// takes the destination beside it:
/// `0x05436` reads an exit id from `es:[bx+9]` and `0x0543D` compares it with
/// the one being taken in `cx`; on a match `0x05471` reads the destination
/// from `es:[bx+11h]` and `0x05478` stores it in `[1F4Ah]`.
///
/// That is Dunkle Schatten's, with eight exit slots. The shape occurs
/// exactly once in the program, which is what makes it safe to look for
/// rather than to be told.
fn find_exit_lists(img: &Image, location: u16) -> Result<(usize, usize)> {
    let b = img.bytes();
    let loc = location.to_le_bytes();
    let mut found = Vec::new();
    for i in 0..b.len().saturating_sub(9) {
        let scan = b[i..i + 3] == MOV_AL_ES_BX
            && b[i + 5..i + 7] == MOV_AH_0
            && b[i + 7..i + 9] == CMP_AX_CX;
        if !scan {
            continue;
        }
        let ids = usize::from(u16::from_le_bytes([b[i + 3], b[i + 4]]));
        // The destination read follows within the same branch: the same
        // three instructions again, and then a store into the location.
        let hit = (i + 9..(i + 64).min(b.len().saturating_sub(11))).find_map(|j| {
            let shape = b[j..j + 3] == MOV_AL_ES_BX
                && b[j + 5..j + 7] == MOV_AH_0
                && b[j + 7] == MOV_MEM_AX
                && b[j + 8..j + 10] == loc;
            shape.then(|| usize::from(u16::from_le_bytes([b[j + 3], b[j + 4]])))
        });
        if let Some(dests) = hit.filter(|&d| d > ids) {
            found.push((ids, dests));
        }
    }
    found.dedup();
    match found.len() {
        1 => Ok(found[0]),
        0 => Err("the program never looks an exit up in a record".into()),
        n => {
            Err(format!("{n} places look an exit up; which is the loop's is not decidable").into())
        }
    }
}

/// Where a record lists the objects its location holds, and how many.
///
/// The entry function biases a record's address by seven and then walks
/// the list a byte at a time, asking each slot whether it is there:
/// `0x0EE01` reads a slot from `es:[bx+12h]`, which is record `+19h`;
/// `0x0EE0A`–`0x0EE0C` skip an empty one; `0x0EE53` closes the loop
/// with `cmp di, 9`.
///
/// Dunkle Schatten counts in `di`, and the bound is taken from the
/// `cmp di, imm8` that closes the loop: nine slots, and the list starts
/// immediately after the destinations.
fn find_object_list(img: &Image) -> Result<(usize, usize)> {
    let b = img.bytes();
    let mut found = Vec::new();
    for i in 0..b.len().saturating_sub(12) {
        let loop_head = b[i..i + 3] == MOV_AL_ES_BX8
            && b[i + 4..i + 6] == MOV_AH_0
            && b[i + 6..i + 8] == [0x89, 0x46]
            && b[i + 9..i + 11] == [0x0B, 0xC0]
            && b[i + 11] == 0x74;
        if !loop_head {
            continue;
        }
        let at = usize::from(b[i + 3]) + ENTRY_BIAS;
        // The bound closes the loop within the next hundred and forty
        // bytes: `cmp di, imm8` / `jl`.
        let bound = (i + 12..(i + 152).min(b.len().saturating_sub(4))).find_map(|j| {
            let idx = b[j] == 0x83 && b[j + 1] == 0xFF && b[j + 3] == 0x7C;
            idx.then(|| usize::from(b[j + 2]))
        });
        if let Some(n) = bound.filter(|&n| n > 0) {
            found.push((at, n));
        }
    }
    found.dedup();
    match found.len() {
        1 => Ok(found[0]),
        0 => Err("the program never walks a record's object list".into()),
        n => Err(format!(
            "{n} places walk a byte list in a record; which is the objects is not decidable"
        )
        .into()),
    }
}

/// Where a record keeps a far pointer besides the handler.
///
/// A record's other far pointer is taken with `mov ax, es:[bx+n+2]` /
/// `mov dx, es:[bx+n]`, and the status line uses it for the location's
/// name. Sites that name offset zero are a different read — a pointer at
/// the head of some other structure — and are left out; every site that
/// names a real offset must agree.
fn find_name_field(img: &Image) -> Result<usize> {
    let b = img.bytes();
    let mut seen: Option<usize> = None;
    for i in 0..b.len().saturating_sub(10) {
        if b[i..i + 3] != MOV_AX_ES_BX || b[i + 5..i + 8] != MOV_DX_ES_BX {
            continue;
        }
        let hi = usize::from(u16::from_le_bytes([b[i + 3], b[i + 4]]));
        let lo = usize::from(u16::from_le_bytes([b[i + 8], b[i + 9]]));
        if hi != lo + 2 || lo == 0 {
            continue;
        }
        match seen {
            None => seen = Some(lo),
            Some(m) if m == lo => {}
            Some(m) => {
                return Err(format!(
                    "records are read at both +{m:#x} and +{lo:#x}; \
                     which one is the name is not decidable"
                )
                .into());
            }
        }
    }
    seen.ok_or_else(|| "no record far pointer besides the handler is ever read".into())
}

#[cfg(test)]
mod resource_tests {
    use super::*;

    /// `0724h:01E1h`: primary colors distinguish the three weights; the
    /// target run and untouched banks retain their previous mappings.
    #[test]
    fn resource_rgb_updates_only_its_flatten_lookup_range() {
        let mut f = Flatten {
            to: 192,
            first: 192,
            count: 16,
            table: None,
        };
        f.enable_updates();
        f.load_rgb(10, &[63, 0, 0, 0, 63, 0, 0, 0, 63, 63, 63, 63, 0, 0, 0]);
        assert_eq!(
            (10..15).map(|i| f.map(i)).collect::<Vec<_>>(),
            [196, 200, 193, 207, 192]
        );
        assert_eq!(f.map(9), 192);
        assert_eq!(f.map(15), 192);
        for i in 192..208 {
            assert_eq!(f.map(i), i);
        }
        f.load_rgb(11, &[0, 0, 0]);
        assert_eq!(f.map(11), 192);
        assert_eq!(f.map(10), 196);
        assert_eq!(f.map(12), 193);
    }

    /// The byte mapping read from `DS:04CBh`. A repeated entry is an
    /// intentional cel alias.
    #[test]
    fn inventory_cels_follow_the_programs_mapping_including_aliases() {
        let expected: &[u8] = &[
            0, 1, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let Some(dir) = motionvm_parsec_testutil::gamedata_ds1() else {
            eprintln!("skipping: no Dunkle Schatten gamedata directory");
            return;
        };
        let img = Image::open_any(&dir, "XP01.EXE").unwrap();
        let count = item_names(&img).unwrap().len();
        assert_eq!(count, expected.len());
        assert_eq!(
            item_resources(&img, count).unwrap(),
            expected
                .iter()
                .map(|&n| usize::from(n) + 2)
                .collect::<Vec<_>>()
        );
    }
}
