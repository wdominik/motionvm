//! `GFX.INF`: how big every sprite is, without opening the container.
//!
//! ```text
//! u16 width, u16 height   per GFX slot, in slot order
//! 0xFFFF, 0xFFFF          the slot is empty
//! ```
//!
//! One entry per slot of the GFX segment and no header, so the file's length
//! is the slot count times four: 4800 bytes for the 1200 slots both games
//! that ship it declare. Only the earlier framing's games do — Victor Loomes
//! (1993-05-21) and Compaq (1994-03-03) — and the reason is their packing.
//! Their sprites are stored packed, so a player that wants to lay out a
//! screen before drawing it cannot read a width out of the container without
//! unpacking the item first; the later games store sprites plainly, where the
//! width is the item's first word, and ship no such file. All four later
//! binaries still name `gfx.inf` and none of their games carries one.
//!
//! motionvm does not read this at run time: [`crate::m16::Container`] unpacks
//! every item as it opens, so the sizes are in the sprites themselves by the
//! time anything asks. It is read to be checked against them — over Victor
//! Loomes' 721 occupied slots the two agree exactly, entry for entry, and
//! every unpacked length is `width * height + 6`.

use crate::error::Result;
use crate::u16le;

/// The marker an empty slot carries in both halves of its entry.
const ABSENT: u16 = 0xffff;

/// Bytes per entry: two `u16`.
pub const ENTRY_LEN: usize = 4;

/// The sizes of one game's sprites, in slot order.
#[derive(Debug, Clone)]
pub struct GfxInf {
    entries: Vec<Option<(u16, u16)>>,
}

impl GfxInf {
    /// Reads the file. A trailing partial entry is ignored rather than
    /// refused: the length is the only thing that says how many slots there
    /// are, and a file that is four bytes long past a whole number of entries
    /// still answers for every slot it does describe.
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        let mut entries = Vec::with_capacity(bytes.len() / ENTRY_LEN);
        for i in 0..bytes.len() / ENTRY_LEN {
            let w = u16le(bytes, i * ENTRY_LEN)?;
            let h = u16le(bytes, i * ENTRY_LEN + 2)?;
            entries.push((w != ABSENT || h != ABSENT).then_some((w, h)));
        }
        Ok(Self { entries })
    }

    /// Reads `GFX.INF` out of a game directory, if the game ships one.
    pub fn open_dir(dir: impl AsRef<std::path::Path>) -> Result<Option<Self>> {
        let Some(path) = crate::find_ci(dir.as_ref(), "GFX.INF") else {
            return Ok(None);
        };
        Ok(Some(Self::parse(&std::fs::read(path)?)?))
    }

    /// How many slots the file describes.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether it describes none.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The size of one slot's sprite, or `None` for a slot marked empty or
    /// past the end of the file.
    pub fn size(&self, slot: usize) -> Option<(u16, u16)> {
        self.entries.get(slot).copied().flatten()
    }

    /// The slots the file marks as filled.
    pub fn present(&self) -> Vec<usize> {
        (0..self.entries.len())
            .filter(|&i| self.entries[i].is_some())
            .collect()
    }
}
