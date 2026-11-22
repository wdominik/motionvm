//! The engine's proportional font.
//!
//! Dunkle Schatten draws its text with one font, and that font is not in
//! the executable: it is resource 15 of `RESOURCE\CMBMOU.DAT`, the file the
//! pointer ships in. `XP00` sets its font record up the way `XP01` does
//! (`0x0A3D`–`0x0AF6`).
//!
//! ## The block
//!
//! | Offset | Field |
//! |---|---|
//! | `+0` | `2C 2E` in Dunkle Schatten's file; not read here |
//! | `+2` | first character code — `20h` |
//! | `+4` | glyph count — `C2h` = 194, so `20h`..`E1h`, which is exactly up to `ß` |
//! | `+6` | 7 in Dunkle Schatten's file; not read here |
//! | `+8` | the **line height**, 12 |
//! | `+0Ah` | the glyph records, eight bytes each |
//!
//! A glyph record, at `block + 0Ah + (ch − first)*8`:
//!
//! | Offset | Field |
//! |---|---|
//! | `+0` | zero when the font has no glyph for this code |
//! | `+2` | the advance in pixels |
//! | `+6` | the glyph cel, as an offset from `block + 0Ah` |
//!
//! 122 of the 194 records carry a glyph in Dunkle Schatten's file, counted.
//!
//! ## Ink is a ramp, not a color
//!
//! A glyph cel is an ordinary flags-1 cel whose literals are 0 and 1, and
//! it is painted as the ink plus the literal, so drawing with ink `k`
//! paints `k` and `k+1`. In Dunkle Schatten an ink names the outline and
//! the next palette entry is the body: `0FEh` is white on black, `0F4h`
//! yellow, `0F0h` gray.

use crate::error::{Error, Result};
use crate::picture::Picture;
use crate::{Record, bytes, records, tail};

/// The letter spacing. It is byte `+5` of the text record `[196Ch]`, not
/// of the font file, whose byte 5 is `00`: the loader writes it as `0FFh`
/// at `0x5D13`, and the status line's painter reads it signed
/// (`0x0C73F`). Every advance the layout and the drawer compute adds it,
/// so glyphs overlap by one column and their outlines meet.
pub const SPACING: i16 = -1;

/// One character's bitmap and metrics.
#[derive(Debug, Clone)]
pub struct Glyph {
    /// Record `+2`: how far the pen moves, before [`SPACING`].
    pub advance: u16,
    /// Cel width.
    pub width: u16,
    /// Cel height.
    pub height: u16,
    /// Row-major, `None` where the stream skips.
    ///
    /// A `Some(n)` pixel is painted as `ink + n`: the cel's own literals are
    /// 0 and 1, and the ink is added to all of them before the blit.
    pub pixels: Vec<Option<u8>>,
}

/// The font `XP01` draws all of its text with.
#[derive(Debug, Clone)]
pub struct Font {
    /// Block `+2`: the character code record 0 describes.
    pub first: u8,
    /// Block `+8`: how far one line sits below the one above it.
    pub line_height: u16,
    /// One entry per record, from [`first`](Self::first) upwards. `None`
    /// where record `+0` is zero.
    pub glyphs: Vec<Option<Glyph>>,
}

impl Font {
    /// Reads the block `XP01`'s boot hands to the font record.
    pub fn parse(block: &[u8]) -> Result<Self> {
        let head = Record(bytes::<0x0A>(block, 0)?);
        let first = head.u16::<2>();
        let count = head.u16::<4>();
        let line_height = head.u16::<8>();
        // Both are words in the block, but the range has to fit a byte: a
        // block whose high bytes are not zero is not a block this reads.
        // Neither is one that describes no character at all, since every
        // lookup indexes `(ch − first)` against the count first.
        let first = match u8::try_from(first) {
            Ok(first)
                if (1..=0xFF).contains(&count)
                    && u16::from(first).saturating_add(count) <= 0x100 =>
            {
                first
            }
            _ => {
                return Err(Error::OutOfRange {
                    field: "font character range",
                    value: crate::pair(first, count),
                    allowed: "a count of 1 or more, and first + count at most 100h",
                });
            }
        };
        let table = 0x0A;
        let glyphs = records::<8>(block, table, usize::from(count))?
            .iter()
            .map(|rec| {
                let rec = Record(rec);
                if rec.u8::<0>() == 0 {
                    return Ok(None);
                }
                let at = usize::from(rec.u16::<6>()).saturating_add(table);
                Ok(Some(glyph(tail(block, at)?, rec.u16::<2>())?))
            })
            .collect::<Result<_>>()?;
        Ok(Font {
            first,
            line_height,
            glyphs,
        })
    }

