//! The four timer words of the 32-bit kernel.
//!
//! One of the groups `plain_word32` hands a word to. Checker 2000's puzzle
//! (module 204) is the one caller: `OPENTIMER _THANDLE ! _THANDLE @ 0
//! SETTIMER` when the puzzle begins, `GIVETIMER` as it runs, `CLOSETIMER`
//! at the end. The objects themselves, and the arithmetic read from R78's
//! four routines, are in [`crate::clock`].

use crate::Engine;
use crate::stack::{pop_n, pop1};
use crate::words::Word;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Result;

impl Engine {
    pub(crate) fn words_timers(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        _mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match word {
            // `( -- handle )`: kind 3, always — `0x56b8e` passes the constant.
            // The handler pushes the eight-byte allocation's address; the
            // scripts only ever hand it back.
            Word::OPENTIMER => {
                let handle = self.open_timer(3);
                stack.push(handle);
            }
            // `( handle reading -- )`: the reading on top (`0x56c13` pops it
            // first), the handle under it.
            Word::SETTIMER => {
                let a = pop_n(stack, 2, "SETTIMER")?;
                let (handle, reading) = (a[0], a[1]);
                self.set_timer(handle, reading);
            }
            // `( handle -- reading )`. A handle nothing opened reads what
            // the original would read through a stray pointer — nothing
            // this port can reproduce — and answers zero here.
            Word::GIVETIMER => {
                let handle = pop1(stack, "GIVETIMER")?;
                stack.push(self.give_timer(handle).unwrap_or(0));
            }
            // `( handle -- )`.
            Word::CLOSETIMER => {
                let handle = pop1(stack, "CLOSETIMER")?;
                self.close_timer(handle);
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
