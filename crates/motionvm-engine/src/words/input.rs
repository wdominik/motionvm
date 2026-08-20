//! Input, timers, the inventory bar, and what did not fit elsewhere.
//!
//! One of the ten groups `plain_word` hands a word to, in the order the
//! original's own match had them. A group that does not know the word
//! answers `None` and the next one is asked.

use crate::Engine;
use crate::Result;
use crate::stack::pop1;
use crate::walk;
use motionvm_forth::Memory;

impl Engine {
    pub(crate) fn words_input(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        mem: &mut Memory,
    ) -> Result<Option<()>> {
        match name {
            // A figure's walk is a command queue plus a gate, and both live in
            // [`walk`] — see there for the whole of it.
            "DOWALK" => {
                let person = pop1(stack, "DOWALK")? as u32;
                walk::do_walk(self, mem, person)?;
            }
            // The key that is waiting, or zero for none.
            //
            // **An ASCII code, not a flag.** `ICTRL` opens with
            // `?KEY DUP _AKTKEY !` (module 4, `0x022a0`) and then tests
            // `_AKTKEY` against codes: 27 for Escape at `0x02c40`, 13 for
            // Return, 103 and 105 for the debug viewer and input line, 8 for
            // backspace, and the range 48…57 for the teleport digits. A `0` or
            // `1` here makes every one of those comparisons false and no key
            // does anything at all.
            //
            // The `0 >` form the game also uses is the "any key" test, and it
            // reads correctly either way — which is how a boolean can sit here
            // looking as if it works.
            "?KEY" => stack.push(self.key),
            // How far the walker moves in one step. Zero means one — the
            // handler substitutes it — and the walk is the only thing that
            // reads it back, in both halves: see [`walk`].
            "STEPMULTI" => {
                let n = pop1(stack, "STEPMULTI")?;
                self.step_multi = if n == 0 { 1 } else { n };
            }
            // Clears `ANIMPLAY`'s running flag at 0xdb4a4, which is what ends
            // the game's main loop and lets `START` run on into `ENDGAME`.
            "QUITANIM" => self.main_loop = false,
            // Not a wait, despite the name. The handler divides: 0xdb4b8
            // becomes 200/n, or -1 for n = -1. With `START`'s `25 DELAY` that
            // is 8 ticks a frame out of a 200-tick second — the game asks to
            // run at 25 frames a second, and `n` is simply that rate.
            "DELAY" => {
                let n = pop1(stack, "DELAY")?;
                self.frame_ticks = if n == -1 {
                    -1
                } else if n == 0 {
                    0
                } else {
                    200 / n
                };
            }
            // The loop the whole game runs in. Its handler at 0x68dff takes
            // three timers, sets the running flag at 0xdb4a4, and then, for as
            // long as that flag holds, steps the animations and executes the
            // word `CTRL` left at 0xdb4a8 — once per frame, gated on at least
            // four ticks having passed.
            //
            // It is native there and it stays native here: the word marks the
            // loop as entered and stops the interpreter, and the frontend's
            // clock drives the controller. `START` pushes ten values before
            // calling it and the handler pops none of them, so they are left
            // alone here too.
            //
            // With no controller registered the handler jumps straight to its
            // exit, so there is nothing to enter.
            "ANIMPLAY" => {
                if self.controller.is_some() {
                    self.main_loop = true;
                    self.entering_loop = true;
                }
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