    /// The glyph for a CP437 byte, or `None` when the font has none.
    pub fn glyph(&self, ch: u8) -> Option<&Glyph> {
        let i = usize::from(ch).checked_sub(usize::from(self.first))?;
        self.glyphs.get(i)?.as_ref()
    }

    /// `1601h:14BAh` (`0x174CA`): the record's `+2` for a code in the
    /// font's range and 0 for one outside it. The original reads the
    /// advance whether or not the record carries a bitmap; a record whose
    /// `+0` is zero is not kept here and answers 0, and every such record
    /// in Dunkle Schatten's file has an advance of 0.
    pub fn advance(&self, ch: u8) -> u16 {
        let Some(i) = usize::from(ch).checked_sub(usize::from(self.first)) else {
            return 0;
        };
        match self.glyphs.get(i) {
            Some(Some(g)) => g.advance,
            _ => 0,
        }
    }

    /// `1601h:1465h` (`0x17497`–`0x174C3`), and its twin `0546h:01A3h` in
    /// `XP00`: how wide a line is.
    ///
    /// The walk ends only at the zero byte (`0x1749A`) and steps over every
    /// byte the font has no glyph for — below its first character, past its
    /// last, or with a record whose `+0` is zero (`0x1749C`–`0x174AF`) —
    /// without taking the byte after it along. Every glyph adds its
    /// advance, and [`SPACING`] is added once fewer times than glyphs were
    /// counted (`0x174B9`–`0x174C3`): `n` glyphs are `n` advances plus
    /// `n − 1` spacings, in the sixteen bits the original measures in.
    pub fn measure(&self, line: &[u8]) -> u16 {
        let mut width = 0u16;
        let mut glyphs = 0u16;
        for &b in line.iter().take_while(|&&b| b != 0) {
            if let Some(g) = self.glyph(b) {
                width = width.wrapping_add(g.advance);
                glyphs = glyphs.wrapping_add(1);
            }
        }
        let gaps = glyphs.saturating_sub(1);
        width.wrapping_add(gaps.wrapping_mul(SPACING.cast_unsigned()))
    }

    /// How many bytes a text a task plays — a caption, a choice of the
    /// menu — takes on one line, and how wide it is.
    ///
    /// `13A7h:0196h` ends the walk on any byte below `20h` except `01`, the
    /// ink control, and `03`, the task's signal, each of which takes the
    /// byte after it along and counts two. Every glyph with a non-zero
    /// advance contributes `advance + spacing`, and the trailing spacing is
    /// taken off again at the end (`0x13C58`).
    pub fn measure_task_text(&self, line: &[u8]) -> (usize, u16) {
        let mut chars = 0usize;
        let mut width = 0u16;
        let mut counted = false;
        let mut walk = line.iter();
        while let Some(&b) = walk.next() {
            if b < 0x20 {
                if b != 1 && b != 3 {
                    break;
                }
                walk.next();
                chars = chars.saturating_add(2);
                continue;
            }
            chars = chars.saturating_add(1);
            let adv = self.advance(b);
            if adv != 0 {
                width = width.wrapping_add(adv).wrapping_add_signed(SPACING);
                counted = true;
            }
        }
        if counted {
            width = width.wrapping_add_signed(SPACING.wrapping_neg());
        }
        (chars, width)
    }
}

/// One glyph cel, decoded twice so that "skipped" and "literal 0" come apart.
fn glyph(cel: &[u8], advance: u16) -> Result<Glyph> {
    let zeroed = Picture::parse_prefix_filled(cel, 0x00)?;
    let ones = Picture::parse_prefix_filled(cel, 0xFF)?;
    let pixels = zeroed
        .pixels
        .iter()
        .zip(&ones.pixels)
        .map(|(&a, &b)| (a == b).then_some(a))
        .collect();
    Ok(Glyph {
        advance,
        width: zeroed.width,
        height: zeroed.height,
        pixels,
    })
}
