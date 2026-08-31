//! Ad Lib instrument banks — `MELODIC.BNK` and `DRUM.BNK`.
//!
//! These are the FM patches the game's music is actually played with. The
//! engine loads both by name during sound initialization and uploads them to
//! the MIDI driver (`0x850DB`/`0x8511C` read the files, `0x8F7C4` uploads
//! them), and it does so **only** for the FM device ids `0xA002` and `0xA009`.
//! The drivers never open them — the strings do not occur in any `HMI*.386`.
//!
//! The layout is the standard Ad Lib bank, and the arithmetic closes exactly on
//! both shipped files: `28 + 128 * 12 + 128 * 30 = 5404`, which is their size
//! to the byte.
//!
//! | Offset | Type | Meaning |
//! |---|---|---|
//! | `0x00` | `u8[2]` | Version, major then minor — `0.0` in both |
//! | `0x02` | `char[6]` | Signature `ADLIB-` |
//! | `0x08` | `u16` | Instruments used |
//! | `0x0a` | `u16` | Instruments named |
//! | `0x0c` | `u32` | Offset of the name table (28) |
//! | `0x10` | `u32` | Offset of the instrument records (1564) |
//! | `0x14` | `u8[8]` | Padding, zero |
//!
//! Both counts read 127 while there are **128** of each record — they are a
//! highest-index rather than a count, and the arithmetic above only closes at
//! 128. So neither is used here; the tables are sized from the two offsets.
//!
//! A name-table entry is twelve bytes: `u16 index`, `u8 key`, `char[9] name`.
//! In `MELODIC.BNK` the key is 1 throughout and the entries are the General
//! MIDI melodic set in program order (`PIANO1` … `GUNSHOT`). In `DRUM.BNK` it
//! is a **MIDI note number** instead — index 36 carries key 35 and the name
//! `Kick` — and only 34 of the 128 records are distinct; the rest are named
//! `Blank`.

use crate::{Error, Result, u16le, u32le};

/// One instrument, as the thirty bytes on disk.
///
/// The bytes are unpacked — one field per byte, not yet folded into OPL
/// registers — and they are kept that way here. Packing them is the driver's
/// job and depends on which chip it is talking to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instrument {
    /// Non-zero for a percussion patch.
    pub percussive: u8,
    /// Which of the percussion voices, when `percussive` is set.
    pub voice: u8,
    /// The modulator and the carrier, in that order.
    pub op: [Operator; 2],
    /// Waveform select, modulator then carrier.
    pub wave: [u8; 2],
}

/// The thirteen per-operator bytes, in the order they are stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Operator {
    /// KSL: how fast the level falls off as the note rises.
    pub key_scale_level: u8,
    /// MULT: the operator's frequency as a multiple of the note's.
    pub frequency_multiplier: u8,
    /// How much of the modulator's output is fed back into itself. Carried per
    /// operator as the bank stores it, though the chip has one per channel.
    pub feedback: u8,
    /// Attack rate of the envelope.
    pub attack: u8,
    /// Sustain level — how loud the note holds, not how long.
    pub sustain: u8,
    /// EG-type: whether the note holds at the sustain level or decays through.
    pub sustaining: u8,
    /// Decay rate, from the attack peak to the sustain level.
    pub decay: u8,
    /// Release rate, once the key is let go.
    pub release: u8,
    /// Total level, inverted: 0 is loudest.
    pub output_level: u8,
    /// Tremolo on or off.
    pub amplitude_vibrato: u8,
    /// Vibrato on or off.
    pub frequency_vibrato: u8,
    /// KSR: whether the envelope speeds up as the note rises.
    pub key_scale_rate: u8,
    /// Additive or FM connection between the pair. Like `feedback`, stored per
    /// operator although it belongs to the channel.
    pub connection: u8,
}

/// A whole bank: names and instruments, in index order.
#[derive(Debug, Clone)]
pub struct Bank {
    /// The name table: which instrument each named patch points at.
    pub names: Vec<Name>,
    /// The instrument records, decoded.
    pub instruments: Vec<Instrument>,
    /// The header's "instruments used" — 127 in both files, one short of the
    /// 128 that are there. Kept because the driver's conversion pass bounds
    /// itself by it and so converts fewer records than the bank holds; see
    /// `motionvm_motion_audio::opl`.
    pub used: u16,
    /// The instrument records as the thirty bytes on disk, index for index with
    /// `instruments`. The driver rewrites them in place into OPL register
    /// values, so a faithful model needs the bytes, not only the fields.
    pub raw: Vec<[u8; Self::INSTRUMENT_BYTES]>,
}

