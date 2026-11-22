//! `PSMP2L.DAT`: the game's music library — PSM 2 songs, the samples
//! beside them, and the module heads that join the two.
//!
//! Read by `13ECh:0009h` in Dunkle Schatten's unpacked `XP01` (`0x13EC9`):
//! the file opened (`0x13F00`), `34h` bytes of header read (`0x13F2D`) and
//! its first sixteen held against `PSMLST PSM 2.00\0` (`0x13F3D`; the
//! program's message for a mismatch is "P2L: Ungültiges Signum!"), then the
//! three tables read whole (`0x13F74`, `0x13F9F`, `0x13FCA`).
//!
//! ```text
//! +00  "PSMLST PSM 2.00\0"
//! +10  table A: u32 offset, u32 bound, u32 length   SM8 samples, 12-byte rows
//! +1C  table B: u32 offset, u32 bound, u32 length   PLX songs,   12-byte rows
//! +28  table C: u32 offset, u32 bound, u32 length   module heads, 16-byte rows
//! ```
//!
//! **The middle word of a triple is a bound, not a length.** It is what a
//! fetch of one entry is checked against (`0x1418C`, `0x14364`; "P2L:
//! Ungültiger Index!"), and it is smaller than the table: Dunkle Schatten's
//! A holds 60 rows and a bound of 21 — the effects, the only samples
//! fetched one by one — and B holds 16 rows and a bound of 0, because no
//! song is fetched on its own; it is reached through a module head. The
//! rows are counted from the length.
//!
//! A row of A or B is `u32 offset, u32 stored, u32 size`; a row of C is
//! `u32 assembled, u32 offset, u32 stored, u32 size`. An entry is read the
//! way `0x1442C` reads it: `stored` bytes at `offset`, and when that is not
//! `size`, unpacked into `size` by `1E05h:0000h` (`0x14492`) — the RIB LZ
//! codec of [`crate::lz`]. All 16 songs of Dunkle Schatten's file are
//! packed, all 60 samples and all 16 heads stored.
//!
//! A **module head** is a PSM 2 module with its sections still to come:
//! `MTCVTS PSM 2.00\0`, then at `+10h` its own length — the offset section
//! 0 will have — eight words at `+14h` naming rows of A (`FFFFFFFFh` for
//! none), and at `+34h` the row of B that is its song. [`P2l::module`]
//! assembles one as the loader `13ECh:028Fh` (`0x1414F`) does with the
//! digital driver installed; [`P2l::song`] is what the same loader answers
//! with without it (`0x1435A`): the song's bare `PLX` section, which is all
//! the FM driver plays — the same bytes the assembled module carries behind
//! its head.

use crate::error::{Error, Result};
use crate::{Record, Rib, bytes, long, lz, records, slice};

/// The sixteen bytes the file opens with (`DS:15E8h`).
pub const MAGIC: &[u8; 16] = b"PSMLST PSM 2.00\0";

/// The word a module head carries for a sample slot it leaves empty.
const NO_SAMPLE: u32 = u32::MAX;

/// One stored entry: where it is, how many bytes it takes in the file, and
/// how many it unpacks to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// Offset in the file.
    pub offset: usize,
    /// Bytes stored there.
    pub stored: usize,
    /// Bytes once unpacked; `stored` again for an entry that is not packed.
    pub size: usize,
}

/// One row of table C: a module's head, and the size it assembles to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleEntry {
    /// The assembled module's length — the buffer the loader asks for
    /// (`0x141CE`).
    pub assembled: usize,
    /// The head itself.
    pub head: Entry,
}

/// An opened `PSMP2L.DAT`.
#[derive(Debug, Clone)]
pub struct P2l {
    data: Vec<u8>,
    /// Table A, every row.
    pub samples: Vec<Entry>,
    /// Table B, every row.
    pub songs: Vec<Entry>,
    /// Table C, every row.
    pub modules: Vec<ModuleEntry>,
    /// The bounds a fetch of one entry is held to: A's, B's and C's.
    pub bounds: [usize; 3],
}

