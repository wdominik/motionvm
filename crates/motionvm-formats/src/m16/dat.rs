//! The `DATA.-n-` container: a whole 16-bit MOTION game, on as many volumes
//! as it took floppies.
//!
//! ```text
//! volume 1
//! 0x00  u16      boot module number            (100 in all three games)
//! 0x02  u16      boot word id                  (401, `RUN`)
//! 0x04  u16[7]   slot counts: gfx, blk, scr, pal, fnt, frt, txt
//! 0x12  u16      volumes the game ships on     (1, 2 or 3)
//! 0x14  u16      spare entries after the offset table
//! 0x16  u16[7]   per segment: non-zero if that segment's items are packed
//! 0x24  u16      spare
//! 0x26  u16[n]   occupancy: a volume bitmask, `1 << (volume - 1)`; 0 = empty
//!       u32[n+s] file offset of every slot, then `s` spare zeros
//!       ...      the items, back to back
//!
//! volume 2 and up
//! 0x00  u32[n+s] the same table over the same slots, for this volume
//!       ...      the items, back to back
//! ```
//!
//! `n` is the sum of the seven counts — 4345 in Die Enviro-Kids greifen ein and in
//! Jeff Jet, 4395 in Hilfe für Amajambere. The
//! seven segments share one slot space in the order above, so a script's
//! sprite id is the slot itself, a block id is the slot minus the GFX count, a
//! module number is the slot minus the GFX and BLK counts, and so on;
//! [`Segment`] does that arithmetic. The engine computes the same bases at run
//! time: its `.blk` path adds the GFX count and its `.fth` path the GFX and
//! BLK counts (`HPPLAY.EXE` file `0x400c`).
//!
//! **Which volume a slot is on** is the occupancy word, and it is a bitmask,
//! not a count: the engine builds `1 << (volume - 1)` from the volume it has
//! open and matches it (`00e0:0017` in the loader above). No slot is on two
//! volumes. A slot's item ends at the next slot's offset *in its own volume*
//! — the offsets of a volume are cumulative over all slots, so a slot living
//! elsewhere repeats its neighbour's offset — or at the end of that volume for
//! the last slot. Measured over the nine volumes of Die Enviro-Kids greifen ein,
//! Jeff Jet, Hilfe für Amajambere and Eddy M.: every volume's items tile it
//! exactly, from its own table's end
//! to its last byte.
//!
//! **Whether items are packed** is the seven words at `0x16`, one per segment
//! in slot order. The loader reads the word for the segment it is loading
//! (`+0x16` for `.gfx` through `+0x22` for `.txt`) and branches on it: zero
//! reads the item straight into place, non-zero reads it aside and runs it
//! through the GFXCRUNCH decoder (`00e0:04eb`, decoder at `1614:000d` = file
//! `0x1934d`). A packed item is an 8-byte header — `u16` unpacked length,
//! `u16` packed length, `u16` 2048, `u16` 9 — and then the stream; the decoder
//! reads the packed length at `+2`, skips the eight bytes, and starts at
//! 9-bit codes with a 2048-entry dictionary, which is [`crate::lzw`] with
//! `max_bits` 11. It never reads the 2048 and the 9 back, so they are the
//! format writing down what the code assumes. Over the four games above the
//! flag and the item shape agree in all 28 segments, without exception.
//!
//! The container's header also names the word the engine runs first: the
//! 16-bit engine has no bootstrap file like the 32-bit engine's `SYSTEM.RSC`
//! — see [`Boot`].
//!
//! Measured on the one volume of Die Enviro-Kids greifen ein: the flag table ends
//! at 8728, the offset
//! table at 26108 with three spare zeros, the first item starts at 26120, and
//! the items account for every byte after the tables. On Jeff Jet's two: seven
//! spare zeros, the first item of volume 1 at 26136 and of volume 2 at 17408 —
//! which is that table's own length, `4 * (4345 + 7)` — 1730 slots flagged,
//! 1728 of them with bytes, all packed, 2 459 890 bytes unfolding to
//! 8 412 811.

use crate::error::{Error, Result};
use crate::{find_ci, lzw, reserve, u16le, u32le};

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
    /// occupied in Die Enviro-Kids greifen ein.
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
    /// The module to load — 100 in every shipped container.
    pub module: u16,
    /// The global word id to run in it — 401 in every one, which is `RUN`.
    pub word: u16,
}

/// Bytes of fixed header before the occupancy table.
pub const HEADER_LEN: usize = 0x26;
/// Bytes of packed-item header before the LZW stream.
const PACK_HEADER_LEN: usize = 8;
/// The dictionary size a packed item's header records — 2048 entries, which is
/// an 11-bit ceiling.
const PACK_DICTIONARY: u16 = 2048;
/// The code width a packed item's header records, and the only one the decoder
/// starts at.
const PACK_INITIAL_WIDTH: u16 = 9;

