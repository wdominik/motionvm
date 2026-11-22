//! The RIB LZ codec: in-place, backwards, byte-oriented.
//!
//! The original decoder is `1E05h:0000h` in Dunkle Schatten's unpacked
//! `XP01` (`0x1E050`). It copies the compressed payload to the start of a `bufSize` buffer, sets
//! the direction flag, and walks two pointers toward each other: tokens are
//! consumed from the *end* of the payload, output is written from the *end*
//! of the buffer, and they stop when they meet. There is no end marker and
//! no checksum.
//!
//! Token dispatch is on the high nibble. **16-bit operands are high-byte-first
//! when the stream is read backwards** — the first byte consumed is the high
//! byte.
//!
//! This is **not** the LZEXE bit-stream used on the packed executables.

use crate::error::{LzError, Result};

/// The largest buffer a RIB header may ask for: 8 MiB, which is already
/// generous for a DOS game — the largest decoded RIB Dunkle Schatten ships
/// is 120 088 bytes, slot 12 of `DSROOM.DAT`.
const MAX_BUF: usize = 0x80_0000;

/// Decompresses a RIB LZ payload into a buffer of `buf_size` bytes.
///
/// A payload exactly `buf_size` long is stored, not packed: the decoder
/// compares the two lengths first (`0x1E059`–`0x1E064`) and, when they are
/// equal, returns at once (`0x1E066` to the exit at `0x1E2E7`) with the
/// bytes left as they were copied. No RIB in Dunkle Schatten's files takes
/// that path, and it is the program's, so it stays.
pub fn decompress(payload: &[u8], buf_size: usize) -> Result<Vec<u8>> {
    if payload.len() == buf_size {
        return Ok(payload.to_vec());
    }
    // A damaged header can ask for a huge buffer. Capping the reservation is
    // what stops a truncated download from aborting the process before the
    // decoder has a chance to report the real problem.
    if buf_size > MAX_BUF {
        return Err(crate::Error::OutOfRange {
            field: "RIB bufSize",
            value: crate::long(buf_size),
            allowed: "at most 8 MiB",
        });
    }

    let mut buf = vec![0u8; buf_size];
    let n = payload.len().min(buf_size);
    if let (Some(to), Some(from)) = (buf.get_mut(..n), payload.get(..n)) {
        to.copy_from_slice(from);
    }

    let mut src = n;
    let mut dst = buf_size;

    while dst > src {
        let token = pull_byte(&buf, &mut src)?;
        let [nibble, hi] = nibbles(token);
        match hi {
            0 => {
                // RLE, count n+4, value = next byte.
                let value = pull_byte(&buf, &mut src)?;
                write_run(&mut buf, &mut dst, biased(nibble, 4), value)?;
            }
            1 => {
                // RLE, count ((n<<8)|next)+20, value = following byte.
                let extra = pull_byte(&buf, &mut src)?;
                let value = pull_byte(&buf, &mut src)?;
                write_run(&mut buf, &mut dst, twelve(nibble, extra, 20), value)?;
            }
            2 => {
                // Literal run of n+1 bytes.
                write_literals(&mut buf, &mut src, &mut dst, biased(nibble, 1))?;
            }
            3 => {
                // Literal run of ((n<<8)|next)+17 bytes.
                let extra = pull_byte(&buf, &mut src)?;
                write_literals(&mut buf, &mut src, &mut dst, twelve(nibble, extra, 17))?;
            }
            4 => match token {
                0x40 => {
                    let count = usize::from(pull_u16(&buf, &mut src)?);
                    write_literals(&mut buf, &mut src, &mut dst, count)?;
                }
                0x41 => {
                    let dist = usize::from(pull_u16(&buf, &mut src)?);
                    let length = usize::from(pull_u16(&buf, &mut src)?);
                    write_match(&mut buf, &mut dst, dist, length)?;
                }
                0x42 => {
                    let dist = usize::from(pull_u16(&buf, &mut src)?);
                    let extra = pull_byte(&buf, &mut src)?;
                    write_match(&mut buf, &mut dst, dist, biased(extra, 17))?;
                }
                _ => {
                    // 0x43..=0x4F: match, 16-bit distance, length n+1 (4..=16).
                    let dist = usize::from(pull_u16(&buf, &mut src)?);
                    write_match(&mut buf, &mut dst, dist, biased(nibble, 1))?;
                }
            },
            5 => {
                // Match, distance ((n<<8)|next)+2, length = nextbyte+14 (14..=269).
                let extra = pull_byte(&buf, &mut src)?;
                let dist = twelve(nibble, extra, 2);
                let extra2 = pull_byte(&buf, &mut src)?;
                write_match(&mut buf, &mut dst, dist, biased(extra2, 14))?;
            }
            _ => {
                // 0x6n..=0xFn: match, distance ((n<<8)|next)+2, length = hi-2 (4..=13).
                let extra = pull_byte(&buf, &mut src)?;
                let dist = twelve(nibble, extra, 2);
                write_match(&mut buf, &mut dst, dist, usize::from(hi).saturating_sub(2))?;
            }
        }
    }
    Ok(buf)
}

