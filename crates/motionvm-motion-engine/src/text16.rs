//! Drawing the way the 16-bit engine does it.
//!
//! One routine, because one routine is all that differs: `ENVIRO.EXE`'s run
//! drawer puts a line down by rules the 32-bit engine's has no equivalent for.
//! The glyph and the measuring underneath are the family's and live in
//! [`crate::text`]; the surface and the blits are the renderer's.

use crate::text::Pen;
use motionvm_render::Framebuffer;

/// One line the way the 16-bit engine's run drawer puts it down
/// (`ENVIRO.EXE` `14ee:11cf`), answering how far the pen advanced.
///
/// Like [`crate::text::draw_line`], except for two things this engine
/// does and the other does not. `#` is eaten — no glyph and no advance
/// (`14ee:135e`), while the measure keeps counting it. And each space after
/// the line's first drawn glyph is widened by the next entry of `pads`, the
/// pixels the justification spread over the line (`14ee:1391`; empty when
/// nothing justifies).
pub(crate) fn draw_line(
    fb: &mut Framebuffer,
    pen: Pen<'_>,
    text: &str,
    x: i32,
    y: i32,
    pads: &[i32],
) -> i32 {
    let mut at = x;
    let mut drawn = false;
    let mut pad = 0usize;
    for ch in text.chars() {
        if ch == '#' {
            continue;
        }
        let Some(width) = crate::text::draw_glyph(fb, pen, ch, at, y) else {
            continue;
        };
        at += width + pen.spacing;
        if ch == ' ' {
            if drawn {
                at += pads.get(pad).copied().unwrap_or(0);
                pad += 1;
            }
        } else {
            drawn = true;
        }
    }
    (at - x - pen.spacing).max(0)
}
