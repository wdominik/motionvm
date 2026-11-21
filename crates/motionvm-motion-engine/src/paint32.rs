//! Drawing straight into the software surface, past the descriptors: the
//! box `WHITEBOX` paints.
//!
//! The 32-bit engine keeps one display-sized software surface (`0xc3aac` in
//! R78, `0xe7d7c` in R109) that the descriptor drawer fills and the presenter
//! copies tiles of to the screen. `WHITEBOX ( x y w h -- )` (R78 `0x61030`)
//! draws into that surface directly, in display coordinates: a filled
//! rectangle in the palette's nearest match to white, `(63, 63, 63)` on the
//! DAC's scale (`0x1b940` picks it, `0x17020` fills), then a one-pixel
//! outline in the nearest match to black, inset by two — `(x+2, y+2, w−4,
//! h−4)` (`0x16f50` draws the four edges) — each behind a hide and a show of
//! the pointer and a mark of its rectangle for the presenter (`0x16ec0`).
//! Nothing marks the drawer's map: the box stays until a descriptor over it
//! is drawn again. Checker 2000's information book paints its page this way
//! (module 218, five sites).
//!
//! Here a screen's surface is its own buffer and the display is composed
//! from them, so the box goes into every active screen's buffer where that
//! screen's window shows through the rectangle — which for a game with one
//! full-screen screen is the same thing — and persists there the same way.
//!
//! One thing more is needed to keep it there. The original's surface holds
//! what was painted over what: the box goes over everything drawn before
//! it, and a descriptor drawn after it — the information page's text, put
//! up once the curtain has opened over the box — goes over the box, and
//! the box stays under the text when the text is drawn again, because the
//! copy `SDAUTOBUF` pastes back was taken with the box in it. The drawer
//! here builds a rectangle it has to draw again out of the descriptor
//! list ([`crate::Engine::draw`]), and a list knows nothing of a box. So
//! every paint is kept on its screen with the number of the drawer's pass
//! it came after ([`Paint`]), every descriptor remembers the pass that
//! last drew it, and a rebuilt rectangle draws what was drawn before the
//! paint, then the paint, then what was drawn after — level order within
//! each, the order the surface would hold. A wipe of the surface — a
//! `FADEOUT`, an `ERASESCR` — takes the paints with it.
//!
//! The nearest match (`0x1b940`, the 8-bit branch; R109 `0x1ee26`): over
//! the 256 entries of the engine's copy of the DAC (R78 `0xbad74`), the
//! entry whose `(|Δr|+1)² + (|Δg|+1)² + (|Δb|+1)²` is least, the first of
//! equals. The extra one on each axis is the routine's own — `inc %edx`
//! after every `sub` — and it is not nothing: against plain squares it adds
//! twice the Manhattan distance, which can move the answer between two
//! entries that are equally near by the one measure and not the other, as
//! the test below shows. `RGB->COL`, `NORMMOUSE` and the 32-bit `TOGFX`
//! resolve their colors through the same routine.

use crate::Engine;
use motionvm_render::Palette;

/// The palette index nearest to a 6-bit color, as `0x1b940` finds it.
pub(crate) fn nearest(palette: &Palette, r: u8, g: u8, b: u8) -> u8 {
    let dist = |i: usize| -> u32 {
        let d = |have: u8, want: u8| u32::from(have.abs_diff(want)) + 1;
        let (pr, pg, pb) = (
            palette.raw[i * 3],
            palette.raw[i * 3 + 1],
            palette.raw[i * 3 + 2],
        );
        d(pr, r).pow(2) + d(pg, g).pow(2) + d(pb, b).pow(2)
    };
    let mut best = (dist(0), 0usize);
    for i in 1..Palette::COLORS {
        let d = dist(i);
        if d < best.0 {
            best = (d, i);
        }
    }
    u8::try_from(best.1).unwrap_or(u8::MAX)
}

/// Something painted straight onto a screen's surface, past the drawer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Paint {
    /// The drawer's last pass before the paint: what that pass and every
    /// pass before it drew is under the paint, what later passes draw is
    /// over it.
    pub(crate) after_pass: u64,
    /// The rectangle, in display coordinates, and whether it is filled or
    /// its one-pixel edge.
    pub(crate) rect: (i32, i32, i32, i32),
    pub(crate) outline: bool,
    pub(crate) color: u8,
}

impl Paint {
    /// Whether this paint, filled, leaves nothing of `other` on the
    /// surface: every pixel of the other's rectangle is inside this one's.
    fn covers(&self, other: &Paint) -> bool {
        let (x, y, w, h) = self.rect;
        let (ox, oy, ow, oh) = other.rect;
        !self.outline
            && ox >= x
            && oy >= y
            && ox.saturating_add(ow) <= x.saturating_add(w)
            && oy.saturating_add(oh) <= y.saturating_add(h)
    }
}