impl P2l {
    /// Reads the header and the three tables.
    pub fn parse(data: Vec<u8>) -> Result<Self> {
        let head = Record(bytes::<0x34>(&data, 0)?);
        if head.bytes::<0, 16>() != *MAGIC {
            return Err(Error::Corrupt {
                what: "P2L",
                detail: "the file does not open with PSMLST PSM 2.00".into(),
            });
        }
        let a = (
            head.u32at::<0x10>(),
            head.u32at::<0x14>(),
            head.u32at::<0x18>(),
        );
        let b = (
            head.u32at::<0x1C>(),
            head.u32at::<0x20>(),
            head.u32at::<0x24>(),
        );
        let c = (
            head.u32at::<0x28>(),
            head.u32at::<0x2C>(),
            head.u32at::<0x30>(),
        );
        let samples = rows::<12>(&data, a.0, a.2)?.iter().map(entry).collect();
        let songs = rows::<12>(&data, b.0, b.2)?.iter().map(entry).collect();
        let modules = rows::<16>(&data, c.0, c.2)?
            .iter()
            .map(|r| {
                let r = Record(r);
                ModuleEntry {
                    assembled: r.u32at::<0>(),
                    head: Entry {
                        offset: r.u32at::<4>(),
                        stored: r.u32at::<8>(),
                        size: r.u32at::<12>(),
                    },
                }
            })
            .collect();
        Ok(Self {
            data,
            samples,
            songs,
            modules,
            bounds: [a.1, b.1, c.1],
        })
    }

    /// Module `n`'s song as the loader hands it without the digital driver
    /// (`0x1435A`): `n` held to C's bound (`0x14364`), the head read
    /// (`0x14398`), its word at `+34h` taken as the row of B (`0x143A8`), and
    /// that row read and unpacked (`0x14424`) — a bare `PLX` section.
    pub fn song(&self, n: usize) -> Result<Vec<u8>> {
        let head = self.head(n)?;
        let row = Record(bytes::<0x38>(&head, 0)?).u32at::<0x34>();
        let song = self.song_row(row)?;
        self.read(song)
    }

    /// Row `n` of table A as a card with a digital channel fetches one
    /// sample (`13ECh:0210h`, `0x140D0`): `n` held to A's bound
    /// (`0x140E6`, the program's message for one past it is "P2L:
    /// Ungültiger Index!"), and the row read (`0x14147`) — an `SM8` block.
    /// The program fetches the effects this way, rows 0 to 20 in Dunkle
    /// Schatten's file.
    pub fn sample(&self, n: usize) -> Result<Vec<u8>> {
        let entry = self
            .samples
            .get(n)
            .filter(|_| n < self.bounds[0])
            .ok_or_else(|| Error::OutOfRange {
                field: "P2L sample index",
                value: long(n),
                allowed: "below the header's bound for table A",
            })?;
        self.read(entry)
    }

    /// Module `n` assembled as the loader does with a digital channel
    /// (`0x14186`–`0x1434C`): the head, its song row cleared to 0 (`0x1425C`)
    /// and the song unpacked behind it (`0x14288`), then each of the eight
    /// sample words — `FFFFFFFFh` made 0, any other replaced by the offset
    /// the sample is appended at (`0x142B1`–`0x1434C`).
    pub fn module(&self, n: usize) -> Result<Vec<u8>> {
        let mut buf = self.head(n)?;
        let head = Record(bytes::<0x38>(&buf, 0)?);
        let row = head.u32at::<0x34>();
        let slots = [
            head.u32::<0x14>(),
            head.u32::<0x18>(),
            head.u32::<0x1C>(),
            head.u32::<0x20>(),
            head.u32::<0x24>(),
            head.u32::<0x28>(),
            head.u32::<0x2C>(),
            head.u32::<0x30>(),
        ];
        let song = self.song_row(row)?;
        put(&mut buf, 0x34, 0)?;
        buf.extend(self.read(song)?);
        let mut words = [0u32; 8];
        for (word, slot) in words.iter_mut().zip(slots) {
            if slot == NO_SAMPLE {
                continue;
            }
            let at = u32::try_from(buf.len()).map_err(|_| Error::OutOfRange {
                field: "P2L module length",
                value: long(buf.len()),
                allowed: "at most 4 GiB",
            })?;
            let sample = self
                .samples
                .get(crate::wide(slot))
                .ok_or_else(|| Error::OutOfRange {
                    field: "P2L module sample row",
                    value: u64::from(slot),
                    allowed: "a row of table A",
                })?;
            buf.extend(self.read(sample)?);
            *word = at;
        }
        for (at, word) in SLOTS.into_iter().zip(words) {
            put(&mut buf, at, word)?;
        }
        Ok(buf)
    }

