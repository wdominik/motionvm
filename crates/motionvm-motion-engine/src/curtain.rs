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
            || self.transitions.slide.is_some()
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
        if self.transitions.slide.is_some() {
            self.advance_slide();
            return;
        }
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
        // it, the way the handler's loop does — or nought, for a curtain that
        // waits for nothing and moves every band at once.
        let ticks = self.step_ticks();
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
    /// One wait of the original's wipe loop is over. A front wipe with
    /// rings left walks the next one and paints it; one whose last ring
    /// has had its wait paints its tail — the whole-view copy at
    /// `05f1:2c5f`, the remaining middle at `05f1:29b8`, neither followed
    /// by a wait — and leaves the queue, with the palette, tune and sample
    /// that waited behind it. A wipe queued behind the finished one then
    /// lays down its first ring at once, as the handlers do (`05f1:2916`,
    /// `05f1:2b1f`: strips first, the spin after), so the picture a step
    /// leaves is always one the original painted right before a wait.
    ///
    /// One ring a call, with no banking: a step lasts exactly the wait at
    /// the front of the queue ([`Engine::step_ticks`]), so there is never
    /// more than one ring's worth to spend — and the finished wipe stays
    /// at the front through its last ring's wait.
    fn advance_wipe(&mut self) {
        let Some(w) = self.transitions.wipes.front_mut() else {
            return;
        };
        if !w.done() {
            w.walked += 1;
            let w = w.clone();
            self.paint_ring(&w);
            return;
        }
        let w = self.transitions.wipes.pop_front().expect("the front wipe");
        self.paint_tail(&w);
        if let Some(p) = w.palette_after {
            self.display.palette = p;
        }
        if let Some((tune, looping)) = w.then_tune {
            self.start_tune(tune, looping);
        }
        if let Some(block) = w.then_sample {
            self.play_sample(block);
        }
        if let Some(next) = self.transitions.wipes.front_mut()
            && !next.done()
            && next.walked == 0
        {
            next.walked = 1;
            let next = next.clone();
            self.paint_ring(&next);
        }
    }

    /// A wipe's end state at once — mode 0's instant form, and the tail
    /// of a walked one: `FADEOUT` fills the view black (`05f1:286b`,
    /// `05f1:29b8`), `FADEIN` copies it whole out of the surface
    /// (`05f1:2a39`, `05f1:2c5f`). Nothing for a hold.
    pub(crate) fn paint_tail(&mut self, wp: &Wipe) {
        if wp.hold {
            return;
        }
        let (x, y, w, h) = wp.area;
        if !wp.opening {
            for row in y..(y + h) {
                for col in x..(x + w) {
                    self.video.set(col, row, 0);
                }
            }
            return;
        }
        self.copy_strips(wp, &[(x, y, w, h)]);
    }

    /// The ring `walked` of a wipe, as the handlers paint it.
    fn paint_ring(&mut self, wp: &Wipe) {
        if wp.hold {
            return;
        }
        let (x, y, w, h) = wp.area;
        if !wp.opening {
            // `FADEOUT` fills the frame between the view and the shrinking
            // inner box with black, ring by ring.
            let (bx, by, bw, bh) = wp.inner();
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
        // `FADEIN` copies strips of the composed surface onto whatever the
        // display holds — it blanks nothing, exactly like the 32-bit band.
        // A ring is four blits (`05f1:2b1f`–`05f1:2c28`): the top strip
        // `x0..=x1` wide and `rh` tall, the left and right strips `rw` wide
        // over the rows between, the bottom strip like the top — and the
        // right strip starts at `x1 − rw`, so the column at `x1` stays
        // whatever it was over those rows until the next ring's strip
        // covers it; the capture of the original shows that notch ring for
        // ring. The rings are cumulative, so each is laid down once, as it
        // is walked.
        let (k, rw, rh) = (wp.walked, wp.xstep, wp.ystep);
        let (cx, cy) = (x + w / 2, y + h / 2);
        let (x0, y0, x1, y1) = (cx - k * rw, cy - k * rh, cx + k * rw, cy + k * rh);
        let mid = y1 - y0 - 2 * rh + 1;
        self.copy_strips(
            wp,
            &[
                (x0, y0, x1 - x0 + 1, rh),
                (x0, y0 + rh, rw, mid),
                (x1 - rw, y0 + rh, rw, mid),
                (x0, y1 - rh, x1 - x0 + 1, rh),
            ],
        );
    }

    /// Blits `strips` of the wipe's screen surface onto the display, each
    /// cut to the view first: the last ring's strips start left of and above
    /// the view and run past its right and bottom edges, and the blitter
    /// paints what falls inside (the capture shows the last ring reaching
    /// the view's last column).
    fn copy_strips(&mut self, wp: &Wipe, strips: &[(i32, i32, i32, i32)]) {
        let (x, y, w, h) = wp.area;
        let Some(s) = self.display.screens.iter().find(|s| s.handle == wp.screen) else {
            return;
        };
        let window = self.display.window(s);
        for &(sx, sy, sw, sh) in strips {
            let (left, top) = (sx.max(x), sy.max(y));
            let (right, bottom) = ((sx + sw).min(x + w), (sy + sh).min(y + h));
            let (sx, sy, sw, sh) = (left, top, right - left, bottom - top);
            if sw <= 0 || sh <= 0 {
                continue;
            }
            let rect = motionvm_render::Rect {
                x: window.x + (sx - x),
                y: window.y + (sy - y),
                w: cell::low16(sw).min(window.w),
                h: cell::low16(sh).min(window.h),
            };
            self.video.copy_from(&s.buffer, rect, sx, sy);
        }
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
    /// Ticks one ring waits — the spin after each ring is painted.
    pub ticks_per_ring: i32,
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
    /// A sample block to play when this wait is over: `PLAYSAMPLE` over a
    /// playing tune spins its half second before it loads the block
    /// (`STERN.EXE` `15e5:036c` then `15e5:03b8`), so the sample starts
    /// where the tune stopped.
    pub(crate) then_sample: Option<i32>,
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
            // Walked already: the one step it stands at the front is the
            // wait, and the next advance takes it off.
            rings: 1,
            walked: 1,
            ticks_per_ring: ticks.max(1),
            palette_after: None,
            hold: true,
            then_tune: None,
            then_sample: None,
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
            palette_after: None,
            hold: false,
            then_tune: None,
            then_sample: None,
        }
    }

    /// The box the picture still fills after `walked` rings of `FADEOUT`.
    ///
    /// Read off the handler's strips, corner by corner, and held to a
    /// capture of the original's supermarket exit ring for ring.
    /// `FADEOUT` (`05f1:2916`–`05f1:2979`) fills with `17f2:0287`, which
    /// takes both corners **inclusive**: its top strip is rows
    /// `y0..=y0+rh`, its left strip columns `x0..=x0+rw`, and the right and
    /// bottom strips end on the view's last column and row — so after `k`
    /// rings the picture stands at `x + k·rw + 1 ..= x + w − 2 − k·rw`, one
    /// pixel narrower on every side than the rings alone would say. (The
    /// opening rings are strips, not a box — see `Engine::paint_ring`.)
    pub(crate) fn inner(&self) -> (i32, i32, i32, i32) {
        let (x, y, w, h) = self.area;
        if self.walked == 0 {
            return (x, y, w, h);
        }
        let (dx, dy) = (self.walked * self.xstep, self.walked * self.ystep);
        (x + dx + 1, y + dy + 1, w - 2 * dx - 2, h - 2 * dy - 2)
    }

    /// Every ring walked and waited for; what is left is the tail.
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
    ///
    /// Nought is a curtain that waits for nothing — R78's, and mode 2's on
    /// either build: every band in the one step it takes, the step costing
    /// the clock nothing (`Profile::curtains_wait`).
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

    /// Banks a frame's worth of ticks and moves as many bands as they buy —
    /// or every band left, for a curtain that waits for nothing.
    pub(crate) fn advance(&mut self, ticks: i32) {
        if self.ticks_per_band <= 0 {
            while !self.done() {
                self.offset += if self.opening { -self.step } else { self.step };
            }
            return;
        }
        self.banked += ticks.max(0);
        let per = self.ticks_per_band;
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
