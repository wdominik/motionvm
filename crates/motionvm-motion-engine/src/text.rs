//! Text the way the engine puts it down: the glyph, the shared line drawer,
//! the measuring, and the darkened backing behind a panel.
//!
//! One glyph and one measure serve both engine generations; the 16-bit run
//! drawer, which lays a line down by rules of its own, is the twin file
//! `text16`. Fonts come from the game's files, the surface is
//! [`Framebuffer`], and everything here draws with the caller's palette
//! indices — nothing text-shaped survives into the renderer below.

use motionvm_motion_formats::font::{Font, FontRefTable};
use motionvm_render::Framebuffer;
use motionvm_render::Palette;

/// The gap the engine leaves after every glyph, across and down. See
/// [`draw_text`] for how it was measured.
pub const SPACING: i32 = 1;

/// Puts one character down at `x`, `y` and answers how wide its glyph is,
/// or `None` where the font has none for it.
///
/// Glyphs are 1-bit masks, so only set bits are written and the gaps stay
/// as they were. This is the whole of what a glyph costs; what a line does
/// between them is the drawer's, and the two generations disagree about
/// that — see [`crate::text16::draw_line`].
pub(crate) fn draw_glyph(
    frame: &mut Framebuffer,
    font: &Font,
    refs: &FontRefTable,
    ch: char,
    x: i32,
    y: i32,
    color: u8,
) -> Option<i32> {
    let byte = cp437_byte(ch)?;
    let index = refs.glyph_for(byte)?;
    let glyph = font.glyphs.get(index as usize)?;
    for gy in 0..glyph.height {
        for gx in 0..glyph.width {
            if glyph.pixel(gx, gy) {
                frame.set(x + gx as i32, y + gy as i32, color);
            }
        }
    }
    Some(glyph.width as i32)
}

/// Draws one line of text, returning how far the pen advanced.
///
/// Glyphs are 1-bit masks, so only set bits are written; the gaps stay as
/// they were.
///
/// One pixel of spacing follows each glyph. That is measured, not assumed:
/// the intro's lines come to 409, 437 and 354 pixels of ink in the original,
/// where the bare glyph widths of the font it uses add up to 361, 382 and
/// 307 over 49, 56 and 48 characters. Where the pixel comes from the format
/// does not say — neither the font header nor the reference table carries a
/// spacing value — so it lives in the engine's drawing code. What is
/// established is the result, not the mechanism.
pub fn draw_text(
    frame: &mut Framebuffer,
    font: &Font,
    refs: &FontRefTable,
    text: &str,
    x: i32,
    y: i32,
    color: u8,
) -> i32 {
    draw_text_spaced(frame, font, refs, text, x, y, color, SPACING)
}

/// The same, with the gap between glyphs given rather than assumed.
///
/// The gap is a global in the original (0xd66ee, with the line gap at
/// 0xd66ec), and the text drawer sets it per pass: the outline pass takes
/// the value from the template, the normal pass puts back 1. See
/// `Engine::draw_text_descriptor`.
// Font, reference table, string, position, color and spacing: six things
// the caller genuinely decides per call, with no subset that travels
// together often enough to earn a struct.
#[allow(clippy::too_many_arguments)]
pub fn draw_text_spaced(
    frame: &mut Framebuffer,
    font: &Font,
    refs: &FontRefTable,
    text: &str,
    x: i32,
    y: i32,
    color: u8,
    spacing: i32,
) -> i32 {
    let mut pen = x;
    for ch in text.chars() {
        if let Some(width) = draw_glyph(frame, font, refs, ch, pen, y, color) {
            pen += width + spacing;
        }
    }
    // The last glyph's spacing is never drawn, so it is not counted either.
    (pen - x - spacing).max(0)
}

/// How wide `text` would be, without drawing it.
///
/// Needed because the game centers text on a point (`SDCEN`, `SDVCEN`)
/// rather than giving a corner, so the left edge cannot be known until the
/// width is.
pub fn text_width(font: &Font, refs: &FontRefTable, text: &str) -> i32 {
    text_width_spaced(font, refs, text, SPACING)
}

/// How wide `text` would be at a given glyph gap.
pub fn text_width_spaced(font: &Font, refs: &FontRefTable, text: &str, spacing: i32) -> i32 {
    let total: i32 = text
        .chars()
        .filter_map(cp437_byte)
        .filter_map(|b| refs.glyph_for(b))
        .filter_map(|i| font.glyphs.get(i as usize))
        .map(|g| g.width as i32 + spacing)
        .sum();
    (total - spacing).max(0)
}

/// Maps a `char` back to the CP437 byte the font tables are indexed by.
fn cp437_byte(ch: char) -> Option<u8> {
    if (ch as u32) < 0x80 {
        return Some(ch as u8);
    }
    (0x80u8..=0xff).find(|&b| motionvm_motion_formats::cp437_char(b) == ch)
}

/// How much the text backing takes off each color channel.
///
/// Read from the table builder at 0x14740: it walks the palette, subtracts 20
/// from red, green and blue, clamps each at zero and asks 0x1ee26 for the
/// nearest entry to the result. The answers become a 256-byte row, and the
/// backing sends every pixel under the text through it.
pub const BACKING_DARKEN: i16 = 0x14;

/// Darkens a rectangle the way the original's text backing does.
///
/// Not a fill: at 0x186d5 the routine reads each pixel, looks it up in the
/// remap row and writes the answer back, so what shows through is the
/// picture underneath, darkened — which is how a 256-color game makes a
/// half-transparent panel. A solid rectangle in some color would be a
/// different thing entirely and look it.
///
/// Takes the remap row rather than the palette it comes from, because
/// building one is [`backing_map`]'s 196 000 operations and the answer
/// depends on nothing else. The original builds its row once per palette
/// too — the table builder at 0x14740 runs when the palette is set, not
/// when a panel is drawn.
pub fn darken_rect(frame: &mut Framebuffer, map: &[u8; 256], x: i32, y: i32, w: i32, h: i32) {
    for row in y.max(0)..(y + h).min(frame.height as i32) {
        for col in x.max(0)..(x + w).min(frame.width as i32) {
            let i = row as usize * frame.width as usize + col as usize;
            frame.pixels[i] = map[frame.pixels[i] as usize];
        }
    }
}

/// The 256-byte remap row the backing uses: every palette entry darkened by
/// [`BACKING_DARKEN`] and brought back to the nearest entry there is.
pub fn backing_map(palette: &Palette) -> [u8; 256] {
    let mut map = [0u8; 256];
    for (i, slot) in map.iter_mut().enumerate() {
        // The subtraction is on the palette as stored — six-bit values, 0 to 63
        // — so twenty is about a third of full scale, not a twelfth. Reading it
        // as an eight-bit step would make the backing nearly invisible.
        let want: [i16; 3] =
            std::array::from_fn(|k| (palette.raw[i * 3 + k] as i16 - BACKING_DARKEN).max(0));
        let mut best = (i32::MAX, 0u8);
        for j in 0..Palette::COLORS {
            let d: i32 = (0..3)
                .map(|k| {
                    let e = want[k] as i32 - palette.raw[j * 3 + k] as i32;
                    e * e
                })
                .sum();
            if d < best.0 {
                best = (d, j as u8);
            }
        }
        *slot = best.1;
    }
    map
}