    /// Row `row` of table B, which a head names unchecked: the Ad Lib path
    /// indexes B with it directly (`0x143E2`, `0x14414`), so a row past the
    /// table is a damaged head.
    fn song_row(&self, row: usize) -> Result<&Entry> {
        self.songs.get(row).ok_or_else(|| Error::OutOfRange {
            field: "P2L module song row",
            value: long(row),
            allowed: "a row of table B",
        })
    }

    /// Head `n` of table C, held to C's bound first.
    fn head(&self, n: usize) -> Result<Vec<u8>> {
        let bound = self.bounds[2];
        let entry = self
            .modules
            .get(n)
            .filter(|_| n < bound)
            .ok_or_else(|| Error::OutOfRange {
                field: "P2L module index",
                value: long(n),
                allowed: "below the header's bound for table C",
            })?;
        self.read(&entry.head)
    }

    /// One entry, as `0x1442C` reads it.
    fn read(&self, e: &Entry) -> Result<Vec<u8>> {
        lz::decompress(slice(&self.data, e.offset, e.stored)?, e.size)
    }
}

/// `MUSADL.DRV`, the Ad Lib driver, out of `PSMDRV.DAT`: the file is one
/// RIB of the program's sound drivers, and the program installs resource 5
/// as the music driver (`0x5B7B`) — itself a RIB holding the one file,
/// 4480 bytes once unpacked in Dunkle Schatten's copy.
pub fn ad_lib_driver(psmdrv: &[u8]) -> Result<Vec<u8>> {
    driver(psmdrv, AD_LIB)
}

/// The resource of `PSMDRV.DAT` that is `MUSADL.DRV` (`0x5B6B`, `push 5`).
pub const AD_LIB: usize = 5;

/// The resource of `PSMDRV.DAT` that is `DMABLAST.DRV`, the digital driver
/// (`0x5BB3`, `si = 1`); 9840 bytes once unpacked in Dunkle Schatten's
/// copy.
pub const DMABLAST: usize = 1;

/// Resource `n` of `PSMDRV.DAT`, one driver file: the outer RIB's entry,
/// itself a RIB holding the one file (`0x5B73`, `0x5BE4`, the same loader
/// for both drivers).
pub fn driver(psmdrv: &[u8], n: usize) -> Result<Vec<u8>> {
    let outer = Rib::parse(psmdrv)?;
    let inner = outer.get(n).ok_or_else(|| Error::Corrupt {
        what: "PSMDRV",
        detail: format!("no resource {n}"),
    })?;
    Ok(Rib::parse(inner)?.decoded)
}

/// Where the eight sample words of a module head are.
const SLOTS: [usize; 8] = [0x14, 0x18, 0x1C, 0x20, 0x24, 0x28, 0x2C, 0x30];

/// A table's rows of `N` bytes: `length` bytes at `offset`, cut into rows —
/// a remainder that is not a whole row is damage.
fn rows<const N: usize>(data: &[u8], offset: usize, length: usize) -> Result<&[[u8; N]]> {
    let (rows, rest) = slice(data, offset, length)?.as_chunks::<N>();
    if !rest.is_empty() {
        return Err(Error::Corrupt {
            what: "P2L",
            detail: format!("a table of {length} bytes is not whole rows of {N}"),
        });
    }
    records::<N>(data, offset, rows.len())
}

/// A row of table A or B.
fn entry(r: &[u8; 12]) -> Entry {
    let r = Record(r);
    Entry {
        offset: r.u32at::<0>(),
        stored: r.u32at::<4>(),
        size: r.u32at::<8>(),
    }
}

