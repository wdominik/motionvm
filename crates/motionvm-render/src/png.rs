//! An 8-bit indexed PNG, written by hand — and read back, along with the
//! true-color kind a capture of the original arrives as.
//!
//! One shape of file is written and no other: the signature, `IHDR`, `PLTE`,
//! an optional `tRNS`, one `IDAT` and `IEND`. That is every picture this
//! workspace writes — the frame F12 saves and the sprites, palettes and font
//! sheets the tools extract — because the index *is* the information here:
//! the engine animates the palette and treats index 0 as transparent, so a
//! picture written as true color has thrown away the thing a comparison
//! against the original is about.
//!
//! Reading is wider than writing, by exactly as much as the other side of a
//! comparison needs: eight bits a sample, indexed or RGB or RGBA, any of the
//! five filters, any number of `IDAT`s, compressed however the writer liked —
//! see [`decode`]. Not interlaced pictures and not other depths: a capture
//! is never one, and a refusal names what it met.
//!
//! ## Why by hand
//!
//! The encoder that was here brought seven packages with it — a CRC, two
//! Adler-32s, two inflate implementations and a deflate — for a file whose
//! every field this crate already decides. What is left below is a checksum,
//! a chunk framer and a zlib stream that does not compress; nothing in it
//! needs a decision, and all of it fits on two screens.
//!
//! ## What it costs
//!
//! The `IDAT` is a zlib stream of **stored** deflate blocks: the bytes as
//! they are, five bytes of framing per 65 535 of them. A 640×480 frame is
//! about 300 KB where a compressing encoder would have written 30. That is
//! the whole of the trade, and it is worth taking here because every picture
//! this writes is a development artifact — a screenshot to compare against a
//! capture, a directory of extracted art — and none of them is shipped or
//! served. Nothing about the file is non-standard: a stored block is deflate,
//! and every decoder reads one.
//!
//! ## What it is checked against
//!
//! The tests below read a written file back with an inflater of their own —
//! deliberately not this code — and check every fixed field, both checksums
//! and every pixel, with the two checksums held against the reference values
//! in their own specifications.
//!
//! A suite cannot say a file is a *PNG*, though, only that it is what this
//! module meant to write. So the output has also been opened with decoders
//! that know nothing about it, on sprites taken out of two of the games:
//! macOS's `sips` reads the dimensions, the format and the alpha and
//! re-encodes the picture to JPEG; Pillow reads it as a paletted image with
//! 256 entries and transparency on index 0; and Python's `zlib` inflates the
//! `IDAT` on its own.

use crate::inflate::{self, BadStream};
use crate::wide;

