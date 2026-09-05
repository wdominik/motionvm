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
use motionvm_motion_forth::cell;

impl Engine {
    /// Whether a transition is running, and so whether the interpreter is held.
    pub fn in_transition(&self) -> bool {
        !self.transitions.curtains.is_empty()
            || !self.transitions.wipes.is_empty()
            || self.transitions.scroll.is_some()
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
    /// One step of a running `->SCRX`/`->SCRY` scroll: the window moves
    /// `step` pixels toward the target and the frame is presented — the
    /// 16-bit handler (`ENVIRO.EXE` file `0xc149`) blits exactly one such
    /// step per `DELAY` tick out of its own loop, the surface untouched.
    fn advance_scroll(&mut self) {
        let Some(sc) = self.transitions.scroll else {
            return;
        };
        let mut done = true;
        if let Some(s) = self.display.screen_mut(sc.screen) {
            let pos = if sc.vertical {
                &mut s.pos.1
            } else {
                &mut s.pos.0
            };
            let step = cell::short(sc.step.max(1));
            let target = cell::short(sc.target);
            if *pos < target {
                *pos = (*pos + step).min(target);
            } else if *pos > target {
                *pos = (*pos - step).max(target);
            }
            done = *pos == target;
        }
        self.present();
        if done {
            self.transitions.scroll = None;
        }
    }

    pub(crate) fn advance_curtain(&mut self) {
        if self.transitions.scroll.is_some() {
            self.advance_scroll();
            return;
        }
        if !self.transitions.wipes.is_empty() {
            self.advance_wipe();
            return;
        }
        // A step, not a frame — see [`Self::step_ticks`]. During a curtain this
        // is exactly one band's worth, so one call moves one band and paints
        // it, the way the handler's loop does.
        let ticks = self.step_ticks().max(1);
        let Some(c) = self.transitions.curtains.front_mut() else {
            return;
        };
        c.advance(ticks);
        let c = c.clone();
        self.paint_curtain(&c);
        if c.done() {
            self.transitions.curtains.pop_front();
            if let Some(p) = c.palette_after {
                self.display.palette = p;
            }
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
            w: cell::low16(w).min(window.w),
            h: cell::low16(bottom - top),
        };
        self.video.copy_from(&s.buffer, band, x, top);
    }
}

impl Engine {
    /// One ring of a running 16-bit box wipe, painted the way the handlers
    /// paint theirs — see [`Wipe`].
    fn advance_wipe(&mut self) {
        let ticks = self.step_ticks().max(1);
        let Some(w) = self.transitions.wipes.front_mut() else {
            return;
        };
        w.advance(ticks);
        let w = w.clone();
        self.paint_wipe(&w);
        if w.done() {
            self.transitions.wipes.pop_front();
            if let Some(p) = w.palette_after {
                self.display.palette = p;
            }
            if let Some((tune, looping)) = w.then_tune {
                self.start_tune(tune, looping);
            }
        }
    }

    /// Paints a wipe's current state at once — mode 0's instant form.
    pub(crate) fn paint_wipe_now(&mut self, wp: &Wipe) {
        self.paint_wipe(wp);
    }

