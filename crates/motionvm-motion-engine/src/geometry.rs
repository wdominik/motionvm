//! How large a descriptor is, and where its corner lands.
//!
//! The original keeps the measured size on the descriptor — width at +0x2E,
//! height at +0x32, written whenever a text is measured — and the `GD*` getters
//! read it back. Here it is computed on demand instead, which is the same
//! answer as long as measuring and drawing agree; [`Engine::extent`] says which
//! of the drawer's two passes it matches, and why the other one may draw
//! taller without that being a drift.

use crate::Field;
use crate::{Descriptor, Engine, Placement};
use motionvm_motion_formats::font::Font;
use motionvm_motion_forth::cell;

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
    /// own font with a gap of [`SPACING`](crate::text::SPACING). This
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
            let Some(text) = self.shown_text(d) else {
                return (0, 0);
            };
            let font = match d
                .font
                .and_then(|f| self.scene.fonts.get(&f))
                .or(self.scene.system_font.as_ref())
            {
                Some(f) => f.clone(),
                None => return (0, 0),
            };
            let Some(refs) = self.scene.font_refs.clone() else {
                return (0, 0);
            };
            let lines: Vec<&str> = text.split('\n').collect();
            let line_height = line_height(&font, crate::text::SPACING);
            let width = lines
                .iter()
                .map(|l| crate::text::text_width(&font, &refs, l))
                .max()
                .unwrap_or(0);
            let height = (cell::count(lines.len()) * line_height - crate::text::SPACING).max(0);
            return (width, height);
        }
        if let Some(id) = d.shows.graphic()
            && let Some(g) = self.load_sprite(id)
        {
            return self.scaling(d).size_of(&g);
        }
        (0, 0)
    }

    /// How a descriptor's graphic is scaled, in the terms its machine stores.
    ///
    /// The 16-bit record holds a target size in pixels per axis, `−1` for the
    /// natural one (`+0xC`/`+0xE`, written by `SDH%SHR`/`SDV%SHR`, read by
    /// the drawer at `016a:100d`); the 32-bit record holds per-mille factors
    /// the drawer multiplies by at paint time. The drawer and the measurer
    /// both go through here, so a descriptor is never drawn at one size and
    /// measured at another.
    pub(crate) fn scaling(&self, d: &Descriptor) -> Scaling {
        if self.profile.scale_in_pixels {
            let axis = |key| d.fields.get(key).filter(|&v| v != -1);
            return match (axis(Field::SDH_PCT_SHR), axis(Field::SDV_PCT_SHR)) {
                (None, None) => Scaling::Natural,
                (w, h) => Scaling::Pixels(w, h),
            };
        }
        // `SD%SHR` scales in thousandths; unset means full size.
        let all = d.fields.get(Field::SD_PCT_SHR).unwrap_or(0);
        let h = cell::unsigned(d.fields.get(Field::SDH_PCT_SHR).unwrap_or(all).max(0));
        let v = cell::unsigned(d.fields.get(Field::SDV_PCT_SHR).unwrap_or(all).max(0));
        let (h, v) = (if h == 0 { 1000 } else { h }, if v == 0 { 1000 } else { v });
        if (h, v) == (1000, 1000) {
            Scaling::Natural
        } else {
            Scaling::PerMille(h, v)
        }
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
        if d.is_text() && !self.profile.text_runs {
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
        // The 16-bit record's `+0` is the anchor less half the box *grown by
        // the template's margins* (`SDCEN` `05f1:14c3`, the drawer
        // `016a:0c53`), where the run drawer then centers each pass's
        // glyphs on the anchor with its own bare width — so `GDX` of a
        // centered text with a template stands a pixel left of its face.
        let (mw, mh) = self.template_margins(d);
        let (w, h) = (w + mw, h + mh);
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

    /// What the 16-bit text box grows by on each axis, out of the
    /// descriptor's template: the width or height margin, plus the offset
    /// where that is negative (`05f1:0ea0`–`0ec7` in `SDX`, the same sum in
    /// `SDCEN` and in the drawer at `016a:0bc0`). The shipped templates give
    /// `−1 −1` offsets and `1 1` margins, so a text with one grows by two
    /// and two. Nothing on the 32-bit machine, whose record holds no such
    /// box.
    pub(crate) fn template_margins(&self, d: &Descriptor) -> (i32, i32) {
        if !self.profile.text_runs || !d.is_text() {
            return (0, 0);
        }
        let Some(t) = d
            .template
            .and_then(|t| self.scene.templates.iter().find(|x| x.id == t))
        else {
            return (0, 0);
        };
        // `DEFTDT`'s arguments, deepest first: color, height margin, width
        // margin, y offset, x offset, line gap, glyph gap, font, id.
        let arg = |i: usize| t.args.get(i).copied().unwrap_or(0);
        let grow = |off: i32, margin: i32| if off < 0 { -off + margin } else { margin };
        (grow(arg(4), arg(2)), grow(arg(3), arg(1)))
    }
}

/// The distance from one baseline to the next, for a font and a glyph gap.
///
/// One function rather than the same expression in two places: it is what
/// [`Engine::extent`] measures with and what `draw_text_descriptor` places each
/// of its passes with, and those two agreeing is the whole reason `GDHEIGHT`
/// describes the block the player sees.
pub(crate) fn line_height(font: &Font, gap: i32) -> i32 {
    i32::from(font.height.max(1)) + gap
}

/// What a descriptor's graphic is drawn at; see [`Engine::scaling`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scaling {
    /// The sprite's own size.
    Natural,
    /// The 32-bit factors, in thousandths per axis.
    PerMille(u32, u32),
    /// The 16-bit target, in pixels per axis; `None` is that axis natural.
    Pixels(Option<i32>, Option<i32>),
}

impl Scaling {
    /// The size `sprite` draws at under this scaling.
    pub(crate) fn size_of(self, sprite: &motionvm_render::Picture) -> (i32, i32) {
        let (w, h) = (i32::from(sprite.width), i32::from(sprite.height));
        match self {
            Scaling::Natural => (w, h),
            Scaling::PerMille(hm, vm) => (w * cell::signed(hm) / 1000, h * cell::signed(vm) / 1000),
            Scaling::Pixels(pw, ph) => (pw.unwrap_or(w), ph.unwrap_or(h)),
        }
    }
}