/// CRC-32, the reflected IEEE 802.3 polynomial — the checksum PNG puts on
/// every chunk, and the one zip and gzip use. Bit at a time.
///
/// No lookup table. This runs over the image data once per file written, and
/// a table would buy a millisecond of a program that has just spent longer
/// composing the picture. Public because it is one algorithm and a caller
/// sealing a file of its own wants the same one; a second copy would only be
/// a second place to get it wrong.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            // The branchless form of "shift, and xor the polynomial back in
            // when the bit that fell off was set".
            crc = (crc >> 1) ^ (0xedb8_8320 & 0u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

/// Adler-32 over the *uncompressed* data, which is what a zlib stream ends
/// with (RFC 1950 §2.2).
pub(crate) fn adler32(bytes: &[u8]) -> u32 {
    // The largest prime below 65 536, and the reason the sums can run
    // 5552 bytes between reductions without overflowing.
    const BASE: u32 = 65_521;
    let (mut a, mut b) = (1u32, 0u32);
    for chunk in bytes.chunks(5552) {
        for &byte in chunk {
            a += u32::from(byte);
            b += a;
        }
        a %= BASE;
        b %= BASE;
    }
    (b << 16) | a
}

/// One chunk: its length, its four-byte type, its data, and a CRC-32 over the
/// type and the data — the length deliberately not covered, which is the
/// format's own arrangement (PNG §5.3).
fn chunk(out: &mut Vec<u8>, tag: &[u8; 4], data: &[u8]) {
    // A chunk that would not fit its length field was refused at the door,
    // by `write_indexed_png`'s size check.
    let len = u32::try_from(data.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    let start = out.len();
    out.extend_from_slice(tag);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(&out[start..]).to_be_bytes());
}

/// A zlib stream carrying `raw` in stored deflate blocks.
///
/// The two header bytes are the only ones with any choice in them: `0x78` is
/// deflate with a 32 KiB window, and `0x01` completes it — the low five bits
/// are a check value that makes the pair a multiple of 31, and the two
/// compression-level bits say "fastest", which is exactly what this is.
///
/// A stored block is a header byte, then `LEN` and its complement as
/// little-endian `u16`s, then `LEN` bytes verbatim. The header byte is 0 for
/// every block but the last, where bit 0 marks the end of the stream. It sits
/// on a byte boundary because a stored block has to start on one, and every
/// block before it ended on one.
pub(crate) fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() + raw.len() / 65_535 * 5 + 11);
    out.extend_from_slice(&[0x78, 0x01]);
    // An empty input still needs one block, so that the stream has an end.
    let mut blocks = raw.chunks(65_535).peekable();
    if blocks.peek().is_none() {
        out.extend_from_slice(&[1, 0, 0, 0xff, 0xff]);
    }
    while let Some(block) = blocks.next() {
        out.push(u8::from(blocks.peek().is_none()));
        // Blocks of 65 535 at most, so the length always fits.
        let len = u16::try_from(block.len()).unwrap_or(u16::MAX);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

/// The bytes of an 8-bit indexed PNG.
///
/// `palette` is RGB triples and `transparent`, when given, the one index that
/// gets an alpha of zero in `tRNS`. The caller has already checked that the
/// sizes agree — see [`crate::write_indexed_png`], which is the only way in.
pub(crate) fn encode(
    width: u32,
    height: u32,
    pixels: &[u8],
    palette: &[u8],
    transparent: Option<u8>,
) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() + palette.len() + 1024);
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    // Eight bits a sample, color type 3 (each sample an index into `PLTE`),
    // the only compression method there is, the only filter method there is,
    // and no interlacing.
    ihdr.extend_from_slice(&[8, 3, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);

    chunk(&mut out, b"PLTE", palette);

    // `tRNS` for an indexed image is one alpha byte per entry, and may stop
    // short: entries it does not reach are opaque. So the one transparent
    // index needs a run only as long as itself.
    if let Some(index) = transparent {
        let mut alpha = vec![255u8; usize::from(index) + 1];
        alpha[usize::from(index)] = 0;
        chunk(&mut out, b"tRNS", &alpha);
    }

    // Every row carries a filter byte in front of it, and this writes 0 —
    // "none". Filters exist to make a row cheaper to compress, and nothing
    // here compresses.
    let stride = wide(width);
    let mut raw = Vec::with_capacity(pixels.len() + wide(height));
    for row in pixels.chunks(stride) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    chunk(&mut out, b"IDAT", &zlib_stored(&raw));

    chunk(&mut out, b"IEND", &[]);
    out
}

/// A picture read back out of a PNG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// Its width in pixels.
    pub width: u32,
    /// Its height in pixels.
    pub height: u32,
    /// The pixels, top row first.
    pub pixels: Pixels,
}

/// The pixels of a decoded picture, in the form the file kept them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pixels {
    /// One index a pixel, into the palette the file carries — what this
    /// crate writes, and what DOSBox-X writes for a 256-color mode.
    Indexed {
        /// `width × height` indices.
        indices: Vec<u8>,
        /// The palette's entries, eight bits a channel, as many as `PLTE`
        /// held.
        palette: Vec<[u8; 3]>,
    },
    /// One color a pixel, eight bits a channel — a screenshot. An alpha
    /// channel, where the file had one, is dropped: a capture is opaque.
    Rgb(Vec<[u8; 3]>),
}

