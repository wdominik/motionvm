//! The `DATA.-n-` container: a whole 16-bit MOTION game in one file.
//!
//! ```text
//! 0x00  u16      boot module number            (100 in ENVIRO)
//! 0x02  u16      boot word id                  (401, `RUN`)
//! 0x04  u16[7]   slot counts: gfx, blk, scr, pal, fnt, frt, txt
//! 0x12  u16[2]   open — 1 and 3 in ENVIRO; the first reads like a volume number
//! 0x16  u8[16]   zero
//! 0x26  u16[n]   occupancy: 1 = occupied, 0 = empty
//!       u32[n]   file offset of every slot
//!       u8[12]   zero
//!       ...      the items, back to back
//! ```
//!
//! `n` is the sum of the seven counts — 4345 in ENVIRO. The seven segments
//! share one slot space in the order above, so a script's sprite id is the
//! slot itself, a block id is the slot minus the GFX count, a module number
//! is the slot minus the GFX and BLK counts, and so on; [`Segment`] does that
//! arithmetic. An empty slot has the same offset as the slot after it, and
//! the length of an occupied one is the distance to the next slot with a
//! different offset (or to the end of the file for the last).
//!
//! The occupancy table is the other way of saying the same thing: measured
//! over all 4345 slots of ENVIRO's `DATA.-1-`, a flag is 1 exactly when the
//! offsets say the slot is occupied. With one volume on hand the table is
//! redundant; the engine's strings `DATA.-#i-` and *"Bitte Diskette #d
//! einlegen!"* show the format is built for several, and the table is
//! presumably where a slot's volume would be recorded — which is why the two
//! open header words and the flags are exposed rather than interpreted.
//!
//! The container's header also names the word the engine runs first: the
//! 16-bit engine has no bootstrap file like the 32-bit engine's `SYSTEM.RSC`
//! — see [`Boot`].
//!
//! Measured on ENVIRO's file: the flag table ends at 8728, the offset table
//! at 26108, the first item starts at 26120, the last occupied item at
//! 7 608 232, and the items account for every byte after the tables.

use crate::error::{Error, Result};
use crate::{find_ci, u16le, u32le};

/// Which of the seven segments a slot belongs to, in slot order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Segment {
    /// Sprites — see [`crate::m16::gfx`].
    Gfx,
    /// Blocks: music, animation catalogs, per-location tables.
    Blk,
    /// Script modules — see [`crate::m16::scr`]. The slot minus the segment
    /// base is the module number.
    Scr,
    /// Palettes, 768 bytes of 6-bit RGB — the layout of [`crate::Palette`].
    Pal,
    /// Fonts — see [`crate::m16::font`].
    Fnt,
    /// Font reference tables — [`crate::font::FontRefTable`]; only slot 0 is
    /// occupied in ENVIRO.
    Frt,
    /// Text tables — see [`crate::m16::text`].
    Txt,
}

impl Segment {
    /// Every segment, in slot order.
    pub const ALL: [Segment; 7] = [
        Segment::Gfx,
        Segment::Blk,
        Segment::Scr,
        Segment::Pal,
        Segment::Fnt,
        Segment::Frt,
        Segment::Txt,
    ];

    /// The segment's name, as the engine's file-name patterns spell it
    /// (`#F0R4i.gfx`, `#F0R3i.blk`, …).
    pub fn name(self) -> &'static str {
        match self {
            Segment::Gfx => "gfx",
            Segment::Blk => "blk",
            Segment::Scr => "scr",
            Segment::Pal => "pal",
            Segment::Fnt => "fnt",
            Segment::Frt => "frt",
            Segment::Txt => "txt",
        }
    }
}

/// The word the engine runs first, named by the container's first two header
/// words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Boot {
    /// The module to load — 100 in ENVIRO.
    pub module: u16,
    /// The global word id to run in it — 401 in ENVIRO, which is `RUN`.
    pub word: u16,
}

