//! The plumbing between a kernel word and its arguments.
//!
//! Words arrive with their arguments on the Forth data stack and nothing else,
//! so every one of them begins by taking a fixed number off it. These three do
//! that, and turn the one thing that can go wrong — too few arguments — into
//! an error that names the word rather than into a panic. Screening a callback
//! value is the machine's, [`motionvm_forth::AddressSpace::callable`].

use crate::{Error, Result};

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
