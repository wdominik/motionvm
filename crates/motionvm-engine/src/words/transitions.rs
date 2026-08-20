//! `FADEIN` and `FADEOUT` — starting a curtain.
//!
//! One of the ten groups `plain_word` hands a word to, in the order the
//! original's own match had them. A group that does not know the word
//! answers `None` and the next one is asked.

use crate::Curtain;
use crate::Engine;
use crate::Fade;
use crate::Result;
use crate::stack::pop_n;
use motionvm_forth::Memory;

impl Engine {
    pub(crate) fn words_transitions(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        _mem: &mut Memory,
    ) -> Result<Option<()>> {
        match name {
            // --- palette ----------------------------------------------------
            // Not a palette fade at all — a curtain. See [`Curtain`] for what
            // the handlers actually do with their three arguments.
            "FADEOUT" | "FADEIN" => {
                let a = pop_n(stack, 3, "FADEIN/FADEOUT")?;
                let (mode, duration) = (a[0], a[1]);
                let opening = name == "FADEIN";
                if mode != 1 {
                    // Mode 2 is not this effect. It is a *translucent* fade:
                    // 0x74eef fills its bands with color 0x102, which the fill
                    // routine reads as shade level 2 in the darkening tables
                    // `SETPAL` builds (0x147ff and its siblings), not as a
                    // color. No call in the game reaches it — all 180 pass
                    // mode 1 — so it is left unbuilt rather than guessed at.
                    self.note_unhandled(format!("{name} (mode {mode})"));
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
                            s.view_pos.0 as i32,
                            s.view_pos.1 as i32,
                            w as i32,
                            s.view.1 as i32,
                        )
                    })
                    .unwrap_or((0, 0, 640, 480));
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
                    .descriptors
                    .iter()
                    .filter(|d| d.active && d.screen == screen)
                    .count();
                let background = self
                    .descriptors
                    .iter()
                    .find(|d| d.active && d.screen == screen && d.block.is_some())
                    .and_then(|d| d.block);
                self.fades.push(Fade {
                    name: name.to_string(),
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
                    self.draw_screen(screen);
                }
                self.curtains.push_back(Curtain {
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
                    ticks_per_band: {
                        let bands = (area.3 / 16).max(1);
                        (duration / bands).max(1)
                    },
                    banked: 0,
                });
            }

            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
