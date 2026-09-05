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
    let mut dictionary = Chain::new(capacity);
    // Scratch for walking a chain, which yields bytes in reverse.
    let mut chain = Vec::with_capacity(capacity);
    let mut out = Vec::with_capacity(expected.min(MAX_PREALLOC));

    let mut width = INITIAL_WIDTH;
    let mut next = FIRST_FREE;
    let mut prev: Option<u16> = None;
    let mut bit_pos = 0usize;

    while out.len() < expected {
        // The field's last bit has to lie inside the data.
        let Some(end) = bit_pos
            .checked_add(crate::wide(width))
            .filter(|&end| end.div_ceil(8) <= data.len())
        else {
            return Err(LzwError::UnexpectedEof {
                produced: out.len(),
                expected,
            });
        };
        let code = read_bits(data, bit_pos, width);
        bit_pos = end;

        match code {
            CLEAR => {
                width = INITIAL_WIDTH;
                next = FIRST_FREE;
                prev = None;
                continue;
            }
            BUMP => {
                // Codes grow to `max_bits` and no further.
                width = width.saturating_add(1).min(max_bits);
                continue;
            }
            _ => {}
        }

        // Expand `code` into `chain`, reversed.
        chain.clear();
        let mut cursor = code;
        if usize::from(cursor) >= next {
            // The KwKwK case: the encoder referenced the entry it is about to
            // create, which is the previous string plus its own first byte.
            let Some(p) = prev else {
                return Err(LzwError::BadCode {
                    code,
                    next,
                    produced: out.len(),
                });
            };
            if usize::from(cursor) != next {
                return Err(LzwError::BadCode {
                    code,
                    next,
                    produced: out.len(),
                });
            }
            chain.push(dictionary.first_byte(p));
            cursor = p;
        }
        // Walk the chain down to its root, which is a literal byte — the
        // string's first — and is what ends the loop, so it is what the loop
        // answers with.
        let first = loop {
            if let Some(byte) = literal(cursor) {
                chain.push(byte);
                break byte;
            }
            let (prefix, suffix) = dictionary.entry(cursor);
            chain.push(suffix);
            cursor = prefix;
        };
        // `chain` is reversed: the root came last.
        out.extend(chain.iter().rev().copied());

        if let Some(p) = prev {
            dictionary.set(next, p, first);
            // Keep counting past the table end so the KwKwK check stays in sync
            // with the encoder; codes that large can never be read back anyway,
            // so a count that saturates is no different from one that runs on.
            next = next.saturating_add(1);
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

/// The dictionary as a prefix/suffix chain rather than owned byte strings:
/// entry `c` is entry `prefix[c]` followed by the single byte `suffix[c]`.
struct Chain {
    prefix: Vec<u16>,
    suffix: Vec<u8>,
}

impl Chain {
    fn new(capacity: usize) -> Self {
        Self {
            prefix: vec![0; capacity],
            suffix: vec![0; capacity],
        }
    }

    /// The prefix code and the last byte of entry `code`. A code has at most
    /// `max_bits` bits and the tables have `1 << max_bits` entries, so every
    /// code the stream can carry names an entry.
    #[expect(
        clippy::indexing_slicing,
        reason = "a code has at most `max_bits` bits, and the tables have `1 << max_bits` entries"
    )]
    fn entry(&self, code: u16) -> (u16, u8) {
        (
            self.prefix[usize::from(code)],
            self.suffix[usize::from(code)],
        )
    }

    /// Defines entry `at` — or nothing, past the end of the tables, which is
    /// where the count goes on to but no code can reach.
    fn set(&mut self, at: usize, prefix: u16, suffix: u8) {
        if let (Some(p), Some(s)) = (self.prefix.get_mut(at), self.suffix.get_mut(at)) {
            *p = prefix;
            *s = suffix;
        }
    }

    /// Walks entry `code` down to its root to find the string's first byte.
    fn first_byte(&self, mut code: u16) -> u8 {
        loop {
            if let Some(byte) = literal(code) {
                return byte;
            }
            let (prefix, suffix) = self.entry(code);
            if prefix == code {
                // Defensive: a self-referential entry would loop forever.
                // Cannot happen for well-formed streams.
                return suffix;
            }
            code = prefix;
        }
    }
}

