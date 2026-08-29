//! How large a descriptor is, and where its corner lands.
//!
//! The original keeps the measured size on the descriptor — width at +0x2E,
//! height at +0x32, written whenever a text is measured — and the `GD*` getters
//! read it back. Here it is computed on demand instead, which is the same
//! answer as long as measuring and drawing agree; [`Engine::extent`] says which
//! of the drawer's two passes it matches, and why the other one may draw
//! taller without that being a drift.

use crate::{Descriptor, Engine, Placement};
use motionvm_formats::font::Font;

impl Engine {
    /// How wide and tall a descriptor draws.
    ///
    /// The original keeps this on the descriptor — width at +0x2E, height at
    /// +0x32, both written when a text is measured — and `GDWIDTH`, `GDCX` and
    /// the clamps read it back.
    ///
    /// **Which of the drawer's two passes this matches, and why.**
    /// `draw_text_descriptor` lays a text down twice: once in the template's
    /// outline font with the template's glyph gap, and once in the descriptor's
    /// own font with a gap of [`SPACING`](motionvm_render::SPACING). This
    /// measures the second — the descriptor's font, the same gap, through the
    /// same [`line_height`] the drawer uses — because that is the pass the
    /// original measures too: with fonts 5, 6 and 8 each registered in turn and
    /// no `SDFNT`, it reports the same width every time.
    ///
    /// So the outline pass can and does draw taller than this reports — font 6
    /// stands at 20 where font 5 stands at 18, one pixel over and one under.
    /// That is the original's shape, not a drift: the outline is a second
    /// typeface underneath, centered on the same point, and nothing measures it.
    ///
    /// **Why it takes `&mut self`, and why that is not a wart.** Measuring a
    /// descriptor can load one: a text pulls in its table, a sprite its pixels.
    /// The load is a cache fill and nothing more — it goes through
    /// [`Engine::load_sprite`](crate::Engine::load_sprite) rather than
    /// `Engine::sprite`, so measuring cannot decide the palette the way drawing
    /// can. That split is what lets the drawer measure freely: every mark on
    /// the damage map is a rectangle, so a descriptor is measured whenever a
    /// field of it is set, long before it is drawn.
    ///
    /// It reads as an artifact of lazy loading that interior caches could hide
    /// behind a `&self` and a `RefCell`. They should not: the state change is
    /// genuine, and hiding it would be a divergence besides, because the
    /// original loads late too. Everything derived from this — `stored_extent`,
    /// `corner`, and the eleven `descriptor_*` getters — inherits the borrow
    /// for the same reason.
    pub(crate) fn extent(&mut self, d: &Descriptor) -> (i32, i32) {
        if d.is_text() {
            let Some(text) = self.descriptor_text(d) else {
                return (0, 0);
            };
            let font = match d
                .font
                .and_then(|f| self.fonts.get(&f))
                .or(self.system_font.as_ref())
            {
                Some(f) => f.clone(),
                None => return (0, 0),
            };
            let Some(refs) = self.font_refs.clone() else {
                return (0, 0);
            };
            let lines: Vec<&str> = text.split('\n').collect();
            let line_height = line_height(&font, motionvm_render::SPACING);
            let width = lines
                .iter()
                .map(|l| motionvm_render::Framebuffer::text_width(&font, &refs, l))
                .max()
                .unwrap_or(0);
            let height = (lines.len() as i32 * line_height - motionvm_render::SPACING).max(0);
            return (width, height);
        }
        if let Some(id) = d.shows.graphic()
            && let Some(g) = self.load_sprite(id)
        {
            let all = d.fields.get("SD%SHR").copied().unwrap_or(0);
            let h = d.fields.get("SDH%SHR").copied().unwrap_or(all).max(0) as u32;
            let v = d.fields.get("SDV%SHR").copied().unwrap_or(all).max(0) as u32;
            let (h, v) = (if h == 0 { 1000 } else { h }, if v == 0 { 1000 } else { v });
            return (
                g.width as i32 * h as i32 / 1000,
                g.height as i32 * v as i32 / 1000,
            );
        }
        (0, 0)
    }