/// A name-table entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The instrument index this entry points at.
    pub index: u16,
    /// 1 for every melodic entry; a MIDI note number in the drum bank.
    pub key: u8,
    /// The patch's name, up to [`Bank::NAME_BYTES`] bytes, NUL-padded.
    pub name: String,
}

impl Bank {
    /// The six bytes at offset 2 that mark the file as an Ad Lib bank.
    pub const SIGNATURE: &'static [u8] = b"ADLIB-";
    /// Bytes per name-table entry: two of index, one of key, nine of text.
    pub const NAME_BYTES: usize = 12;
    /// Bytes per instrument record.
    pub const INSTRUMENT_BYTES: usize = 30;

    /// Reads a whole bank.
    ///
    /// Both shipped files close exactly on the arithmetic in the module
    /// header, so anything that does not is rejected rather than guessed at.
    pub fn parse(data: &[u8]) -> Result<Self> {
        let sig = data.get(2..8).ok_or(Error::Truncated {
            off: 2,
            need: 6,
            have: data.len(),
        })?;
        if sig != Self::SIGNATURE {
            let mut found = [0u8; 6];
            found.copy_from_slice(sig);
            return Err(Error::MissingBankSignature { found });
        }
        let used = u16le(data, 8)?;
        let _named = u16le(data, 10)?;
        let names_at = u32le(data, 12)? as usize;
        let data_at = u32le(data, 16)? as usize;
        if data_at < names_at || data_at > data.len() {
            return Err(Error::BankTablesOutOfRange {
                names: names_at,
                data: data_at,
                have: data.len(),
            });
        }

        // Sized from the offsets rather than from the counts, because the
        // counts are one short: both files say 127 and hold 128.
        let count = (data_at - names_at) / Self::NAME_BYTES;
        let mut names = Vec::with_capacity(count);
        for i in 0..count {
            let o = names_at + i * Self::NAME_BYTES;
            let raw = data.get(o..o + Self::NAME_BYTES).ok_or(Error::Truncated {
                off: o,
                need: Self::NAME_BYTES,
                have: data.len(),
            })?;
            let text = &raw[3..];
            let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
            names.push(Name {
                index: u16::from_le_bytes([raw[0], raw[1]]),
                key: raw[2],
                name: String::from_utf8_lossy(&text[..end]).into_owned(),
            });
        }

        let records = (data.len() - data_at) / Self::INSTRUMENT_BYTES;
        let mut instruments = Vec::with_capacity(records);
        let mut raw = Vec::with_capacity(records);
        for i in 0..records {
            let o = data_at + i * Self::INSTRUMENT_BYTES;
            let r = data
                .get(o..o + Self::INSTRUMENT_BYTES)
                .ok_or(Error::Truncated {
                    off: o,
                    need: Self::INSTRUMENT_BYTES,
                    have: data.len(),
                })?;
            instruments.push(Instrument {
                percussive: r[0],
                voice: r[1],
                op: [operator(&r[2..15]), operator(&r[15..28])],
                wave: [r[28], r[29]],
            });
            let mut bytes = [0u8; Self::INSTRUMENT_BYTES];
            bytes.copy_from_slice(r);
            raw.push(bytes);
        }

        Ok(Self {
            names,
            instruments,
            used,
            raw,
        })
    }

    /// The instrument a name-table entry points at.
    pub fn instrument(&self, slot: usize) -> Option<&Instrument> {
        let index = self.names.get(slot)?.index as usize;
        self.instruments.get(index)
    }
}

fn operator(b: &[u8]) -> Operator {
    Operator {
        key_scale_level: b[0],
        frequency_multiplier: b[1],
        feedback: b[2],
        attack: b[3],
        sustain: b[4],
        sustaining: b[5],
        decay: b[6],
        release: b[7],
        output_level: b[8],
        amplitude_vibrato: b[9],
        frequency_vibrato: b[10],
        key_scale_rate: b[11],
        connection: b[12],
    }
}
