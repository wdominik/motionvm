//! Where *Dunkle Schatten*'s own `XP01.EXE` keeps what the runtime needs.
//!
//! Every address here is this program's. How an address turns
//! into a string, a far pointer or a checked instruction immediate is
//! [`crate::exe`]; what each one holds is read out of this program's
//! code, and each is taken from the image through an opcode check, so a
//! drifted address fails the load instead of yielding a plausible wrong
//! number.
//!
//! The location dispatch itself is not here: it is the parsec1994 generation's, and
//! [`crate::titles::xp1994`] reads it out of the program it is handed.
//! What is here is where a fresh game starts, and the row of status-line
//! templates.

use crate::num;
use std::path::Path;

use crate::exe::Image;
use crate::titles::xp1994::Locations;

use crate::error::Result;
#[cfg(test)]
use crate::exe::op::MOV_BX_MEM;
use crate::exe::op::PUSH_BX_MEM;

/// `mov word [imm16], imm16` — a store of a constant into a global.
const MOV_MEM_IMM: [u8; 2] = [0xC7, 0x06];

/// `mov dx, [bx+imm16]` — the low word of a far pointer out of a row.
#[cfg(test)]
const MOV_DX_MEM_BX: [u8; 2] = [0x8B, 0x97];
/// `mov [imm16], dx` — a far pointer's low word stored into a row.
const MOV_MEM_DX: [u8; 2] = [0x89, 0x16];

/// `mov word [1F4Ah], 2` — the location a fresh game opens in, taken when
/// what the loader read left the word at zero (`0x4F2A`).
const START_LOC: usize = 0x4F37;
/// The `mov word [1F48h], 7Fh` beside it.
const START_ENTRY: usize = 0x4F3D;

/// `mov dx, [bx+1060h]` — the row of names a wrong number on the
/// Wohnzimmer's telephone asks for.
///
/// The instruction is inside the Wohnzimmer's handler (`0100:0073`):
/// `0x117D` counts the row, `0x1189` draws an index into it out of the
/// generator, and this instruction reads the drawn entry. Substitution 0
/// of a dialogue then stands for whoever was drawn — *Nein, hier ist
/// bestimmt nicht «0»*, scene 12 of that location. No other site in the
/// program reads the row.
#[cfg(test)]
const WRONG_NUMBERS: usize = 0x119A;
/// `mov [1689h], dx` — where the drawn name lands: entry 0 of the
/// substitution row a `02 n` control reads (`13A7h:0023h` is handed
/// `DS:1689h` at `0x0D93A`). Entry 1 is the word the program keeps there
/// from the start.
const SUBST_ROW: usize = 0x11A1;
/// Two entries; the row is what `02 00` and `02 01` reach.
const SUBSTITUTIONS: usize = 2;

/// The `mov bx, [288Dh]` that reads the verb before a status line is set.
#[cfg(test)]
const VERB_INDEX: usize = 0x10124;
/// The `push word [bx+1084h]` that pushes the template's offset. The
/// segment comes from the `push word [bx+1086h]` six bytes earlier, and
/// the two together are one far pointer, so the row starts at the lower.
const VERB_ROW: usize = 0x1012F;
/// How many far pointers the row holds. Counted in the image: entry 15 is
/// the last that points at a string, and it repeats entry 14's `%c??? …`
/// the way a fixed-size array's unused tail does.
pub(super) const VERB_TEMPLATES: usize = 16;

/// What the runtime reads out of this title's programs.
///
/// The three fields under `cfg(test)` are read for the tests in the title
/// module alone, which pin them against the program; nothing at run time
/// asks for them.
pub(crate) struct ExeText {
    /// The two panel variants, each channel's original `(start, divisor)`.
    pub panel_curves: crate::titles::palette1994::PanelCurves,
    /// `0590:18EF` restores this transformed gray ramp after room entry.
    pub gray_ramp: (u8, Vec<u8>),
    /// The global holding the entry id (`1F48h`).
    #[cfg(test)]
    pub entry: u16,
    /// Where a fresh game starts: the location and the entry id it opens
    /// with.
    pub start: (u16, u16),
    /// The carried list shipped at `DS:1602h`, counted by `DS:15FEh`.
    pub initial_items: Vec<u8>,
    /// The initial first visible inventory position (`DS:15FFh`).
    pub initial_scroll: usize,
    /// Where the carried list lives: the count byte, the scroll byte and
    /// the list's first byte, as the instructions that read them name them.
    pub inventory_at: (u16, u16, u16),
    /// Where the row of status-line templates starts, in `[288Dh]` order.
    /// Index 0 is the empty one; 1 is `%cBetrachte %c%s`. Taken out of
    /// the `push word [bx+1084h]` that indexes it, not written down.
    pub template_row: usize,
    /// The global holding the current verb (`288Dh`).
    #[cfg(test)]
    pub verb_index: u16,
    /// Whom a wrong number asks for, in row order.
    ///
    /// The Wohnzimmer's handler draws one of these before a rolled
    /// telephone scene, and the scene that names the caller's party
    /// carries a `02 00` substitution rather than the words.
    #[cfg(test)]
    pub wrong_numbers: Vec<String>,
    /// The substitution row as the image ships it: entry 0 before the
    /// draw, entry 1 the fixed word.
    pub substitutions: Vec<String>,
}

