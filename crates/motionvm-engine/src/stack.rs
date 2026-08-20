//! The plumbing between a kernel word and its arguments.
//!
//! Words arrive with their arguments on the Forth data stack and nothing else,
//! so every one of them begins by taking a fixed number off it. These three do
//! that, and turn the two things that can go wrong — too few arguments, and a
//! callback value that does not name anything runnable — into errors that name
//! the word rather than into a panic.

use crate::{Error, Result};
use motionvm_forth::Memory;

/// A callback argument, or 0 where it does not name a word that could be run.
///
/// `NEWSETDESC` does not take what it is given. Zero and -1 clear the field
/// outright (0x70c83-0x70c8f), and anything else is put through three checks
/// before the store at 0x70cc9 — the first two of which are mirrored here:
/// 0x68241 takes the module from bits 16..30 and rejects the address if that
/// module is not loaded, and 0x682a0 additionally rejects an offset at or past
/// the module's own cell count (0x68314). A cell that answers to
/// [`Memory::fetch`] has passed both.
///
/// It matters because the field is used without checking: the frame walk hands
/// +0x14 straight to the interpreter once the wait runs out. Not every call
/// site passes an address — location 1 hands over a plain 177 — and stored
/// unfiltered that is a jump to nowhere.
///
/// The handler's third check (0x68367, on the word header behind the address)
/// is not mirrored: these two already reject everything seen, and the header's
/// layout would have to be guessed.
pub(crate) fn callable(value: i32, mem: &Memory) -> i32 {
    if value == 0 || value == -1 {
        return 0;
    }
    let v = value as u32;
    // The module half is masked here and nowhere else. `Address::new` takes a
    // module number from code that knows it is one; this takes an `i32` out of
    // module memory that may be anything at all, and a value with rubbish in
    // its top bits would otherwise become an address in a module number that
    // cannot exist rather than being rejected. Fourteen bits is far more than
    // the game's numbering needs — the highest module it names is 330.
    let addr = motionvm_forth::Address::new((v >> 16) & 0x3fff, v & 0xffff);
    if mem.fetch(addr).is_ok() { value } else { 0 }
}

/// Pops `n` values, deepest first in the returned vector.
pub(crate) fn pop_n(stack: &mut Vec<i32>, n: usize, word: &'static str) -> Result<Vec<i32>> {
    if stack.len() < n {
        return Err(Error::StackUnderflow {
            word,
            at: motionvm_forth::Address(0),
        });
    }
    let at = stack.len() - n;
    Ok(stack.split_off(at))
}

pub(crate) fn pop1(stack: &mut Vec<i32>, word: &'static str) -> Result<i32> {
    stack.pop().ok_or(Error::StackUnderflow {
        word,
        at: motionvm_forth::Address(0),
    })
}
