//! PARSEC `PRSC` executables: the two-stage stub in front of LZEXE 0.91.
//!
//! Packed files have `PRSC` at MZ extra-bytes 0x1C. The stub sits at packed
//! `CS:IP` (`CS:01D0`). Stage 1 decrypts the bilingual
//! anti-debug notice at `CS:000C`. Stage 2 decrypts five words at `CS:0000`
//! with key `0x1234`; the second word is the paragraph of the LZEXE 0.91
//! info block, and the fifth decrypts to `0x534E` (`"NS"`).
//!
//! The LZEXE bit-stream starts at load offset 0 (file offset `e_cparhdr*16`,
//! 0x20 when the header is two paragraphs). Using packed `CS` in the stock
//! UNLZEXE start formula is wrong for PRSC: the stub sits *after* the
//! compressed image. With load-offset 0, Dunkle Schatten's `XP01`
//! decompresses to 150 096 bytes and 2 441 relocs.

use crate::error::{Error, Result};
use crate::{Cursor, Record, bytes, records, sign16};

/// MZ extra-bytes tag of a PARSEC-packed executable.
pub const MAGIC_PRSC: [u8; 4] = *b"PRSC";

/// An unpacked PARSEC executable: original load image plus the reloc table
/// LZEXE stored at `CS_lz:0158`.
#[derive(Debug, Clone)]
pub struct Unpacked {
    /// Original `CS:IP` from the LZEXE info block (Turbo-C `0000:0000`).
    pub ip: u16,
    /// Original `CS`.
    pub cs: u16,
    /// Original `SP`.
    pub sp: u16,
    /// Original `SS`.
    pub ss: u16,
    /// Decompressed load image, no MZ header: 64 176, 150 096 and 28 816
    /// bytes for Dunkle Schatten's `XP00`, `XP01` and `XP02`.
    pub image: Vec<u8>,
    /// Relocations as `(offset, segment)` pairs, 0.91 encoding.
    pub relocs: Vec<(u16, u16)>,
}

/// The fixed part of the Turbo-C entry prologue, from image offset 3.
///
/// The entry loads the program's data segment into DX as an immediate
/// (the word at offset 1) and stores it in a word of the code segment
/// whose address differs per program (offset 3). It then asks DOS for its
/// version (offsets 8 and `0Ah`), reads the PSP's words at 2 and `2Ch`
/// (offsets `0Ch` and `10h`), and sets DS from DX (offset `14h`), so DS is
/// what offset 1 held.
///
/// Only the two immediates vary between Dunkle Schatten's three programs;
/// every other byte from 0 to 0x15 is the same in all of them. The match is
/// checked in two pieces so that the `mov cs:[imm16], dx` target is
/// skipped rather than pinned.
const PROLOGUE_HEAD: [u8; 3] = [0x2E, 0x89, 0x16];
/// The prologue from `mov ah, 30h` through `mov ds, dx`, at image offset 8.
const PROLOGUE_TAIL: [u8; 14] = [
    0xB4, 0x30, 0xCD, 0x21, 0x8B, 0x2E, 0x02, 0x00, 0x8B, 0x1E, 0x2C, 0x00, 0x8E, 0xDA,
];

impl Unpacked {
    /// Unpacks a `PRSC` MZ file. Anything else is `Corrupt`.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let head = Record(bytes::<0x20>(data, 0)?);
        if head.bytes::<0, 2>() != *b"MZ" {
            return Err(Error::Corrupt {
                what: "EXE",
                detail: "not an MZ executable".into(),
            });
        }
        let tag = head.bytes::<0x1C, 4>();
        if tag != MAGIC_PRSC {
            return Err(Error::Corrupt {
                what: "EXE",
                detail: format!("expected PRSC at 0x1C, found {tag:02x?}"),
            });
        }
        let load = paragraphs(head.u16::<8>());
        let cs_off = load.saturating_add(paragraphs(head.u16::<0x16>()));

        // Stage 2 only: the five words at CS:0000 are not touched by stage 1
        // (that decrypts CS:000C). Key 0x1234, add, RCR 1.
        let mut bx = 0x1234u16;
        let mut vec = [0u16; 5];
        for (slot, word) in vec.iter_mut().zip(records::<2>(data, cs_off, 5)?) {
            let ax = u16::from_le_bytes(*word) ^ bx;
            let (sum, carry) = bx.overflowing_add(ax);
            bx = sum.wrapping_shr(1) | u16::from(carry).wrapping_shl(15);
            *slot = ax;
        }
        let [_, cs_lz, _, _, check] = vec;
        if check != 0x534E {
            return Err(Error::Corrupt {
                what: "PRSC",
                detail: format!("stage-2 checksum {check:#06x}, expected 0x534E"),
            });
        }

