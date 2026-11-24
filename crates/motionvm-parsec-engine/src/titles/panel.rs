//! The strip's widgets: what the pointer can hit below the playfield.
//!
//! The program keeps one table of 18-byte records — the same
//! `MouseObject` shape a room's hotspot table carries in its own `+8`
//! array — laid out for the strip rather than for the playfield, and
//! registers it with the pointer in one loop: `mov si, n` records from
//! `seg:0000`, each handed to `179Eh:0210h`. Dunkle Schatten's are
//! seventeen at `2115:0000` (`0x0C3F7`), the same segment its status line
//! is prepared in.
//!
//! What the table says:
//!
//! | | verb buttons | their ids in reading order | slots | arrows |
//! |---|---|---|---:|---|
//! | Dunkle Schatten | 6 × 56×22 in two rows | `103h`, `104h`, `102h`, `106h`, `105h`, `101h` | 8 | `115h`, `116h`, a row of 4 |
//! | Captain Zins | 6 × 56×22 in two rows | `101h`, `102h`, `103h`, `104h`, `105h`, `106h` | 6 | `115h`, `116h`, a row of 3 |
//!
//! Reading the table is the only way to know where a verb button is or
//! which verb it picks.
//!
//! A click on a panel widget becomes a verb by its **low byte**:
//! an id above 100h is a panel widget (`0x0FBF2`), its low byte is the verb
//! (`0x0FBFB`), and that goes to `[288Dh]` (`0x0FC1D`).
//!

use crate::num;
use motionvm_parsec_formats::hotspot::MouseObject;

use crate::exe::Image;

use crate::error::Result;

/// Bytes per record.
const STRIDE: usize = 0x12;

/// The id of the record that covers the whole strip, which is what makes
/// a table recognisable: it is the first, and no other id is `114h`.
const STRIP_ID: u16 = 0x114;

/// The id range the verb buttons live in.
///
/// Read off the tables rather than out of an instruction: the buttons are
/// the widgets whose ids fall here, and the strip (`114h`), the arrows
/// (`115h`, `116h`) and the inventory slots (`11Eh` upwards) fall outside
/// it. The arrows scroll the slots ([`Panel::arrows`]); what a click on
/// the strip itself does is the panel's mode machine, which is unread.
///
/// Dunkle Schatten's buttons start at `101h`, as its status rows do: it
/// indexes sixteen templates from one, with an empty slot at zero.
const VERBS: std::ops::RangeInclusive<u16> = 0x100..=0x10F;

/// The first inventory slot's id: `0x0CAB6` takes anything from `11Eh`
/// up as a slot. Where the slots stop is the program's
/// ([`Panel::slots`]).
const FIRST_SLOT: u16 = 0x11E;

/// The game's strip.
pub(crate) struct Panel {
    /// The widgets, in table order.
    pub widgets: Vec<MouseObject>,
    /// The inventory slots' ids, first to last.
    pub slots: std::ops::RangeInclusive<u16>,
    /// Where the carried items are drawn.
    pub grid: Grid,
    /// The two arrows beside the slots.
    pub arrows: Arrows,
}

/// The arrows that scroll the carried items through the slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Arrows {
    /// The id that scrolls back.
    pub up: u16,
    /// The id that scrolls on.
    pub down: u16,
    /// How far a click with the second button scrolls: a row.
    pub row: u8,
}

/// Which way an arrow scrolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scroll {
    /// Towards the first item.
    Up,
    /// Towards the last.
    Down,
}

/// The two rows the strip draws its carried items in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Grid {
    /// Slots in a row.
    pub columns: u16,
    /// Pixels from one column to the next.
    pub step: u16,
    /// The first column's left edge.
    pub x: u16,
    /// The two rows' top edges.
    pub rows: (u16, u16),
}

impl Grid {
    /// Where slot `n` is drawn: the remainder of `n` by the columns is the
    /// column, the quotient the row.
    pub(crate) fn origin(&self, n: usize) -> (i32, i32) {
        let cols = usize::from(self.columns.max(1));
        let col = num::count(n % cols);
        let y = if n < cols { self.rows.0 } else { self.rows.1 };
        (col * i32::from(self.step) + i32::from(self.x), i32::from(y))
    }

    /// The slots it has.
    pub(crate) fn len(&self) -> usize {
        usize::from(self.columns) * 2
    }
}

