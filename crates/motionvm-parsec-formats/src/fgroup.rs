//! Outer FGROUP tables.
//!
//! The engine's own name, from unpacked EXE strings: the outer table is an
//! **FGROUP** (entries **FDIR**). Used when a file holds several RIBs (or a
//! mix of RIB and `NULLPTR` slots). Layout:
//!
//! ```text
//! u32 off[n]
//! u32 0
//! u32 size[n]
//! ```
//!
//! `off[0] = 4*(2n+1)`, and in each of Dunkle Schatten's three FGROUPs
//! `off[i]+size[i] = off[i+1]` with the last slot ending at the end of the
//! file. Empty slot payload is the literal `NULLPTR\r\n`, nine bytes.
//!
//! Most `.DAT` files are a single RIB with no FGROUP wrapper (`CMBMOU.DAT`,
//! `DIALOG.DAT`, `PANELS.DAT`, …). `DSROOM.DAT` is an FGROUP of 60 room
//! slots, and `INTROS.DAT` and `EXTROS.DAT` are FGROUPs too.

use crate::error::{Error, Result};
use crate::rib::{Rib, table_count};
use crate::{NULLPTR, bytes, records, u32at, wide};

/// One slot of an FGROUP: a RIB, an empty `NULLPTR` marker, or raw bytes
/// that are neither.
#[derive(Debug, Clone)]
pub enum Slot {
    /// A parsed inner RIB.
    Rib(Rib),
    /// The literal `NULLPTR\r\n` empty-slot marker.
    Null,
    /// Bytes that are not a RIB and not the empty marker. The intros and
    /// outros keep their palettes this way — `start, count` banks of 128,
    /// 192 and 224 colors — and `INTROS.DAT` three 34-byte texts.
    Raw(Vec<u8>),
}

/// An outer FGROUP: `n` slots, each independently a RIB or empty.
#[derive(Debug, Clone)]
pub struct Fgroup {
    /// Slots in table order. Slot 0 of `DSROOM.DAT` is [`Slot::Null`].
    pub slots: Vec<Slot>,
}

/// The most slots a table may name; a larger count is a damaged header.
const MAX_SLOTS: usize = 4096;

impl Fgroup {
    /// Parses a buffer as an FGROUP. The caller has already decided the file
    /// is not a single RIB — [`Self::looks_like`] is the cheap check.
    pub fn parse(data: &[u8]) -> Result<Self> {
        bytes::<12>(data, 0)?;
        let off0 = u32at(data, 0)?;
        // off[0] = 4*(2n+1): an odd number of words, three at least.
        let n = table_count(off0).ok_or_else(|| Error::Corrupt {
            what: "FGROUP",
            detail: format!("off[0] is {off0}, which is not 4*(2n+1)"),
        })?;
        if n > MAX_SLOTS {
            return Err(Error::OutOfRange {
                field: "FGROUP n",
                value: crate::long(n),
                allowed: "1 to 4096",
            });
        }
        let table = records::<4>(data, 0, n.saturating_mul(2).saturating_add(1))?;
        let (offs, rest) = table.split_at_checked(n).unwrap_or_default();
        let (zero, sizes) = rest.split_first().unwrap_or((&[0; 4], &[]));
        let zero = u32::from_le_bytes(*zero);
        if zero != 0 {
            return Err(Error::Corrupt {
                what: "FGROUP",
                detail: format!("separator word is {zero}, expected 0"),
            });
        }
        let mut slots = Vec::with_capacity(n);
        for (i, (off, size)) in offs.iter().zip(sizes).enumerate() {
            let start = wide(u32::from_le_bytes(*off));
            let size = wide(u32::from_le_bytes(*size));
            let blob =
                data.get(start..start.saturating_add(size))
                    .ok_or_else(|| Error::Corrupt {
                        what: "FGROUP",
                        detail: format!(
                            "slot {i} at {start:#x}+{size} runs past the {}-byte file",
                            data.len()
                        ),
                    })?;
            let slot = if blob.starts_with(NULLPTR) {
                Slot::Null
            } else if Rib::looks_like(blob) {
                Slot::Rib(Rib::parse(blob)?)
            } else {
                Slot::Raw(blob.to_vec())
            };
            slots.push(slot);
        }
        Ok(Fgroup { slots })
    }

    /// Cheap shape check: `off[0] = 4*(2n+1)` for a plausible `n`, and the
    /// separator word is zero. Does not parse the slots.
    pub fn looks_like(data: &[u8]) -> bool {
        if bytes::<12>(data, 0).is_err() {
            return false;
        }
        let Some(n) = u32at(data, 0)
            .ok()
            .filter(|off0| *off0 <= data.len())
            .and_then(table_count)
            .filter(|n| *n <= MAX_SLOTS)
        else {
            return false;
        };
        records::<4>(data, 0, n.saturating_add(1))
            .ok()
            .and_then(<[_]>::last)
            == Some(&[0; 4])
    }
}

/// Opens a `.DAT` file that is either a single RIB or an FGROUP of them.
///
/// The two layouts are distinguished by magic / header arithmetic, not by
/// the file name. `CMBMOU.DAT` is a single RIB; `DSROOM.DAT` is an FGROUP.
pub fn open_dat(data: &[u8]) -> Result<Dat> {
    if Rib::looks_like(data) {
        Ok(Dat::Rib(Rib::parse(data)?))
    } else if Fgroup::looks_like(data) {
        Ok(Dat::Fgroup(Fgroup::parse(data)?))
    } else {
        Err(Error::Corrupt {
            what: "DAT",
            detail: "neither a RIB nor an FGROUP".into(),
        })
    }
}

/// A `.DAT` file's outer shape.
#[derive(Debug, Clone)]
pub enum Dat {
    /// A single RIB, the common case.
    Rib(Rib),
    /// Several slots.
    Fgroup(Fgroup),
}

impl Dat {
    /// Every RIB this file holds, flattening an FGROUP. `NULLPTR` slots are
    /// skipped; raw slots are ignored.
    pub fn ribs(&self) -> Vec<&Rib> {
        match self {
            Self::Rib(r) => vec![r],
            Self::Fgroup(g) => g
                .slots
                .iter()
                .filter_map(|s| match s {
                    Slot::Rib(r) => Some(r),
                    _ => None,
                })
                .collect(),
        }
    }
}
