//! Paletted pictures and sprite cels.
//!
//! Two header shapes sit in RIB resources:
//!
//! - **Picture.** `u16 width`, `u16 height`, then `width*height` pixels.
//!   The stills of `INTROS.DAT` and `EXTROS.DAT` are 320×200; the panel
//!   below the 320×128 playfield is 320×72 (`PANELS.DAT` resource 0).
//! - **Cel.** `u16 width`, `u16 height`, `u16 flags`, `u16 0`, then pixels.
//!   Walk-cycle frames (`PLAY00.DAT` 25×46, flags `2`), verb icons
//!   (`PANELS.DAT` 24×24, flags `1`), cursors (`CMBMOU.DAT`, flags `2`)
//!   and `RDV1.00` tiles (16×16, flags `0` or `1`) all share this header.
//!   Flags 0 and 2 are `w*h` raw pixels, flags 1 an opcode stream.
//!
//! Classification of a whole blob is by size: `4+w*h == len` for a picture,
//! `8+w*h == len` for a cel, with sane bounds (width and height at least 1
//! and at most 320 and 200). A greedy reader that accepted 1×N would
//! false-positive on walk graphs and hotspot blobs. Cels embedded in an
//! `RDV1.00` resource are the 8-byte form as a prefix: [`Picture::parse_prefix`].

use crate::error::{Error, Result};
use crate::{Record, bytes, slice, tail, u16le};

/// Largest picture the VGA mode can hold. Anything larger is not a picture
/// this engine would have drawn.
const MAX_W: u16 = 320;
/// Same, for height.
const MAX_H: u16 = 200;

/// The index a cel's own skip runs leave behind. Sprites are drawn over a
/// scene and the engine treats 0 as see-through, so a decoded cel carries 0
/// wherever the stream skips instead of writing.
const TRANSPARENT: u8 = 0;

/// A paletted image: one byte per pixel, top row first, no padding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    /// Pixels per row.
    pub width: u16,
    /// Rows.
    pub height: u16,
    /// `width * height` palette indices.
    pub pixels: Vec<u8>,
}

impl Picture {
    /// Parses a picture (`u16 w, u16 h` + pixels) or a cel (the same plus a
    /// 4-byte flags/reserved header). The two are distinguished by whether
    /// `4+w*h` or `8+w*h` equals the blob.
    pub fn parse(data: &[u8]) -> Result<Self> {
        parse_with_len(data, true, TRANSPARENT)
    }

    /// Parses a picture or cel that is a *prefix* of `data`.
    ///
    /// `RDV1.00` offset tables name cels in the middle of the room resource.
    /// The bytes after a cel are the next record, not padding, so
    /// [`parse`](Self::parse) — which classifies by whether `4+w*h` or
    /// `8+w*h` equals the whole slice — refuses every embedded tile. This
    /// reader takes the header, then exactly `w*h` pixels, and ignores the
    /// tail. It is only for a pointer that already names a cel; feeding it
    /// a hotspot blob or a walk graph would false-positive the way a greedy
    /// 1×N reader would.
    pub fn parse_prefix(data: &[u8]) -> Result<Self> {
        parse_with_len(data, false, TRANSPARENT)
    }

    /// [`parse_prefix`](Self::parse_prefix) with the pixels the stream never
    /// writes left as `fill` instead of index 0.
    ///
    /// A flags-1 stream says where it draws and where it skips, so "index 0"
    /// and "not drawn" are the same byte in the decoded buffer only because
    /// the buffer starts zeroed. The font needs them apart: a glyph's own
    /// literals are 0 (outline) and 1 (body), and the current ink is added
    /// to every literal before the glyph is drawn — so 0 is a color there,
    /// not a hole. Decoding the same cel under two fills and keeping the
    /// pixels that agree is what separates the two.
    pub(crate) fn parse_prefix_filled(data: &[u8], fill: u8) -> Result<Self> {
        parse_with_len(data, false, fill)
    }

    /// `true` when `data` is a well-formed picture or cel of this format.
    pub fn looks_like(data: &[u8]) -> bool {
        Self::parse(data).is_ok()
    }
}

