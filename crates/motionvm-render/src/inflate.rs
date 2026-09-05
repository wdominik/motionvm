//! A zlib stream read back: RFC 1950 around RFC 1951, for the pictures a
//! comparison is held against.
//!
//! The writer in [`super::png`] emits stored blocks only, and its tests read
//! those back with a dozen lines. A capture of the original engine is not
//! ours to shape: DOSBox-X's screenshot and every image tool since compress,
//! so reading one means the whole of deflate — stored blocks, the fixed code,
//! and the dynamic codes a block carries its own lengths for. That is what
//! is below, and it is the reader's half of the same decision the writer
//! made: a few hundred lines this crate owns, against the seven packages an
//! inflate dependency brought with it.
//!
//! Decoding walks the canonical code a bit at a time, the way the reference
//! decoder `puff` does. A table would be faster; a screenshot is under a
//! megabyte, and this runs once per comparison.
//!
//! Everything a damaged stream can do is a refusal, never a panic: every read
//! is bounded, every length checked, and the checksum at the end has to agree.

use super::png::adler32;

/// Why a zlib stream could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BadStream {
    /// The two-byte header is not deflate with a plain window.
    Header,
    /// The data ended before the stream did.
    Truncated,
    /// A code the block's tables cannot hold, a length past the output, a
    /// distance before its start, or a block type deflate does not have.
    Corrupt(&'static str),
    /// The Adler-32 at the end does not agree with the bytes inflated.
    Checksum,
}

impl std::fmt::Display for BadStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Header => write!(f, "not a zlib stream of deflate data"),
            Self::Truncated => write!(f, "the compressed data ends early"),
            Self::Corrupt(what) => write!(f, "corrupt compressed data: {what}"),
            Self::Checksum => write!(f, "the inflated data fails its checksum"),
        }
    }
}

/// The bytes a zlib stream carries.
pub(crate) fn zlib(stream: &[u8]) -> Result<Vec<u8>, BadStream> {
    let &[cmf, flg, ..] = stream else {
        return Err(BadStream::Truncated);
    };
    // Method 8 is deflate; a window over 32 KiB is not zlib; the pair is a
    // multiple of 31; and a preset dictionary is something a PNG never has.
    if cmf & 0x0f != 8
        || cmf >> 4 > 7
        || ((u16::from(cmf) << 8) | u16::from(flg)) % 31 != 0
        || flg & 0x20 != 0
    {
        return Err(BadStream::Header);
    }
    let mut bits = Bits::new(stream.get(2..).unwrap_or_default());
    let out = inflate(&mut bits)?;
    let trailer = bits.trailer()?;
    if trailer != adler32(&out) {
        return Err(BadStream::Checksum);
    }
    Ok(out)
}

/// The deflate blocks, one after another, until the one marked last.
fn inflate(bits: &mut Bits<'_>) -> Result<Vec<u8>, BadStream> {
    let mut out = Vec::new();
    loop {
        let last = bits.take(1)? == 1;
        match bits.take(2)? {
            0 => stored(bits, &mut out)?,
            1 => {
                let (lit, dist) = Huffman::fixed();
                codes(bits, &mut out, &lit, &dist)?;
            }
            2 => {
                let (lit, dist) = Huffman::dynamic(bits)?;
                codes(bits, &mut out, &lit, &dist)?;
            }
            _ => return Err(BadStream::Corrupt("block type 3")),
        }
        if last {
            return Ok(out);
        }
    }
}

/// A stored block: back to a byte boundary, a length and its complement,
/// then the bytes as they are.
fn stored(bits: &mut Bits<'_>, out: &mut Vec<u8>) -> Result<(), BadStream> {
    bits.align();
    let len = bits.take(16)?;
    let nlen = bits.take(16)?;
    if len != !nlen & 0xffff {
        return Err(BadStream::Corrupt(
            "a stored block's length disagrees with its complement",
        ));
    }
    out.extend_from_slice(bits.bytes(len)?);
    Ok(())
}

/// The base lengths and extra bits of the length codes 257 to 285.
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
/// The same for the thirty distance codes.
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DISTANCE_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];

