//! Laying a line out and painting it.
//!
//! The layout routine at `0x12712` of Dunkle Schatten's `XP01` lays a
//! string out into a 20h-byte block. Nothing here wraps text.
//!
//! The block, from the stores in that routine:
//!
//! | Offset | Field |
//! |---|---|
//! | `+0` | mode |
//! | `+2` | x, after centering |
//! | `+4` | y, after the mode's own placement |
//! | `+6` | box width — the widest line, plus 8 in modes 1 and 2 |
//! | `+8` | box height — one line height per line, plus 8 in modes 1 and 2 |
//! | `+0Ah` | the ink row |
//! | `+0Eh` | the string |
//!
//! **The spacing.** Every advance the layout and the painters compute adds
//! the text record's `+5` to a glyph's own width, read signed (`mov al,
//! es:[bx+5] ; cbw` at `0x0C73F`, `0x112F3`, `0x1275D`, `0x1289D`,
//! `0x12A9F`, `0x12C8A`, `0x12CD2`), and the status line's painter only
//! when the width is not zero (`0x0C7B8`, before the sum at `0x0C7CA`).
//! The loader writes that byte as `0FFh` (`0x5D13`, the program's only
//! byte store to a `+5`), so the spacing is
//! [`motionvm_parsec_formats::font::SPACING`], −1: glyphs overlap by one
//! column and their outlines meet.

use crate::num;
use motionvm_parsec_formats::Font;
use motionvm_render::Framebuffer;

/// One line of CP437 bytes, left to right from `x`.
///
/// Returns where the pen ends up. A byte the font has no record for is
/// skipped without moving the pen, and a glyph whose advance is zero does
/// not move it either.
pub(crate) fn line(fb: &mut Framebuffer, font: &Font, line: &[u8], x: i32, y: i32, ink: u8) -> i32 {
    let mut pen = x;
    for &b in line {
        let Some(g) = font.glyph(b) else {
            continue;
        };
        glyph(fb, font, b, pen, y, ink);
        if g.advance != 0 {
            pen += i32::from(g.advance) + i32::from(motionvm_parsec_formats::font::SPACING);
        }
    }
    pen
}

/// One glyph. Its pixels are offsets into the ink's two-entry ramp: an ink
/// is the outline and the next entry the body, so a pixel of 0 comes out
/// in the ink and a pixel of 1 in the entry after it.
pub(crate) fn glyph(fb: &mut Framebuffer, font: &Font, ch: u8, x: i32, y: i32, ink: u8) {
    let Some(g) = font.glyph(ch) else {
        return;
    };
    for row in 0..i32::from(g.height) {
        let base = num::index(row) * usize::from(g.width);
        for col in 0..i32::from(g.width) {
            if let Some(Some(v)) = g.pixels.get(base + num::index(col)) {
                fb.set(x + col, y + row, ink.wrapping_add(*v));
            }
        }
    }
}

/// A one-pixel rectangle outline.
pub(crate) fn frame_rect(fb: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, index: u8) {
    if w <= 0 || h <= 0 {
        return;
    }
    for cx in x..x + w {
        fb.set(cx, y, index);
        fb.set(cx, y + h - 1, index);
    }
    for cy in y..y + h {
        fb.set(x, cy, index);
        fb.set(x + w - 1, cy, index);
    }
}
