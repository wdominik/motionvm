//! The four data tables `MUSADL.DRV` carries, read out of the shipped file.
//!
//! Read rather than embedded, the way the 32-bit game's FM driver reads
//! `HMIMDRV.386`: the file is part of every install, and the bytes stay the
//! original's.
//!
//! File offsets in the comments are `MUSADL.DRV`'s unless marked otherwise.

use crate::error::{Error, Result};

/// The PC timer's rate in Hz; the driver counts its tick periods in PIT
/// cycles (`0x4a9` = 1193 of them to a millisecond in the fade arithmetic).
pub const PIT_HZ: u32 = 1_193_182;

/// The four tables `MUSADL.DRV` carries, checked out of the file.
///
/// Read from the shipped driver rather than embedded, the way the 32-bit
/// game's FM driver reads `HMIMDRV.386`: the file is part of every install,
/// and the bytes stay the original's.
pub struct Driver {
    /// One default per OPL register (`0x5c4`); `0xFF` marks a register the
    /// init leaves untouched. The init writes every other one, in ascending
    /// register order, which is the first thing a capture of the original
    /// holds.
    pub defaults: [u8; 256],
    /// The modulator operator offsets per channel (`0x6c4`).
    pub mod_ops: [u8; 9],
    /// The carrier operator offsets per channel (`0x6cd`).
    pub car_ops: [u8; 9],
    /// The note table (`0x6d6`): 96 words of `block << 10 | fnum`, twelve
    /// F-numbers repeated over eight octaves.
    pub notes: [u16; 96],
}

impl Driver {
    /// Reads the tables out of `MUSADL.DRV`, verifying the header the way
    /// the manager in `ENVIRO.EXE` does (`198c:0078`): the `MUS\0` tag,
    /// version 1.00, the entry and import counts, and the `NS` trailer at the
    /// offset the header names.
    ///
    /// Two builds ship. The 1995/96 games all carry the same 4480-byte one
    /// with fifteen entries; Victor Loomes carries a 3915-byte build with
    /// fourteen, whose four tables sit `0xd0` earlier and whose contents are
    /// byte-for-byte the same — the defaults, both operator-offset tables and
    /// all 96 notes compare equal between the two files. So the build decides
    /// where to read, not what the tables mean.
    pub fn parse(file: &[u8]) -> Result<Self> {
        let entries = file
            .get(6..8)
            .map(|e| u16::from_le_bytes([e[0], e[1]]))
            .unwrap_or(0);
        // Where the defaults begin; everything else follows it.
        let base = match entries {
            15 => 0x5c4,
            14 => 0x4f4,
            _ => return Err(Error::Psm("not a MUS driver with 14 or 15 entries")),
        };
        let head_ok = file.len() >= base + 0x1d2
            && &file[..4] == b"MUS\0"
            && file[4..6] == [0x00, 0x01]
            && file[8..10] == [0x07, 0x00];
        if !head_ok {
            return Err(Error::Psm("not a MUS 1.00 driver with 7 imports"));
        }
        let trailer = u16::from_le_bytes([file[0x0a], file[0x0b]]) as usize;
        if file.get(trailer..trailer + 2) != Some(b"NS".as_ref()) {
            return Err(Error::Psm("the NS trailer is not where the header says"));
        }
        let mut defaults = [0u8; 256];
        defaults.copy_from_slice(&file[base..base + 0x100]);
        let mut mod_ops = [0u8; 9];
        mod_ops.copy_from_slice(&file[base + 0x100..base + 0x109]);
        let mut car_ops = [0u8; 9];
        car_ops.copy_from_slice(&file[base + 0x109..base + 0x112]);
        let mut notes = [0u16; 96];
        for (i, n) in notes.iter_mut().enumerate() {
            *n = u16::from_le_bytes([file[base + 0x112 + 2 * i], file[base + 0x113 + 2 * i]]);
        }
        Ok(Self {
            defaults,
            mod_ops,
            car_ops,
            notes,
        })
    }
}