fn parse_with_len(data: &[u8], exact: bool, fill: u8) -> Result<Picture> {
    let head = Record(bytes::<4>(data, 0)?);
    let width = head.u16::<0>();
    let height = head.u16::<2>();
    if width == 0 || height == 0 || width > MAX_W || height > MAX_H {
        return Err(Error::OutOfRange {
            field: "picture size",
            value: crate::pair(width, height),
            allowed: "1..=320 by 1..=200",
        });
    }
    // At most 320×200, so the product is exact.
    let area = usize::from(width).saturating_mul(usize::from(height));
    let header = if exact {
        // The whole blob is the image: its length says which header it has.
        if data.len() == area.saturating_add(8) {
            8
        } else if data.len() == area.saturating_add(4) {
            4
        } else {
            return Err(Error::SizeMismatch {
                width,
                height,
                have: data.len(),
                need: area.saturating_add(4),
            });
        }
    } else if data.len() >= 8 && u16le(data, 6)? == 0 {
        // Embedded cels are the 8-byte form (`w, h, flags, 0`). A 4-byte
        // picture is taken only when the reserved word is *not* 0, so a
        // 320×200 still whose first pixels happen to be zeros is not taken
        // as a cel. Flags 1 is a stream, not `w*h` raw pixels.
        if u16le(data, 4)? == 1 {
            return Ok(Picture {
                width,
                height,
                pixels: decode_flags1(tail(data, 8)?, width, height, fill),
            });
        }
        8
    } else {
        4
    };
    let Ok(pixels) = slice(data, header, area) else {
        return Err(Error::Truncated {
            off: header,
            need: area,
            have: data.len().saturating_sub(header),
        });
    };
    if header == 8 && u16le(data, 4)? == 1 {
        return Ok(Picture {
            width,
            height,
            pixels: decode_flags1(pixels, width, height, fill),
        });
    }
    Ok(Picture {
        width,
        height,
        pixels: pixels.to_vec(),
    })
}

/// The flags-1 stream, fully visible.
///
/// Each word's high byte is the opcode; 0 ends. A skip leaves `fill`,
/// which is index 0 for every caller but the font — there a glyph's own
/// literals include 0, so the two have to be told apart. The destination
/// is a tight `w×h` buffer, not the 320-wide framebuffer, so there is no
/// gap to add at the end of a row.
fn decode_flags1(src: &[u8], w: u16, h: u16, fill: u8) -> Vec<u8> {
    let bx = usize::from(w);
    let len = bx.saturating_mul(usize::from(h));
    let mut s = Stream {
        src,
        si: 0,
        out: vec![fill; len],
        di: 0,
    };
    let Some((_, mut ah)) = s.word() else {
        return s.out;
    };
    loop {
        match ah {
            0 => break,
            2 => {
                // Skip a whole row.
                s.skip(bx);
            }
            4 => {
                // An opaque row.
                s.copy(bx);
            }
            6 => {
                // Copy `ah`, skip `al`, newline.
                let Some((al, n)) = s.word() else {
                    break;
                };
                s.copy(usize::from(n));
                s.skip(usize::from(al));
            }
            8 => {
                // Skip `al`, copy `ah`, newline.
                let Some((al, n)) = s.word() else {
                    break;
                };
                s.skip(usize::from(al));
                s.copy(usize::from(n));
            }
            0x0A => {
                // Copy `ah`, skip `al`, copy byte-count, newline.
                let Some((al, n)) = s.word() else {
                    break;
                };
                s.copy(usize::from(n));
                s.skip(usize::from(al));
                let Some(extra) = s.byte() else {
                    break;
                };
                s.copy(usize::from(extra));
            }
            0x0C => {
                // Skip byte, copy `ah`, skip `al`, newline.
                let Some(first) = s.byte() else {
                    break;
                };
                s.skip(usize::from(first));
                let Some((al, n)) = s.word() else {
                    break;
                };
                s.copy(usize::from(n));
                s.skip(usize::from(al));
            }
            0x0E => {
                // Skip `al`, then as many copy/skip pairs as the data
                // word's high byte says.
                let Some((al, pairs)) = s.word() else {
                    break;
                };
                s.skip(usize::from(al));
                for _ in 0..pairs {
                    let Some((skip_n, n)) = s.word() else {
                        break;
                    };
                    s.copy(usize::from(n));
                    s.skip(usize::from(skip_n));
                }
            }
            _ => break,
        }
        // Every opcode ends by fetching the next one.
        let Some((_, next)) = s.word() else {
            break;
        };
        ah = next;
        if s.di >= len {
            break;
        }
    }
    s.out
}