/// Why a PNG could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unreadable {
    /// The file does not open with the PNG signature.
    NotPng,
    /// A chunk runs past the end of the file, or `IHDR` or `IEND` is missing.
    Truncated,
    /// A chunk's CRC does not agree with its bytes.
    Crc {
        /// The chunk's type, as four ASCII bytes.
        chunk: [u8; 4],
    },
    /// A picture this reader does not take: another depth than eight bits, a
    /// grayscale, an interlaced one.
    Unsupported {
        /// Bits a sample, from `IHDR`.
        depth: u8,
        /// The color type, from `IHDR`.
        color: u8,
        /// Whether the picture is interlaced.
        interlaced: bool,
    },
    /// The compressed pixel data could not be inflated.
    Stream(BadStream),
    /// The inflated data is not one filter byte and a row of samples per
    /// row, or a filter byte names no filter.
    Rows,
    /// An indexed picture without a palette, or with an index past it.
    Palette,
}

impl std::fmt::Display for Unreadable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotPng => write!(f, "not a PNG"),
            Self::Truncated => write!(f, "the file ends inside a chunk"),
            Self::Crc { chunk } => write!(
                f,
                "the {} chunk fails its CRC",
                String::from_utf8_lossy(chunk)
            ),
            Self::Unsupported {
                depth,
                color,
                interlaced,
            } => write!(
                f,
                "a PNG of {depth} bits a sample, color type {color}{}; this reads eight-bit \
                 indexed, RGB and RGBA pictures, not interlaced",
                if *interlaced { ", interlaced" } else { "" }
            ),
            Self::Stream(why) => write!(f, "{why}"),
            Self::Rows => write!(f, "the pixel data is not whole rows with a filter each"),
            Self::Palette => write!(f, "an index past the palette, or no palette at all"),
        }
    }
}

/// The picture in `file`.
pub(crate) fn decode(file: &[u8]) -> Result<Decoded, Unreadable> {
    let body = file
        .strip_prefix(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a])
        .ok_or(Unreadable::NotPng)?;
    let mut header = None;
    let mut palette = Vec::new();
    let mut compressed = Vec::new();
    let mut at = 0usize;
    loop {
        let len = wide(be32(body, at)?);
        let tag: [u8; 4] = body
            .get(at + 4..at + 8)
            .and_then(|t| t.try_into().ok())
            .ok_or(Unreadable::Truncated)?;
        let data = body
            .get(at + 8..at + 8 + len)
            .ok_or(Unreadable::Truncated)?;
        let crc = be32(body, at + 8 + len)?;
        if crc != crc32(&body[at + 4..at + 8 + len]) {
            return Err(Unreadable::Crc { chunk: tag });
        }
        match &tag {
            b"IHDR" => header = Some(Header::parse(data)?),
            b"PLTE" => palette = data.as_chunks::<3>().0.to_vec(),
            b"IDAT" => compressed.extend_from_slice(data),
            b"IEND" => break,
            _ => {}
        }
        at += 12 + len;
    }
    let header = header.ok_or(Unreadable::Truncated)?;
    let raw = inflate::zlib(&compressed).map_err(Unreadable::Stream)?;
    let samples = unfilter(&raw, header)?;
    let pixels = match header.color {
        3 => {
            if palette.is_empty() || samples.iter().any(|&i| usize::from(i) >= palette.len()) {
                return Err(Unreadable::Palette);
            }
            Pixels::Indexed {
                indices: samples,
                palette,
            }
        }
        _ => Pixels::Rgb(
            samples
                .chunks_exact(usize::from(header.channels()))
                .map(|p| [p[0], p[1], p[2]])
                .collect(),
        ),
    };
    Ok(Decoded {
        width: header.width,
        height: header.height,
        pixels,
    })
}

fn be32(bytes: &[u8], at: usize) -> Result<u32, Unreadable> {
    bytes
        .get(at..at + 4)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_be_bytes)
        .ok_or(Unreadable::Truncated)
}

/// What `IHDR` says, once it has passed the reader's door.
#[derive(Debug, Clone, Copy)]
struct Header {
    width: u32,
    height: u32,
    /// 3 for indexed, 2 for RGB, 6 for RGBA — the three this reads.
    color: u8,
}

impl Header {
    fn parse(data: &[u8]) -> Result<Self, Unreadable> {
        let (width, height) = (be32(data, 0)?, be32(data, 4)?);
        let &[depth, color, _, _, interlace] = data.get(8..13).ok_or(Unreadable::Truncated)? else {
            return Err(Unreadable::Truncated);
        };
        if depth != 8 || !matches!(color, 2 | 3 | 6) || interlace != 0 {
            return Err(Unreadable::Unsupported {
                depth,
                color,
                interlaced: interlace != 0,
            });
        }
        Ok(Self {
            width,
            height,
            color,
        })
    }

