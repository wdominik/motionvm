//! Inner RIB bundles.
//!
//! The engine's own name, from unpacked EXE strings: the inner bundle is a
//! **RIB**. Layout:
//!
//! ```text
//! "RIB\0"          # 52 49 42 00
//! u32 bufSize      # decompressed size
//! # then the compressed payload, placed at the start of a bufSize buffer
//! ```
//!
//! After decompression the buffer is:
//!
//! ```text
//! u32 off[0..n]
//! u32 0
//! u32 size[0..n]
//! # resources at off[i], length size[i]
//! ```
//!
//! `off[0] = 8n+4 = 4*(2n+1)` — the same directory shape as the outer
//! [`crate::fgroup`] table — and `off[n-1]+size[n-1] = bufSize`: the last
//! resource runs to the end of the buffer with no slack, in all 65 RIBs of
//! Dunkle Schatten's files that carry a table, and in every live
//! `DSROOM.DAT` slot that last resource is the `RDV1.00` room. A second
//! flavor stores a single object without an offset table — seven slots of
//! `INTROS.DAT` and `EXTROS.DAT`, six of them 320×200 stills. The RIB layer
//! does not care, and this reader reports it as [`Rib::raw`], with no
//! resources and the whole decoded buffer as the object.
//!
//! Empty FGROUP slots use the literal `NULLPTR\r\n` rather than a RIB; see
//! [`crate::fgroup`].

use crate::error::{Error, Result};
use crate::lz;
use crate::{MAGIC_RIB, Record, bytes, records, slice, tail, u32at, wide};
use std::ops::Range;

/// One decompressed RIB: a list of resources sitting in a shared buffer.
#[derive(Debug, Clone)]
pub struct Rib {
    /// The `bufSize` the header declared, which is also `decoded.len()`.
    pub buf_size: usize,
    /// The decompressed buffer, including the offset table.
    pub decoded: Vec<u8>,
    /// Resource payloads, in table order. Empty when the RIB is the raw-object
    /// flavor and [`Self::raw`] is `true` — then [`Self::decoded`] *is* the
    /// object.
    pub resources: Vec<Vec<u8>>,
    /// Whether the payload was stored uncompressed (`len == bufSize`).
    pub stored: bool,
    /// Whether the inner offset table did not parse and the whole buffer is
    /// one object.
    pub raw: bool,
}

impl Rib {
    /// Parses a buffer that starts with `RIB\0`.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let head = Record(bytes::<8>(data, 0)?);
        let magic = head.bytes::<0, 4>();
        if magic != MAGIC_RIB {
            return Err(Error::Corrupt {
                what: "RIB",
                detail: format!("expected RIB\\0, found {magic:02x?}"),
            });
        }
        let buf_size = head.u32at::<4>();
        let payload = tail(data, 8)?;
        let stored = payload.len() == buf_size;
        let decoded = lz::decompress(payload, buf_size)?;
        parse_inner(decoded, buf_size, stored)
    }

    /// `true` when this buffer starts with `RIB\0`.
    pub fn looks_like(data: &[u8]) -> bool {
        data.starts_with(&MAGIC_RIB)
    }

    /// Resource `i`, or `None` when the index is past the table.
    pub fn get(&self, i: usize) -> Option<&[u8]> {
        self.resources.get(i).map(Vec::as_slice)
    }
}

/// The directory a RIB's inner table uses: `u32 off[n]`, `u32 0`,
/// `u32 size[n]`, with `off[0] = 4*(2n+1)` and the resources following.
///
/// `Ok(None)` means the buffer does not open with that shape at all, which
/// for a RIB is the raw-object flavor. An `Err` means it does open with it
/// but a resource runs past the end, which is damage rather than a
/// different format. `what` names the caller in that message.
pub fn directory(buf: &[u8], what: &'static str) -> Result<Option<Vec<Range<usize>>>> {
    let Ok(off0) = u32at(buf, 0) else {
        return Ok(None);
    };
    let Some(n) = table_count(off0).filter(|n| *n <= 4096) else {
        return Ok(None);
    };
    // The table is n offsets + 1 zero + n sizes = 2n+1 u32s = off[0] bytes.
    let Ok(table) = records::<4>(buf, 0, n.saturating_mul(2).saturating_add(1)) else {
        return Ok(None);
    };
    let Some((offs, rest)) = table.split_at_checked(n) else {
        return Ok(None);
    };
    let Some((zero, sizes)) = rest.split_first() else {
        return Ok(None);
    };
    if u32::from_le_bytes(*zero) != 0 {
        return Ok(None);
    }
    let mut out = Vec::with_capacity(n);
    for (i, (off, size)) in offs.iter().zip(sizes).enumerate() {
        let start = wide(u32::from_le_bytes(*off));
        let size = wide(u32::from_le_bytes(*size));
        let end = start.saturating_add(size);
        if end > buf.len() {
            return Err(Error::Corrupt {
                what,
                detail: format!(
                    "resource {i} at {start:#x}+{size} runs past the {}-byte buffer",
                    buf.len()
                ),
            });
        }
        out.push(start..end);
    }
    Ok(Some(out))
}

/// The `n` of a directory whose first offset is `off0`, when `off0` is
/// `8n+4 = 4*(2n+1)` for an `n` of one or more: the table is `n` offsets,
/// the zero and `n` sizes, and the first resource starts right behind it.
/// The outer FGROUP table and a RIB's inner one share the shape.
pub(crate) fn table_count(off0: usize) -> Option<usize> {
    off0.checked_sub(4)
        .filter(|w| *w >= 8 && w.is_multiple_of(8))
        .and_then(|w| w.checked_div(8))
}

fn parse_inner(decoded: Vec<u8>, buf_size: usize, stored: bool) -> Result<Rib> {
    match directory(&decoded, "RIB")? {
        Some(ranges) => {
            let resources = ranges
                .into_iter()
                .map(|r| slice(&decoded, r.start, r.len()).map(<[u8]>::to_vec))
                .collect::<Result<_>>()?;
            Ok(Rib {
                buf_size,
                decoded,
                resources,
                stored,
                raw: false,
            })
        }
        // Raw-object flavor: the whole buffer is the payload.
        None => Ok(Rib {
            buf_size,
            decoded,
            resources: Vec::new(),
            stored,
            raw: true,
        }),
    }
}
