//! The RSC resource container (`001.RSC`, `002.RSC`, `003.RSC`).
//!
//! ```text
//! 0x00  u32[6]      slot counts: gfx, txt, blk, fnt, scr, pal
//! 0x18  u8[24]      reserved (all zero in the shipped files)
//! 0x30  u32[n]      offset table, bit 31 = "item present"
//! ```
//!
//! `n` is `2*gfx + txt + blk + fnt + scr + pal` — the graphics count covers two
//! parallel tables, one for 8-bit sprites and one for the (unused here) 16-bit
//! hicolor variant. The table is sorted, and an item's length is the distance to
//! the next entry; equal neighbors mean an empty slot. The final entry doubles
//! as the end sentinel.
//!
//! Note the segment order does *not* match the "Scanning for …" order printed by
//! `ENGINE.EXE`; it was determined from the data itself.

use crate::cursor::Cursor;
use crate::error::{Error, Result};
use crate::{dwords, past_end};

/// The seven resource tables inside a container, in offset-table order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// 8-bit paletted sprite, `32BITGFX`/LZW compressed.
    Gfx8,
    /// 16-bit hicolor sprite. Present in the format, unused by this game.
    Gfx16,
    /// String table.
    Text,
    /// "Block" — mixed data. 26 of the 250 are HMI songs (ids 0-25); the
    /// rest are shade tables, route/click/item tables and dialogue records.
    Block,
    /// Proportional bitmap font.
    Font,
    /// Compiled Forth module ("scriptor file").
    Script,
    /// 768-byte VGA palette.
    Palette,
}

impl Kind {
    /// Every kind a container can hold, in the order the header lists them.
    pub const ALL: [Kind; 7] = [
        Kind::Gfx8,
        Kind::Gfx16,
        Kind::Text,
        Kind::Block,
        Kind::Font,
        Kind::Script,
        Kind::Palette,
    ];

    /// Where this kind's table sits in the header: the variant's position,
    /// which is what a fieldless enum's discriminant is.
    #[expect(
        clippy::as_conversions,
        reason = "a fieldless enum's discriminant is its position in the header's tables"
    )]
    pub(crate) fn slot(self) -> usize {
        self as usize
    }

    /// The kind's short name, as the tools print it.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Gfx8 => "gfx8",
            Kind::Gfx16 => "gfx16",
            Kind::Text => "text",
            Kind::Block => "block",
            Kind::Font => "font",
            Kind::Script => "script",
            Kind::Palette => "palette",
        }
    }
}

const HEADER_LEN: usize = 0x30;
const COUNT_FIELDS: usize = 6;

/// One opened `*.RSC` container.
pub struct Rsc {
    data: Vec<u8>,
    /// Start index into `offsets` for each `Kind`, plus the slot count.
    segments: [(usize, usize); 7],
    offsets: Vec<u32>,
    source: String,
}

impl std::fmt::Debug for Rsc {
    /// Where it came from and how it is laid out, not the file itself.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Rsc")
            .field("source", &self.source)
            .field("len", &self.data.len())
            .field("segments", &self.segments)
            .field("slots", &self.offsets.len())
            .finish()
    }
}

impl Rsc {
    /// Reads one `NNN.RSC` container from disk.
    pub fn open(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let path = path.as_ref();
        let data = std::fs::read(path)?;
        Self::from_bytes(data, path.display().to_string())
    }

    /// The same from memory. `source` is only for error messages.
    pub fn from_bytes(data: Vec<u8>, source: String) -> Result<Self> {
        if data.len() < HEADER_LEN {
            return Err(Error::Corrupt {
                what: "RSC container",
                detail: format!("file is only {} bytes", data.len()),
            });
        }
        let counts = dwords::<COUNT_FIELDS>(&data, 0)?.map(crate::wide);
        let [gfx, text, block, font, script, palette] = counts;
        let sizes = [gfx, gfx, text, block, font, script, palette];
        let total = sizes
            .iter()
            .try_fold(0usize, |sum, &n| sum.checked_add(n))
            .filter(|&total| total != 0 && total <= 1 << 22);
        let Some(total) = total else {
            return Err(Error::Corrupt {
                what: "RSC container",
                detail: format!("implausible slot counts {counts:?}"),
            });
        };

        let mut table = Cursor::new(&data, HEADER_LEN);
        let entries = table.records::<4>(total)?;
        let table_end = table.position();
        // The first data byte must sit exactly where the table ends; that is the
        // strongest single check that the segment layout was read correctly.
        let first = entries
            .first()
            .map_or(0, |o| u32::from_le_bytes(*o) & 0x7fff_ffff);
        if crate::wide(first) != table_end {
            return Err(Error::Corrupt {
                what: "RSC container",
                detail: format!("first item at {first:#x} but offset table ends at {table_end:#x}"),
            });
        }
        let offsets = entries.iter().map(|o| u32::from_le_bytes(*o)).collect();

        let mut segments = [(0usize, 0usize); 7];
        let mut cursor = 0usize;
        for (seg, &n) in segments.iter_mut().zip(sizes.iter()) {
            *seg = (cursor, n);
            cursor = cursor.saturating_add(n);
        }

        Ok(Self {
            data,
            segments,
            offsets,
            source,
        })
    }