/// Bytes of fixed header before the occupancy table.
pub const HEADER_LEN: usize = 0x26;
/// Zero bytes between the offset table and the first item.
const TABLE_PAD: usize = 12;

/// One opened `DATA.-n-` container.
pub struct Container {
    data: Vec<u8>,
    counts: [usize; 7],
    bases: [usize; 7],
    flags: Vec<u16>,
    offsets: Vec<u32>,
    boot: Boot,
    open_fields: [u16; 2],
    source: String,
}

impl Container {
    /// Opens the container of the game installed in `dir` — `DATA.-1-`, in
    /// whatever case it is spelled.
    pub fn open_dir(dir: impl AsRef<std::path::Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let path = find_ci(dir, "DATA.-1-").ok_or_else(|| Error::Corrupt {
            what: "DATA container",
            detail: format!("no DATA.-1- in {}", dir.display()),
        })?;
        Self::open(path)
    }

    /// Reads one container from disk.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path = path.as_ref();
        let data = std::fs::read(path)?;
        Self::from_bytes(data, path.display().to_string())
    }

    /// The same from memory. `source` is only for error messages.
    pub fn from_bytes(data: Vec<u8>, source: String) -> Result<Self> {
        if data.len() < HEADER_LEN {
            return Err(Error::Corrupt {
                what: "DATA container",
                detail: format!("file is only {} bytes", data.len()),
            });
        }
        let boot = Boot {
            module: u16le(&data, 0)?,
            word: u16le(&data, 2)?,
        };
        let mut counts = [0usize; 7];
        for (i, c) in counts.iter_mut().enumerate() {
            *c = u16le(&data, 4 + i * 2)? as usize;
        }
        let open_fields = [u16le(&data, 18)?, u16le(&data, 20)?];
        let total: usize = counts.iter().sum();
        // Seven u16 counts cannot exceed 7 * 65535, so the only implausible
        // total is zero — a header of nothing but zeros, which a truncated or
        // wrong file produces as readily as a real one produces 4345.
        if total == 0 {
            return Err(Error::Corrupt {
                what: "DATA container",
                detail: "every slot count is zero".into(),
            });
        }
        let flags_end = HEADER_LEN + total * 2;
        let table_end = flags_end + total * 4;
        if data.len() < table_end + TABLE_PAD {
            return Err(Error::Corrupt {
                what: "DATA container",
                detail: format!(
                    "the tables for {total} slots need {} bytes, file has {}",
                    table_end + TABLE_PAD,
                    data.len()
                ),
            });
        }
        let mut flags = Vec::with_capacity(total);
        for i in 0..total {
            flags.push(u16le(&data, HEADER_LEN + i * 2)?);
        }
        let mut offsets = Vec::with_capacity(total);
        for i in 0..total {
            offsets.push(u32le(&data, flags_end + i * 4)?);
        }
        // The first item must lie past the tables; an offset inside them is
        // the one thing that proves the counts were not read correctly.
        if (offsets[0] as usize) < table_end {
            return Err(Error::Corrupt {
                what: "DATA container",
                detail: format!(
                    "first item at {:#x} but the offset table ends at {table_end:#x}",
                    offsets[0]
                ),
            });
        }
        let mut bases = [0usize; 7];
        let mut cursor = 0;
        for (base, &n) in bases.iter_mut().zip(counts.iter()) {
            *base = cursor;
            cursor += n;
        }
        Ok(Self {
            data,
            counts,
            bases,
            flags,
            offsets,
            boot,
            open_fields,
            source,
        })
    }

    /// Where this container came from, for error messages.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The module and word the engine starts with.
    pub fn boot(&self) -> Boot {
        self.boot
    }

    /// The two header words at 18 and 20 whose meaning is open — 1 and 3 in
    /// ENVIRO. The first reads like the volume number of this file.
    pub fn open_fields(&self) -> [u16; 2] {
        self.open_fields
    }

    /// Number of slots reserved for `segment` (not the number that are filled).
    pub fn slot_count(&self, segment: Segment) -> usize {
        self.counts[segment as usize]
    }

    /// Slot index of a local id, or the error the container reports for one
    /// past the segment's count.
    fn slot(&self, segment: Segment, id: usize) -> Result<usize> {
        let count = self.counts[segment as usize];
        if id >= count {
            return Err(Error::IdOutOfRange {
                kind: segment.name(),
                id,
                count,
            });
        }
        Ok(self.bases[segment as usize] + id)
    }

    /// Whether slot `idx` holds an item: its offset differs from the next
    /// slot's — or, for the last slot, lies before the end of the file.
    fn occupied(&self, idx: usize) -> bool {
        match self.offsets.get(idx + 1) {
            Some(&next) => next != self.offsets[idx],
            None => (self.offsets[idx] as usize) < self.data.len(),
        }
    }

    /// Where the item in slot `idx` ends: at the next slot's offset, or at the
    /// end of the file for the last slot. Only meaningful for an occupied
    /// slot — an empty one shares its offset with the next, so it "ends"
    /// where it starts.
    fn end_of(&self, idx: usize) -> usize {
        self.offsets
            .get(idx + 1)
            .map_or(self.data.len(), |&o| o as usize)
    }

    /// The raw bytes of one item, or `None` if the slot is empty.
    ///
    /// `id` is the local id the script uses — the sprite number, the block
    /// number, the module number — not the slot index.
    pub fn item(&self, segment: Segment, id: usize) -> Result<Option<&[u8]>> {
        let idx = self.slot(segment, id)?;
        if !self.occupied(idx) {
            return Ok(None);
        }
        let start = self.offsets[idx] as usize;
        let end = self.end_of(idx);
        if end < start {
            return Err(Error::Corrupt {
                what: "DATA container",
                detail: format!(
                    "{} {id} starts at {start:#x} after the next item at {end:#x}",
                    segment.name()
                ),
            });
        }
        let slice = self.data.get(start..end).ok_or(Error::Truncated {
            off: start,
            need: end - start,
            have: self.data.len(),
        })?;
        Ok(Some(slice))
    }

    /// The occupancy word of one slot, as stored.
    pub fn occupancy(&self, segment: Segment, id: usize) -> Result<u16> {
        Ok(self.flags[self.slot(segment, id)?])
    }

    /// Local ids of every filled slot of `segment`, ascending — filled as the
    /// offsets say, not as the occupancy table says.
    pub fn present(&self, segment: Segment) -> Vec<usize> {
        let base = self.bases[segment as usize];
        (0..self.counts[segment as usize])
            .filter(|&id| self.occupied(base + id))
            .collect()
    }

    /// Slots whose occupancy word disagrees with the offsets — 1 on an empty
    /// slot, or anything but 1 on an occupied one. Empty for ENVIRO's file;
    /// what a non-empty answer would mean on a multi-volume game is open.
    pub fn occupancy_mismatches(&self) -> Vec<usize> {
        (0..self.offsets.len())
            .filter(|&idx| (self.flags[idx] == 1) != self.occupied(idx))
            .collect()
    }

    /// Where the first item begins: the end of the offset table plus twelve
    /// zero bytes. 26120 in ENVIRO.
    pub fn first_item_offset(&self) -> usize {
        HEADER_LEN + self.offsets.len() * 6 + TABLE_PAD
    }

    /// Bytes after the last item. Zero in ENVIRO's file — unlike the 32-bit
    /// engine's containers, this one carries nothing unreferenced.
    pub fn trailing_slack(&self) -> usize {
        let end = (0..self.offsets.len())
            .rev()
            .find(|&idx| self.occupied(idx))
            .map_or(self.first_item_offset(), |idx| self.end_of(idx));
        self.data.len().saturating_sub(end)
    }
}