    fn paint_wipe(&mut self, wp: &Wipe) {
        if wp.hold {
            return;
        }
        let (x, y, w, h) = wp.area;
        let (bx, by, bw, bh) = wp.inner();
        if !wp.opening {
            // `FADEOUT` fills the frame between the view and the shrinking
            // inner box with black, ring by ring; the tail fill at
            // `05f1:29b8` takes the remaining middle when the rings are out.
            for row in y..(y + h) {
                for col in x..(x + w) {
                    let inside = col >= bx && col < bx + bw && row >= by && row < by + bh;
                    if !inside {
                        self.video.set(col, row, 0);
                    }
                }
            }
            return;
        }
        // `FADEIN` copies the grown box out of the composed surface onto
        // whatever the display holds — it blanks nothing, exactly like the
        // 32-bit band (`05f1:2b1f`–`05f1:2c28`; full view at `05f1:2c5f`).
        if bw <= 0 || bh <= 0 {
            return;
        }
        let Some(s) = self.display.screens.iter().find(|s| s.handle == wp.screen) else {
            return;
        };
        let window = self.display.window(s);
        let rect = motionvm_render::Rect {
            x: window.x + (bx - x),
            y: window.y + (by - y),
            w: cell::low16(bw).min(window.w),
            h: cell::low16(bh).min(window.h),
        };
        self.video.copy_from(&s.buffer, rect, bx, by);
    }
}

/// One box transition, as the 16-bit engine's `FADEIN` and `FADEOUT` in
/// mode 1 draw it (`ENVIRO.EXE` `05f1:29e4`, `05f1:2827`).
///
/// Both take `( mode duration step -- )` like their 32-bit namesakes, but
/// the picture moves as a rectangle, not a band: `FADEOUT` fills four
/// strips a ring, the black frame growing in from the view's edges;
/// `FADEIN` composes the screen once, then blits the same strips out of
/// the surface, the box growing from the center until a final whole-view
/// copy squares the rounding. A ring is `(width / (2·step) + 7) & !7`
/// wide and `height / (2·step)` tall, one fewer ring where that width
/// times the rings would overrun the view, and each ring waits
/// `200 / duration` ticks of the 200 Hz clock (`05f1:2998`) — `1 50 8`,
/// the game's one shape, is seven rings of 24×12 at 4 ticks each.
#[derive(Debug, Clone)]
pub struct Wipe {
    /// `FADEIN` opens, `FADEOUT` closes.
    pub opening: bool,
    /// The screen the effect belongs to.
    pub screen: u32,
    /// That screen's view rectangle on the display (`+0x10`, `+0x12`,
    /// `+8`, `+0xA` of the screen block).
    pub area: (i32, i32, i32, i32),
    /// One ring's width, rounded up to a multiple of eight (`05f1:28db`).
    pub xstep: i32,
    /// One ring's height, truncated (`05f1:28eb`).
    pub ystep: i32,
    /// How many rings there are to walk (`05f1:2905` may take one off the
    /// argument).
    pub rings: i32,
    /// How many have been walked.
    pub walked: i32,
    /// Ticks one ring waits, and how many are banked toward the next.
    pub ticks_per_ring: i32,
    /// Banked ticks.
    pub banked: i32,
    /// A palette a later `SETPAL` asked for while this wipe still waited —
    /// installed when the wipe finishes, so the script's order stays the
    /// screen's order. See the `SETPAL` handler for the path that needs it.
    pub palette_after: Option<motionvm_render::Palette>,
    /// A wait and not a wipe: the interpreter stands for `ticks_per_ring`
    /// ticks and nothing is painted — `ENDTUNE`'s half second, see the
    /// constructor `Wipe::hold`.
    pub hold: bool,
    /// A tune to start when this wait is over — `( tune looping )` as
    /// `STARTTUNE` popped them. The 16-bit Play routine (`1696:02ce`,
    /// `0e87:01f2`) runs the stop routine first when a tune is playing, so
    /// the new one begins only after the old one's half-second wait.
    pub(crate) then_tune: Option<(i32, i32)>,
}

impl Wipe {
    /// A plain wait of `ticks` unit-3 ticks, queued where a wipe would be so
    /// that it takes its turn among them.
    ///
    /// What `ENDTUNE` does after starting the music's fade-out: its stop
    /// routine (`ENVIRO.EXE` `1696:02fd`, `HPPLAY.EXE` `1639:02ef`,
    /// `BMZ.EXE` `166d:02f1`, `LL.EXE` `0e87:0221`) resets the tick
    /// counter and spins until it reads 100 — the 200 Hz reading of the
    /// driver's millisecond count (`110a:042e`: `ds:7bb8 / 5`) — before it
    /// stops the driver and returns to the script. Half a second in which
    /// the frame loop does not turn, so the frontend sees one step that
    /// long. A `SETPAL` behind it attaches the way it attaches to a wipe,
    /// and lands when the wait is over — which is when the original's,
    /// issued after the handler returned, would have been programmed.
    pub(crate) fn hold(ticks: i32) -> Self {
        Wipe {
            opening: false,
            screen: 0,
            area: (0, 0, 0, 0),
            xstep: 0,
            ystep: 0,
            rings: 1,
            walked: 0,
            ticks_per_ring: ticks.max(1),
            banked: 0,
            palette_after: None,
            hold: true,
            then_tune: None,
        }
    }

    /// Builds the wipe the way the handler sets one up.
    pub(crate) fn new(
        opening: bool,
        screen: u32,
        area: (i32, i32, i32, i32),
        duration: i32,
        step: i32,
    ) -> Self {
        let (_, _, w, h) = area;
        let step = step.max(1);
        let xstep = ((w / (2 * step) + 7) & !7).max(8);
        let ystep = (h / (2 * step)).max(1);
        let rings = if xstep * step * 2 > w { step - 1 } else { step };
        Wipe {
            opening,
            screen,
            area,
            xstep,
            ystep,
            rings: rings.max(0),
            walked: 0,
            // `200 / duration` (`05f1:299e`); the clamp is ours — the game
            // passes 50 everywhere, and a zero would spin the ring counter
            // forever where the original would simply not wait.
            ticks_per_ring: (200 / duration.max(1)).max(1),
            banked: 0,
            palette_after: None,
            hold: false,
            then_tune: None,
        }
    }

    /// The box the picture still fills (closing) or already fills
    /// (opening), as the walked rings leave it.
    pub(crate) fn inner(&self) -> (i32, i32, i32, i32) {
        let (x, y, w, h) = self.area;
        if self.opening {
            let (cx, cy) = (x + w / 2, y + h / 2);
            if self.walked >= self.rings {
                return (x, y, w, h);
            }
            let (hw, hh) = (self.walked * self.xstep, self.walked * self.ystep);
            (cx - hw, cy - hh, hw * 2, hh * 2)
        } else {
            if self.walked >= self.rings {
                return (x, y, 0, 0);
            }
            let (dx, dy) = (self.walked * self.xstep, self.walked * self.ystep);
            (x + dx, y + dy, w - 2 * dx, h - 2 * dy)
        }
    }

    /// Banks a frame's worth of ticks and walks as many rings as they buy.
    pub(crate) fn advance(&mut self, ticks: i32) {
        self.banked += ticks.max(0);
        let per = self.ticks_per_ring.max(1);
        while self.banked >= per && !self.done() {
            self.banked -= per;
            self.walked += 1;
        }
    }

    pub(crate) fn done(&self) -> bool {
        self.walked >= self.rings
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
    /// A palette a later `SETPAL` asked for while this curtain still waited —
    /// installed when the curtain finishes, so the script's order stays the
    /// screen's order. See the `SETPAL` handler for the path that needs it.
    pub palette_after: Option<motionvm_render::Palette>,
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