/// Where one slot's bytes are: which store, and the range inside it. An empty
/// range is a slot with no bytes, whatever its occupancy word says.
#[derive(Clone, Copy)]
struct Span {
    store: usize,
    start: usize,
    end: usize,
}

impl Span {
    const EMPTY: Span = Span {
        store: 0,
        start: 0,
        end: 0,
    };

    fn is_empty(self) -> bool {
        self.end == self.start
    }
}

/// One opened game: every `DATA.-n-` volume of it, and the items unpacked.
///
/// Packing is undone once, when the container is opened, into a buffer of its
/// own; [`Container::item`] hands out a slice either way and callers cannot
/// tell a packed game from a plain one. A volume whose every item was unpacked
/// is dropped once the last of them is decoded, because nothing can borrow from
/// it again.
pub struct Container {
    /// The volumes as read, and behind them one store holding the unpacked
    /// items back to back. A volume nothing points into is empty here; its
    /// length is kept in `volume_lens`.
    stores: Vec<Vec<u8>>,
    volume_lens: Vec<usize>,
    counts: [usize; 7],
    bases: [usize; 7],
    flags: Vec<u16>,
    packed: [bool; 7],
    spare: usize,
    spans: Vec<Span>,
    tables: Vec<Vec<u32>>,
    boot: Boot,
    source: String,
}

impl Container {
    /// Opens the game installed in `dir`: `DATA.-1-`, and every further volume
    /// its header declares, in whatever case they are spelled.
    ///
    /// A declared volume that is not there is an error naming the file rather
    /// than a container that quietly holds half a game — Jeff Jet keeps every
    /// palette, both fonts and its font reference table on volume 2, so a
    /// missing second volume does not degrade the game, it blanks it.
    pub fn open_dir(dir: impl AsRef<std::path::Path>) -> Result<Self> {
        let dir = dir.as_ref();
        let first = find_ci(dir, "DATA.-1-").ok_or_else(|| Error::Corrupt {
            what: "DATA container",
            detail: format!("no DATA.-1- in {}", dir.display()),
        })?;
        let head = std::fs::read(&first)?;
        let declared = Self::declared_volumes(&head)?;
        let mut stores = Vec::with_capacity(declared);
        stores.push(head);
        for n in 2..=declared {
            let name = format!("DATA.-{n}-");
            let path = find_ci(dir, &name).ok_or_else(|| Error::Corrupt {
                what: "DATA container",
                detail: format!(
                    "{} says the game is on {declared} volumes, but there is no {name} in {}",
                    first.display(),
                    dir.display()
                ),
            })?;
            stores.push(std::fs::read(path)?);
        }
        Self::from_volumes(stores, first.display().to_string())
    }

    /// The same from memory, for a game on one volume. `source` is only for
    /// error messages.
    pub fn from_bytes(data: Vec<u8>, source: String) -> Result<Self> {
        Self::from_volumes(vec![data], source)
    }

