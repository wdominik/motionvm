//! Making screens and configuring them.
//!
//! One of the groups `plain_word` hands a word to, in the order the
//! original's own match had them — **an order that is load-bearing**: two of
//! the arms match on table membership rather than on a literal, so a group
//! that moves across one of them changes which words it catches. A group that
//! does not know the word answers `None` and the next one is asked.

use crate::Engine;
use crate::stack::pop_n;
use crate::stack::pop1;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Result;

impl Engine {
    pub(crate) fn words_screens(
        &mut self,
        name: &str,
        stack: &mut Vec<i32>,
        _mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match name {
            // --- screens ----------------------------------------------------
            "NEWSCREEN" => {
                let h = self.display.new_screen();
                stack.push(h as i32);
            }
            "SCRSIZE" => {
                let a = pop_n(stack, 2, "SCRSIZE")?;
                if let Some(s) = self.display.current_mut() {
                    s.set_size(a[0].max(0) as u16, a[1].max(0) as u16);
                }
            }
            "SCRFVSIZE" | "SCRVSIZE" | "SCRVPOS" | "SCRPOS" => {
                let a = pop_n(stack, 2, "screen word")?;
                let (x, y) = (a[0], a[1]);
                if let Some(s) = self.display.current_mut() {
                    match name {
                        "SCRFVSIZE" => s.full_view = (x.max(0) as u16, y.max(0) as u16),
                        // Through `set_view`, because the visible window is
                        // also what the damage map is measured in.
                        "SCRVSIZE" => s.set_view(x.max(0) as u16, y.max(0) as u16),
                        "SCRVPOS" => s.view_pos = (x as i16, y as i16),
                        _ => s.pos = (x as i16, y as i16),
                    }
                }
            }

            // --- screens, continued -----------------------------------------
            "ACTSCR" => {
                let h = pop1(stack, "ACTSCR")? as u32;
                self.select_screen(h);
            }
            "GSCRACT" => {
                let active = self
                    .display
                    .current
                    .and_then(|h| self.display.screens.iter().find(|s| s.handle == h))
                    .is_some_and(|s| s.active);
                stack.push(i32::from(active));
            }

            // `SCRX` writes +0x24 of a record hanging off the active screen and
            // `GSCRX` reads the same field straight back; `GSCRY` reads +0x26.
            // A getter and a setter on one pair of coordinates, in other words,
            // so it is kept rather than refused. What it shifts is a separate
            // question — the game only ever passes zero — and the renderer
            // therefore does not composite with it yet. A non-zero value is
            // recorded so it cannot pass unnoticed if that ever changes.
            "SCRX" => {
                let n = pop1(stack, "SCRX")?;
                self.set_screen_origin_x(n);
            }
            // Height goes on first, so the width ends up on top — which is
            // how `DOORDER` reads it, adding the first value it pops to an x
            // coordinate.
            "GSCRVSIZE" => {
                let (w, h) = self.screen_view_size();
                stack.push(h);
                stack.push(w);
            }
            "GSCRX" => {
                let x = self.screen_origin_x();
                stack.push(x);
            }
            "GSCRY" => {
                let y = self.screen_origin_y();
                stack.push(y);
            }
            // Bit 2 of the screen's flag byte at +0x13, set and cleared. What a
            // frozen screen does differently is not established yet, so the
            // flag is only carried.
            "FREEZESCR" | "UNFREEZESCR" => {
                let freeze = name == "FREEZESCR";
                if let Some(s) = self.display.current_mut() {
                    s.frozen = freeze;
                }
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
