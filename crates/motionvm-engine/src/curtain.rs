//! Transitions: the band wipe `FADEIN` and `FADEOUT` run.
//!
//! Not a fade in the sense of color. The handler walks a horizontal band down
//! and up from the middle of the view, eight rows at a time, and either reveals
//! what was drawn behind it or hides what was showing. The picture never
//! changes while it runs — `FADEIN` draws once before the first band and
//! `FADEOUT` draws nothing at all.
//!
//! While one is running the interpreter is held: the original's handler spins
//! inside itself and its frame loop does not turn, so the engine counts a step
//! as a band rather than as a frame. See [`crate::clock`].

use crate::Engine;

impl Engine {
    /// Whether a transition is running, and so whether the interpreter is held.
    pub fn in_transition(&self) -> bool {
        !self.curtains.is_empty()
    }

    /// Moves the running transition on, dropping it when finished.
    ///
    /// This is where the two directions stop being mirror images. Both mark
    /// bands in the update map and let the presenter copy them, so both only
    /// ever change the rows they have reached — but they start from different
    /// pictures:
    ///
    /// * `FADEOUT` fills the screen's rectangle of the surface with color 0
    ///   (0x74d44) and *then* clears the map (0x74d49), so every band it marks
    ///   turns black. Black grows in from both edges.
    /// * `FADEIN` draws the screen (0x74af9) and *then* clears the map
    ///   (0x74afe), so every band it marks shows the new picture. **It blanks
    ///   nothing.** Outside the band, video memory still holds the previous
    ///   frame — which is why a `FADEIN` with no `FADEOUT` before it reads as
    ///   the new picture opening over the old one rather than out of black.
    ///   The menu is built on that: a document page turn is `SHOW_DOC` and
    ///   two `FADEIN`s with no `FADEOUT` anywhere (module 4, 0x03bc0/0x03be8).
    ///
    /// Masking the whole rectangle instead — blacking out everything the band
    /// had not reached — is right for one direction and wrong for the other,
    /// and made every menu and help page blink through black.
    ///
    /// The whole covered range is rewritten each time, not just the newest
    /// band: a frame buys several bands when `ticks_per_band` is small, and
    /// the ranges only ever grow.
    pub(crate) fn advance_curtain(&mut self) {
        // A step, not a frame — see [`Self::step_ticks`]. During a curtain this
        // is exactly one band's worth, so one call moves one band and paints
        // it, the way the handler's loop does.
        let ticks = self.step_ticks().max(1);
        let Some(c) = self.curtains.front_mut() else {
            return;
        };
        c.advance(ticks);
        let c = c.clone();
        self.paint_curtain(&c);
        if c.done() {
            self.curtains.pop_front();
        }
    }

    fn paint_curtain(&mut self, c: &Curtain) {
        let (x, y, w, h) = c.area;
        let (top, bottom) = c.visible();
        if !c.opening {
            for row in y..(y + h) {
                if row >= top && row < bottom {
                    continue;
                }
                for col in x..(x + w) {
                    self.video.set(col, row, 0);
                }
            }
            return;
        }
        let (top, bottom) = (top.max(y), bottom.min(y + h));
        if bottom <= top {
            return;
        }
        let Some(s) = self.display.screens.iter().find(|s| s.handle == c.screen) else {
            return;
        };
        // The same mapping the composer uses, narrowed to the band: `SCRPOS`
        // scrolls the window over the surface, `SCRVPOS` places it.
        let window = self.display.window(s);
        let band = motionvm_render::Rect {
            x: window.x,
            y: window.y + (top - y),
            w: (w as u16).min(window.w),
            h: (bottom - top) as u16,
        };
        self.video.copy_from(&s.buffer, band, x, top);
    }
}