impl Engine {
    /// The palette index nearest to a 6-bit color on the palette the script
    /// last set — the one copy the original's lookup reads (R78 `0xbad74`),
    /// which `SETPAL` rewrites at once, ahead of the fade that carries it to
    /// the display; `RGB->COL` searches the same copy.
    pub(crate) fn nearest_color(&self, r: u8, g: u8, b: u8) -> u8 {
        nearest(self.script_palette(), r, g, b)
    }

    /// Puts a paint onto the surface of every active screen it touches, and
    /// on the screen's list, so a rebuild puts it back.
    fn paint(&mut self, rect: (i32, i32, i32, i32), outline: bool, color: u8) {
        let paint = Paint {
            after_pass: self.draw_pass,
            rect,
            outline,
            color,
        };
        let handles: Vec<u32> = self
            .display
            .screens
            .iter()
            .filter(|s| s.active)
            .map(|s| s.handle)
            .collect();
        for handle in handles {
            if let Some(s) = self.display.screen_mut(handle) {
                // A filled paint leaves nothing of a paint it covers whole
                // — not of the paint, and not of what was drawn over that
                // paint inside its rectangle — so the covered one is gone
                // from the list as it is from the surface.
                if !outline {
                    s.paints.retain(|p| !paint.covers(p));
                }
                s.paints.push(paint);
            }
        }
        self.apply_paint(&paint, None);
        self.dirty = true;
    }

    /// The pixels of one paint, onto every active screen showing them or
    /// onto the one named — each screen's window over the display clipped
    /// once, and the rows filled inside it.
    pub(crate) fn apply_paint(&mut self, paint: &Paint, only: Option<u32>) {
        let (x, y, w, h) = paint.rect;
        if w <= 0 || h <= 0 {
            return;
        }
        let windows: Vec<_> = self
            .display
            .screens
            .iter()
            .filter(|s| s.active && only.is_none_or(|o| o == s.handle))
            .map(|s| (s.handle, self.display.window(s), s.view_pos))
            .collect();
        for (handle, window, at) in windows {
            let Some(s) = self.display.screen_mut(handle) else {
                continue;
            };
            // The paint in the screen's own coordinates: the display's
            // origin is the screen's view position, the window its place
            // on the surface.
            let (px, py) = (x - i32::from(at.0), y - i32::from(at.1));
            let (right, bottom) = (px.saturating_add(w), py.saturating_add(h));
            let (win_w, win_h) = (i32::from(window.w), i32::from(window.h));
            let clip = |v: i32, hi: i32| v.clamp(0, hi);
            let mut row = |r: i32, from: i32, to: i32| {
                if (0..win_h).contains(&r) {
                    for c in clip(from, win_w)..clip(to, win_w) {
                        s.buffer.set(window.x + c, window.y + r, paint.color);
                    }
                }
            };
            if paint.outline {
                row(py, px, right);
                row(bottom - 1, px, right);
                for r in clip(py, win_h)..clip(bottom, win_h) {
                    row(r, px, px + 1);
                    row(r, right - 1, right);
                }
            } else {
                for r in clip(py, win_h)..clip(bottom, win_h) {
                    row(r, px, right);
                }
            }
        }
    }

    /// `0x17020`'s fill: every pixel of the rectangle.
    pub(crate) fn fill_display_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u8) {
        self.paint((x, y, w, h), false, color);
    }

    /// `0x16f50`'s outline: the two columns at the sides, the two rows at
    /// top and bottom, one pixel wide.
    pub(crate) fn outline_display_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u8) {
        self.paint((x, y, w, h), true, color);
    }

    /// `WHITEBOX ( x y w h -- )`: the white box with its black inner edge.
    pub(crate) fn white_box(&mut self, x: i32, y: i32, w: i32, h: i32) {
        let white = self.nearest_color(63, 63, 63);
        let black = self.nearest_color(0, 0, 0);
        self.fill_display_rect(x, y, w, h, white);
        self.outline_display_rect(x + 2, y + 2, w - 4, h - 4, black);
    }
}

#[cfg(test)]
mod tests {
    use super::nearest;
    use crate::{Engine, Profile};
    use motionvm_render::Palette;

    /// A palette whose entry 5 is white and entry 9 is black, with a
    /// near-white at 7 that loses by one on one channel, and the rest gray.
    fn palette() -> Palette {
        let mut raw = [20u8; Palette::BYTES];
        raw[15..18].copy_from_slice(&[63, 63, 63]);
        raw[21..24].copy_from_slice(&[62, 63, 63]);
        raw[27..30].copy_from_slice(&[0, 0, 0]);
        Palette { raw }
    }

