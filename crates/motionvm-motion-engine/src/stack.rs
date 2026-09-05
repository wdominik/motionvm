//! The plumbing between a kernel word and its arguments.
//!
//! Words arrive with their arguments on the Forth data stack and nothing else,
//! so every one of them begins by taking a fixed number off it. These two do
//! that, and turn the one thing that can go wrong — too few arguments — into
//! an error that names the word rather than into a panic. Screening a callback
//! value is the machine's, [`motionvm_motion_forth::AddressSpace::callable`].
//!
//! **They do not say where.** A word here is handed a stack and a memory and
//! nothing that would locate it; the interpreter stamps the address on the way
//! out ([`Error::located`]), which is the one frame that knows both the word
//! and where it stood. Inventing an address here — module 0, cell 0 — would
//! put a plausible and wrong place in every underflow report.

use motionvm_motion_forth::{Error, Result};

/// Pops `n` values, deepest first in the returned vector.
pub(crate) fn pop_n(stack: &mut Vec<i32>, n: usize, word: &'static str) -> Result<Vec<i32>> {
    if stack.len() < n {
        return Err(Error::StackUnderflow { word, at: None });
    }
    let at = stack.len() - n;
    Ok(stack.split_off(at))
}

pub(crate) fn pop1(stack: &mut Vec<i32>, word: &'static str) -> Result<i32> {
    stack.pop().ok_or(Error::StackUnderflow { word, at: None })
}
