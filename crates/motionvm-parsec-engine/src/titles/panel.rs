//! The strip's widgets: what the pointer can hit below the playfield.
//!
//! The program keeps one table of seventeen 18-byte records — the same
//! `MouseObject` shape a room's hotspot table carries in its own `+8`
//! array — laid out for the strip rather than for the playfield. Dunkle
//! Schatten's sits at `2115:0000`, the same segment its status line is
//! prepared in.
//!
//! What the table says:
//!
//! | | verb buttons | their ids in reading order | slots | arrows |
//! |---|---|---|---:|---|
//! | Dunkle Schatten | 6 × 56×22 in two rows | `103h`, `104h`, `102h`, `106h`, `105h`, `101h` | 8 | `115h`, `116h` |
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

/// How many records the panel table holds: seventeen.
const RECORDS: usize = 17;

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
/// it. What a click on one of *those* does is the panel's mode machine,
/// which is unread.
///
/// Dunkle Schatten's buttons start at `101h`, as its status rows do: it
/// indexes sixteen templates from one, with an empty slot at zero.
const VERBS: std::ops::RangeInclusive<u16> = 0x100..=0x10F;

/// The id range the inventory slots live in. `0x0CAB6` takes anything
/// from `11Eh` up as a slot and `0x0CAA1` stops at `125h`.
const SLOTS: std::ops::RangeInclusive<u16> = 0x11E..=0x125;

/// The game's strip.
pub(crate) struct Panel {
    /// The widgets, in table order.
    pub widgets: Vec<MouseObject>,
}

impl Panel {
    /// Finds the strip's table in `img`.
    ///
    /// By shape: seventeen records whose first covers the full width at
    /// the row the strip starts on and carries id `114h`. Requiring one
    /// match and no more is what keeps this from picking up a room's
    /// hotspot widgets, which have the same record shape.
    pub(crate) fn read(img: &Image) -> Result<Self> {
        let b = img.bytes();
        let at = Self::find(b)?;
        let widgets = (0..RECORDS)
            .map(|i| MouseObject::parse(b, at + i * STRIDE))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(Panel { widgets })
    }

    /// Where the strip's table sits in `b`, by the shape [`Self::read`]
    /// describes.
    fn find(b: &[u8]) -> Result<usize> {
        let mut found = Vec::new();
        for a in 0..b.len().saturating_sub(STRIDE * RECORDS) {
            if word(b, a + 0x10) != STRIP_ID || word(b, a + 4) & 1 != 1 || word(b, a + 8) != 0 {
                continue;
            }
            let full_width = word(b, a + 0x0C) == 320;
            let rows = word(b, a + 0x0E);
            if !full_width || rows == 0 || word(b, a + 0x0A) + rows != 200 {
                continue;
            }
            found.push(a);
        }
        match found.len() {
            1 => Ok(found[0]),
            0 => Err("no strip widget table in this program".into()),
            n => Err(
                format!("{n} tables claim the strip; which is the panel's is not decidable").into(),
            ),
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
    /// The slots are `11Eh` upwards and `0x0CAA1` bounds them at `125h`,
    /// so there are eight — two rows of four, as the drawing loop's two
    /// rows say.
    pub(crate) fn slot_of(w: &MouseObject) -> Option<usize> {
        SLOTS
            .contains(&w.id)
            .then(|| usize::from(w.id - *SLOTS.start()))
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

    /// The strip is found by its shape in Dunkle Schatten's `XP01.EXE`.
    #[test]
    fn the_strip_is_laid_out_the_way_the_program_says() {
        let Some(dir) = motionvm_parsec_testutil::gamedata_ds1() else {
            eprintln!("skipping: no Dunkle Schatten gamedata directory");
            return;
        };
        let img = Image::open_any(&dir, "XP01.EXE").expect("Dunkle Schatten");
        let p = Panel::read(&img).expect("Dunkle Schatten");
        assert_eq!(Panel::find(img.bytes()).expect("Dunkle Schatten"), 0x21150);
        assert_eq!(p.widgets.len(), RECORDS);
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
