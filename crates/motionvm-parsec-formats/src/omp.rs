//! The object map: resource 2 of a `DSROOM.DAT` slot, tagged `OMP\0`.
//!
//! The Wald and the 30 rows of the maze share one picture, slot 20, and each
//! carries a map in its own slot that tells the room loader what to make of
//! that picture's records for this row: which objects show and on which
//! cel, where they stand, what they are called. The loader loads the row's
//! own slot beside the picture's for it (`0x7B6D`) and binds the map's
//! entries to the picture's records (`0x7BFB` on).
//!
//! ```text
//! +0    "OMP\0"
//! +4    u16 count       # entries, from +20h, 1Ch bytes each
//! +6    u16 flags       # what else the map renames (see ObjectMap::flags)
//! entry:
//! +0    u16 id          # the record it binds: an object's +13h, a hotspot's id
//! +2    u16 new id      # FFh: the object's own defaults at +18h..+1Fh
//! +4    i16 cel         # below 0 hidden, FFh state 3, else the frame shown
//! +6    u16 live        # the object's +1
//! +8    i16 dx, +0Ah i16 dy
//! +0Ch  far object      # the records the entry is bound to, filled at
//! +10h  far hotspot     # load time (0x7DF1, 0x802E, 0x81AB, 0x82D6)
//! +14h  far node
//! +18h  far AN entry
//! ```
//!
//! Measured over Dunkle Schatten's `DSROOM.DAT`: 31 slots carry one — the
//! Wald's (23) and the maze's (30 to 59) — each 760 bytes, 26 entries, flags
//! `3Fh`, and every byte of every entry past `+0Ch` zero.

use crate::{Error, Record, Result, bytes, records};

/// The tag the block opens with.
const MAGIC: &[u8; 4] = b"OMP\0";
/// The header's length: entries start at `+20h`.
const HEADER: usize = 0x20;
/// Bytes per entry.
const ENTRY: usize = 0x1C;

/// One row's object map.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectMap {
    /// `+6`. The loader tests bit 0 (the hotspots renamed, `0x7FB5`),
    /// bit 2 (no samples of the row's own, `0x8355`), bit 3 (no song of
    /// the row's own, `0x8339`), bit 4 (the animation entries renamed,
    /// `0x826A`) and bit 5 (the walk graph's nodes renamed, `0x811E`).
    /// Between two forest rows the slot unload keeps what a set bit names
    /// instead of releasing it: the hotspots (bit 0, `0x8433`), the
    /// dialogue bank (bit 1, `0x8547`), the samples (bit 2, `0x85D4`), the
    /// song (bit 3, `0x85ED`), the animation entries (bit 4, `0x8592`) and
    /// the walk graph (bit 5, `0x84FC`).
    pub flags: u16,
    /// The entries, in file order: a record binds the first one free whose
    /// id is its own.
    pub entries: Vec<MapEntry>,
}

/// One entry of an [`ObjectMap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapEntry {
    /// `+0`: the id of the record it binds.
    pub id: u16,
    /// `+2`: the id the record takes, `FFh` for the object's own defaults.
    pub new_id: u16,
    /// `+4`: the cel, read as the loader compares it (`0x7EC0`, `0x7EC7`).
    pub cel: i16,
    /// `+6`, the byte the loader copies into the object's `+1`.
    pub live: u8,
    /// `+8` and `+0Ah`: what the loader adds to the record's position.
    pub dx: i16,
    /// See [`Self::dx`].
    pub dy: i16,
}

impl ObjectMap {
    /// Parses resource 2 of a slot.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let head = Record(bytes::<HEADER>(data, 0)?);
        if &head.bytes::<0, 4>() != MAGIC {
            return Err(Error::Corrupt {
                what: "object map",
                detail: "no OMP tag".into(),
            });
        }
        let count = usize::from(head.u16::<4>());
        let entries = records::<ENTRY>(data, HEADER, count)?
            .iter()
            .map(|e| {
                let e = Record(e);
                MapEntry {
                    id: e.u16::<0>(),
                    new_id: e.u16::<2>(),
                    cel: e.i16::<4>(),
                    live: e.u8::<6>(),
                    dx: e.i16::<8>(),
                    dy: e.i16::<0x0A>(),
                }
            })
            .collect();
        Ok(ObjectMap {
            flags: head.u16::<6>(),
            entries,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(entries: &[[u8; ENTRY]]) -> Vec<u8> {
        let mut b = vec![0u8; HEADER];
        b[..4].copy_from_slice(MAGIC);
        b[4..6].copy_from_slice(&u16::try_from(entries.len()).unwrap().to_le_bytes());
        b[6..8].copy_from_slice(&0x3Fu16.to_le_bytes());
        for e in entries {
            b.extend_from_slice(e);
        }
        b
    }

    #[test]
    fn an_entry_reads_its_fields() {
        let mut e = [0u8; ENTRY];
        e[0..2].copy_from_slice(&213u16.to_le_bytes());
        e[2..4].copy_from_slice(&213u16.to_le_bytes());
        e[4..6].copy_from_slice(&(-1i16).to_le_bytes());
        e[6] = 1;
        e[8..10].copy_from_slice(&(-176i16).to_le_bytes());
        e[10..12].copy_from_slice(&9i16.to_le_bytes());
        let m = ObjectMap::parse(&block(&[e])).unwrap();
        assert_eq!(m.flags, 0x3F);
        assert_eq!(
            m.entries,
            [MapEntry {
                id: 213,
                new_id: 213,
                cel: -1,
                live: 1,
                dx: -176,
                dy: 9
            }]
        );
    }

    #[test]
    fn a_short_or_untagged_block_is_refused() {
        let mut e = [0u8; ENTRY];
        e[0] = 1;
        let b = block(&[e, e]);
        assert!(ObjectMap::parse(&b[..b.len() - 1]).is_err());
        let mut c = block(&[]);
        c[0] = b'X';
        assert!(ObjectMap::parse(&c).is_err());
    }
}