        let info_off = load.saturating_add(paragraphs(cs_lz));
        let info = Record(bytes::<16>(data, info_off)?);
        let relocs = reloc91(data, info_off)?;
        let image = lzexe91(data, load)?;
        Ok(Unpacked {
            ip: info.u16::<0>(),
            cs: info.u16::<2>(),
            sp: info.u16::<4>(),
            ss: info.u16::<6>(),
            image,
            relocs,
        })
    }

    /// The program's data segment as an offset into [`Self::image`].
    ///
    /// Turbo-C's entry code opens by loading `DGROUP` into `DX` and, twenty
    /// bytes later, into `DS`; the immediate at image offset 1 is therefore
    /// the segment every `DS:xxxx` address is relative to.
    /// The load image is laid out for base segment 0, so the byte offset is
    /// that segment times sixteen.
    ///
    /// Reading it beats writing it down. The value differs per program —
    /// `0E3Ah`, `21EFh` and `01E2h` for Dunkle Schatten's `XP00`, `XP01`
    /// and `XP02` — so a constant would have to be re-derived by hand for
    /// every program, and a wrong one reads plausible garbage rather than
    /// failing.
    ///
    /// `Corrupt` when the image does not open with that prologue, or when
    /// the segment it names lies outside the image. Either means this is
    /// not a PARSEC program built the way Dunkle Schatten's are, and a
    /// caller must not go on to read tables from it.
    pub fn dgroup(&self) -> Result<usize> {
        let img = &self.image;
        let prologue = bytes::<0x16>(img, 0).ok().map(Record).filter(|p| {
            p.u8::<0>() == 0xBA
                && p.bytes::<3, 3>() == PROLOGUE_HEAD
                && p.bytes::<8, 14>() == PROLOGUE_TAIL
        });
        let Some(prologue) = prologue else {
            return Err(Error::Corrupt {
                what: "EXE",
                detail: "no Turbo-C `mov dx, DGROUP` prologue at image offset 0".into(),
            });
        };
        let seg = prologue.u16::<1>();
        let off = paragraphs(seg);
        if off >= img.len() {
            return Err(Error::Corrupt {
                what: "EXE",
                detail: format!(
                    "DGROUP {seg:04X}h is image offset {off:#x}, past the {} byte image",
                    img.len()
                ),
            });
        }
        Ok(off)
    }
}

/// A count of 16-byte paragraphs, as bytes.
fn paragraphs(n: u16) -> usize {
    usize::from(n).saturating_mul(16)
}

/// LZEXE 0.91 reloc table at `CS_lz:0158`.
fn reloc91(data: &[u8], cs_off: usize) -> Result<Vec<(u16, u16)>> {
    let mut c = Cursor::new(data, cs_off.saturating_add(0x158));
    let mut entries = Vec::new();
    let mut rel_seg: u32 = 0;
    let mut rel_off: u32 = 0;
    loop {
        let span = match c.u8()? {
            0 => match c.u16()? {
                0 => {
                    rel_seg = rel_seg.wrapping_add(0x0FFF);
                    continue;
                }
                1 => break,
                w => u32::from(w),
            },
            b => u32::from(b),
        };
        rel_off = rel_off.wrapping_add(span);
        rel_seg = rel_seg.wrapping_add(rel_off.wrapping_shr(4));
        rel_off &= 0x0F;
        entries.push((crate::low_word(rel_off), crate::low_word(rel_seg)));
        if entries.len() > 16_384 {
            return Err(Error::Corrupt {
                what: "PRSC",
                detail: "reloc table longer than a DOS EXE would hold".into(),
            });
        }
    }
    Ok(entries)
}

/// The LZEXE bit buffer: sixteen flag bits a word, low bit first, with the
/// literal and operand bytes interleaved in the same stream.
struct Bits<'a> {
    src: Cursor<'a>,
    buf: u16,
    count: u8,
}

impl<'a> Bits<'a> {
    fn new(src: &'a [u8], pos: usize) -> Result<Self> {
        let mut src = Cursor::new(src, pos);
        let buf = src.u16()?;
        Ok(Bits {
            src,
            buf,
            count: 16,
        })
    }

