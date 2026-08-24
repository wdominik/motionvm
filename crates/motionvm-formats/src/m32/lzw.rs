//! The LZW variant used by MOTION's `GFXCRUNCH` codec.
//!
//! It is ordinary LZW over an 8-bit alphabet, MSB-first, starting at 9-bit
//! codes, with two reserved codes:
//!
//! * `256` — clear: reset the dictionary and go back to 9-bit codes.
//! * `257` — bump: widen codes by one bit (capped at the stream's `max_bits`).
//!
//! The bump code is what makes this variant unusual. Ordinary LZW (GIF, TIFF)
//! widens implicitly when the dictionary fills up, and uses 257 as end-of-input.
//! Here the *encoder* signals every widening explicitly, and there is no
//! end-of-input marker at all — decoding stops once the declared number of
//! output bytes has been produced.
//!
//! Verified against all 1678 sprites in `001.RSC`/`002.RSC`/`003.RSC`: every
//! one decodes to exactly the size recorded in its header.

pub use crate::error::lzw::LzwError;

const CLEAR: u16 = 256;
const BUMP: u16 = 257;
/// First dictionary slot handed out to a learned string.
const FIRST_FREE: usize = 258;
const INITIAL_WIDTH: u32 = 9;
/// The widest dictionary GFXCRUNCH ever asks for.
const MAX_WIDTH: u32 = 12;
/// How much output is reserved up front, at most.
///
/// `expected` is a `u32` straight out of a sprite header, so a damaged one can
/// ask for four gigabytes before a single code has been read. Reserving is only
/// an optimization — the loop below grows the vector as it goes — so capping it
/// costs nothing on real data and turns a hostile length into a slow decode
/// instead of an out-of-memory abort.
const MAX_PREALLOC: usize = 1 << 22;

/// Decompresses `data` until exactly `expected` bytes have been produced.
///
/// `max_bits` is the ceiling the bump code may widen to (11 or 12 in practice).
/// Anything outside 9 to 12 is rejected before it is used as a shift distance.
pub fn decode(data: &[u8], max_bits: u32, expected: usize) -> Result<Vec<u8>, LzwError> {
    if !(INITIAL_WIDTH..=MAX_WIDTH).contains(&max_bits) {
        return Err(LzwError::BadWidth { max_bits });
    }
    let capacity = 1usize << max_bits;
    // Dictionary as a prefix/suffix chain rather than owned byte strings: entry
    // `c` is entry `prefix[c]` followed by the single byte `suffix[c]`.
    let mut prefix = vec![0u16; capacity];
    let mut suffix = vec![0u8; capacity];
    // Scratch for walking a chain, which yields bytes in reverse.
    let mut chain = Vec::with_capacity(capacity);
    let mut out = Vec::with_capacity(expected.min(MAX_PREALLOC));

    let mut width = INITIAL_WIDTH;
    let mut next = FIRST_FREE;
    let mut prev: Option<u16> = None;
    let mut bit_pos = 0usize;
    let total_bits = data.len() * 8;

    while out.len() < expected {
        if bit_pos + width as usize > total_bits {
            return Err(LzwError::UnexpectedEof {
                produced: out.len(),
                expected,
            });
        }
        let code = read_bits(data, bit_pos, width);
        bit_pos += width as usize;

        match code {
            CLEAR => {
                width = INITIAL_WIDTH;
                next = FIRST_FREE;
                prev = None;
                continue;
            }
            BUMP => {
                if width < max_bits {
                    width += 1;
                }
                continue;
            }
            _ => {}
        }

        // Expand `code` into `chain`, reversed.
        chain.clear();
        let mut cursor = code;
        if cursor as usize >= next {
            // The KwKwK case: the encoder referenced the entry it is about to
            // create, which is the previous string plus its own first byte.
            let Some(p) = prev else {
                return Err(LzwError::BadCode {
                    code,
                    next,
                    produced: out.len(),
                });
            };
            if cursor as usize != next {
                return Err(LzwError::BadCode {
                    code,
                    next,
                    produced: out.len(),
                });
            }
            chain.push(first_byte(p, &prefix, &suffix));
            cursor = p;
        }
        loop {
            if (cursor as usize) < 256 {
                chain.push(cursor as u8);
                break;
            }
            chain.push(suffix[cursor as usize]);
            cursor = prefix[cursor as usize];
        }
        // `chain` is reversed, so its last element is the string's first byte.
        let first = *chain.last().expect("chain always gets at least one byte");
        out.extend(chain.iter().rev().copied());

        if let Some(p) = prev {
            if next < capacity {
                prefix[next] = p;
                suffix[next] = first;
            }
            // Keep counting past the table end so the KwKwK check stays in sync
            // with the encoder; codes that large can never be read back anyway.
            next += 1;
        }
        prev = Some(code);
    }

    if out.len() != expected {
        return Err(LzwError::Overrun {
            produced: out.len(),
            expected,
        });
    }
    Ok(out)
}

/// Walks a dictionary chain to its root to find the string's first byte.
fn first_byte(mut code: u16, prefix: &[u16], suffix: &[u8]) -> u8 {
    while code as usize >= 256 {
        let next = prefix[code as usize];
        if next == code {
            // Defensive: a self-referential entry would loop forever. Cannot
            // happen for well-formed streams.
            return suffix[code as usize];
        }
        code = next;
    }
    code as u8
}

/// Reads `width` bits (at most 12) MSB-first from an arbitrary bit offset.
#[inline]
fn read_bits(data: &[u8], bit_pos: usize, width: u32) -> u16 {
    let byte = bit_pos >> 3;
    // Three bytes always cover a <=12-bit field at any alignment (7 + 12 <= 24).
    let mut acc = (data[byte] as u32) << 16;
    if byte + 1 < data.len() {
        acc |= (data[byte + 1] as u32) << 8;
    }
    if byte + 2 < data.len() {
        acc |= data[byte + 2] as u32;
    }
    let shift = 24 - (bit_pos & 7) - width as usize;
    ((acc >> shift) & ((1 << width) - 1)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_bits_is_msb_first() {
        // 0b1010_0101_1100_0011
        let d = [0xA5, 0xC3];
        assert_eq!(read_bits(&d, 0, 9), 0b1_0100_1011);
        assert_eq!(read_bits(&d, 4, 9), 0b0_1011_1000);
    }

    #[test]
    fn literals_pass_through() {
        // Three 9-bit literal codes: 'A', 'B', 'C'.
        let codes = [b'A' as u16, b'B' as u16, b'C' as u16];
        let mut bits = Vec::new();
        for c in codes {
            for i in (0..9).rev() {
                bits.push((c >> i) & 1);
            }
        }
        let mut data = vec![0u8; bits.len().div_ceil(8)];
        for (i, b) in bits.iter().enumerate() {
            data[i / 8] |= (*b as u8) << (7 - i % 8);
        }
        assert_eq!(decode(&data, 11, 3).unwrap(), b"ABC");
    }
}
