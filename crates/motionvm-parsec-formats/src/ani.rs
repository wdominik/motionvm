//! `AN` animation blocks: what moves in a room besides the player.
//!
//! A live slot of `DSROOM.DAT` carries one as its resource 1,
//! the `ANI` of the six the program names (`NOD ANI OMP DLG MBX RDO` at
//! `DS:054Bh` of Dunkle Schatten's `XP01`), and `DIALOG.DAT` carries one
//! more as its resource 2, the block the player's own figure animates from
//! (`0x0525F` loads it into `[1ED4h]`). The room loader relocates a block
//! with `0563h:0147h` (`0x7B4E`), the entry pass `14DDh:0293h` binds its
//! entries to the room's objects, and the animator in segment `0883h`
//! plays them. The block is read here; what the bytes *do* is the
//! animator's business and documented with it.
//!
//! **Shape.** Two magic bytes `AN`, a word, and from `+4` a directory of
//! `u32` offsets ended by a zero — the same walk `0563h:0147h` makes,
//! turning each into a far pointer. Each offset names an **entry**:
//!
//! ```text
//! +0    u16 chain      # offset of a chain of part nodes, 0 for none
//! +2    i16 id         # the object animated; negative is −(figure id)
//! +4    u8  count      # how many animations the entry holds
//! +5..+73h             # the animator's own working state, zero on disk but +0Ch..+0Fh
//! +0Ch  u8  lo, +0Dh u8 hi, +0Eh u8 delay   # the random and default waits
//! +74h  "ANIS"         # checked by 14DDh:0293h (0x1509D) and 0883h:1F82h (0x0A81D)
//! +78h  u16 off[count] # each animation's byte stream, offset from the block's start
//! ```
//!
//! The working state is what makes an entry `78h` bytes long: the
//! animator keeps its cursor, its stack of frames and its loop counters
//! inside the entry it is playing (`0x08E1E` writes the cursor to `+68h`,
//! `0x0A253` a frame to `+18h`), so a block is a program and its variables
//! in one. On disk every byte of it from `+5` to `+73h` is zero in every
//! shipped block but `+0Ch`..`+0Fh`: the three waits above, and `+0Fh`,
//! which this reader does not read.
//!
//! **Chains.** An entry whose `+0` is not zero animates a composite: a
//! head object and parts placed relative to it. A node is
//!
//! ```text
//! +0    u16 next       # the next node, 0 at the end
//! +2    u16 kind       # low byte 0: the head; else the placement of a part
//!                      # high byte 0: one cel per frame at +0Eh; 100h: eight bytes per frame at +0Ah
//! +4    u16 id         # the object
//! +6    far object     # filled at run time
//! +0Ah  i16 dx, +0Ch i16 dy   # a part's offset (high byte 0)
//! +0Eh  u8 cel[frame]  # a part's cel per frame (high byte 0)
//! +0Ah  {kind, cel, x(i16), y(i16), mirror, ?, ?}[frame]   # (high byte 1)
//! ```
//!
//! `0883h:1A82h` reads them that way (`0x0A3F1` the per-frame cel,
//! `0x0A404`–`0x0A43B` the eight-byte form). Dunkle Schatten's Küche is
//! the case that shows it: the mother is object 200, a head with twelve
//! cels, and object 201 below it, her body with one.
//!
//! **Corpus.** Every live slot of `DSROOM.DAT` that carries a resource 1
//! parses through this, and so does `DIALOG.DAT` resource 2;
//! `tests/ds1_files.rs` counts them.

use crate::error::{Error, Result};
use crate::{Cursor, Record, bytes, records, sign16, u16le};

/// The two bytes a block opens with.
pub const MAGIC: &[u8; 2] = b"AN";
/// The four bytes at `+74h` of every entry.
pub const ENTRY_MAGIC: &[u8; 4] = b"ANIS";
/// Where an entry's animation offsets start: past the header and the magic.
pub const ENTRY_OFFSETS: usize = 0x78;

/// One block, kept as the bytes the animator walks plus the directory
/// the relocation pass builds from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Animations {
    bytes: Vec<u8>,
    entries: Vec<usize>,
}

/// The fixed part of an entry, as the reader checks it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// Where the entry sits in the block.
    pub at: usize,
    /// `+0`: the chain of part nodes, or 0.
    pub chain: u16,
    /// `+2`: the object's id; below zero the negated id of a figure
    /// (`14DDh:0098h` compares `−[+13h]` at `0x14E9D`).
    pub id: i16,
    /// `+4`: how many animations the entry holds.
    pub count: u8,
    /// `+0Ch`, `+0Dh`: the bounds of a random wait.
    pub delay_lo: u8,
    /// See [`Self::delay_lo`].
    pub delay_hi: u8,
    /// `+0Eh`: the wait a frame takes when its opcode names none.
    pub delay: u8,
}