    /// Where this container came from, for error messages.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Where `kind`'s slots start in the offset table, and how many there
    /// are.
    #[expect(
        clippy::indexing_slicing,
        reason = "a kind's slot is below seven, the table's length"
    )]
    fn segment(&self, kind: Kind) -> (usize, usize) {
        self.segments[kind.slot()]
    }

    /// Number of slots reserved for `kind` (not the number that are filled).
    pub fn slot_count(&self, kind: Kind) -> usize {
        self.segment(kind).1
    }

    /// The raw bytes of one item, or `None` if the slot is empty.
    ///
    /// Item length is the gap to the next offset, so the last slot of the last
    /// segment can never hold data — it is the end sentinel.
    pub fn item(&self, kind: Kind, id: usize) -> Result<Option<&[u8]>> {
        let (base, count) = self.segment(kind);
        if id >= count {
            return Err(Error::IdOutOfRange {
                kind: kind.name(),
                id,
                count,
            });
        }
        let Some(&[start, next, ..]) = self.offsets.get(base.saturating_add(id)..) else {
            return Ok(None);
        };
        let start = crate::wide(start & 0x7fff_ffff);
        let end = crate::wide(next & 0x7fff_ffff);
        if end <= start {
            return Ok(None);
        }
        let item = self
            .data
            .get(start..end)
            .ok_or_else(|| past_end(&self.data, start, end.saturating_sub(start)))?;
        Ok(Some(item))
    }

    /// Ids of every filled slot of `kind`, ascending.
    pub fn present(&self, kind: Kind) -> Vec<usize> {
        let (base, count) = self.segment(kind);
        let from_base = self.offsets.get(base..).unwrap_or_default();
        from_base
            .iter()
            .zip(from_base.iter().skip(1))
            .take(count)
            .enumerate()
            .filter(|(_, (start, next))| (*start & 0x7fff_ffff) != (*next & 0x7fff_ffff))
            .map(|(id, _)| id)
            .collect()
    }

    /// Bytes after the last indexed item.
    ///
    /// `002.RSC` and `003.RSC` carry a few hundred kilobytes of unreferenced
    /// sprite data here, apparently left behind when items were replaced during
    /// development. Nothing in the index points at it.
    pub fn trailing_slack(&self) -> usize {
        let end = self
            .offsets
            .iter()
            .map(|o| crate::wide(o & 0x7fff_ffff))
            .max()
            .unwrap_or(0);
        self.data.len().saturating_sub(end)
    }
}

/// Several containers addressed as one id space.
///
/// The engine loads `%03d.rsc` files from `RSCPATH` and merges them: each file
/// fills a different part of the shared id range, so a lookup tries every bank.
pub struct Bank {
    banks: Vec<Rsc>,
}

impl std::fmt::Debug for Bank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(&self.banks).finish()
    }
}

/// Whether that path is named the way the engine names a container: three
/// digits, then `.RSC`.
///
/// The engine loads `%03d.rsc` from `RSCPATH`, so the stem is three digits and
/// nothing else — `OLD.RSC` and `001.RSC.bak` are not containers. Both halves
/// are compared case-insensitively, because a copied install often arrives
/// lower-cased.
pub fn is_container_name(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("rsc"))
        && path
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit()))
}

/// Whether `dir` holds a container at all — which is what makes it a 32-bit
/// game's directory.
///
/// Named by pattern rather than by number, because how many a game ships is
/// the game's business: Dunkle Schatten 2 has three, and [`Bank::open_dir`]
/// merges however many it finds.
///
/// A directory that cannot be read answers `false` rather than an error: the
/// question asked is "is a game here", and an unreadable directory holds none
/// that can be opened.
pub fn has_container(dir: &std::path::Path) -> bool {
    std::fs::read_dir(dir)
        .map(|entries| entries.flatten().any(|e| is_container_name(&e.path())))
        .unwrap_or(false)
}

impl Bank {
    /// Opens every `NNN.RSC` in `dir`, in ascending numeric order.
    pub fn open_dir(dir: impl AsRef<std::path::Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let mut files: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| is_container_name(p))
            .collect();
        // Sorted, and not merely for the ascending order the doc promises:
        // `read_dir` hands entries back in whatever order the filesystem keeps
        // them, which differs between machines and between copies of the same
        // install. Ids overlap across banks and the first bank holding one
        // wins, so an unsorted walk would make which item a lookup finds a
        // property of the disk. Nothing else in this engine is allowed to
        // depend on that either.
        files.sort();
        let banks = files
            .into_iter()
            .map(Rsc::open)
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { banks })
    }

    /// The containers this bank merges, in the order they were found.
    pub fn banks(&self) -> &[Rsc] {
        &self.banks
    }

    /// Finds an item by id across all banks; later banks do not shadow earlier
    /// ones because in practice no id is filled twice.
    pub fn item(&self, kind: Kind, id: usize) -> Result<Option<&[u8]>> {
        for b in &self.banks {
            if id < b.slot_count(kind)
                && let Some(x) = b.item(kind, id)?
            {
                return Ok(Some(x));
            }
        }
        Ok(None)
    }

    /// `(bank index, id)` for every filled slot of `kind`.
    pub fn present(&self, kind: Kind) -> Vec<(usize, usize)> {
        let mut v: Vec<(usize, usize)> = self
            .banks
            .iter()
            .enumerate()
            .flat_map(|(i, b)| b.present(kind).into_iter().map(move |id| (i, id)))
            .collect();
        v.sort_by_key(|&(_, id)| id);
        v
    }
}