    /// The same from memory, volume by volume, volume 1 first.
    pub fn from_volumes(volumes: Vec<Vec<u8>>, source: String) -> Result<Self> {
        let Some(head) = volumes.first() else {
            return Err(Error::Corrupt {
                what: "DATA container",
                detail: "no volumes at all".into(),
            });
        };
        let declared = Self::declared_volumes(head)?;
        if declared != volumes.len() {
            return Err(Error::Corrupt {
                what: "DATA container",
                detail: format!(
                    "the header says {declared} volume(s) but {} were handed over",
                    volumes.len()
                ),
            });
        }
        let boot = Boot {
            module: u16le(head, 0)?,
            word: u16le(head, 2)?,
        };
        let mut counts = [0usize; 7];
        for (i, c) in counts.iter_mut().enumerate() {
            *c = u16le(head, 4 + i * 2)? as usize;
        }
        let mut packed = [false; 7];
        for (i, p) in packed.iter_mut().enumerate() {
            *p = u16le(head, 0x16 + i * 2)? != 0;
        }
        let spare = u16le(head, 20)? as usize;
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
        let mut bases = [0usize; 7];
        let mut cursor = 0;
        for (base, &n) in bases.iter_mut().zip(counts.iter()) {
            *base = cursor;
            cursor += n;
        }
        let flags_end = HEADER_LEN + total * 2;
        let mut flags = reserve(total, head.len(), 2);
        for i in 0..total {
            flags.push(u16le(head, HEADER_LEN + i * 2)?);
        }
        // Volume 1 keeps its table behind the header and the occupancy words;
        // the others are nothing but a table of the same shape and their items.
        let mut tables = Vec::with_capacity(volumes.len());
        for (v, bytes) in volumes.iter().enumerate() {
            let at = if v == 0 { flags_end } else { 0 };
            let mut table = reserve(total + spare, bytes.len(), 4);
            for i in 0..total + spare {
                table.push(u32le(bytes, at + i * 4)?);
            }
            tables.push(table);
        }
        let mut spans = vec![Span::EMPTY; total];
        let mut unpacked: Vec<u8> = Vec::new();
        let unpacked_store = volumes.len();
        for idx in 0..total {
            let flag = flags[idx];
            if flag == 0 {
                continue;
            }
            let volume = flag.trailing_zeros() as usize;
            // A word naming a volume the header does not declare is a
            // contradiction the container reports rather than reads past;
            // `occupancy_mismatches` is where it surfaces.
            let Some(bytes) = volumes.get(volume) else {
                continue;
            };
            let table = &tables[volume];
            let start = table[idx] as usize;
            let end = if idx + 1 < total {
                table[idx + 1] as usize
            } else {
                bytes.len()
            };
            let segment = Self::segment_of(&bases, &counts, idx);
            let local = idx - bases[segment as usize];
            if end < start {
                return Err(Error::Corrupt {
                    what: "DATA container",
                    detail: format!(
                        "{} {local} starts at {start:#x} after the next item at {end:#x}",
                        segment.name(),
                    ),
                });
            }
            if end == start {
                continue;
            }
            // The first item of a volume lies past that volume's table. An
            // offset inside it is the one thing that proves the counts were not
            // read correctly.
            let table_end = if volume == 0 {
                flags_end + (total + spare) * 4
            } else {
                (total + spare) * 4
            };
            if start < table_end {
                return Err(Error::Corrupt {
                    what: "DATA container",
                    detail: format!(
                        "an item of volume {} starts at {start:#x} but that volume's \
                         offset table ends at {table_end:#x}",
                        volume + 1
                    ),
                });
            }
            let item = bytes.get(start..end).ok_or(Error::Truncated {
                off: start,
                need: end - start,
                have: bytes.len(),
            })?;
            if packed[segment as usize] {
                let shaped = item.len() > PACK_HEADER_LEN
                    && u16le(item, 2)? as usize == item.len() - PACK_HEADER_LEN
                    && u16le(item, 4)? == PACK_DICTIONARY
                    && u16le(item, 6)? == PACK_INITIAL_WIDTH;
                if !shaped {
                    return Err(Error::Corrupt {
                        what: "DATA container",
                        detail: format!(
                            "{} {local} is flagged packed, but its {} bytes do not open \
                             on a GFXCRUNCH header",
                            segment.name(),
                            item.len()
                        ),
                    });
                }
                let want = u16le(item, 0)? as usize;
                let raw = lzw::decode(
                    &item[PACK_HEADER_LEN..],
                    PACK_DICTIONARY.max(2).ilog2(),
                    want,
                )?;
                let at = unpacked.len();
                unpacked.extend_from_slice(&raw);
                spans[idx] = Span {
                    store: unpacked_store,
                    start: at,
                    end: unpacked.len(),
                };
            } else {
                spans[idx] = Span {
                    store: volume,
                    start,
                    end,
                };
            }
        }
        let volume_lens: Vec<usize> = volumes.iter().map(Vec::len).collect();
        let mut stores = volumes;
        stores.push(unpacked);
        // A volume whose every item was unpacked cannot be borrowed from again,
        // and holding its bytes would double a packed game's footprint for
        // nothing. Jeff Jet packs all seven segments, so both its volumes go.
        for (v, store) in stores.iter_mut().enumerate().take(volume_lens.len()) {
            if !spans.iter().any(|s| !s.is_empty() && s.store == v) {
                *store = Vec::new();
            }
        }
        Ok(Self {
            stores,
            volume_lens,
            counts,
            bases,
            flags,
            packed,
            spare,
            spans,
            tables,
            boot,
            source,
        })
    }

    /// How many volumes the header of `head` says the game is on, with a
    /// header that says none read as the one volume it is.
    ///
    /// Zero is not a count any shipped container holds — Die Enviro-Kids greifen ein
    /// says 1, Jeff Jet and Hilfe für Amajambere 2, Eddy M. 3 — and reading it
    /// as one is what lets a
    /// hand-built fixture of nothing but slot counts still open.
    fn declared_volumes(head: &[u8]) -> Result<usize> {
        if head.len() < HEADER_LEN {
            return Err(Error::Corrupt {
                what: "DATA container",
                detail: format!("file is only {} bytes", head.len()),
            });
        }
        Ok((u16le(head, 18)? as usize).max(1))
    }

