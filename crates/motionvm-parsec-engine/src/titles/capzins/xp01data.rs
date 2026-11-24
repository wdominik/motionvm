//! Where *Captain Zins*'s own `XP01.EXE` keeps what the runtime needs.
//!
//! Every address here is this program's. Each value is taken from the
//! image through an opcode check, so a drifted address fails the load
//! instead of yielding a plausible wrong number.
//!
//! The location dispatch itself is the generation's
//! ([`crate::titles::xp1994`]); what is here is where a fresh game starts,
//! the row of status-line templates, the substitution row, the carried
//! list and the panel's colour rows.

use std::path::Path;

use crate::error::Result;
use crate::exe::Image;
use crate::exe::op::{PUSH_BX_MEM, PUSH_IMM16};
use crate::num;
use crate::titles::xp1994::Locations;

/// The walk on that row 0 names (`0060h:0500h`): `cmp ax, 3 ; jne`, the
/// kind it steps over beside 0 (`0x00B19`).
const WALK_SKIPS_RESTORE: usize = 0x00B19;
/// `mov [1A5Ch], dx`, the row it lands on (`0x00B4C`).
const WALK_STORES_ROW: usize = 0x00B4C;
/// `mov word [1A5Ah], 7Fh`, the entry it lands with (`0x00B50`).
const WALK_STORES_ENTRY: usize = 0x00B50;

/// `push word [bx+0D3Eh]` after `mov bx, [1E58h] ; shl bx, 2` and `push
/// word [bx+0D40h]` (`0x0B578`–`0x0B583`): the template's far pointer, the
/// row starting at the lower word.
const VERB_ROW: usize = 0x0B583;
/// How many far pointers the row holds, counted in the image as Dunkle
/// Schatten's are: entry 15 is the last that points into `DGROUP`, and it
/// repeats entry 14's `%c??? …` the way a fixed-size array's unused tail
/// does.
pub(super) const VERB_TEMPLATES: usize = 16;

/// `push ds ; push 11D2h ; call far 0EEBh:0031h` (`0x08E06`): the row a
/// `02 n` control reads.
const SUBST_ROW: usize = 0x08E07;
/// Two entries, `02 00` and `02 01`.
const SUBSTITUTIONS: usize = 2;

/// `mov al, [1138h]` (`0x0821C`), the carried count the item routines read.
const INVENTORY_COUNT: usize = 0x0821C;
/// `mov al, [bx+113Ch]` (`0x08226`), the list it counts.
const INVENTORY_LIST: usize = 0x08226;
/// `mov al, [1139h]` (`0x07F79`), the first visible slot the strip's hit
/// test reads.
const INVENTORY_SCROLL: usize = 0x07F79;

/// The channel rows the init builds, each `push divisor ; push start ;
/// push row ; call far` (`0x010F1`, `0x010FF`, `0x0110D`).
const CHANNEL_ROWS: [(u8, usize); 3] = [(1, 0x010F1), (2, 0x010FF), (3, 0x0110D)];
/// The panel loader's two variants, `push b ; push g ; push r` of row
/// numbers (`0x077EF`, `0x07811`).
const PANEL_VARIANTS: [usize; 2] = [0x077EF, 0x07811];
/// Row 0's divisor, `push word [1B0h]` (`0x010E1`), then `push 0 ; push 0 ;
/// call far` (`0x010E5`).
const ROW_ZERO: usize = 0x010E1;
/// The gray ramp's builder `0105h:18EDh` (`0x0293D`).
const GRAY_RAMP: usize = 0x0293D;

/// What the runtime reads out of this title's program.
pub(crate) struct ExeText {
    /// The two panel variants, each channel's original `(start, divisor)`.
    pub panel_curves: crate::titles::palette1994::PanelCurves,
    /// The transformed gray ramp the room entry restores, and its first
    /// DAC entry.
    pub gray_ramp: (u8, Vec<u8>),
    /// Where a fresh game starts: the walk on's answer from row 0.
    pub start: (u16, u16),
    /// The carried list shipped at `DS:113Ch`, counted by `DS:1138h`.
    pub initial_items: Vec<u8>,
    /// The first visible slot as shipped (`DS:1139h`).
    pub initial_scroll: usize,
    /// Where the carried list lives: the count byte, the scroll byte and
    /// the list's first byte, as the instructions that read them name them.
    pub inventory_at: (u16, u16, u16),
    /// Where the row of status-line templates starts, in `[1E58h]` order.
    pub template_row: usize,
    /// The substitution row as the image ships it.
    pub substitutions: Vec<String>,
}

impl ExeText {
    /// Reads this title's program.
    pub(super) fn read(dir: &Path) -> Result<Self> {
        let xp01 = Image::open(dir, "XP01.EXE", 138_752)?;
        let locations = Locations::read(&xp01)?;

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

        Ok(ExeText {
            panel_curves: panel_curves(&xp01)?,
            gray_ramp: gray_ramp(&xp01)?,
            start: start(&xp01, &locations)?,
            initial_items,
            initial_scroll,
            inventory_at: (count_at, scroll_at, list_at),
            template_row,
            substitutions,
        })
    }
}