    /// Bytes a pixel.
    fn channels(self) -> u8 {
        match self.color {
            2 => 3,
            6 => 4,
            _ => 1,
        }
    }
}

/// The five filters undone, row by row (PNG §9): each row's filter byte
/// says how its bytes were predicted from the one to the left, the one above,
/// and the one above-left.
fn unfilter(raw: &[u8], header: Header) -> Result<Vec<u8>, Unreadable> {
    let bpp = usize::from(header.channels());
    let stride = wide(header.width)
        .checked_mul(bpp)
        .ok_or(Unreadable::Rows)?;
    let rows = wide(header.height);
    if raw.len() != rows.checked_mul(stride + 1).ok_or(Unreadable::Rows)? {
        return Err(Unreadable::Rows);
    }
    let mut out = vec![0u8; rows * stride];
    for (r, row) in raw.chunks_exact(stride + 1).enumerate() {
        let (&filter, line) = row.split_first().ok_or(Unreadable::Rows)?;
        let (above, current) = out.split_at_mut(r * stride);
        let above = above
            .get(r.wrapping_sub(1).wrapping_mul(stride)..)
            .unwrap_or_default();
        let current = &mut current[..stride];
        for i in 0..stride {
            let a = if i >= bpp { current[i - bpp] } else { 0 };
            let b = above.get(i).copied().unwrap_or(0);
            let c = if i >= bpp {
                above.get(i - bpp).copied().unwrap_or(0)
            } else {
                0
            };
            let predicted = match filter {
                0 => 0,
                1 => a,
                2 => b,
                3 => u8::try_from((u16::from(a) + u16::from(b)) / 2).unwrap_or(0),
                4 => paeth(a, b, c),
                _ => return Err(Unreadable::Rows),
            };
            current[i] = line[i].wrapping_add(predicted);
        }
    }
    Ok(out)
}