impl Panel {
    /// Reads the strip's table out of `img`.
    ///
    /// The registration loop names it: `mov si, n ; mov word [bp-2], seg ;
    /// mov word [bp-4], 0 ; jmp` and, in the loop, `push word [bp-2] ;
    /// push word [bp-4] ; call far` (`BE n 00 C7 46 FE seg C7 46 FC 00 00
    /// EB 12 FF 76 FE FF 76 FC 9A`), once in the program. Its first record
    /// must be the strip's own: id `114h`, the full width, down to the
    /// bottom. The slots end where the click's bound says, `cmp si, 100h ;
    /// jge ; jmp ; cmp si, last ; jle` (`81 FE 00 01 7D 03 E9 rel 81 FE
    /// last 7E`; `125h` at `0x0CAA1` in Dunkle Schatten).
    pub(crate) fn read(img: &Image) -> Result<Self> {
        let b = img.bytes();
        let shape = [
            0xC7, 0x46, 0xFC, 0x00, 0x00, 0xEB, 0x12, 0xFF, 0x76, 0xFE, 0xFF, 0x76, 0xFC, 0x9A,
        ];
        let hits: Vec<usize> = (0..b.len().saturating_sub(24))
            .filter(|&a| {
                b[a] == 0xBE
                    && b[a + 2] == 0
                    && b[a + 3..a + 6] == [0xC7, 0x46, 0xFE]
                    && b[a + 8..a + 22] == shape
            })
            .collect();
        let [a] = hits[..] else {
            return Err(format!("{} strip registrations in this program", hits.len()).into());
        };
        let records = usize::from(b[a + 1]);
        let at = usize::from(word(b, a + 6)) * 16;
        let widgets = (0..records)
            .map(|i| MouseObject::parse(b, at + i * STRIDE))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let strip = widgets.first().is_some_and(|w| {
            w.id == STRIP_ID && w.x == 0 && w.w == 320 && w.h != 0 && w.y + w.h == 200
        });
        if !strip {
            return Err("the strip's registered table does not open with the strip".into());
        }
        let ends: Vec<u16> = (0..b.len().saturating_sub(14))
            .filter(|&i| {
                b[i..i + 7] == [0x81, 0xFE, 0x00, 0x01, 0x7D, 0x03, 0xE9]
                    && b[i + 9..i + 11] == [0x81, 0xFE]
                    && b[i + 13] == 0x7E
            })
            .map(|i| word(b, i + 11))
            .collect();
        let [last] = ends[..] else {
            return Err(
                format!("{} bounds on the strip's slots in this program", ends.len()).into(),
            );
        };
        let grid = Self::read_grid(b)?;
        if usize::from(last - FIRST_SLOT) + 1 != grid.len() {
            return Err("the strip's slots and the rows they are drawn in disagree".into());
        }
        let arrows = Self::read_arrows(b)?;
        if u16::from(arrows.row) != grid.columns {
            return Err("the strip's arrows scroll by other than a row".into());
        }
        Ok(Panel {
            widgets,
            slots: FIRST_SLOT..=last,
            grid,
            arrows,
        })
    }

    /// The drawing loop puts slot `n` at `x + step · (n mod c)` in the
    /// first row for `n < c`, else the second: `cmp word [bp-2], c ; jge ;
    /// mov ax, y0 ; jmp ; mov ax, y1 ; push ax ; mov ax, [bp-2] ; mov bx,
    /// c ; cwd ; idiv bx` (`83 7E FE c 7D 05 B8 y0 EB 03 B8 y1 50 8B 46 FE
    /// BB c 00 99 F7 FB`), then `imul dx, dx, step` (`6B D2 step`) or `shl
    /// dx, k` (`C1 E2 k`), then `add dx, x` (`81 C2 x`). Dunkle Schatten's
    /// is at `0x0C550`: four columns 27 pixels apart from `0C8h`, rows
    /// `92h` and `0ADh`.
    fn read_grid(b: &[u8]) -> Result<Grid> {
        let w = |i: usize| word(b, i);
        let at = (0..b.len().saturating_sub(40))
            .find(|&i| {
                b[i..i + 3] == [0x83, 0x7E, 0xFE]
                    && b[i + 4..i + 7] == [0x7D, 0x05, 0xB8]
                    && b[i + 9..i + 12] == [0xEB, 0x03, 0xB8]
                    && b[i + 14..i + 18] == [0x50, 0x8B, 0x46, 0xFE]
                    && b[i + 18] == 0xBB
                    && b[i + 19] == b[i + 3]
                    && b[i + 21..i + 24] == [0x99, 0xF7, 0xFB]
            })
            .ok_or("no drawing of the strip's slots in this program")?;
        let t = &b[at + 24..];
        let (step, add) = match t {
            [0x6B, 0xD2, s, rest @ ..] => (u16::from(*s), rest),
            [0xC1, 0xE2, k, rest @ ..] => (1u16 << (k & 0x0F), rest),
            _ => return Err("the strip's slots step in an unread way".into()),
        };
        if add.get(..2) != Some(&[0x81, 0xC2][..]) {
            return Err("the strip's slots start in an unread way".into());
        }
        Ok(Grid {
            columns: u16::from(b[at + 3]),
            step,
            x: u16::from_le_bytes([add[2], add[3]]),
            rows: (w(at + 7), w(at + 12)),
        })
    }

