//! Setting the palette.
//!
//! One of the groups `plain_word32` hands a word to, in the order the
//! original's own match had them — **an order that is load-bearing**: two of
//! the arms match on table membership rather than on a literal, so a group
//! that moves across one of them changes which words it catches. A group that
//! does not know the word answers `None` and the next one is asked.

use crate::Engine;
use crate::stack::pop1;
use crate::words::Word;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Result;

use crate::stack::pop_n;
use motionvm_motion_forth::cell;

impl Engine {
    pub(crate) fn words_palette(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        _mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match word {
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
            // the supermarket flashed pink on the way to the shopping center.
            // So while a fade is queued, the palette queues with it, onto the
            // one most recently asked for, and the display takes it when that
            // fade finishes — the script's order, kept on the screen.
            Word::SETPAL => {
                let id = pop1(stack, "SETPAL")?;
                if let Some(p) = self.load_palette(id) {
                    if let Some(w) = self.transitions.wipes.back_mut() {
                        w.palette_after = Some(p);
                    } else if let Some(c) = self.transitions.curtains.back_mut() {
                        c.palette_after = Some(p);
                    } else {
                        self.display.palette = p;
                    }
                }
            }
            Word::CUTPAL => {
                pop1(stack, "CUTPAL")?;
                self.note_no_effect(word);
            }

            // The handler pops blue, green, red and hands them to the
            // engine's nearest-entry lookup (R78 `0x61289` → `0x1b940`, R109
            // `0x75e57` → `0x1ee26`), which answers a palette index, not a
            // packed color. Six-bit components, the same scale the palettes
            // are stored in; the lookup's own metric is in [`crate::paint`].
            Word::RGB_TO_COL => {
                let c = pop_n(stack, 3, "RGB->COL")?;
                let [r, g, b] = [
                    cell::low8(c[0].clamp(0, 63)),
                    cell::low8(c[1].clamp(0, 63)),
                    cell::low8(c[2].clamp(0, 63)),
                ];
                // The script's palette, not the display's: after a `SETPAL`
                // the original's lookup already searches the new entries,
                // even while a fade still hides the switch.
                let best = self.nearest_color(r, g, b);
                stack.push(i32::from(best));
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