/// Where a fresh game starts. `_main` stores no start: `[1A5Ch]` is 0 in
/// the image, the start's load (`0FBFh:007Ah`, `0x006F5`) leaves it so
/// without a saved game, and the main loop calls the walk on for row 0,
/// whose kind is 0 (`0x0075E`, `0x00768`). The walk takes the first row
/// whose kind is neither 0 nor 3 and enters it on `7Fh`. This is that
/// walk's answer, the routine's shape checked.
fn start(img: &Image, locations: &Locations) -> Result<(u16, u16)> {
    let b = img.bytes();
    if b.get(WALK_SKIPS_RESTORE..WALK_SKIPS_RESTORE + 3) != Some(&[0x3D, 3, 0])
        || img.imm16(WALK_STORES_ROW, &[0x89, 0x16], "`mov [imm16], dx`")? != locations.location
    {
        return Err("unexpected walk on at 0060h:0500h".into());
    }
    let entry_word = img.imm16(
        WALK_STORES_ENTRY,
        &[0xC7, 0x06],
        "`mov word [imm16], imm16`",
    )?;
    if entry_word.wrapping_add(2) != locations.location {
        return Err(format!(
            "the walk on stores its entry into {entry_word:04X}h, not beside the location {:04X}h",
            locations.location
        )
        .into());
    }
    let entry = img.word(WALK_STORES_ENTRY + 4)?;
    let row = (0..locations.count)
        .find(|&n| {
            locations
                .kind(img, n)
                .is_some_and(|k| k != 0 && k != u16::from(b[WALK_SKIPS_RESTORE + 1]))
        })
        .ok_or("no row a fresh game could start in")?;
    Ok((num::word(row), entry))
}

/// The channel rows 1 to 3 the init builds (`CHANNEL_ROWS`), and the two
/// panel variants that pick a row for each of B, G, R (`PANEL_VARIANTS`).
/// Row zero uses a setting of its own and is not one of them. Every row is
/// built by one far call, the one row zero's is built by.
fn panel_curves(img: &Image) -> Result<crate::titles::palette1994::PanelCurves> {
    let push = |at: usize| -> Result<u8> {
        let b = img
            .bytes()
            .get(at..at + 2)
            .ok_or("truncated palette argument")?;
        if b[0] != 0x6A || b[1] >= 0x80 {
            return Err(format!("expected positive push imm8 at {at:#x}").into());
        }
        Ok(b[1])
    };
    let builder = img.bytes().get(ROW_ZERO + 8..ROW_ZERO + 13);
    let mut rows = [[0; 2]; 4];
    for (slot, at) in CHANNEL_ROWS {
        if push(at + 4)? != slot || img.bytes().get(at + 6..at + 11) != builder {
            return Err("unexpected panel channel initialization".into());
        }
        rows[usize::from(slot)] = [push(at + 2)?, push(at)?];
    }
    let mut variants = [[[0; 2]; 3]; 2];
    for (variant, at) in PANEL_VARIANTS.into_iter().enumerate() {
        for (channel, curve) in variants[variant].iter_mut().enumerate() {
            let slot = usize::from(push(at + (2 - channel) * 2)?);
            if !(1..=3).contains(&slot) {
                return Err("unread panel channel row".into());
            }
            *curve = rows[slot];
        }
    }
    Ok(variants)
}

/// `0x0293D..0x02976` builds RGB values backwards from `3Fh` in steps of
/// four (`mov dx, 3Fh`, `0x02945`; `sub dx, 4`, `0x02956`), applies channel
/// row zero (`call far` at `0x02969`, the routine row zero is built with)
/// and writes 16 entries (`push 10h`, `0x02971`) from DAC `C0h` (`push
/// 0C0h`, `0x02973`).
fn gray_ramp(img: &Image) -> Result<(u8, Vec<u8>)> {
    let b = img.bytes();
    let setting = img.imm16(ROW_ZERO, &[0xFF, 0x36], "push channel-zero divisor")?;
    let divisor = u8::try_from(img.word(img.at(usize::from(setting)))?)?;
    let row_zero_call = b.get(ROW_ZERO + 8..ROW_ZERO + 11);
    if b.get(ROW_ZERO + 4..ROW_ZERO + 8) != Some(&[0x6A, 0, 0x6A, 0])
        || row_zero_call != Some(&[0x9A, 0x60, 0x03])
        || b.get(GRAY_RAMP + 0x19..GRAY_RAMP + 0x1C) != Some(&[0x83, 0xEA, 4])
        || b.get(GRAY_RAMP + 0x2C..GRAY_RAMP + 0x2F) != Some(&[0x9A, 0xAF, 0x03])
        || b.get(GRAY_RAMP + 0x2F..GRAY_RAMP + 0x31) != b.get(ROW_ZERO + 11..ROW_ZERO + 13)
        || b.get(GRAY_RAMP + 0x34..GRAY_RAMP + 0x36) != Some(&[0x6A, 16])
    {
        return Err("unexpected gray-ramp builder".into());
    }
    let peak = img.imm16(GRAY_RAMP + 8, &[0xBA], "mov dx,gray-ramp peak")?;
    let start = u8::try_from(img.imm16(GRAY_RAMP + 0x36, &[0x68], "push gray-ramp start")?)?;
    let table = crate::titles::palette1994::channel_curve(0, divisor)?;
    if !(60..=63).contains(&peak) || usize::from(start) + 16 > 256 {
        return Err("invalid gray-ramp range".into());
    }
    let mut rgb = Vec::with_capacity(48);
    for i in 0..16 {
        rgb.extend([table[usize::from(peak) - (15 - i) * 4]; 3]);
    }
    Ok((start, rgb))
}