/// One node of a chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainNode {
    /// Where the node sits in the block.
    pub at: usize,
    /// `+0`: the next node, or 0.
    pub next: u16,
    /// `+2`: the head when its low byte is zero, else a part's placement;
    /// the high byte says which per-frame form follows.
    pub kind: u16,
    /// `+4`: the object's id.
    pub id: u16,
    /// `+0Ah`: a part's x offset from the head, in the one-cel-per-frame form.
    pub dx: i16,
    /// `+0Ch`: the same for y.
    pub dy: i16,
}

impl Animations {
    /// Whether `data` opens like a block.
    pub fn looks_like(data: &[u8]) -> bool {
        data.len() >= 8 && data.starts_with(MAGIC)
    }

    /// Parses a block: the directory, and every entry it names checked
    /// for its magic and its offsets.
    pub fn parse(data: &[u8]) -> Result<Self> {
        if !Self::looks_like(data) {
            return Err(Error::Corrupt {
                what: "AN animations",
                detail: format!("no AN magic in {} bytes", data.len()),
            });
        }
        let mut entries = Vec::new();
        let mut directory = Cursor::new(data, 4);
        loop {
            let off = directory.u32at()?;
            if off == 0 {
                break;
            }
            check_entry(data, off)?;
            entries.push(off);
            // A directory longer than the block could hold is a loop
            // through garbage, not a block.
            if entries.len().saturating_mul(4).saturating_add(4) > data.len() {
                return Err(Error::Corrupt {
                    what: "AN animations",
                    detail: "directory does not end".into(),
                });
            }
        }
        Ok(Animations {
            bytes: data.to_vec(),
            entries,
        })
    }

    /// The whole block, for the animator to walk.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// How many entries the directory names.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the directory names none.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Entry `i` of the directory.
    pub fn entry(&self, i: usize) -> Option<Entry> {
        let at = *self.entries.get(i)?;
        let e = Record(bytes::<0x0F>(&self.bytes, at).ok()?);
        Some(Entry {
            at,
            chain: e.u16::<0>(),
            id: e.i16::<2>(),
            count: e.u8::<4>(),
            delay_lo: e.u8::<0x0C>(),
            delay_hi: e.u8::<0x0D>(),
            delay: e.u8::<0x0E>(),
        })
    }

    /// Every entry, in directory order.
    pub fn entries(&self) -> impl Iterator<Item = Entry> + '_ {
        (0..self.entries.len()).filter_map(|i| self.entry(i))
    }

    /// Where animation `k` of entry `i` starts, or `None` when the entry
    /// holds fewer or the slot is zero — which is how `0883h:0582h` refuses
    /// one (`0x08DF4`, `or ax, ax`).
    pub fn animation(&self, i: usize, k: usize) -> Option<usize> {
        let e = self.entry(i)?;
        let streams = records::<2>(
            &self.bytes,
            e.at.checked_add(ENTRY_OFFSETS)?,
            usize::from(e.count),
        )
        .ok()?;
        let off = usize::from(u16::from_le_bytes(*streams.get(k)?));
        (off != 0).then_some(off)
    }

    /// The byte at `off`, when the block has one.
    pub fn byte(&self, off: usize) -> Option<u8> {
        self.bytes.get(off).copied()
    }

    /// The little-endian word at `off`.
    pub fn word(&self, off: usize) -> Option<u16> {
        u16le(&self.bytes, off).ok()
    }

    /// The same, signed.
    pub fn int(&self, off: usize) -> Option<i16> {
        self.word(off).map(sign16)
    }

    /// The chain node at `at`, or `None` when the block ends before its
    /// fixed part.
    pub fn node(&self, at: usize) -> Option<ChainNode> {
        if at == 0 {
            return None;
        }
        let n = Record(bytes::<0x0E>(&self.bytes, at).ok()?);
        Some(ChainNode {
            at,
            next: n.u16::<0>(),
            kind: n.u16::<2>(),
            id: n.u16::<4>(),
            dx: n.i16::<0x0A>(),
            dy: n.i16::<0x0C>(),
        })
    }

    /// A part's cel for `frame` in the one-cel-per-frame form, `+0Eh + frame`.
    pub fn node_cel(&self, node: &ChainNode, frame: u8) -> Option<u8> {
        let at = node.at.checked_add(0x0E)?.checked_add(usize::from(frame))?;
        self.byte(at)
    }

    /// A part's eight bytes for `frame` in the eight-byte form,
    /// `+0Ah + 8 × frame`: kind, cel, x, y, mirror, and three the
    /// animator does not read.
    pub fn node_frame(&self, node: &ChainNode, frame: u8) -> Option<[u8; 8]> {
        let frames = records::<8>(
            &self.bytes,
            node.at.checked_add(0x0A)?,
            usize::from(frame).checked_add(1)?,
        )
        .ok()?;
        frames.last().copied()
    }
}

