//! Text the way the engine puts it down: the glyph, the shared line drawer,
//! the measuring, and the darkened backing behind a panel.
//!
//! One glyph and one measure serve both engine generations; the 16-bit run
//! drawer, which lays a line down by rules of its own, is the twin file
//! `text16`. Fonts come from the game's files, the surface is
//! [`Framebuffer`], and everything here draws with the caller's palette
//! indices — nothing text-shaped survives into the renderer below.

use motionvm_motion_formats::font::{Font, FontRefTable};
use motionvm_motion_forth::cell;
use motionvm_render::Framebuffer;
use motionvm_render::Palette;

/// The gap the engine leaves after every glyph, across and down. See
/// [`draw_line`] for how it was measured.
pub const SPACING: i32 = 1;

/// What a line is drawn with: the face, the color and the glyph gap.
///
/// These are not arguments in the original — they are the drawer's globals,
/// set before it runs and read out of the data segment as it goes: the
/// current font, the glyph gap (0xd66ee in the 32-bit engine, `ds:0x13c4`
/// in the 16-bit one) and the line gap beside it. A pass sets them and
/// draws; the next pass sets them again. `Engine::draw_text_descriptor`
/// does exactly that — the outline pass draws with the template's font,
/// gap and color, the pass on top with the descriptor's own and a gap of 1
/// — which is why this is a value made per pass and not a field of
/// anything.
///
/// The two halves of a face travel together because the file format splits
/// what one font is: [`Font`] carries the glyphs, [`FontRefTable`] the
/// CP437 byte that reaches each one.
#[derive(Debug, Clone, Copy)]
pub struct Pen<'a> {
    /// The glyphs.
    pub font: &'a Font,
    /// The table that maps a CP437 byte to one of them.
    pub refs: &'a FontRefTable,
    /// The palette index every set bit is written with.
    pub color: u8,
    /// The gap left after each glyph.
    pub spacing: i32,
}

impl Pen<'_> {
    /// How wide `text` would be drawn with this pen, without drawing it.
    pub fn width(&self, text: &str) -> i32 {
        text_width_spaced(self.font, self.refs, text, self.spacing)
    }
}

/// Puts one character down at `x`, `y` and answers how wide its glyph is,
/// or `None` where the font has none for it.
///
/// Glyphs are 1-bit masks, so only set bits are written and the gaps stay
/// as they were. This is the whole of what a glyph costs; what a line does
/// between them is the drawer's, and the two generations disagree about
/// that — see [`crate::text16::draw_line`].
pub(crate) fn draw_glyph(
    frame: &mut Framebuffer,
    pen: Pen<'_>,
    ch: char,
    x: i32,
    y: i32,
) -> Option<i32> {
    let byte = cp437_byte(ch)?;
    let index = pen.refs.glyph_for(byte)?;
    let glyph = pen.font.glyphs.get(usize::from(index))?;
    for gy in 0..glyph.height {
        for gx in 0..glyph.width {
            if glyph.pixel(gx, gy) {
                frame.set(x + i32::from(gx), y + i32::from(gy), pen.color);
            }
        }
    }
    Some(i32::from(glyph.width))
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
pub fn draw_line(frame: &mut Framebuffer, pen: Pen<'_>, text: &str, x: i32, y: i32) -> i32 {
    let mut at = x;
    for ch in text.chars() {
        if let Some(width) = draw_glyph(frame, pen, ch, at, y) {
            at += width + pen.spacing;
        }
    }
    // The last glyph's spacing is never drawn, so it is not counted either.
    (at - x - pen.spacing).max(0)
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
        .filter_map(|i| font.glyphs.get(usize::from(i)))
        .map(|g| i32::from(g.width) + spacing)
        .sum();
    (total - spacing).max(0)
}

/// Maps a `char` back to the CP437 byte the font tables are indexed by.
fn cp437_byte(ch: char) -> Option<u8> {
    if let Ok(byte) = u8::try_from(ch)
        && byte < 0x80
    {
        return Some(byte);
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
    for row in y.max(0)..(y + h).min(i32::from(frame.height)) {
        for col in x.max(0)..(x + w).min(i32::from(frame.width)) {
            // Both clipped at zero by the ranges above.
            let i =
                cell::at(row).unwrap_or(0) * usize::from(frame.width) + cell::at(col).unwrap_or(0);
            frame.pixels[i] = map[usize::from(frame.pixels[i])];
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
            std::array::from_fn(|k| (i16::from(palette.raw[i * 3 + k]) - BACKING_DARKEN).max(0));
        let mut best = (i32::MAX, 0u8);
        for (j, index) in (0..Palette::COLORS).zip(0u8..=u8::MAX) {
            let d: i32 = (0..3)
                .map(|k| {
                    let e = i32::from(want[k]) - i32::from(palette.raw[j * 3 + k]);
                    e * e
                })
                .sum();
            if d < best.0 {
                best = (d, index);
            }
        }
        *slot = best.1;
    }
    map
}