/// A flags-1 stream being decoded: where it reads, and where it writes in
/// the cel it fills. A read past the stream's end and a write past the
/// cel's are where the decoder stops, never a fault.
struct Stream<'a> {
    src: &'a [u8],
    si: usize,
    out: Vec<u8>,
    di: usize,
}

impl Stream<'_> {
    /// The next word as its low and high byte.
    fn word(&mut self) -> Option<(u8, u8)> {
        let &[lo, hi] = self.src.get(self.si..)?.first_chunk::<2>()?;
        self.si = self.si.saturating_add(2);
        Some((lo, hi))
    }

    /// The next byte.
    fn byte(&mut self) -> Option<u8> {
        let b = *self.src.get(self.si)?;
        self.si = self.si.saturating_add(1);
        Some(b)
    }

    /// `n` literal pixels, or as many as the stream and the cel both hold.
    fn copy(&mut self, n: usize) {
        for _ in 0..n {
            let (Some(&b), Some(slot)) = (self.src.get(self.si), self.out.get_mut(self.di)) else {
                return;
            };
            *slot = b;
            self.si = self.si.saturating_add(1);
            self.di = self.di.saturating_add(1);
        }
    }

    /// `n` pixels left as they are, no further than the cel's end.
    fn skip(&mut self, n: usize) {
        self.di = self.di.saturating_add(n).min(self.out.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_4byte_header_picture_parses() {
        let mut v = vec![4, 0, 2, 0];
        v.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let p = Picture::parse(&v).unwrap();
        assert_eq!((p.width, p.height), (4, 2));
        assert_eq!(p.pixels, vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn flags1_opcode_four_copies_a_row() {
        // w=4 h=1, flags 1: opcode ah=4, four pixels, opcode 0.
        let mut v = vec![4, 0, 1, 0, 1, 0, 0, 0];
        v.extend_from_slice(&[0x00, 0x04, 9, 8, 7, 6, 0x00, 0x00]);
        let p = Picture::parse_prefix(&v).unwrap();
        assert_eq!(p.pixels, vec![9, 8, 7, 6]);
    }

    #[test]
    fn an_8byte_header_cel_parses() {
        let mut v = vec![4, 0, 2, 0, 2, 0, 0, 0];
        v.extend_from_slice(&[9; 8]);
        let p = Picture::parse(&v).unwrap();
        assert_eq!(p.pixels.len(), 8);
        assert_eq!(p.pixels[0], 9);
    }

    #[test]
    fn a_zero_dimension_is_refused() {
        assert!(Picture::parse(&[0, 0, 10, 0]).is_err());
    }

    #[test]
    fn an_oversized_header_is_refused() {
        // 400x200 would be a picture the VGA mode cannot hold.
        let v = vec![144, 1, 200, 0]; // 400, 200
        assert!(Picture::parse(&v).is_err());
    }

    #[test]
    fn parse_prefix_reads_an_embedded_cel_and_ignores_the_tail() {
        // 16×16 tile, flags 0, then 100 bytes of whatever follows in an RDV.
        let mut v = vec![16, 0, 16, 0, 0, 0, 0, 0];
        v.extend(std::iter::repeat_n(7u8, 256));
        v.extend_from_slice(&[0xAA; 100]);
        assert!(Picture::parse(&v).is_err(), "the tail is not a small pad");
        let p = Picture::parse_prefix(&v).unwrap();
        assert_eq!((p.width, p.height), (16, 16));
        assert_eq!(p.pixels.len(), 256);
        assert!(p.pixels.iter().all(|&b| b == 7));
    }
}