/// One curtain transition, as `FADEOUT` and `FADEIN` in mode 1 draw it.
///
/// Both handlers take `( mode duration step -- )` — `1 50 8` throughout the
/// intro — and work in bands of `step` pixels on the screen `ACTSCR` selected.
///
/// `FADEOUT` runs `i` from 0 up to half the height and copies two bands each
/// time, at `base + i` and `base + height - step - i`: black grows in from the
/// top and bottom edges. `FADEIN` runs `i` from half the height back down to
/// zero and copies a single band at `base + i` that is `(height/2 - i) * 2`
/// tall, so the picture opens outwards from the middle.
///
/// Both therefore come down to one symmetric band of visible rows, growing or
/// shrinking around the center — which is all this has to hold, because the
/// interpreter is stopped while the curtain runs. During a fade out the game
/// has not yet swapped the picture or changed the palette, so the frame draws
/// itself correctly and only wants masking.
#[derive(Debug, Clone)]
pub struct Curtain {
    /// `FADEIN` opens, `FADEOUT` closes.
    pub opening: bool,
    /// Which screen the effect belongs to — `ACTSCR` picks it, and it is not
    /// the display: the title macro fades the status bar while the picture
    /// above it stays put.
    pub screen: u32,
    /// That screen's rectangle on the display.
    ///
    /// The width is the one the handler itself reads, and the two directions
    /// do not read the same field: `FADEIN` takes `+0x18` at 0x74b22, which is
    /// `SCRSIZE`, and `FADEOUT` takes `+0x1C` at 0x74d70, which is `SCRVSIZE`.
    /// Every screen the game builds sets both to the same value, so nothing in
    /// the shipped game can tell them apart — it is recorded because it was
    /// read, not because it shows.
    pub area: (i32, i32, i32, i32),
    /// The third argument, eight everywhere the game uses it. One `advance` is
    /// one band: the curtain closes from both edges at once, so eight a side
    /// makes the sixteen the handler divides the height by.
    pub step: i32,
    /// How far the bands have traveled.
    pub offset: i32,
    /// Ticks one band is allowed to take, and how many are banked so far.
    ///
    /// The handler works this out at 0x74cf6: `duration / bands`, where the
    /// bands are `screen height / 16` (0x74cd0) and the duration is the second
    /// argument.
    /// It then spins after each band until that many ticks have passed
    /// (0x74db9). With `1 50 8 FADEOUT` on a 400-pixel screen that is 26
    /// passes at 2 ticks — 0.28 s for the whole fade, with the spin's
    /// quantization counted; see `clock.rs` for that arithmetic.
    ///
    /// Advancing one band per frame instead ties the fade to the frame rate,
    /// and it showed the moment the frame rate became the right one: at 25
    /// frames a second a band took 40 ms instead of 10.
    pub ticks_per_band: i32,
    /// The band the curtain has reached, in ticks banked toward the next.
    pub banked: i32,
}

/// One `FADEIN` or `FADEOUT`, as it was started.
///
/// The handlers themselves are faithful — the curtain, the band height and the
/// duration all match. What could not be checked by reading them is the *order*
/// they run in relative to everything that changes the picture, and that is
/// what this records.
#[derive(Debug, Clone)]
pub struct Fade {
    /// `FADEOUT` or `FADEIN`, as the word was named.
    pub name: String,
    /// The screen it ran on.
    pub screen: u32,
    /// Whether the screen was already active when the fade began. A `FADEIN` on
    /// a screen that is active means the picture was on show before the fade.
    pub was_active: bool,
    /// How many descriptors the screen was holding at that moment.
    pub showing: usize,
    /// Its background block, which is what makes one scene look like another.
    pub background: Option<u32>,
}

impl Curtain {
    /// The rows still showing a picture, as a half-open range.
    pub(crate) fn visible(&self) -> (i32, i32) {
        let (_, y, _, h) = self.area;
        if self.opening {
            (y + self.offset, y + h - self.offset)
        } else {
            (y + self.offset + self.step, y + h - self.offset - self.step)
        }
    }

    /// Banks a frame's worth of ticks and moves as many bands as they buy.
    pub(crate) fn advance(&mut self, ticks: i32) {
        self.banked += ticks.max(0);
        let per = self.ticks_per_band.max(1);
        while self.banked >= per && !self.done() {
            self.banked -= per;
            self.offset += if self.opening { -self.step } else { self.step };
        }
    }

    pub(crate) fn done(&self) -> bool {
        let half = self.area.3 / 2;
        if self.opening {
            self.offset < 0
        } else {
            self.offset > half
        }
    }
}
