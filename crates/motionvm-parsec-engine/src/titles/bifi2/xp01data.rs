//! Where *Bi-Fi 2*'s own `XP01.EXE` keeps what the runtime needs.
//!
//! Every value here is taken from the image through an opcode check, so a
//! drifted address fails the load instead of yielding a plausible wrong
//! number: where a fresh game starts, the row of status-line templates,
//! the substitution row and the carried list.

use crate::error::Result;
use crate::exe::Image;
use crate::exe::op::{PUSH_BX_MEM, PUSH_IMM16};
use crate::titles::xp1994::Locations;

/// `push word [bx+116Ch] ; push word [bx+116Ah]` after `mov bx, [2D38h] ;
/// shl bx, 2` (`0x0B02D`–`0x0B038`): the template's far pointer, the row
/// starting at the lower word.
const VERB_ROW: usize = 0x0B038;

/// `push ds ; push 1768h ; call far 0E17h:002Dh` (`0x08972`, and alike at
/// `0x08C49`, `0x0985F`, `0x098AF`): the row a `02 n` control reads.
const SUBST_ROW: usize = 0x08973;
/// The row holds one far pointer: the word after it, `[176Ch]`, is read
/// on its own (`mov ax, [176Ch]`, `0x08F76`).
pub(super) const SUBSTITUTIONS: usize = 1;

/// `mov al, [1662h]` (`0x07E33`), the carried count the item routines
/// read; `mov al, [bx+1664h]` (`0x07E3E`), the list it counts; `mov al,
/// [1663h]` (`0x07BD8`), the first visible slot the strip's hit test reads.
const INVENTORY_COUNT: usize = 0x07E33;
const INVENTORY_LIST: usize = 0x07E3E;
const INVENTORY_SCROLL: usize = 0x07BD8;

/// The channel rows the start builds, each `push divisor ; push start ;
/// push row ; call far` (`0x010E3`, `0x010F1`, `0x010FF`), after row zero's
/// `push word [2BAh] ; push 0 ; push 0 ; call far 02AFh:01A0h` (`0x010D3`).
const ROW_ZERO: usize = 0x010D3;
const CHANNEL_ROWS: [(u8, usize); 3] = [(1, 0x010E3), (2, 0x010F1), (3, 0x010FF)];
/// The panel loader's two variants, `push b ; push g ; push r` of row
/// numbers (`0x07482`, `0x074AC`).
const PANEL_VARIANTS: [usize; 2] = [0x07482, 0x074AC];
/// `add ax, 80h` (`0x0744A`): the variants' first DAC entry is the panel
/// bank's plus `80h`.
const PANEL_UPPER: usize = 0x0744A;

/// `les bx, [2CEAh] ; push word es:[bx+2] ; push word es:[bx] ; push 8Ch ;
/// push 0 ; call far 1B72h:0048h` (`0x07614`–`0x07624`), in the status
/// painter `0751h:0067h`: the strip's 60-row picture goes on row 8Ch,
/// below the twelve rows of the band, which the painter fills with 0
/// (`push 0 ; push 0Ch ; push 140h ; push 80h ; push 0 ; call far
/// 1B72h:00D2h`, `0x0780A`–`0x07816`).
const STRIP_ROW: usize = 0x0761F;

/// The entry pass `0F48h:0260h` loads each of the room's banks through the
/// palette loader, and blacks the first color of one that starts at
/// register 0 before it does (`0x0F980`–`0x0F990`), then `call far
/// 0104h:090Eh` (`0x0F99B`). The Kaffee's bank, for one, starts at
/// register 0 with `(0, 63, 0)` there; loaded this way, the band the
/// status painter fills with 0 (`0x0780A`) and its darker copy at 80h show
/// black.
const ROOM_BLACK_ZERO: usize = 0x0F980;

/// What the runtime reads out of this title's program.
pub(crate) struct ExeText {
    /// Where a fresh game starts: the walk on's answer from the row the
    /// image's location word holds.
    pub start: (u16, u16),
    /// The carried list as shipped.
    pub initial_items: Vec<u8>,
    /// The first visible slot as shipped.
    pub initial_scroll: usize,
    /// The count byte, the scroll byte and the list's first byte. Unlike
    /// Dunkle Schatten's (`[15FEh]`, `[15FFh]`, `[1602h]`) there are no two
    /// range bytes between the scroll byte and the list: Bi-Fi's strip
    /// loader keeps no range (`0x073C7`–`0x07476`).
    pub inventory_at: (u16, u16, u16),
    /// Where the row of status-line templates starts, and how many far
    /// pointers into `DGROUP` it holds.
    pub templates: (usize, usize),
    /// The substitution row as the image ships it.
    pub substitutions: Vec<String>,
    /// The panel's two variants' channel curves.
    pub panel_curves: crate::titles::palette1994::PanelCurves,
    /// The row the status painter blits the strip's picture on.
    pub strip_row: i32,
}