    /// Which segment slot `idx` belongs to.
    fn segment_of(bases: &[usize; 7], counts: &[usize; 7], idx: usize) -> Segment {
        let mut found = Segment::Txt;
        for (i, seg) in Segment::ALL.iter().enumerate() {
            if idx >= bases[i] && idx < bases[i] + counts[i] {
                found = *seg;
                break;
            }
        }
        found
    }

    /// Where the container was read from, for error messages.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The word the engine starts on.
    pub fn boot(&self) -> Boot {
        self.boot
    }

    /// How many volumes the game is on — 1 for Die Enviro-Kids greifen ein, 2 for
    /// the other two.
    pub fn volumes(&self) -> usize {
        self.volume_lens.len()
    }

    /// How many spare `u32` entries follow the offset table — 3 in
    /// Die Enviro-Kids greifen ein, 7 in the other two. They are zero in every
    /// shipped container; what the authoring
    /// tool kept them for is open.
    pub fn spare_offsets(&self) -> usize {
        self.spare
    }

    /// Whether this segment's items are stored packed. Reading them does not
    /// depend on it — [`Container::item`] hands out unpacked bytes either way.
    pub fn packed(&self, segment: Segment) -> bool {
        self.packed[segment as usize]
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

    /// The bytes of one item, unpacked, or `None` if the slot is empty.
    ///
    /// `id` is the local id the script uses — the sprite number, the block
    /// number, the module number — not the slot index.
    pub fn item(&self, segment: Segment, id: usize) -> Result<Option<&[u8]>> {
        let span = self.spans[self.slot(segment, id)?];
        if span.is_empty() {
            return Ok(None);
        }
        Ok(Some(&self.stores[span.store][span.start..span.end]))
    }

    /// The occupancy word of one slot, as stored: `1 << (volume - 1)`, or 0.
    pub fn occupancy(&self, segment: Segment, id: usize) -> Result<u16> {
        Ok(self.flags[self.slot(segment, id)?])
    }

    /// Local ids of every filled slot of `segment`, ascending.
    pub fn present(&self, segment: Segment) -> Vec<usize> {
        let base = self.bases[segment as usize];
        (0..self.counts[segment as usize])
            .filter(|&id| !self.spans[base + id].is_empty())
            .collect()
    }

    /// Slots whose occupancy word and volume tables disagree: a word naming a
    /// volume the header does not declare, a flagged slot whose volume gives it
    /// no bytes, or an unflagged slot that some volume does.
    ///
    /// Empty for the file of Die Enviro-Kids greifen ein. Jeff Jet answers with two
    /// GFX slots, 1319 and
    /// 1848, that are flagged for a volume whose table gives them a length of
    /// zero — the container saying a sprite is there and then not having one.
    pub fn occupancy_mismatches(&self) -> Vec<usize> {
        (0..self.spans.len())
            .filter(|&idx| (self.flags[idx] != 0) != !self.spans[idx].is_empty())
            .collect()
    }

    /// Where volume 1's items begin: the end of its offset table, spare entries
    /// and all. 26120 in Die Enviro-Kids greifen ein, 26136 in Jeff Jet, 26436 in
    /// Hilfe für Amajambere.
    ///
    /// The first item sits exactly there in both, and in Amajambere; Eddy M.'s
    /// second and third volumes leave 24 bytes between the two, so this is
    /// where items may begin rather than where they do.
    pub fn first_item_offset(&self) -> usize {
        HEADER_LEN + self.spans.len() * 2 + (self.spans.len() + self.spare) * 4
    }

    /// Bytes after the last item, summed over the volumes. Zero in every
    /// measured game — unlike the 32-bit engine's containers, this one carries
    /// nothing unreferenced.
    pub fn trailing_slack(&self) -> usize {
        let total = self.spans.len();
        (0..self.volume_lens.len())
            .map(|v| {
                let table = &self.tables[v];
                let end = (0..total)
                    .filter(|&idx| {
                        self.flags[idx] != 0
                            && self.flags[idx].trailing_zeros() as usize == v
                            && !self.spans[idx].is_empty()
                    })
                    .map(|idx| {
                        if idx + 1 < total {
                            table[idx + 1] as usize
                        } else {
                            self.volume_lens[v]
                        }
                    })
                    .max()
                    .unwrap_or(self.volume_lens[v]);
                self.volume_lens[v].saturating_sub(end)
            })
            .sum()
    }
}