    #[test]
    fn the_nearest_entry_wins_and_the_first_of_equals() {
        let p = palette();
        assert_eq!(nearest(&p, 63, 63, 63), 5);
        assert_eq!(nearest(&p, 0, 0, 0), 9);
        // Equidistant from two grays: the first.
        assert_eq!(nearest(&p, 20, 20, 20), 0);
    }

    /// The plus one is more than a squared distance: black against (2,0,0)
    /// scores 9+1+1 = 11 and against (1,1,1) 4+4+4 = 12, so the first wins
    /// where plain squares — 4 against 3 — would pick the second.
    #[test]
    fn the_plus_one_can_change_the_answer() {
        let mut raw: Vec<u8> = [63, 0, 63].repeat(Palette::COLORS);
        raw[0..3].copy_from_slice(&[2, 0, 0]);
        raw[3..6].copy_from_slice(&[1, 1, 1]);
        let p = Palette::from_6bit(&raw);
        assert_eq!(nearest(&p, 0, 0, 0), 0);
    }

    /// The box, on one full-size screen: white inside, a black edge two in
    /// from the border, white again outside the edge.
    #[test]
    fn a_white_box_has_a_black_edge_two_pixels_in() {
        let mut e = Engine::new(Profile::motion32());
        e.select_mode(crate::video::MODE_640X480X256);
        e.enter_graphics().expect("the mode");
        e.display.palette = palette();
        let h = e.display.new_screen();
        let s = e.display.screen_mut(h).unwrap();
        s.set_size(640, 480);
        s.set_view(640, 480);
        e.white_box(50, 50, 540, 380);
        e.present();
        let px = |e: &Engine, x, y| e.video.get(x, y).unwrap();
        assert_eq!(px(&e, 50, 50), 5, "the corner is white");
        assert_eq!(px(&e, 51, 51), 5, "one in, still white");
        assert_eq!(px(&e, 52, 52), 9, "two in, the black edge");
        assert_eq!(px(&e, 53, 53), 5, "three in, white again");
        assert_eq!(px(&e, 587, 427), 9, "the far corner of the edge");
        assert_eq!(px(&e, 589, 429), 5, "the far corner of the box");
        assert_eq!(
            px(&e, 590, 430),
            0,
            "outside the box, the surface as it was"
        );
    }

    /// The list a rebuild replays: a box painted over another whole takes
    /// the first's two paints off it — the information book's pages come
    /// one over the other without a wipe — while a box that only overlaps
    /// leaves them, and an outline covers nothing.
    #[test]
    fn a_covering_fill_takes_the_covered_paints_off_the_list() {
        let mut e = Engine::new(Profile::motion32());
        e.select_mode(crate::video::MODE_640X480X256);
        e.enter_graphics().expect("the mode");
        e.display.palette = palette();
        let h = e.display.new_screen();
        let s = e.display.screen_mut(h).unwrap();
        s.set_size(640, 480);
        s.set_view(640, 480);
        let paints = |e: &Engine| e.display.screens[0].paints.len();
        e.white_box(25, 122, 452, 317);
        assert_eq!(paints(&e), 2, "the fill and its edge");
        e.white_box(25, 122, 452, 317);
        assert_eq!(paints(&e), 2, "the same box again replaces both");
        e.white_box(100, 150, 40, 40);
        assert_eq!(paints(&e), 4, "a box inside the page adds its two");
        e.outline_display_rect(0, 0, 640, 480, 9);
        assert_eq!(paints(&e), 5, "an outline around everything covers nothing");
        e.fill_display_rect(0, 0, 640, 480, 5);
        assert_eq!(
            paints(&e),
            1,
            "a fill over the whole screen is all that is left"
        );
    }

    /// A paint reaching past the screen's window is clipped to it, and one
    /// wholly outside paints nothing.
    #[test]
    fn a_paint_is_clipped_to_the_window() {
        let mut e = Engine::new(Profile::motion32());
        e.select_mode(crate::video::MODE_640X480X256);
        e.enter_graphics().expect("the mode");
        e.display.palette = palette();
        let h = e.display.new_screen();
        let s = e.display.screen_mut(h).unwrap();
        s.set_size(640, 480);
        s.set_view(640, 480);
        e.fill_display_rect(630, 470, 100, 100, 5);
        e.outline_display_rect(-10, -10, 20, 20, 9);
        e.fill_display_rect(700, 700, 10, 10, 9);
        e.present();
        let px = |e: &Engine, x, y| e.video.get(x, y).unwrap();
        assert_eq!(px(&e, 639, 479), 5, "the fill reaches the last pixel");
        assert_eq!(px(&e, 629, 469), 0, "and starts where it was asked to");
        assert_eq!(px(&e, 9, 0), 9, "the outline's right column");
        assert_eq!(px(&e, 0, 9), 9, "and its bottom row");
        assert_eq!(px(&e, 5, 5), 0, "nothing inside the outline");
    }
}