    fn get(&mut self) -> Result<u8> {
        let b = u8::from(self.buf & 1 != 0);
        self.buf = self.buf.wrapping_shr(1);
        self.count = self.count.saturating_sub(1);
        if self.count == 0 {
            self.buf = self.src.u16()?;
            self.count = 16;
        }
        Ok(b)
    }

    fn byte(&mut self) -> Result<u8> {
        self.src.u8()
    }
}

/// LZEXE 0.91 bit-stream. `start` is the file offset of the first compressed
/// byte (load image origin for PRSC).
fn lzexe91(data: &[u8], start: usize) -> Result<Vec<u8>> {
    let mut bits = Bits::new(data, start)?;
    let mut image = Vec::new();
    loop {
        if bits.get()? == 1 {
            image.push(bits.byte()?);
            continue;
        }
        let (span, length) = if bits.get()? == 0 {
            // Two flag bits are the length less two; the byte is the low
            // half of a distance whose high half is all ones.
            let length = match (bits.get()?, bits.get()?) {
                (0, 0) => 2,
                (0, _) => 3,
                (_, 0) => 4,
                _ => 5,
            };
            (sign16(u16::from_le_bytes([bits.byte()?, 0xFF])), length)
        } else {
            // Thirteen bits of distance below `E000h`, and the length less
            // two in the low three bits of the second byte — or, when those
            // are zero, in a byte of its own.
            let lo = bits.byte()?;
            let hi = bits.byte()?;
            let span = sign16(u16::from_le_bytes([lo, hi.wrapping_shr(3) | 0xE0]));
            let length = match hi & 7 {
                0 => match bits.byte()? {
                    0 => break,
                    1 => continue,
                    n => usize::from(n).saturating_add(1),
                },
                n => usize::from(n).saturating_add(2),
            };
            (span, length)
        };
        let pos = image.len();
        for i in 0..length {
            let b = pos
                .checked_add(i)
                .and_then(|at| at.checked_add_signed(isize::from(span)))
                .and_then(|src| image.get(src))
                .copied()
                .ok_or_else(|| Error::Corrupt {
                    what: "LZEXE",
                    detail: format!("match at {pos} span {span} i {i} is before the output"),
                })?;
            image.push(b);
        }
        if image.len() > 0x80_0000 {
            return Err(Error::OutOfRange {
                field: "LZEXE output",
                value: crate::long(image.len()),
                allowed: "at most 8 MiB",
            });
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_non_mz_is_refused() {
        assert!(Unpacked::parse(b"RIB\0xxxx").is_err());
    }

    #[test]
    fn a_plain_mz_without_prsc_is_refused() {
        let mut v = vec![b'M', b'Z'];
        v.resize(0x20, 0);
        assert!(Unpacked::parse(&v).is_err());
    }

    /// An image shaped like the ones the game ships, with `DGROUP`
    /// at `seg` and enough length behind it to be addressable.
    fn with_prologue(seg: u16) -> Unpacked {
        let [lo, hi] = seg.to_le_bytes();
        let mut image = vec![0xBA, lo, hi];
        image.extend_from_slice(&PROLOGUE_HEAD);
        image.extend_from_slice(&[0x87, 0x02]);
        image.extend_from_slice(&PROLOGUE_TAIL);
        image.resize(usize::from(seg) * 16 + 0x100, 0);
        Unpacked {
            ip: 0,
            cs: 0,
            sp: 0x200,
            ss: seg,
            image,
            relocs: Vec::new(),
        }
    }

    #[test]
    fn the_data_segment_is_the_immediate_the_prologue_loads() {
        assert_eq!(with_prologue(0x1BB0).dgroup().unwrap(), 0x1bb00);
        assert_eq!(with_prologue(0x0D3D).dgroup().unwrap(), 0xd3d0);
    }

    #[test]
    fn an_image_that_does_not_open_that_way_has_no_data_segment() {
        // A different opcode where `mov dx, imm16` belongs.
        let mut wrong = with_prologue(0x1BB0);
        wrong.image[0] = 0xB8;
        assert!(wrong.dgroup().is_err());

        // The immediate is loaded but never reaches DS.
        let mut never_ds = with_prologue(0x1BB0);
        never_ds.image[0x14] = 0x90;
        assert!(never_ds.dgroup().is_err());

        // A segment that names a byte the image does not have.
        let mut past_end = with_prologue(0x1BB0);
        past_end.image.truncate(0x1000);
        assert!(past_end.dgroup().is_err());
    }
}