/// The byte a code names when it is a literal — every code below 256 — and
/// `None` for a dictionary entry. `u8::try_from` is exactly that test.
fn literal(code: u16) -> Option<u8> {
    u8::try_from(code).ok()
}

/// Reads `width` bits (at most 12) MSB-first from an arbitrary bit offset.
///
/// Three bytes always cover a 12-bit field at any alignment (7 + 12 <= 24),
/// and the caller has checked that the field lies inside the data; a byte past
/// the end is one the field does not reach, and reads as zero.
#[inline]
#[expect(
    clippy::arithmetic_side_effects,
    reason = "a shift of at most 7 + 12 out of 24 bits"
)]
fn read_bits(data: &[u8], bit_pos: usize, width: u32) -> u16 {
    let window = data.get(bit_pos >> 3..).unwrap_or_default();
    let at = |i: usize| u32::from(window.get(i).copied().unwrap_or(0));
    let acc = (at(0) << 16) | (at(1) << 8) | at(2);
    let shift = 24 - (bit_pos & 7) - crate::wide(width);
    crate::low_word((acc >> shift) & ((1 << width) - 1))
}

/// The low byte of the test encoder's bit buffer — what it emits.
#[cfg(test)]
fn low_byte(v: u32) -> u8 {
    v.to_le_bytes()[0]
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
        let codes = [u16::from(b'A'), u16::from(b'B'), u16::from(b'C')];
        let mut bits = Vec::new();
        for c in codes {
            for i in (0..9).rev() {
                bits.push((c >> i) & 1);
            }
        }
        let mut data = vec![0u8; bits.len().div_ceil(8)];
        for (i, b) in bits.iter().enumerate() {
            data[i / 8] |= u8::try_from(*b).unwrap() << (7 - i % 8);
        }
        assert_eq!(decode(&data, 11, 3).unwrap(), b"ABC");
    }
}

#[cfg(test)]
mod round_trip {
    use super::{BUMP, CLEAR, FIRST_FREE, INITIAL_WIDTH, decode, low_byte};
    use std::collections::HashMap;

    /// An encoder for this variant, written for the tests and nowhere else.
    ///
    /// The decoder is checked against all 1678 shipped sprites, which says it
    /// reads what `GFXCRUNCH` wrote. It does not say what it does with a
    /// stream that game never produced, and the shipped corpus cannot be made
    /// to produce one: every sprite is the same encoder's output. So the tests
    /// grow an encoder of their own and feed the decoder streams the game
    /// never made — a dictionary that fills exactly at a bump, a clear in the
    /// middle, one byte repeated past the widest code.
    ///
    /// Deliberately plain: greedy longest-match with an owned-`Vec` dictionary,
    /// which is the slow shape the decoder avoids. An oracle that shared the
    /// decoder's cleverness would agree with it for the decoder's reasons.
    fn encode(input: &[u8], max_bits: u32) -> Vec<u8> {
        let mut out = BitWriter::default();
        let mut dict: HashMap<Vec<u8>, u16> = HashMap::new();
        let mut next = u16::try_from(FIRST_FREE).unwrap();
        let mut width = INITIAL_WIDTH;
        let mut current: Vec<u8> = Vec::new();

        for &b in input {
            let mut wider = current.clone();
            wider.push(b);
            let known = wider.len() == 1 || dict.contains_key(&wider);
            if known {
                current = wider;
                continue;
            }
            out.put(code_of(&current, &dict), width);
            // The dictionary learns the string that did not fit, and the
            // encoder says out loud when the next code would not fit the
            // current width — the bump this variant has and ordinary LZW
            // does not.
            if u32::from(next) < (1u32 << max_bits) {
                dict.insert(wider, next);
                next += 1;
                if u32::from(next) == 1u32 << width && width < max_bits {
                    out.put(BUMP, width);
                    width += 1;
                }
            }
            current = vec![b];
        }
        if !current.is_empty() {
            out.put(code_of(&current, &dict), width);
        }
        out.finish()
    }