impl ExeText {
    /// Reads this title's program.
    pub(super) fn read(dir: &Path) -> Result<Self> {
        let xp01 = Image::open(dir, "XP01.EXE", 150_096)?;
        let locations = Locations::read(&xp01)?;

        let start_location = store(&xp01, START_LOC, locations.location)?;
        let entry = xp01.imm16(START_ENTRY, &MOV_MEM_IMM, "`mov word [imm16], imm16`")?;
        let start_entry = xp01.word(START_ENTRY + 4)?;
        // The three globals sit together, and the pair beside the start is
        // what says which is which. A build that put them elsewhere would
        // fail here rather than start somewhere plausible.
        if entry + 2 != locations.location {
            return Err(format!(
                "the entry global is {entry:04X}h and the location {:04X}h, \
                 which are not the adjacent pair the parsec1994 generation keeps",
                locations.location
            )
            .into());
        }

        let template_row =
            usize::from(xp01.imm16(VERB_ROW, &PUSH_BX_MEM, "`push word [bx+imm16]`")?);
        let row = usize::from(xp01.imm16(SUBST_ROW, &MOV_MEM_DX, "`mov [imm16], dx`")?);
        let substitutions = xp01.ptr_table(xp01.at(row), SUBSTITUTIONS)?;

        // `0x0CD67` reads the carried count, `0x0CD71` indexes its list;
        // `0x0CABC` reads the scroll position before inventory hit-testing.
        // These are initialized data, not zero-filled globals. In Dunkle Schatten
        // the shipped list contains item 19 before any location script runs.
        let count_at = xp01.imm16(0x0CD67, &[0xA0], "mov al,[inventory count]")?;
        let list_at = xp01.imm16(0x0CD71, &[0x8A, 0x87], "mov al,[bx+inventory]")?;
        let scroll_at = xp01.imm16(0x0CABC, &[0xA0], "mov al,[inventory scroll]")?;
        let read_byte = |off: u16| -> Result<u8> {
            xp01.bytes()
                .get(xp01.at(usize::from(off)))
                .copied()
                .ok_or_else(|| format!("inventory byte DS:{off:04X} out of range").into())
        };
        let count = usize::from(read_byte(count_at)?);
        let start = xp01.at(usize::from(list_at));
        let initial_items = xp01
            .bytes()
            .get(start..start + count)
            .ok_or("initial inventory out of range")?
            .to_vec();
        let initial_scroll = usize::from(read_byte(scroll_at)?);

        let panel_curves = panel_curves(&xp01)?;

        Ok(ExeText {
            panel_curves,
            gray_ramp: gray_ramp(&xp01)?,
            #[cfg(test)]
            entry,
            start: (start_location, start_entry),
            initial_items,
            initial_scroll,
            inventory_at: (count_at, scroll_at, list_at),
            template_row,
            #[cfg(test)]
            verb_index: xp01.imm16(VERB_INDEX, &MOV_BX_MEM, "`mov bx, [imm16]`")?,
            #[cfg(test)]
            wrong_numbers: xp01.ptr_list(xp01.at(usize::from(xp01.imm16(
                WRONG_NUMBERS,
                &MOV_DX_MEM_BX,
                "`mov dx, [bx+imm16]`",
            )?)))?,
            substitutions,
        })
    }
}

/// The immediate of a `mov word [global], imm16` that stores into `global`.
///
/// Both halves are checked: the opcode and modrm say it is a store of a
/// constant, and the destination says it is a store into *this* global. A
/// site that drifted onto another variable fails rather than answering with
/// that variable's constant.
fn store(img: &Image, at: usize, global: u16) -> Result<u16> {
    let dest = img.imm16(at, &MOV_MEM_IMM, "`mov word [imm16], imm16`")?;
    if dest != global {
        return Err(format!("{at:#x} stores into {dest:04X}h, not {global:04X}h").into());
    }
    img.word(at + 4)
}

/// `XP01.EXE`'s `0x059A3..0x059C5` builds channel rows 1..3; the panel loader
/// at `0x0C332` and `0x0C354` selects rows separately for B, G, R.
/// Row zero uses a mutable setting and is not silently treated as identity.
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
    let mut rows = [[0; 2]; 4];
    for (slot, at) in [(1, 0x059A3), (2, 0x059B1), (3, 0x059BF)] {
        if push(at + 4)? != num::byte(slot)
            || img.bytes().get(at + 6..at + 11) != Some(&[0x9A, 0x60, 3, 0x24, 7])
        {
            return Err("unexpected panel channel initialization".into());
        }
        rows[slot] = [push(at + 2)?, push(at)?];
    }
    let mut variants = [[[0; 2]; 3]; 2];
    for (variant, at) in [(0, 0x0C332), (1, 0x0C354)] {
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

/// `0x071EF..0x07236` builds RGB values backwards from 63 in steps of
/// four, applies channel row zero, and writes 16 entries at DAC 192.
/// The room-entry completion at `0x0EC84` calls this routine after effects.
fn gray_ramp(img: &Image) -> Result<(u8, Vec<u8>)> {
    let b = img.bytes();
    let setting = img.imm16(0x05993, &[0xFF, 0x36], "push channel-zero divisor")?;
    let divisor = u8::try_from(img.word(img.at(usize::from(setting)))?)?;
    if b.get(0x05997..0x059A0) != Some(&[0x6A, 0, 0x6A, 0, 0x9A, 0x60, 3, 0x24, 7])
        || b.get(0x07208..0x0720B) != Some(&[0x83, 0xEA, 4])
        || b.get(0x07223..0x07225) != Some(&[0x6A, 16])
        || b.get(0x0721B..0x07220) != Some(&[0x9A, 0xAF, 3, 0x24, 7])
    {
        return Err("unexpected gray-ramp builder".into());
    }
    let peak = img.imm16(0x071F7, &[0xBA], "mov dx,gray-ramp peak")?;
    let start = u8::try_from(img.imm16(0x07225, &[0x68], "push gray-ramp start")?)?;
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