/// A token's low and high nibble, the high one the dispatch.
fn nibbles(token: u8) -> [u8; 2] {
    [token & 0x0f, token.wrapping_shr(4)]
}

/// A byte operand plus the bias its token adds.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a byte plus a bias below 32 fits sixteen bits"
)]
fn biased(v: u8, bias: u16) -> usize {
    usize::from(u16::from(v) + bias)
}

/// `((n<<8)|next) + bias`: twelve bits out of the token's low nibble and
/// the byte after it, plus the bias the token adds.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "twelve bits plus a bias below 32 fit sixteen"
)]
fn twelve(nibble: u8, next: u8, bias: u16) -> usize {
    usize::from(u16::from_le_bytes([next, nibble]) + bias)
}

/// The byte below `src`, which then stands on it: the stream is read from
/// its end.
fn pull_byte(buf: &[u8], src: &mut usize) -> Result<u8> {
    let at = src
        .checked_sub(1)
        .ok_or(LzError::UnexpectedEof { remaining: 0 })?;
    let b = buf
        .get(at)
        .copied()
        .ok_or(LzError::UnexpectedEof { remaining: 0 })?;
    *src = at;
    Ok(b)
}

/// 16-bit operand, high-byte-first because the stream is read backwards.
fn pull_u16(buf: &[u8], src: &mut usize) -> Result<u16> {
    let hi = pull_byte(buf, src)?;
    let lo = pull_byte(buf, src)?;
    Ok(u16::from_le_bytes([lo, hi]))
}

/// Where a run of `count` bytes ending at `dst` starts, or the refusal
/// when it would start before the buffer.
fn run_start(dst: usize, count: usize) -> Result<usize> {
    Ok(dst
        .checked_sub(count)
        .ok_or(LzError::Underrun { count, dst })?)
}

/// Writes `b` at `at`, which every caller has already placed below `dst`
/// and so inside the buffer.
fn put(buf: &mut [u8], at: usize, b: u8) {
    if let Some(slot) = buf.get_mut(at) {
        *slot = b;
    }
}

fn write_run(buf: &mut [u8], dst: &mut usize, count: usize, value: u8) -> Result<()> {
    let start = run_start(*dst, count)?;
    for at in (start..*dst).rev() {
        put(buf, at, value);
    }
    *dst = start;
    Ok(())
}

fn write_literals(buf: &mut [u8], src: &mut usize, dst: &mut usize, count: usize) -> Result<()> {
    let start = run_start(*dst, count)?;
    for at in (start..*dst).rev() {
        let b = pull_byte(buf, src)?;
        put(buf, at, b);
    }
    *dst = start;
    Ok(())
}

fn write_match(buf: &mut [u8], dst: &mut usize, dist: usize, length: usize) -> Result<()> {
    let start = run_start(*dst, length)?;
    for at in (start..*dst).rev() {
        let b = at
            .checked_add(dist)
            .and_then(|from| buf.get(from))
            .copied()
            .ok_or(LzError::BadDistance { dist, dst: at })?;
        put(buf, at, b);
    }
    *dst = start;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stored payload is returned unchanged — the ratio-1.0 path.
    #[test]
    fn a_stored_payload_is_copied() {
        let p = vec![1, 2, 3, 4];
        assert_eq!(decompress(&p, 4).unwrap(), p);
    }

    /// A token that would write before the start of the buffer is refused
    /// rather than wrapping.
    #[test]
    fn a_run_that_overruns_the_buffer_is_refused() {
        // bufSize 4, a 0x0F RLE (count 19) cannot fit.
        let payload = [0x0F, 0xAA];
        assert!(decompress(&payload, 4).is_err());
    }

    /// An impossible bufSize is refused before allocation.
    #[test]
    fn an_impossible_bufsize_is_refused() {
        assert!(decompress(&[1], 9 * 1024 * 1024).is_err());
    }
}