/// A block's literals, lengths and distances, until its end-of-block code.
fn codes(
    bits: &mut Bits<'_>,
    out: &mut Vec<u8>,
    lit: &Huffman,
    dist: &Huffman,
) -> Result<(), BadStream> {
    loop {
        let symbol = lit.decode(bits)?;
        match symbol {
            0..=255 => out.push(u8::try_from(symbol).unwrap_or(0)),
            256 => return Ok(()),
            257..=285 => {
                let i = usize::from(symbol - 257);
                let length = usize::from(LENGTH_BASE[i]) + bits.take(LENGTH_EXTRA[i])?;
                let d = usize::from(dist.decode(bits)?);
                let Some((&base, &extra)) = DISTANCE_BASE.get(d).zip(DISTANCE_EXTRA.get(d)) else {
                    return Err(BadStream::Corrupt("a distance code past the thirty"));
                };
                let distance = usize::from(base) + bits.take(extra)?;
                let Some(from) = out.len().checked_sub(distance) else {
                    return Err(BadStream::Corrupt(
                        "a distance before the start of the output",
                    ));
                };
                // The copy may overlap its own output: a distance of one and
                // a length of ten is ten copies of the last byte.
                for k in 0..length {
                    let byte = out[from + k];
                    out.push(byte);
                }
            }
            _ => return Err(BadStream::Corrupt("a literal code past 285")),
        }
    }
}