    /// The rectangle the text backing covers, which is not the text's own.
    ///
    /// Every number here is read off a handler, none measured from a picture:
    ///
    /// * `SDTDT` at 0x73333 and 0x7334c adds **4** to the measured width and
    ///   height before storing them at +0x2e and +0x32.
    /// * `0x6b1dc` (0x6b26e) and `0x6b296` (0x6b328) add **10** on top, but
    ///   only while the color says there is a backing.
    /// * `0x6beec` (0x6bfa8) and `0x6c57c` (0x6c639) take **5** off the corner.
    /// * The template contributes `-1, -1, +2, +2` — its fields +4, +6, +8 and
    ///   +0xa, filled by `DEFTDT` from the `8 2 2 -1 -1 -1 -1 font n` that
    ///   module 3 passes for all nine of them.
    ///
    /// The result is six to the left and above, sixteen more in each
    /// direction. That is not symmetric, and it follows from the constants
    /// rather than from a wish: the glyphs are drawn at an origin the text
    /// record shifts by its fields +0x24 and +0x2c, and those two are not read
    /// yet. If the backing sits visibly off to one side, they are the reason.
    pub(crate) fn backing_rect(&mut self, d: &Descriptor) -> (i32, i32, i32, i32) {
        let (w, h) = self.stored_extent(d);
        let (x, y) = self.corner(d);
        (x - 5 - 1, y - 5 - 1, w + 10 + 2, h + 10 + 2)
    }

    /// The size the descriptor *stores*, which is what every placement mode is
    /// resolved against — not the size the glyphs actually cover.
    ///
    /// `SDTDT` writes the measured size plus **4** into +0x2e and +0x32
    /// (0x73333 and 0x7334c, each `add $4,%edx`), and it does so after every
    /// change of text, color or template. Sprites get their size from the
    /// picture instead, with no such padding, which is why this only applies to
    /// text.
    ///
    /// It matters because the placement halves exactly this value: `0x6bdfa`,
    /// the routine behind `GDX`, does `desk[+4] − desk[+0x2e]/2` for a center
    /// and `− desk[+0x2e]` for a far edge. Halving the bare measured size
    /// instead puts a centered text two pixels off — and `TSC` clamps the speech
    /// with `GDX`, so the error moved the sentence as well as its backing.
    pub(crate) fn stored_extent(&mut self, d: &Descriptor) -> (i32, i32) {
        let (w, h) = self.extent(d);
        if d.is_text() && !self.text16 {
            (w + 4, h + 4)
        } else {
            // The 16-bit `GDWIDTH`/`GDHEIGHT` (`05f1:1705`, `05f1:177b`)
            // measure the text live — the block measure at `14ee:16fd` with
            // the descriptor's font and the resting gaps of 1 — and answer
            // the bare size: no 4, no template margins. (The drawer stores
            // a padded box at +8/+0xA for its own restore rectangle, but no
            // getter reads it.)
            (w, h)
        }
    }

    /// Where a descriptor's top left corner ends up, once its placement modes
    /// are applied to the coordinates it was given.
    ///
    /// This is what `GDX` and `GDY` answer with in the original — 0x6bdfa, not
    /// the stored number. The difference matters: `SETT1` and `TSC` both decide
    /// whether a text overhangs the screen by comparing `GDX` against the
    /// screen origin, and a `GDX` that answers with the center never overhangs.
    pub(crate) fn corner(&mut self, d: &Descriptor) -> (i32, i32) {
        let (w, h) = self.stored_extent(d);
        let x = match d.x_mode {
            Placement::Edge => d.x,
            Placement::Center => d.x - w / 2,
            Placement::FarEdge => d.x - w,
        };
        let y = match d.y_mode {
            Placement::Edge => d.y,
            Placement::Center => d.y - h / 2,
            Placement::FarEdge => d.y - h,
        };
        (x, y)
    }
}

/// The distance from one baseline to the next, for a font and a glyph gap.
///
/// One function rather than the same expression in two places: it is what
/// [`Engine::extent`] measures with and what `draw_text_descriptor` places each
/// of its passes with, and those two agreeing is the whole reason `GDHEIGHT`
/// describes the block the player sees.
pub(crate) fn line_height(font: &Font, gap: i32) -> i32 {
    font.height.max(1) as i32 + gap
}