    /// The strip's hit test sends the two arrows apart with `cmp ax, 115h ;
    /// je ; cmp ax, 116h ; je` (`3D 15 01 74 r 3D 16 01 74`), once in the
    /// program; the back arrow's arm, with the second button, moves the
    /// first visible item a row back: `cmp word [event], 6 ; jne ; add al,
    /// −row` (`83 3E lo hi 06 75 r 04 −row`). Dunkle Schatten's is at
    /// `0x0CAE3`, a row of four; Captain Zins's at `0x07FA0`, three.
    fn read_arrows(b: &[u8]) -> Result<Arrows> {
        let hits: Vec<usize> = (0..b.len().saturating_sub(10))
            .filter(|&i| b[i] == 0x3D && b[i + 3] == 0x74 && b[i + 5] == 0x3D && b[i + 8] == 0x74)
            .filter(|&i| word(b, i + 1) == 0x115 && word(b, i + 6) == 0x116)
            .collect();
        let [at] = hits[..] else {
            return Err(
                format!("{} tests of the strip's arrows in this program", hits.len()).into(),
            );
        };
        let row = b
            .get(at + 10..at + 0x40)
            .and_then(|t| {
                t.windows(9)
                    .find(|w| w[..2] == [0x83, 0x3E] && w[4..6] == [0x06, 0x75] && w[7] == 0x04)
                    .map(|w| w[8].wrapping_neg())
            })
            .ok_or("the strip's back arrow scrolls in an unread way")?;
        Ok(Arrows {
            up: word(b, at + 1),
            down: word(b, at + 6),
            row,
        })
    }

    /// Which way a widget scrolls, if it is an arrow.
    pub(crate) fn arrow_of(&self, w: &MouseObject) -> Option<Scroll> {
        if w.id == self.arrows.up {
            Some(Scroll::Up)
        } else if w.id == self.arrows.down {
            Some(Scroll::Down)
        } else {
            None
        }
    }

    /// The widget under `(x, y)`, front to back, ignoring the strip's own
    /// record — it covers everything and would answer for every point.
    pub(crate) fn hit(&self, x: i32, y: i32) -> Option<&MouseObject> {
        self.widgets.iter().rev().find(|w| {
            w.flags & 1 == 1
                && w.id != STRIP_ID
                && x >= i32::from(w.x)
                && y >= i32::from(w.y)
                && x < i32::from(w.x) + i32::from(w.w)
                && y < i32::from(w.y) + i32::from(w.h)
        })
    }

    /// The verb a widget picks, when it is one of the verb buttons.
    pub(crate) fn verb_of(w: &MouseObject) -> Option<u8> {
        VERBS.contains(&w.id).then_some(num::low8(w.id))
    }

    /// Which inventory slot a widget is, if it is one.
    ///
    /// The slots are `11Eh` upwards to the program's bound — in Dunkle
    /// Schatten `125h`, eight in two rows of four, as the drawing loop's
    /// two rows say.
    pub(crate) fn slot_of(&self, w: &MouseObject) -> Option<usize> {
        self.slots
            .contains(&w.id)
            .then(|| usize::from(w.id - FIRST_SLOT))
    }
}

fn word(b: &[u8], at: usize) -> u16 {
    b.get(at..at + 2)
        .map(|w| u16::from_le_bytes([w[0], w[1]]))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The strip is read from where Dunkle Schatten's `XP01.EXE` registers
    /// it: seventeen records at `2115:0000`, the slots up to `125h`.
    #[test]
    fn the_strip_is_laid_out_the_way_the_program_says() {
        let Some(dir) = motionvm_parsec_testutil::gamedata_ds1() else {
            eprintln!("skipping: no Dunkle Schatten gamedata directory");
            return;
        };
        let img = Image::open_any(&dir, "XP01.EXE").expect("Dunkle Schatten");
        let p = Panel::read(&img).expect("Dunkle Schatten");
        assert_eq!(p.widgets.len(), 17);
        assert_eq!(p.slots, 0x11E..=0x125);
        assert_eq!(p.widgets[0].id, STRIP_ID, "strip");
        let got: Vec<u16> = p
            .widgets
            .iter()
            .filter(|w| Panel::verb_of(w).is_some())
            .map(|w| w.id)
            .collect();
        assert_eq!(
            got,
            [0x103, 0x104, 0x102, 0x106, 0x105, 0x101],
            "verb buttons"
        );
    }

    /// A hit on a verb button answers with the verb its low byte names,
    /// and the strip's own record never answers.
    #[test]
    fn a_verb_button_names_its_verb_by_its_low_byte() {
        let Some(dir) = motionvm_parsec_testutil::gamedata_ds1() else {
            eprintln!("skipping: no Dunkle Schatten gamedata directory");
            return;
        };
        let img = Image::open_any(&dir, "XP01.EXE").expect("Dunkle Schatten");
        let p = Panel::read(&img).expect("Dunkle Schatten");
        // Dunkle Schatten's first button is at (6,146) 56×22 and is
        // `103h`, so it picks verb 3 — `%cNimm/Gib %c%s`.
        let w = p.hit(20, 155).expect("the first button");
        assert_eq!(w.id, 0x103);
        assert_eq!(Panel::verb_of(w), Some(3));
        // The bottom right one is `101h`, Betrachte.
        assert_eq!(p.hit(150, 182).and_then(Panel::verb_of), Some(1));
        // A scroll arrow is not a verb.
        assert_eq!(p.hit(310, 150).map(|w| w.id), Some(0x115));
        assert_eq!(p.hit(310, 150).and_then(Panel::verb_of), None);
        // Empty strip: the covering record does not answer.
        assert!(p.hit(160, 195).is_none());
    }
}