/// The bits of the stream, least significant first, as deflate reads them.
struct Bits<'a> {
    data: &'a [u8],
    at: usize,
    /// Bits already taken from the byte at `at`.
    used: u8,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            at: 0,
            used: 0,
        }
    }

    /// The next `n` bits (at most 16) as a number, low bit first.
    fn take(&mut self, n: u8) -> Result<usize, BadStream> {
        let mut value = 0usize;
        for i in 0..n {
            let &byte = self.data.get(self.at).ok_or(BadStream::Truncated)?;
            let bit = usize::from(byte >> self.used) & 1;
            value |= bit << i;
            self.used += 1;
            if self.used == 8 {
                self.used = 0;
                self.at += 1;
            }
        }
        Ok(value)
    }

    /// Drops the rest of the current byte.
    fn align(&mut self) {
        if self.used != 0 {
            self.used = 0;
            self.at += 1;
        }
    }

    /// `n` whole bytes, from a byte boundary.
    fn bytes(&mut self, n: usize) -> Result<&'a [u8], BadStream> {
        let end = self.at.checked_add(n).ok_or(BadStream::Truncated)?;
        let taken = self.data.get(self.at..end).ok_or(BadStream::Truncated)?;
        self.at = end;
        Ok(taken)
    }

    /// The four big-endian bytes after the last block: the checksum.
    fn trailer(&mut self) -> Result<u32, BadStream> {
        self.align();
        let bytes = self.bytes(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
}

/// A canonical Huffman code: how many codes there are of each length, and
/// the symbols in code order — enough to walk the code bit by bit.
struct Huffman {
    count: [u16; 16],
    symbol: Vec<u16>,
}

impl Huffman {
    /// The code the lengths describe; a length of zero is a symbol with no
    /// code. An over-subscribed set of lengths is refused, an incomplete
    /// one is allowed — a single distance code of length one is what a
    /// block with one distance uses.
    fn from_lengths(lengths: &[u8]) -> Result<Self, BadStream> {
        let mut count = [0u16; 16];
        for &len in lengths {
            count[usize::from(len & 15)] += 1;
        }
        count[0] = 0;
        let mut left = 1i32;
        for &c in &count[1..] {
            left = (left << 1) - i32::from(c);
            if left < 0 {
                return Err(BadStream::Corrupt("an over-subscribed code"));
            }
        }
        let mut offset = [0u16; 16];
        for len in 1..15 {
            offset[len + 1] = offset[len] + count[len];
        }
        let mut symbol = vec![0u16; lengths.len()];
        for (s, &len) in lengths.iter().enumerate() {
            if len != 0 {
                let at = usize::from(offset[usize::from(len)]);
                symbol[at] = u16::try_from(s).unwrap_or(u16::MAX);
                offset[usize::from(len)] += 1;
            }
        }
        Ok(Self { count, symbol })
    }

    /// One symbol off the stream.
    fn decode(&self, bits: &mut Bits<'_>) -> Result<u16, BadStream> {
        let (mut code, mut first, mut index) = (0i32, 0i32, 0i32);
        for len in 1..16 {
            code |= i32::try_from(bits.take(1)?).unwrap_or(0);
            let count = i32::from(self.count[len]);
            if code - count < first {
                let at = usize::try_from(index + (code - first)).unwrap_or(usize::MAX);
                return self
                    .symbol
                    .get(at)
                    .copied()
                    .ok_or(BadStream::Corrupt("a code outside its table"));
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err(BadStream::Corrupt("a code longer than fifteen bits"))
    }

    /// The fixed literal and distance codes of block type 1 (RFC 1951 §3.2.6).
    fn fixed() -> (Self, Self) {
        let mut lengths = [8u8; 288];
        lengths[144..256].fill(9);
        lengths[256..280].fill(7);
        let lit = Self::from_lengths(&lengths).unwrap_or_else(|_| unreachable!("a complete code"));
        let dist =
            Self::from_lengths(&[5u8; 30]).unwrap_or_else(|_| unreachable!("a complete code"));
        (lit, dist)
    }

    /// The codes a type 2 block carries: first the code that the code
    /// lengths are themselves coded with, then the lengths of the literal
    /// and distance codes, run-length coded (RFC 1951 §3.2.7).
    fn dynamic(bits: &mut Bits<'_>) -> Result<(Self, Self), BadStream> {
        const ORDER: [usize; 19] = [
            16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
        ];
        let nlit = bits.take(5)? + 257;
        let ndist = bits.take(5)? + 1;
        let ncode = bits.take(4)? + 4;
        if nlit > 286 || ndist > 30 {
            return Err(BadStream::Corrupt("more codes than deflate has"));
        }
        let mut code_lengths = [0u8; 19];
        for &at in ORDER.iter().take(ncode) {
            code_lengths[at] = u8::try_from(bits.take(3)?).unwrap_or(0);
        }
        let coder = Self::from_lengths(&code_lengths)?;

        let mut lengths = Vec::with_capacity(nlit + ndist);
        while lengths.len() < nlit + ndist {
            let symbol = coder.decode(bits)?;
            let (repeat, value) = match symbol {
                0..=15 => (1, u8::try_from(symbol).unwrap_or(0)),
                16 => {
                    let &last = lengths
                        .last()
                        .ok_or(BadStream::Corrupt("a repeat with nothing to repeat"))?;
                    (3 + bits.take(2)?, last)
                }
                17 => (3 + bits.take(3)?, 0),
                _ => (11 + bits.take(7)?, 0),
            };
            if lengths.len() + repeat > nlit + ndist {
                return Err(BadStream::Corrupt("code lengths running past their count"));
            }
            lengths.extend(std::iter::repeat_n(value, repeat));
        }
        if lengths[256] == 0 {
            return Err(BadStream::Corrupt("a block with no end-of-block code"));
        }
        Ok((
            Self::from_lengths(&lengths[..nlit])?,
            Self::from_lengths(&lengths[nlit..])?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(s: &str) -> Vec<u8> {
        s.as_bytes()
            .chunks(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    /// Three streams Python's `zlib` wrote — an implementation this crate
    /// does not own — one of each block kind: a short sentence, which the
    /// encoder puts under the fixed code; a level-9 stream over 1440 bytes
    /// of arithmetic and repetition, which gets a dynamic code; and a
    /// level-0 stream, which is stored.
    #[test]
    fn the_three_block_kinds_come_back_as_python_wrote_them() {
        let fixed = "78da2bc94855282ccd4cce56482aca2fcf5348cbaf50c82acd2d2856c82f4b2d5228014ae72456552aa4e4a70300613c0ffa";
        assert_eq!(
            zlib(&hex(fixed)).unwrap(),
            b"the quick brown fox jumps over the lazy dog"
        );

        let mut want: Vec<u8> = (0..900u32)
            .map(|i| ((i * 7 + i / 3) % 251).to_le_bytes()[0])
            .collect();
        want.extend(b"abcabcabc".repeat(60));
        let dynamic = "78daedd243921c000000c0b56ddbb66ddb9e99b56ddbb66ddbb66ddbf6291fc8037248553fa1c160517088a998380524e4d5f44c2d1d3dfcc3e352734baa9bbb062717d6f7cf6e5f7fa091b1082919d8f9c56455758c2decddfc426352b28baa1a3b06c6e7d6764f6e9ebfa0103108c8e9d878456494b58c40b6ae3ec1d149998515f5ed7da3b32bdbc7578f9f10f0e878a4b42cdcc2528a9a06001b67afa0c8848cfcb2bad69e91e9a5adc38b87773038541c126a262e410905753d332b47cf80f0f8b4dcd29ae6eea1c9c58dfdf3bbd75f18646c224a460e7e7139555d130b0777bfb0d8949ce2aaa6ce8189f9b5bdd39b976f28244c020a7a363e5119156d23733b57df90e8e4acc2ca86f6feb1d9d59de3eba74f4804747c325a561e6169254d43a08d8b775054624641795d5befc8ccf2d6d1e5c307381c1a2e090d339790a48286be99b5936760447c7a5e696d4bf7f0d4e2e6c1f9fddb2f2c0a36311523a780b8bc9aaea9a583877f585c6a4e497553d7e0c4c2faded9edcb0f3412162105033b9f98ac8a8eb1b9bd9b6f684c727651656347fff8dceaeec9f5f3172422063e391d2baf88b4b29621c8d6c527382a29b3a0a2bead6f746665fbe8eaf103021e0d8f9486855b484a51c30060edec151899909e5f56dbda333cbdb4797871ff0e068b8a434ccdc4292821afae676ae5e811101e97965b52d3dc3534b9b0b17f76f7fa03838c4544c9c0c12f26a7aa636261efee171a9b925d5cd5d839303ebfb67b7af3fc0d858849404ecfc62b2aa3ac6d04b273f509894eca2aac6868ef1b9b5dd939be7afa844040c723a365e1119652d23400da387b07452666e497d7b5f68e4c2f6f1d5e3ebc83c3a1e292503373094a2aa8eb9b5939790644c4a7e595d6b4740f4d2d6e1c9cdfbdfdc2a060135131720888cba9e99a583ab8fb87c5a6e6145737750e4eccafef9ddebe7c4323611252d0b3f389caaa681b9bdbb9f986c4246715553674f48fcdadee9c5c3f7d412260e093d1b1f288482b6919026d5dbc83a312330bcaebdb7a476796b78f2e1f3fc0e1d170496998b985241535f401d64e5e811109e97965b52d3dc3534b9b0717f76f60ff5e780010f4df7f7ff507b863827d";
        assert_eq!(zlib(&hex(dynamic)).unwrap(), want);

        let stored = "7801011600e9ff73746f7265642c206e6f7420636f6d7072657373656460a80884";
        assert_eq!(zlib(&hex(stored)).unwrap(), b"stored, not compressed");
    }

    /// What the writer in this crate emits reads back too, and the empty
    /// stream is a stream.
    #[test]
    fn our_own_stored_stream_reads_back() {
        use super::super::png::zlib_stored;
        let raw: Vec<u8> = (0..70_000u32).map(|i| (i % 253).to_le_bytes()[0]).collect();
        assert_eq!(zlib(&zlib_stored(&raw)).unwrap(), raw);
        assert_eq!(zlib(&zlib_stored(&[])).unwrap(), Vec::<u8>::new());
    }

    /// Damage is a refusal with a name, never a panic: a bad header, a stream
    /// cut anywhere, a flipped checksum.
    #[test]
    fn damage_is_refused_by_name() {
        let good = hex(
            "78da2bc94855282ccd4cce56482aca2fcf5348cbaf50c82acd2d2856c82f4b2d5228014ae72456552aa4e4a70300613c0ffa",
        );
        assert_eq!(zlib(&[0x78, 0x00]), Err(BadStream::Header));
        assert_eq!(zlib(&[0x78]), Err(BadStream::Truncated));
        for cut in 2..good.len() {
            assert!(
                zlib(&good[..cut]).is_err(),
                "a stream cut at {cut} read as whole"
            );
        }
        let mut flipped = good.clone();
        *flipped.last_mut().unwrap() ^= 1;
        assert_eq!(zlib(&flipped), Err(BadStream::Checksum));
        let mut corrupt = good;
        corrupt[2] = 0x07; // block type 3, the one deflate does not have
        assert!(matches!(zlib(&corrupt), Err(BadStream::Corrupt(_))));
    }
}