impl ExeText {
    /// Reads this title's program.
    pub(super) fn read(xp01: &Image, locations: &Locations) -> Result<Self> {
        let template_row =
            usize::from(xp01.imm16(VERB_ROW, &PUSH_BX_MEM, "`push word [bx+imm16]`")?);
        let row = usize::from(xp01.imm16(SUBST_ROW, &PUSH_IMM16, "`push imm16`")?);
        let substitutions = xp01.ptr_table(xp01.at(row), SUBSTITUTIONS)?;

        let count_at = xp01.imm16(INVENTORY_COUNT, &[0xA0], "mov al,[inventory count]")?;
        let list_at = xp01.imm16(INVENTORY_LIST, &[0x8A, 0x87], "mov al,[bx+inventory]")?;
        let scroll_at = xp01.imm16(INVENTORY_SCROLL, &[0xA0], "mov al,[inventory scroll]")?;
        let read_byte = |off: u16| -> Result<u8> {
            xp01.bytes()
                .get(xp01.at(usize::from(off)))
                .copied()
                .ok_or_else(|| format!("inventory byte DS:{off:04X} out of range").into())
        };
        let count = usize::from(read_byte(count_at)?);
        let first = xp01.at(usize::from(list_at));
        let initial_items = xp01
            .bytes()
            .get(first..first + count)
            .ok_or("initial inventory out of range")?
            .to_vec();
        let initial_scroll = usize::from(read_byte(scroll_at)?);

        if xp01.bytes().get(PANEL_UPPER..PANEL_UPPER + 3) != Some(&[0x05, 0x80, 0x00][..]) {
            return Err("the panel's variants do not go to the upper half".into());
        }
        let panel_curves = crate::titles::palette1994::read_panel_curves(
            xp01,
            ROW_ZERO,
            CHANNEL_ROWS,
            PANEL_VARIANTS,
        )?;
        let strip_row = i32::from(xp01.imm16(STRIP_ROW, &PUSH_IMM16, "the strip's row")?);
        crate::titles::parsec1994::program::check_room_black_zero(xp01, ROOM_BLACK_ZERO)?;
        Ok(ExeText {
            panel_curves,
            strip_row,
            start: start(xp01, locations)?,
            initial_items,
            initial_scroll,
            inventory_at: (count_at, scroll_at, list_at),
            templates: (template_row, templates(xp01, template_row)?),
            substitutions,
        })
    }
}

/// How many entries of the row at `DS:row` point into `DGROUP`: the row is
/// a fixed array of far pointers, and what follows it does not.
fn templates(img: &Image, row: usize) -> Result<usize> {
    let dgroup = u16::try_from(img.at(0) / 16)?;
    let mut n = 0;
    while img.word(img.at(row + 4 * n + 2))? == dgroup {
        n += 1;
    }
    if n == 0 {
        return Err("no status-line template in the row".into());
    }
    Ok(n)
}

/// Where a fresh game starts. The location word `[200Ch]` and the word
/// beside it `[200Eh]` are as the image ships them when the start's load
/// finds no saved game, and the main loop calls the walk on for the row
/// the location word names when it is of kind 0 (`0x0074B`–`0x00768`).
/// This is that walk's answer ([`super::onward::walk`]).
fn start(img: &Image, locations: &Locations) -> Result<(u16, u16)> {
    let word = |off: u16| img.word(img.at(usize::from(off)));
    let from = word(super::logic::WORDS.location)?;
    let mut kind = |n: u16| locations.kind(img, usize::from(n)).unwrap_or(0);
    let (row, exit) = super::onward::walk(&mut kind, from);
    let entry = match exit {
        None => 0,
        Some(()) => {
            let here = word(super::logic::WORDS.here)?;
            let at = super::onward::row_byte(here, super::onward::FIRST_EXIT);
            u16::from(
                *img.bytes()
                    .get(usize::from(at.seg) * 16 + usize::from(at.off))
                    .ok_or("the start's exit lies outside the program")?,
            )
        }
    };
    Ok((row, entry))
}