/// Checks the entry a directory names at `off`: its fixed part inside the
/// block, its magic, and every animation offset it holds inside the block.
fn check_entry(data: &[u8], off: usize) -> Result<()> {
    let Ok(head) = bytes::<ENTRY_OFFSETS>(data, off) else {
        return Err(Error::Corrupt {
            what: "AN animations",
            detail: format!("entry at {off:#x} runs past {} bytes", data.len()),
        });
    };
    let head = Record(head);
    if head.bytes::<0x74, 4>() != *ENTRY_MAGIC {
        return Err(Error::Corrupt {
            what: "AN animations",
            detail: format!("entry at {off:#x} has no ANIS magic"),
        });
    }
    let count = usize::from(head.u8::<4>());
    let streams = records::<2>(data, off.saturating_add(ENTRY_OFFSETS), count)?;
    for (k, stream) in streams.iter().enumerate() {
        let stream = usize::from(u16::from_le_bytes(*stream));
        if stream >= data.len() {
            return Err(Error::Corrupt {
                what: "AN animations",
                detail: format!(
                    "entry at {off:#x} animation {k} starts at {stream:#x}, past {} bytes",
                    data.len()
                ),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A block with one entry, one animation, and a two-node chain.
    fn block() -> Vec<u8> {
        let mut b = vec![0u8; 0x100];
        b[..2].copy_from_slice(MAGIC);
        b[4..8].copy_from_slice(&0x10u32.to_le_bytes());
        // The entry at 0x10: chain at 0x90, id 200, one animation.
        b[0x10..0x12].copy_from_slice(&0x90u16.to_le_bytes());
        b[0x12..0x14].copy_from_slice(&200u16.to_le_bytes());
        b[0x14] = 1;
        b[0x1C] = 10;
        b[0x1D] = 20;
        b[0x1E] = 15;
        b[0x84..0x88].copy_from_slice(ENTRY_MAGIC);
        b[0x88..0x8A].copy_from_slice(&0xA0u16.to_le_bytes());
        // The chain: a head at 0x90 and a part at 0xB0.
        b[0x90..0x92].copy_from_slice(&0xB0u16.to_le_bytes());
        b[0x94..0x96].copy_from_slice(&200u16.to_le_bytes());
        b[0x9E] = 3;
        b[0xB2..0xB4].copy_from_slice(&1u16.to_le_bytes());
        b[0xB4..0xB6].copy_from_slice(&201u16.to_le_bytes());
        b[0xBA..0xBC].copy_from_slice(&(-4i16).to_le_bytes());
        // The animation: end at once.
        b[0xA0] = 0;
        b
    }

    #[test]
    fn a_block_lists_its_entries_and_their_animations() {
        let a = Animations::parse(&block()).expect("block");
        assert_eq!(a.len(), 1);
        let e = a.entry(0).expect("entry");
        assert_eq!((e.at, e.chain, e.id, e.count), (0x10, 0x90, 200, 1));
        assert_eq!((e.delay_lo, e.delay_hi, e.delay), (10, 20, 15));
        assert_eq!(a.animation(0, 0), Some(0xA0));
        assert_eq!(a.animation(0, 1), None, "past the count");
        let head = a.node(0x90).expect("head");
        assert_eq!((head.next, head.kind, head.id), (0xB0, 0, 200));
        assert_eq!(a.node_cel(&head, 0), Some(3));
        let part = a.node(0xB0).expect("part");
        assert_eq!((part.next, part.kind, part.id, part.dx), (0, 1, 201, -4));
        assert_eq!(a.node(0), None, "zero is the end of a chain");
    }

    #[test]
    fn a_missing_magic_is_refused() {
        let mut b = block();
        b[0x85] = b'X';
        assert!(Animations::parse(&b).is_err());
        b = block();
        b[0] = b'X';
        assert!(Animations::parse(&b).is_err());
        assert!(!Animations::looks_like(&b));
    }

    #[test]
    fn an_animation_past_the_block_is_refused() {
        let mut b = block();
        b[0x88..0x8A].copy_from_slice(&0x1000u16.to_le_bytes());
        assert!(Animations::parse(&b).is_err());
    }
}
