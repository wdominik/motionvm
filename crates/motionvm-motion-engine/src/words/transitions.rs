//! `FADEIN` and `FADEOUT` — starting a curtain.
//!
//! One of the groups `plain_word32` hands a word to, in the order the
//! original's own match had them — **an order that is load-bearing**: two of
//! the arms match on table membership rather than on a literal, so a group
//! that moves across one of them changes which words it catches. A group that
//! does not know the word answers `None` and the next one is asked.

use crate::Curtain;
use crate::Engine;
use crate::Fade;
use crate::stack::pop_n;
use crate::words::Word;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Result;

impl Engine {
    pub(crate) fn words_transitions(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        _mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match word {
            // --- palette ----------------------------------------------------
            // Not a palette fade at all — a curtain. See [`Curtain`] for what
            // the handlers actually do with their three arguments.
            Word::FADEOUT | Word::FADEIN => {
                let a = pop_n(stack, 3, "FADEIN/FADEOUT")?;
                let (mode, duration) = (a[0], a[1]);
                let opening = word == Word::FADEIN;
                // Both handlers take mode 1 and mode 2 and nothing else.
                // `FADEIN`'s mode 2 (R78 `0x60447`, R109 `0x74b6e`) is the
                // mode-1 curtain with one thing before its band loop: the
                // white box and black frame `WHITEBOX` paints, at 25,122 and
                // 452 by 317 — the same two routines with the same
                // arguments, `0x17310` and `0x17290` on R78 — and no wait
                // between bands on either build. Checker 2000's shell opens
                // every information page with it (module 218, `SI2`), and
                // the page's texts go onto the box. `FADEOUT`'s mode 2 is a
                // different effect — a translucent fade, its bands filled
                // through the darkening tables `SETPAL` builds (R109
                // `0x74eef`, color `0x102`) — which no shipped call reaches,
                // so it stays unbuilt rather than guessed at.
                let boxed = mode == 2 && opening;
                if mode != 1 && !boxed {
                    self.note_unhandled(word, Some(format!("mode {mode}")));
                    return Ok(Some(()));
                }
                let screen = self.display.current.unwrap_or(0);
                // The third argument is popped and thrown away: `-4(%ebp)` is
                // written once in each handler and never read. The band height
                // is the literal 8 at 0x74b11 and 0x74d62. The game passes 8
                // everywhere, so this changes nothing — it stops the argument
                // from looking load-bearing.
                let step = 8;
                let area = self
                    .display
                    .screens
                    .iter()
                    .find(|s| s.handle == screen)
                    .map(|s| {
                        let w = if opening { s.size.0 } else { s.view.0 };
                        (
                            i32::from(s.view_pos.0),
                            i32::from(s.view_pos.1),
                            i32::from(w),
                            i32::from(s.view.1),
                        )
                    })
                    .unwrap_or_else(|| {
                        (
                            0,
                            0,
                            i32::from(self.display.size.width),
                            i32::from(self.display.size.height),
                        )
                    });
                // The flag `GSCRACT` reads is the same one the handlers touch:
                // `FADEOUT` clears bit 0x80 of byte 0x13, `FADEIN` sets it. That
                // coupling is what makes a location fade in at all — `INCLLOC`
                // ends with `GSCRACT NOT IF … FADEIN`, so the picture is only
                // revealed because fading out left the screen inactive.
                //
                // For the report: what the screen was before, and what it is
                // showing. The complaint that fades happen at the wrong moment
                // is about the order of several words, not about one handler,
                // so the order has to be readable.
                let was = self
                    .display
                    .screens
                    .iter()
                    .find(|s| s.handle == screen)
                    .map(|s| s.active);
                let showing = self
                    .scene
                    .descriptors
                    .iter()
                    .filter(|d| d.active && d.screen == screen)
                    .count();
                let background = self
                    .scene
                    .descriptors
                    .iter()
                    .find(|d| d.active && d.screen == screen && d.shows.table().is_some())
                    .and_then(|d| d.shows.graphic());
                self.transitions.fades.push(Fade {
                    name: word.name().to_string(),
                    screen,
                    was_active: was.unwrap_or(false),
                    showing,
                    background,
                });
                if let Some(s) = self.display.current_mut() {
                    s.active = opening;
                }
                // `FADEIN` draws once at 0x74af9, right after setting the
                // screen active, and then reveals what it drew. It hands the
                // drawer *the fading screen*, nothing else — the band loop is
                // synchronous in the handler, so no other screen changes
                // while it runs. `FADEOUT` never calls the drawer at all — it
                // hides whatever the buffers already hold.
                if opening {
                    // `0x6b0fe` before the draw at `0x74af9`: it marks every
                    // descriptor of the screen (through 0x6a8f9), which is what
                    // puts a whole picture back after a `FADEOUT` wiped the
                    // surface. Without it the drawer would repaint only what
                    // had changed since, and a fade would open onto scraps.
                    self.repaint_screen(screen);
                    self.draw_screen(screen);
                    if boxed {
                        // After the draw and before the loop, onto the
                        // surface the bands then reveal (R78 `0x604a8`–
                        // `0x604f8`).
                        self.white_box(25, 122, 452, 317);
                    }
                } else {
                    // `0x74d44`: `FADEOUT` fills the screen's rectangle with
                    // color 0 before the first band moves. It matters because
                    // the surface persists — leave it and the old picture is
                    // still there when the next scene is composed over it.
                    // Everything saved under a descriptor goes with it: those
                    // copies are of a picture that is no longer there.
                    if let Some(s) = self.display.screen_mut(screen) {
                        s.buffer.fill(0);
                        s.paints.clear();
                    }
                    self.forget_rebuilds(screen);
                }
                self.transitions.curtains.push_back(Curtain {
                    opening,
                    screen,
                    area,
                    step,
                    offset: if opening { area.3 / 2 } else { 0 },
                    // `bands = height/16` at 0x74cd0, `ticks = duration/bands` at
                    // 0x74cf6. The handler divides by the band count, not by
                    // the step, and the sixteen there is a constant.
                    //
                    // **The two clamps are ours and the original has neither.**
                    // It divides signed and truncating, so a screen under
                    // sixteen pixels tall gives no bands and divides by zero,
                    // and a duration shorter than the band count gives a delay
                    // of nought — a curtain that waits not at all. Neither can
                    // arise from the shipped data: the three screens are 480,
                    // 400 and 80 tall, giving 30, 25 and 5 bands, and every one
                    // of the 28 fade calls in modules 2, 4 and 5 passes a
                    // duration of 50, so the delays are 1, 2 and 10.
                    //
                    // They are kept rather than removed because the alternative
                    // is a panic reachable only from data that does not exist,
                    // and written down here because a guard nobody knows about
                    // is how a rebuild quietly stops being a reproduction.
                    //
                    // **R78 waits for nothing**, in either mode: its loops
                    // mark and present and go on (`0x6040f`–`0x6043b`), the
                    // duration is popped and never read, and a curtain takes
                    // the presenter's time and no more — on DOSBox-X the
                    // whole fade is over within one to three frames at 70 a
                    // second. Nor does mode 2 wait on R109 (`0x74c2f`–
                    // `0x74c6b`, no timer call). Nought ticks a band is that
                    // pace here: every band in one step, the step costing
                    // the master clock nothing; the presenter's own
                    // milliseconds are not modeled.
                    ticks_per_band: if self.profile.curtains_wait && !boxed {
                        let bands = (area.3 / 16).max(1);
                        (duration / bands).max(1)
                    } else {
                        0
                    },
                    banked: 0,
                    palette_after: None,
                });
            }

            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