    /// The code a string stands for: its byte, for a string of one, else the
    /// slot the dictionary gave it.
    fn code_of(s: &[u8], dict: &HashMap<Vec<u8>, u16>) -> u16 {
        if s.len() == 1 {
            u16::from(s[0])
        } else {
            *dict
                .get(s)
                .expect("the encoder only emits what it has learned")
        }
    }

    /// MSB-first, which is what the decoder reads.
    #[derive(Default)]
    struct BitWriter {
        out: Vec<u8>,
        acc: u32,
        bits: u32,
    }

    impl BitWriter {
        fn put(&mut self, code: u16, width: u32) {
            self.acc = (self.acc << width) | u32::from(code);
            self.bits += width;
            while self.bits >= 8 {
                self.bits -= 8;
                self.out.push(low_byte(self.acc >> self.bits));
            }
        }

        fn finish(mut self) -> Vec<u8> {
            if self.bits > 0 {
                self.out.push(low_byte(self.acc << (8 - self.bits)));
            }
            self.out
        }
    }

    /// A seeded generator, so a failure is reproducible and no dependency is
    /// needed. The same LCG the machines use for `RANDOM`.
    struct Lcg(u32);

    impl Lcg {
        fn next(&mut self) -> u32 {
            self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            self.0
        }
    }

    fn round_trip(input: &[u8], max_bits: u32) {
        let encoded = encode(input, max_bits);
        let decoded = decode(&encoded, max_bits, input.len())
            .unwrap_or_else(|e| panic!("{} bytes at {max_bits} bits: {e}", input.len()));
        assert_eq!(decoded, input, "{} bytes at {max_bits} bits", input.len());
    }

    /// What comes out is what went in, over inputs of every shape the codec
    /// can meet.
    #[test]
    fn what_is_encoded_decodes_back() {
        for max_bits in 9..=12 {
            round_trip(&[], max_bits);
            round_trip(b"a", max_bits);
            round_trip(b"aaaaaaaaaaaaaaaa", max_bits);
            round_trip(b"abababababababab", max_bits);
            // Every byte once: no string is ever learned twice.
            let all: Vec<u8> = (0..=255).collect();
            round_trip(&all, max_bits);
            // Long enough to fill the dictionary at nine bits and to keep
            // going after it is full at twelve.
            let mut rng = Lcg(0x1234_5678);
            for len in [255usize, 1000, 5000] {
                let noise: Vec<u8> = (0..len).map(|_| low_byte(rng.next() >> 24)).collect();
                round_trip(&noise, max_bits);
                // A run of one byte is the case the decoder's KwKwK branch
                // exists for, and it is the one a random input almost never
                // produces.
                let runs: Vec<u8> = (0..len)
                    .map(|i| u8::try_from((i / 17) % 4).unwrap())
                    .collect();
                round_trip(&runs, max_bits);
            }
        }
    }

    /// A width outside the codec's range is refused rather than used as a
    /// shift distance.
    #[test]
    fn an_impossible_width_is_refused() {
        for bad in [0, 1, 8, 13, 32, 64] {
            assert!(decode(&[0, 0, 0, 0], bad, 4).is_err(), "{bad} bits");
        }
    }

    /// The clear code puts the dictionary back, so a stream that uses one
    /// decodes as if it had started there.
    #[test]
    fn a_clear_starts_the_dictionary_over() {
        let head = b"abcabcabcabc";
        let tail = b"abcabcabcabc";
        let mut stream = BitWriter::default();
        for &b in head {
            stream.put(u16::from(b), INITIAL_WIDTH);
        }
        stream.put(CLEAR, INITIAL_WIDTH);
        for &b in tail {
            stream.put(u16::from(b), INITIAL_WIDTH);
        }
        let bytes = stream.finish();
        let want: Vec<u8> = head.iter().chain(tail).copied().collect();
        assert_eq!(decode(&bytes, 12, want.len()).expect("decodes"), want);
    }
}