/// The Paeth predictor: whichever of the three neighbors is nearest to
/// their linear estimate, ties going left, then up.
fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = i16::from(a) + i16::from(b) - i16::from(c);
    let (pa, pb, pc) = (
        (p - i16::from(a)).abs(),
        (p - i16::from(b)).abs(),
        (p - i16::from(c)).abs(),
    );
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference values from RFC 1950 §2.2 and the PNG specification's own
    /// example, so the two checksums are held against something outside this
    /// file.
    #[test]
    fn the_checksums_agree_with_their_specifications() {
        // Adler-32 of "Wikipedia" is 0x11E60398 — the worked example in the
        // algorithm's own description.
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
        assert_eq!(adler32(b""), 1, "the empty sum is 1, not 0");
        // CRC-32 of "123456789" is 0xCBF43926, the check value every
        // description of this polynomial carries.
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    /// A zlib stream of stored blocks, read back the way a decoder reads one.
    ///
    /// Deliberately not sharing a line with the writer: a reader built out of
    /// the same code agrees with it however wrong both are.
    fn inflate_stored(stream: &[u8]) -> Vec<u8> {
        assert_eq!(stream[0] & 0x0f, 8, "deflate");
        assert_eq!(stream[0] >> 4, 7, "a 32 KiB window");
        assert_eq!(
            (u16::from(stream[0]) * 256 + u16::from(stream[1])) % 31,
            0,
            "the header's check value"
        );
        assert_eq!(stream[1] & 0x20, 0, "no preset dictionary");
        let mut out = Vec::new();
        let mut at = 2;
        loop {
            let header = stream[at];
            assert_eq!(header & 0x06, 0, "a stored block");
            let len = usize::from(u16::from_le_bytes([stream[at + 1], stream[at + 2]]));
            let nlen = u16::from_le_bytes([stream[at + 3], stream[at + 4]]);
            assert_eq!(nlen, !u16::try_from(len).unwrap(), "LEN and its complement");
            out.extend_from_slice(&stream[at + 5..at + 5 + len]);
            at += 5 + len;
            if header & 1 == 1 {
                break;
            }
        }
        let adler = u32::from_be_bytes(stream[at..at + 4].try_into().expect("4 bytes"));
        assert_eq!(adler, adler32(&out), "the stream's own checksum");
        assert_eq!(at + 4, stream.len(), "nothing after the stream");
        out
    }

    /// The chunks of a PNG, in order, each checked against its own CRC.
    fn chunks(png: &[u8]) -> Vec<(String, Vec<u8>)> {
        assert_eq!(
            &png[..8],
            &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a],
            "the signature"
        );
        let mut out = Vec::new();
        let mut at = 8;
        while at < png.len() {
            let len = usize::try_from(u32::from_be_bytes(
                png[at..at + 4].try_into().expect("4 bytes"),
            ))
            .unwrap();
            let tag = String::from_utf8(png[at + 4..at + 8].to_vec()).expect("a chunk type");
            let data = png[at + 8..at + 8 + len].to_vec();
            let want = u32::from_be_bytes(
                png[at + 8 + len..at + 12 + len]
                    .try_into()
                    .expect("4 bytes"),
            );
            assert_eq!(crc32(&png[at + 4..at + 8 + len]), want, "{tag}'s CRC");
            out.push((tag, data));
            at += 12 + len;
        }
        assert_eq!(at, png.len(), "the chunks account for the file");
        out
    }

    #[test]
    fn a_written_picture_reads_back_pixel_for_pixel() {
        let (w, h) = (7u32, 5u32);
        let pixels: Vec<u8> = (0..w * h)
            .map(|i| u8::try_from(i * 3 % 256).unwrap())
            .collect();
        let palette: Vec<u8> = (0..256u32)
            .map(|i| u8::try_from(i).unwrap())
            .flat_map(|i| [i, 0, 255 - i])
            .collect();
        let png = encode(w, h, &pixels, &palette, Some(0));

        let chunks = chunks(&png);
        let tags: Vec<&str> = chunks.iter().map(|(t, _)| t.as_str()).collect();
        assert_eq!(tags, ["IHDR", "PLTE", "tRNS", "IDAT", "IEND"]);

        let ihdr = &chunks[0].1;
        assert_eq!(u32::from_be_bytes(ihdr[..4].try_into().unwrap()), w);
        assert_eq!(u32::from_be_bytes(ihdr[4..8].try_into().unwrap()), h);
        assert_eq!(&ihdr[8..], &[8, 3, 0, 0, 0], "8-bit indexed, uninterlaced");
        assert_eq!(chunks[1].1, palette);
        assert_eq!(chunks[2].1, [0], "one alpha byte, and it is index 0's");
        assert!(chunks[4].1.is_empty(), "IEND carries nothing");

        let raw = inflate_stored(&chunks[3].1);
        assert_eq!(raw.len(), (wide(w) + 1) * wide(h));
        for (y, row) in raw.chunks(wide(w) + 1).enumerate() {
            assert_eq!(row[0], 0, "row {y} is unfiltered");
            assert_eq!(&row[1..], &pixels[y * wide(w)..(y + 1) * wide(w)]);
        }
    }

    /// More than one stored block, which is the only part of the stream that
    /// a small picture never exercises.
    #[test]
    fn a_picture_larger_than_one_stored_block_comes_back_whole() {
        let (w, h) = (640u32, 480u32);
        let pixels: Vec<u8> = (0..w * h).map(|i| u8::try_from(i % 251).unwrap()).collect();
        let palette = vec![0u8; 768];
        let png = encode(w, h, &pixels, &palette, None);

        let chunks = chunks(&png);
        assert_eq!(chunks.len(), 4, "no tRNS was asked for");
        let stream = &chunks[2].1;
        let raw = inflate_stored(stream);
        // (640 + 1) × 480 is 307 680, which is five stored blocks and a
        // remainder.
        assert!(
            raw.len() > 65_535 * 4,
            "the picture spans several blocks: {}",
            raw.len()
        );
        let back: Vec<u8> = raw
            .chunks(wide(w) + 1)
            .flat_map(|row| row[1..].to_vec())
            .collect();
        assert_eq!(back, pixels);
    }

    #[test]
    fn an_empty_stream_still_has_an_end() {
        assert!(inflate_stored(&zlib_stored(b"")).is_empty());
    }

    fn hex(s: &str) -> Vec<u8> {
        s.as_bytes()
            .chunks(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    /// A 5×5 RGB picture Python wrote — every row under a different filter,
    /// the stream compressed at level 9 by its `zlib` — comes back pixel
    /// for pixel.
    #[test]
    fn a_compressed_rgb_picture_reads_back_through_every_filter() {
        let file = hex(
            "89504e470d0a1a0a0000000d4948445200000005000000050802000000020db1b2000000434944415478da6360f8cfa0f19021e03043c5528605ed0c8cec40fe235e38626267606067e065679062675067673061e66b6090f82825f15151e2a3bac4473d16347900476d1158bd44c48b0000000049454e44ae426082",
        );
        let want = hex(
            "00ff0028e10050c30078a500a0870007ff002fe10d57c31a7fa527a787340eff0036e11a5ec33486a54eae876815ff003de12765c34e8da575b5879c1cff0044e1346cc36894a59cbc87d0",
        );
        let got = decode(&file).unwrap();
        assert_eq!((got.width, got.height), (5, 5));
        let Pixels::Rgb(pixels) = got.pixels else {
            panic!("an RGB file read as something else");
        };
        assert_eq!(pixels.concat(), want);
    }

    /// An indexed picture with a three-entry palette, compressed by the same
    /// outside implementation.
    #[test]
    fn a_compressed_indexed_picture_keeps_its_palette_and_indices() {
        let file = hex(
            "89504e470d0a1a0a0000000d4948445200000004000000030803000000832a5ef400000009504c5445fc000000fc000000fcca6eda25000000154944415478da6360606462646062626060600432010069000d39d6d0160000000049454e44ae426082",
        );
        let got = decode(&file).unwrap();
        assert_eq!(
            got.pixels,
            Pixels::Indexed {
                indices: vec![0, 1, 2, 1, 2, 2, 0, 0, 1, 0, 1, 2],
                palette: vec![[252, 0, 0], [0, 252, 0], [0, 0, 252]],
            }
        );
    }

    /// What this module writes, it reads: a 300×200 picture over a full
    /// palette, through `encode` and back.
    #[test]
    fn a_written_picture_decodes_to_what_was_written() {
        let (w, h) = (300u32, 200u32);
        let pixels: Vec<u8> = (0..w * h).map(|i| (i * 7).to_le_bytes()[0]).collect();
        let palette: Vec<u8> = (0..768u32).map(|i| (i * 3).to_le_bytes()[0]).collect();
        let file = encode(w, h, &pixels, &palette, Some(0));
        let got = decode(&file).unwrap();
        assert_eq!((got.width, got.height), (w, h));
        assert_eq!(
            got.pixels,
            Pixels::Indexed {
                indices: pixels,
                palette: palette.as_chunks::<3>().0.to_vec(),
            }
        );
    }

    /// The refusals, each by its name: not a PNG, a chunk cut short, a CRC
    /// that disagrees, a depth this does not read, an index past the palette.
    #[test]
    fn what_cannot_be_read_is_refused_by_name() {
        let good = encode(2, 2, &[0, 1, 2, 3], &[0; 12], None);
        assert_eq!(decode(b"GIF89a").unwrap_err(), Unreadable::NotPng);
        assert_eq!(decode(&good[..40]).unwrap_err(), Unreadable::Truncated);
        let mut bad_crc = good;
        bad_crc[20] ^= 1; // inside IHDR's data
        assert_eq!(
            decode(&bad_crc).unwrap_err(),
            Unreadable::Crc { chunk: *b"IHDR" }
        );
        let sixteen = encode(1, 1, &[0], &[0; 3], None);
        let mut sixteen = sixteen;
        sixteen[24] = 16; // the depth byte, with the CRC redone
        let crc = crc32(&sixteen[12..29]).to_be_bytes();
        sixteen[29..33].copy_from_slice(&crc);
        assert!(matches!(
            decode(&sixteen).unwrap_err(),
            Unreadable::Unsupported { depth: 16, .. }
        ));
        let past = encode(1, 1, &[5], &[0; 6], None);
        assert_eq!(decode(&past).unwrap_err(), Unreadable::Palette);
    }
}
