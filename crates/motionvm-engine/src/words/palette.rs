//! Setting the palette.
//!
//! One of the ten groups `plain_word` hands a word to, in the order the
//! original's own match had them. A group that does not know the word
//! answers `None` and the next one is asked.

use crate::Engine;
use crate::stack::pop1;
use motionvm_forth::AddressSpace;
use motionvm_forth::Result;

use crate::stack::pop_n;

impl Engine {
    pub(crate) fn words_palette(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        _mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match name {
            // --- palette ----------------------------------------------------
            // Selects the palette a later draw uses. Loading it here keeps the
            // colors right even before anything is drawn.
            //
            // The original programs the DAC right here (`05f1:01ff` calls the
            // set-palette primitive at `116a:00ef`; 32-bit alike), and that is
            // safe there because the fades run *inside* their words —
            // `FADEOUT` (`05f1:2827`) spins its rings before it returns, so by
            // the time a script's `SETPAL` runs, the view it recolors is
            // black. This engine replays queued fades after the word instead
            // (see `Wipe`), and one script path queues without pausing: the
            // LEAVE verb's `CALCLEAVE` (module 606) runs `INCLLOC` through the
            // order machine's nested call — walking out of any door. Applied
            // immediately, the new location's palette lands on the old
            // location's still-standing picture for the whole closing wipe;
            // the supermarket flashed pink on the way to the shopping centre.
            // So while a fade is queued, the palette queues with it, onto the
            // one most recently asked for, and the display takes it when that
            // fade finishes — the script's order, kept on the screen.
            "SETPAL" => {
                let id = pop1(stack, "SETPAL")?;
                if let Some(p) = self.load_palette(id) {
                    if let Some(w) = self.wipes.back_mut() {
                        w.palette_after = Some(p);
                    } else if let Some(c) = self.curtains.back_mut() {
                        c.palette_after = Some(p);
                    } else {
                        self.display.palette = p;
                    }
                }
            }
            "CUTPAL" => {
                pop1(stack, "CUTPAL")?;
                self.note_no_effect(name);
            }

            // The handler pops blue, green, red and hands them to a lookup that
            // returns a single 16-bit value: a palette index, not a packed
            // color. Six-bit components, the same scale the palettes are
            // stored in.
            "RGB->COL" => {
                let c = pop_n(stack, 3, "RGB->COL")?;
                let want = [
                    c[0].clamp(0, 63) as u8,
                    c[1].clamp(0, 63) as u8,
                    c[2].clamp(0, 63) as u8,
                ];
                // The script's palette, not the display's: after a `SETPAL`
                // the original's lookup already searches the new entries,
                // even while a fade still hides the switch.
                let pal = self.script_palette();
                // Nearest entry rather than an exact match: the game asks for
                // pure white, and whether the palette in force holds exactly
                // 63,63,63 is not something a caller can know.
                let best = (0..256)
                    .min_by_key(|&i| {
                        let o = i * 3;
                        want.iter()
                            .zip(&pal.raw[o..o + 3])
                            .map(|(&a, &b)| {
                                let d = a as i32 - (b & 63) as i32;
                                d * d
                            })
                            .sum::<i32>()
                    })
                    .unwrap_or(0);
                stack.push(best as i32);
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
