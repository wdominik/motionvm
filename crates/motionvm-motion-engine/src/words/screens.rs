//! Making screens and configuring them.
//!
//! One of the groups `plain_word32` hands a word to, in the order the
//! original's own match had them — **an order that is load-bearing**: two of
//! the arms match on table membership rather than on a literal, so a group
//! that moves across one of them changes which words it catches. A group that
//! does not know the word answers `None` and the next one is asked.

use crate::Engine;
use crate::stack::pop_n;
use crate::stack::pop1;
use crate::words::Word;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Result;
use motionvm_motion_forth::cell;

impl Engine {
    pub(crate) fn words_screens(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        _mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match word {
            // --- screens ----------------------------------------------------
            Word::NEWSCREEN => {
                let h = self.display.new_screen();
                stack.push(cell::signed(h));
            }
            Word::SCRSIZE => {
                let a = pop_n(stack, 2, "SCRSIZE")?;
                if let Some(s) = self.display.current_mut() {
                    s.set_size(cell::low16(a[0].max(0)), cell::low16(a[1].max(0)));
                }
            }
            Word::SCRFVSIZE | Word::SCRVSIZE | Word::SCRVPOS | Word::SCRPOS => {
                let a = pop_n(stack, 2, "screen word")?;
                let (x, y) = (a[0], a[1]);
                if let Some(s) = self.display.current_mut() {
                    match word {
                        Word::SCRFVSIZE => {
                            s.full_view = (cell::low16(x.max(0)), cell::low16(y.max(0)));
                        }
                        // Through `set_view`, because the visible window is
                        // also what the damage map is measured in.
                        Word::SCRVSIZE => s.set_view(cell::low16(x.max(0)), cell::low16(y.max(0))),
                        Word::SCRVPOS => s.view_pos = (cell::short(x), cell::short(y)),
                        _ => s.pos = (cell::short(x), cell::short(y)),
                    }
                }
            }

            // --- screens, continued -----------------------------------------
            Word::ACTSCR => {
                let h = cell::unsigned(pop1(stack, "ACTSCR")?);
                self.select_screen(h);
            }
            Word::GSCRACT => {
                let active = self
                    .display
                    .current
                    .and_then(|h| self.display.screens.iter().find(|s| s.handle == h))
                    .is_some_and(|s| s.active);
                stack.push(i32::from(active));
            }

            // `( x -- )` and `( y -- )`: one half of the scroll register
            // `SCRPOS` writes whole — `+0x24` and `+0x26` of the record
            // hanging off the active screen, which `GSCRX`/`GSCRY` read back
            // — and a whole redraw when the value changes (R78 `0x5ee40`;
            // see [`Engine::set_screen_scroll`]). Dunkle Schatten 2 only
            // ever writes zero; Checker 2000 sets `SCRY` once.
            Word::SCRX => {
                let n = pop1(stack, "SCRX")?;
                self.set_screen_scroll(false, n);
            }
            Word::SCRY => {
                let n = pop1(stack, "SCRY")?;
                self.set_screen_scroll(true, n);
            }
            // `( x -- )` and `( y -- )`: the view slides to a new scroll
            // position, four pixels a refresh, over a copy of the old and the
            // new picture laid side by side — see [`crate::Slide`] for the
            // reading of R78 `0x60690`. Run as a transition: the interpreter
            // is held, as the handler holds it, until the last step.
            Word::TO_SCRX | Word::TO_SCRY => {
                let target = pop1(stack, word.name())?;
                self.start_slide(word == Word::TO_SCRY, target);
            }
            // Height goes on first, so the width ends up on top — which is
            // how `DOORDER` reads it, adding the first value it pops to an x
            // coordinate.
            Word::GSCRVSIZE => {
                let (w, h) = self.screen_view_size();
                stack.push(h);
                stack.push(w);
            }
            Word::GSCRX => {
                let x = self.screen_origin_x();
                stack.push(x);
            }
            Word::GSCRY => {
                let y = self.screen_origin_y();
                stack.push(y);
            }
            // Bit 2 of the screen's flag byte at +0x13, set and cleared. What a
            // frozen screen does differently is not established yet, so the
            // flag is only carried.
            Word::FREEZESCR | Word::UNFREEZESCR => {
                let freeze = word == Word::FREEZESCR;
                if let Some(s) = self.display.current_mut() {
                    s.frozen = freeze;
                }
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
