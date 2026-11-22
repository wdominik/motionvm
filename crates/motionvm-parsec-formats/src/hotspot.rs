//! Adventure hotspot tables.
//!
//! RIB resource 4 of a `DSROOM.DAT` slot; 20 slots carry one. Layout:
//!
//! ```text
//! u32 8
//! u16 0
//! u16 count
//! u32 0
//! # then count records of 46 bytes:
//! u32 active          # 1 = live
//! u16 x, y, w, h      # rectangle in the 320×128 playfield
//! u16 verbMask        # bit 15: the hover takes the record
//! u32 nameOff         # into this blob's string pool (CP437, NUL)
//! u32 textOff[7]      # per-verb canned answers; 0 = none
//! ```
//!
//! The string pool starts after the records. Names and answers are
//! NUL-terminated, sometimes wrapping with embedded CR (`\r`) and LF
//! (`\n`).
//!
//! Verb *names* are **not** in this blob. `textOff[0]` is what the hover
//! shows at the pointer, and the verb handlers read `[2]` Benutze, `[3]`
//! Betrachte, `[4]` Sprich mit and `[5]` Nimm; see [`Hotspot::texts`].

use crate::error::Result;
use crate::{Record, bytes, c_string, cp437_to_string, records};

/// Bytes in one hotspot record.
pub const RECORD: usize = 46;
/// Bytes in the table header before the first 46-byte record when the
/// 18-byte mouse-object count at `+4` is 0, as it is in every Dunkle
/// Schatten slot. The 18-byte objects, when there are any, start at `+8`.
pub const HEADER: usize = 12;
/// Bytes in one mouse object (stride `12h`).
pub const MOUSE_RECORD: usize = 18;

/// One clickable rectangle in a room.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hotspot {
    /// `1` when the original considers this hotspot live, which all 132
    /// records in Dunkle Schatten's files are. Any other value is stored
    /// as read.
    pub active: u32,
    /// Left edge, in playfield pixels.
    pub x: u16,
    /// Top edge, in playfield pixels.
    pub y: u16,
    /// Width.
    pub w: u16,
    /// Height.
    pub h: u16,
    /// Bitfield. Bit 15 marks a record the hover takes; the rest is the
    /// verb machine's to read.
    pub verb_mask: u16,
    /// Display name, decoded from CP437. Empty when `nameOff` was 0 or ran
    /// off the blob.
    pub name: String,
    /// Per-verb canned answers, decoded. Index `i` is `textOff[i]`; an empty
    /// string means that slot was 0 or invalid.
    ///
    /// The verb handlers read `[2]` Benutze, `[3]` Betrachte, `[4]` Sprich
    /// mit and `[5]` Nimm. They name run-time offsets `+1Eh`, `+22h`,
    /// `+26h` and `+2Ah`, and the run-time record is the file record
    /// shifted four bytes on from the verb mask, so `texts[0]` is `+16h`
    /// rather than `+12h` there.
    pub texts: [String; 7],
}

impl Hotspot {
    /// `true` when `(px, py)` sits inside the rectangle. The original hit-test
    /// is inclusive on the origin and exclusive on the far edge, which is
    /// what `x <= px < x+w` is.
    pub fn contains(&self, px: i32, py: i32) -> bool {
        let x = i32::from(self.x);
        let y = i32::from(self.y);
        px >= x
            && py >= y
            && px < x.saturating_add(i32::from(self.w))
            && py < y.saturating_add(i32::from(self.h))
    }
}

/// One 18-byte mouse object. Dunkle Schatten's strip is seventeen of them
/// at `2115:0000` (`0x0C40C`).
///
/// ```text
/// +0  far next
/// +4  u16 flags    # bit 0 live
/// +6  u16 extra
/// +8  u16 x, y, w, h
/// +10 u16 id
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseObject {
    /// Word at `+4`. [`MouseObject::live`] tests bit 0.
    pub flags: u16,
    /// Left edge (`+8`).
    pub x: u16,
    /// Top edge (`+A`).
    pub y: u16,
    /// Width (`+C`).
    pub w: u16,
    /// Height (`+E`).
    pub h: u16,
    /// Object id at `+10h`. On the strip the verb buttons are `101h` to
    /// `106h`, the strip itself `114h`, the arrows `115h` and `116h`, and
    /// the inventory slots run from `11Eh`.
    pub id: u16,
}

impl MouseObject {
    /// Parses one record at `off`.
    pub fn parse(data: &[u8], off: usize) -> Result<Self> {
        Ok(Self::read(Record(bytes::<MOUSE_RECORD>(data, off)?)))
    }

