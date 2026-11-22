//! Little-endian fields out of a byte slice, each read refused rather than
//! run past the end.

use crate::error::{Error, Result};

/// The refusal for a read of `need` bytes at `off` that runs past `d`.
fn past_end(d: &[u8], off: usize, need: usize) -> Error {
    Error::Truncated {
        off,
        need,
        have: d.len(),
    }
}

/// `count` records of `N` bytes each, back to back from `off`, or `Err` if
/// the last of them would run past the end.
fn records<const N: usize>(d: &[u8], off: usize, count: usize) -> Result<&[[u8; N]]> {
    d.get(off..)
        .and_then(|rest| rest.as_chunks().0.get(..count))
        .ok_or_else(|| past_end(d, off, count.saturating_mul(N)))
}

/// `N` little-endian `u16`s back to back at `off`.
pub(crate) fn words<const N: usize>(d: &[u8], off: usize) -> Result<[u16; N]> {
    let raw = records::<2>(d, off, N)?;
    let mut out = [0u16; N];
    for (word, bytes) in out.iter_mut().zip(raw) {
        *word = u16::from_le_bytes(*bytes);
    }
    Ok(out)
}

/// `N` little-endian `u32`s back to back at `off`.
pub(crate) fn dwords<const N: usize>(d: &[u8], off: usize) -> Result<[u32; N]> {
    let raw = records::<4>(d, off, N)?;
    let mut out = [0u32; N];
    for (word, bytes) in out.iter_mut().zip(raw) {
        *word = u32::from_le_bytes(*bytes);
    }
    Ok(out)
}

/// Reads a little-endian `u16` at `off`, or `Err` if it would run past the end.
pub(crate) fn u16le(d: &[u8], off: usize) -> Result<u16> {
    d.get(off..)
        .and_then(|rest| rest.first_chunk())
        .map(|b| u16::from_le_bytes(*b))
        .ok_or_else(|| past_end(d, off, 2))
}

/// A 32-bit offset read out of a file, as an index into it.
///
/// Lossless on every target this workspace builds for, all of which have a
/// `usize` at least 32 bits wide; `usize` has no `From<u32>` because Rust
/// does not promise that for every target it has.
#[expect(
    clippy::as_conversions,
    reason = "a widening on every target this builds for; `usize` has no `From<u32>`"
)]
pub(crate) fn wide(n: u32) -> usize {
    n as usize
}
