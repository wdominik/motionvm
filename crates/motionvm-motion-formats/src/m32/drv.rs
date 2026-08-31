//! The `.386` driver archives — `HMIMDRV.386`, `HMIDRV.386`, `HMIDET.386`.
//!
//! Human Machine Interfaces shipped its device drivers as small 32-bit flat
//! images bundled into one file each: `HMIMDRV.386` holds the MIDI drivers,
//! `HMIDRV.386` the digital ones, `HMIDET.386` the detection stubs. Which image
//! is used is decided by the **device id** in `HMISET.CFG`, so that is the key
//! a reader needs.
//!
//! ## Layout
//!
//! A 44-byte file header, then one record per driver, each of them a 48-byte
//! header followed by the image itself:
//!
//! | Offset | Type | File header |
//! |---|---|---|
//! | `0x00` | `char[32]` | `3`, NUL-padded — the archive's own name |
//! | `0x20` | `u32` | Number of records |
//! | `0x24` | `u32` | Size of this header, 44 in all three files |
//! | `0x28` | `u32` | See below |
//!
//! | Offset | Type | Record header |
//! |---|---|---|
//! | `+0x00` | `char[32]` | File name, e.g. `fmmidi3.com` |
//! | `+0x20` | `u32` | Bytes to allocate for the loaded image |
//! | `+0x24` | `u32` | Bytes of image that follow this header |
//! | `+0x28` | `u32` | **Device id** — what `HMISET.CFG` names |
//! | `+0x2c` | `u32` | Flags: `0x4000`/`0x8000` mark the two halves of a
//!                     driver pair, `0xc000` both. Zero for every MIDI driver |
//!
//! The next record starts at `here + 0x30 + image`, and the chain closes
//! **exactly** on the file size in all three archives — there is no directory
//! and no index, so the only way to reach the last driver is to walk.
//!
//! `mem` is larger than `image` by the driver's uninitialized data: 92 bytes in
//! most of them, 1732 in `fmmidi3.com`.
//!
//! The `u32` at `0x28` of the file header is the archive's own size **truncated
//! to sixteen bits and sign-extended**: `0xFFFFCC5C` for the 117 852 bytes of
//! `HMIMDRV.386` (`0x1CC5C`), `0x473E` for the 83 774 of `HMIDET.386`
//! (`0x1473E`), `0xFFFFD785` for the 317 317 of `HMIDRV.386` (`0x4D785`). All
//! three files are larger than 64 KB, so the value is useless as a length; it
//! is read here only to be checked against, never to size anything.
//!
//! ## What is in `HMIMDRV.386`
//!
//! Eight MIDI drivers. Only two of them can make a sound from what the game
//! ships, because only `0xA002` and `0xA009` cause `ENGINE.EXE` to upload
//! `MELODIC.BNK` and `DRUM.BNK`:
//!
//! | Device | Name | |
//! |---|---|---|
//! | `0xA000` | `smii.com` | Sound Master II |
//! | `0xA001` | `mpu401.com` | MPU-401 |
//! | `0xA002` | `fmmidi.com` | **OPL2** — a Sound Blaster or SB Pro |
//! | `0xA004` | `mt32.com` | Roland MT-32 |
//! | `0xA006` | `intspkr.com` | PC speaker |
//! | `0xA008` | `awemidi.com` | AWE32 |
//! | `0xA009` | `fmmidi3.com` | **OPL3** — what the game asks for as
//!              "Sound Blaster 16", and the only driver this engine rebuilds |
//! | `0xA00A` | `gusmidi.com` | Gravis Ultrasound |

use crate::{Error, Result, reserve, u32le};

/// One driver in an archive: its header fields and its image.
#[derive(Debug, Clone)]
pub struct Driver {
    /// The driver's name, as the record carries it.
    pub name: String,
    /// Bytes the loader reserves — image plus uninitialized data.
    pub mem: u32,
    /// The device id `HMISET.CFG` selects the driver by.
    pub device: u32,
    /// The record's flag word. What its bits select is not established.
    pub flags: u32,
    /// Offset of the image within the archive file.
    pub at: usize,
    /// The driver image itself, `mem` bytes of which the loader reserves.
    pub image: Vec<u8>,
}

/// A whole `.386` archive.
#[derive(Debug, Clone)]
pub struct DriverArchive {
    /// The archive's own name, from its 44-byte header.
    pub name: String,
    /// The drivers it holds, in chain order.
    pub drivers: Vec<Driver>,
}

impl DriverArchive {
    /// Bytes of archive header before the first record.
    pub const HEADER_BYTES: usize = 44;
    /// Bytes per driver record.
    pub const RECORD_BYTES: usize = 48;

    /// Reads a `.386` driver archive, walking its record chain.
    ///
    /// The chain must land exactly on the end of the file; anything else
    /// means the archive is not what it says it is.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let count = u32le(data, 0x20)? as usize;
        let header = u32le(data, 0x24)? as usize;
        if header != Self::HEADER_BYTES {
            return Err(Error::DriverArchiveHeader { size: header });
        }
        let name = name_at(data, 0);

        let mut drivers = reserve(count, data.len(), Self::RECORD_BYTES);
        let mut at = header;
        for _ in 0..count {
            let mem = u32le(data, at + 0x20)?;
            let size = u32le(data, at + 0x24)? as usize;
            let device = u32le(data, at + 0x28)?;
            let flags = u32le(data, at + 0x2c)?;
            let start = at + Self::RECORD_BYTES;
            let image = data.get(start..start + size).ok_or(Error::Truncated {
                off: start,
                need: size,
                have: data.len(),
            })?;
            drivers.push(Driver {
                name: name_at(data, at),
                mem,
                device,
                flags,
                at: start,
                image: image.to_vec(),
            });
            at = start + size;
        }
        // The chain has to land on the end of the file. Anything else means the
        // walk went wrong, and since there is no directory to fall back on,
        // stopping here is the only honest answer.
        if at != data.len() {
            return Err(Error::DriverArchiveChain {
                end: at,
                have: data.len(),
            });
        }
        Ok(Self { name, drivers })
    }

    /// The driver with this device id, e.g. `0xA009` for the OPL3 one.
    pub fn device(&self, id: u32) -> Option<&Driver> {
        self.drivers.iter().find(|d| d.device == id)
    }
}

fn name_at(data: &[u8], off: usize) -> String {
    let raw = data.get(off..off + 32).unwrap_or(&[]);
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).into_owned()
}
