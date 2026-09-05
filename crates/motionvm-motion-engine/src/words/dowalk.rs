//! `DOWALK` — the figure's walk, over either machine's module memory.
//!
//! One of the groups `plain_word32` hands a word to, in the order the
//! original's own match had them — **an order that is load-bearing**: two of
//! the arms match on table membership rather than on a literal, so a group
//! that moves across one of them changes which words it catches. A group that
//! does not know the word answers `None` and the next one is asked.

use crate::Engine;
use crate::stack::pop1;
use crate::walk;
use crate::words::Word;
use motionvm_motion_forth::AddressSpace;
use motionvm_motion_forth::Result;
use motionvm_motion_forth::cell;

impl Engine {
    pub(crate) fn words_dowalk(
        &mut self,
        word: Word,
        stack: &mut Vec<i32>,
        mem: &mut dyn AddressSpace,
    ) -> Result<Option<()>> {
        match word {
            // A figure's walk is a command queue plus a gate, and both live in
            // [`walk`] — see there for the whole of it.
            Word::DOWALK => {
                let person = cell::unsigned(pop1(stack, "DOWALK")?);
                walk::do_walk(self, mem, person)?;
            }
            _ => return Ok(None),
        }
        Ok(Some(()))
    }
}