    fn read(r: Record<'_, MOUSE_RECORD>) -> Self {
        MouseObject {
            flags: r.u16::<4>(),
            x: r.u16::<8>(),
            y: r.u16::<10>(),
            w: r.u16::<12>(),
            h: r.u16::<14>(),
            id: r.u16::<16>(),
        }
    }

    /// Inclusive origin, exclusive far edge, same as [`Hotspot::contains`].
    pub fn contains(&self, px: i32, py: i32) -> bool {
        let x = i32::from(self.x);
        let y = i32::from(self.y);
        px >= x
            && py >= y
            && px < x.saturating_add(i32::from(self.w))
            && py < y.saturating_add(i32::from(self.h))
    }

    /// `true` when `+4` bit 0 is set.
    pub fn live(&self) -> bool {
        self.flags & 1 != 0
    }
}

/// A hotspot table: header plus `count` records plus the string pool.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotspotTable {
    /// The records, in file order. Two may share a rectangle;
    /// [`HotspotTable::hit`] picks between them.
    pub spots: Vec<Hotspot>,
    /// 18-byte mouse objects from header `+8`, count at `+4`. No Dunkle
    /// Schatten slot carries any (`0x1543B` would register them).
    pub widgets: Vec<MouseObject>,
}

impl HotspotTable {
    /// Parses a hotspot blob. A blob shorter than the header, such as the
    /// nine-byte `NULLPTR` marker, is the empty table, not an error.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let Ok(head) = bytes::<HEADER>(data, 0) else {
            return Ok(HotspotTable {
                spots: Vec::new(),
                widgets: Vec::new(),
            });
        };
        // The leading `u32 8` and the `u32 0` at `+8` are not checked: the
        // records are found by the counts.
        let head = Record(head);
        let n18 = usize::from(head.u16::<4>());
        let count = usize::from(head.u16::<6>());
        let widgets = records::<MOUSE_RECORD>(data, 8, n18)?
            .iter()
            .map(|r| MouseObject::read(Record(r)))
            .collect();
        // 46-byte records start after the 18-byte block (origin `+8`).
        // Count 0 leaves the `u32 0` at `+8` and the records at `HEADER`
        // (12).
        let spots_off = if n18 == 0 {
            HEADER
        } else {
            n18.saturating_mul(MOUSE_RECORD).saturating_add(8)
        };
        let spots = records::<RECORD>(data, spots_off, count)?
            .iter()
            .map(|r| {
                let r = Record(r);
                let mut texts: [String; 7] = Default::default();
                let offs = r.bytes::<18, 28>();
                for (slot, off) in texts.iter_mut().zip(offs.as_chunks::<4>().0) {
                    *slot = cstring(data, crate::wide(u32::from_le_bytes(*off)));
                }
                Hotspot {
                    active: r.u32::<0>(),
                    x: r.u16::<4>(),
                    y: r.u16::<6>(),
                    w: r.u16::<8>(),
                    h: r.u16::<10>(),
                    verb_mask: r.u16::<12>(),
                    name: cstring(data, r.u32at::<14>()),
                    texts,
                }
            })
            .collect();
        Ok(HotspotTable { spots, widgets })
    }

    /// The live hotspot under `(px, py)`, last match winning when two overlap.
    ///
    /// Records with `active == 0` are skipped.
    pub fn hit(&self, px: i32, py: i32) -> Option<&Hotspot> {
        self.spots
            .iter()
            .rev()
            .find(|s| s.active != 0 && s.contains(px, py))
    }
}

fn cstring(data: &[u8], off: usize) -> String {
    cp437_to_string(c_string(data, off))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_blob_is_an_empty_table() {
        let t = HotspotTable::parse(&[]).unwrap();
        assert!(t.spots.is_empty());
        assert!(t.widgets.is_empty());
        assert!(HotspotTable::parse(b"NULLPTR").unwrap().spots.is_empty());
    }

    #[test]
    fn a_count_that_runs_past_the_buffer_is_refused() {
        let mut v = vec![8, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0];
        v.resize(20, 0); // 10 records would need 12+460
        assert!(HotspotTable::parse(&v).is_err());
    }

    #[test]
    fn an_eighteen_byte_mouse_object_has_id_at_plus_ten() {
        let mut rec = [0u8; 18];
        rec[4] = 1;
        rec[8] = 2;
        rec[10] = 0x8F;
        rec[12] = 0x2D;
        rec[14] = 0x1C;
        rec[16] = 0x00;
        rec[17] = 0x01;
        let o = MouseObject::parse(&rec, 0).unwrap();
        assert_eq!(o.id, 0x100);
        assert_eq!((o.x, o.y, o.w, o.h), (2, 0x8F, 0x2D, 0x1C));
        assert!(o.live());
        assert!(o.contains(2, 143));
        assert!(!o.contains(47, 143));
    }
}