/// Writes `v` as a `u32` at `at`, which the head's own length holds.
fn put(buf: &mut [u8], at: usize, v: u32) -> Result<()> {
    let have = buf.len();
    let field = buf
        .get_mut(at..)
        .and_then(|rest| rest.first_chunk_mut::<4>())
        .ok_or(Error::Truncated {
            off: at,
            need: 4,
            have,
        })?;
    *field = v.to_le_bytes();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A library of one song, one sample and one module head, every entry
    /// stored: the song a bare `PLX`, the head naming the sample in its
    /// third slot.
    fn library() -> Vec<u8> {
        let song = b"PLX\0song".to_vec();
        let sample = b"SM8\0pcm".to_vec();
        let mut head = b"MTCVTS PSM 2.00\0".to_vec();
        head.extend_from_slice(&0x38u32.to_le_bytes());
        for k in 0..8 {
            let w = if k == 2 { 0u32 } else { u32::MAX };
            head.extend_from_slice(&w.to_le_bytes());
        }
        head.extend_from_slice(&0u32.to_le_bytes());
        let mut f = vec![0u8; 0x34];
        let song_at = f.len();
        f.extend(&song);
        let sample_at = f.len();
        f.extend(&sample);
        let head_at = f.len();
        f.extend(&head);
        let a_at = f.len();
        for v in [sample_at, sample.len(), sample.len()] {
            f.extend_from_slice(&u32::try_from(v).unwrap().to_le_bytes());
        }
        let b_at = f.len();
        for v in [song_at, song.len(), song.len()] {
            f.extend_from_slice(&u32::try_from(v).unwrap().to_le_bytes());
        }
        let c_at = f.len();
        let assembled = head.len() + song.len() + sample.len();
        for v in [assembled, head_at, head.len(), head.len()] {
            f.extend_from_slice(&u32::try_from(v).unwrap().to_le_bytes());
        }
        f[..16].copy_from_slice(MAGIC);
        let triples = [(a_at, 1, 12), (b_at, 0, 12), (c_at, 1, 16)];
        for (i, (o, n, l)) in triples.into_iter().enumerate() {
            for (j, v) in [o, n, l].into_iter().enumerate() {
                let at = 0x10 + 12 * i + 4 * j;
                f[at..at + 4].copy_from_slice(&u32::try_from(v).unwrap().to_le_bytes());
            }
        }
        f
    }

    #[test]
    fn the_ad_lib_path_hands_over_the_bare_song() {
        let p = P2l::parse(library()).unwrap();
        assert_eq!(p.bounds, [1, 0, 1]);
        assert_eq!(p.song(0).unwrap(), b"PLX\0song");
    }

    #[test]
    fn a_module_is_assembled_to_the_size_its_row_says() {
        let p = P2l::parse(library()).unwrap();
        let m = p.module(0).unwrap();
        assert_eq!(m.len(), p.modules[0].assembled);
        let word = |at: usize| u32::from_le_bytes(m[at..at + 4].try_into().unwrap());
        assert_eq!(&m[0x38..0x3c], b"PLX\0", "the song behind the head");
        assert_eq!(word(0x34), 0, "the song row cleared");
        assert_eq!(word(0x14), 0, "an empty slot is 0");
        assert_eq!(word(0x1c), 0x38 + 8, "the sample's offset");
        assert_eq!(&m[0x40..0x44], b"SM8\0");
    }

    #[test]
    fn a_sample_is_fetched_within_its_bound() {
        let p = P2l::parse(library()).unwrap();
        assert_eq!(p.sample(0).unwrap(), b"SM8\0pcm");
        assert!(matches!(p.sample(1), Err(Error::OutOfRange { .. })));
    }

    #[test]
    fn a_module_past_the_bound_is_refused() {
        let p = P2l::parse(library()).unwrap();
        assert!(matches!(p.song(1), Err(Error::OutOfRange { .. })));
        assert!(matches!(p.module(1), Err(Error::OutOfRange { .. })));
    }

    #[test]
    fn a_wrong_signature_is_refused() {
        let mut f = library();
        f[0] = b'X';
        assert!(matches!(P2l::parse(f), Err(Error::Corrupt { .. })));
    }
}
