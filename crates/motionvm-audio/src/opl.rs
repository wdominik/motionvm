//! The FM driver, rebuilt from `fmmidi3.com`.
//!
//! `HMIMDRV.386` holds eight MIDI drivers (see [`motionvm_formats::m32::drv`]); the one
//! the game asks for as "Sound Blaster 16" is device `0xA009`, `fmmidi3.com`, a
//! 32-bit flat image of 14 416 bytes. This module is that driver: it takes the
//! MIDI messages the [sequencer](crate::sequencer) produces and answers with
//! the byte-for-byte register writes the original sends to the OPL3. **No
//! sound is computed here** — that is the next layer's job, and keeping the two
//! apart is what makes this one checkable against a recording of the original.
//!
//! Addresses below are load addresses in the driver image, i.e. file offset
//! minus `0x3218`.
//!
//! ## What makes this driver unusual
//!
//! **Every voice sounds on both register banks at once.** `program_voice`
//! (`0x2162`) writes each register to port `0x388` *and* `0x38A`, and gives the
//! `0xC0` byte `|0x20` on the first bank and `|0x10` on the second. So the nine
//! channels of bank 0 and the nine of bank 1 are not eighteen voices — they are
//! nine voices, each of them wired to one output side. Panning is done by
//! attenuating the copy on one side (`0x276D`), never by the `0xC0` bits.
//!
//! **The chip's rhythm mode is not used.** `0xBD` only ever gets `0xC0` (deep
//! vibrato and deep tremolo) or `0x00`. Percussion is nine ordinary voices
//! playing patches out of `DRUM.BNK`.
//!
//! **Only nine voices exist.** `alloc_voice` (`0x2033`) searches `0..9`, and
//! the four-operator mode the OPL3 offers is never switched on — `0x104` stays
//! zero, which is why a recording of the original identifies as "dual OPL2".
//!
//! ## The three faults that are reproduced here
//!
//! They are the original's, they are audible, and they are left in on purpose;
//! each one is marked at the place it happens.
//!
//! 1. **The conversion pass stops two records early** (`0x0D17`): it runs while
//!    `i < used - 2`, and `used` is the header's 127 while the bank holds 128.
//!    Instruments 125, 126 and 127 are therefore played from *unconverted*
//!    bytes. In `MELODIC.BNK` those are `HELICOPT`, `APPLAUSE` and `GUNSHOT` —
//!    the last three of the General MIDI set, and no song in the game uses
//!    them.
//! 2. **A downward pitch bend indexes the bend range by the OPL voice**
//!    (`0x1BCD`) where every other site indexes it by the MIDI channel.
//! 3. **The percussion path tests the melodic patch's connection bit**
//!    (`0x1487`): it reads `program[9]` out of `MELODIC.BNK` to decide whether
//!    the drum patch's modulator gets the velocity, instead of looking at the
//!    drum record it is actually playing.
//!
//! ## What is still open
//!
//! Which side of the stereo image each bank is. The chip's own answer is that
//! `0xC0` bit 4 is channel A and bit 5 channel B, and in the usual wiring A is
//! left and B is right — which would make bank 0 (`|0x20`) the right side. The
//! driver attenuates bank 0 when controller 10 is **at or above** 64, i.e. it
//! quietens the right side as the pan moves right. Read literally that is
//! backwards, and no recording can settle it, because a recording only shows
//! which register was written. It is implemented exactly as read.

use crate::error::{Error, Result};
use crate::sequencer::{Kind, Message};
use motionvm_formats::m32::bnk::Bank;
use motionvm_formats::m32::drv::Driver;

/// One write to the chip.
///
/// `bank` is 0 for the base port and 1 for base + 2; on an OPL3 that is the
/// same as the register's ninth bit, so `reg as u16 | (bank << 8)` is the
/// address a DRO recording stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Write {
    /// 0 for the base port, 1 for base + 2 — the register's ninth address bit.
    pub bank: u8,
    /// The register within the bank.
    pub reg: u8,
    /// The byte written to it.
    pub value: u8,
}

impl Write {
    /// The 9-bit register address, the form a DRO capture uses.
    pub fn address(self) -> u16 {
        self.reg as u16 | ((self.bank as u16) << 8)
    }
}

/// The four tables the driver keeps in its own image.
///
/// They are read out of the driver rather than written down here, because two
/// of them are hand-made and not quite what the arithmetic would give — see
/// [`Tables::OCTAVE_DOWN_AT`].
#[derive(Debug, Clone)]
pub struct Tables {
    /// `0x3614`, 103 entries: `(block << 10) | fnum` for MIDI notes 12…114,
    /// which is every note the chip can express. Indexed `note - 12`; the
    /// compiler folded that offset into the address, which is why the image
    /// also refers to it as `0x35E4`.
    pub frequency: Vec<u32>,
    /// `0x35B9`, 18 entries: the register offset of each operator, modulator
    /// then carrier, for the nine channels.
    pub operators: [u8; 18],
    /// `0x35D0`, 64 entries indexed by `velocity >> 1`: an attenuation in the
    /// chip's 0…63 units. Falls from 63 at silence to 0 at full.
    pub velocity: [u8; 64],
    /// `0x37B0`, 12 entries: the same note one block higher, i.e. the f-number
    /// halved, indexed **backwards** by `11 - (note - 12) % 12`. Used when a
    /// pitch bend crosses a block boundary.
    ///
    /// Its seventh entry is 248 where halving gives 243 — a slip in HMI's own
    /// table. That is the reason this is read from the file instead of being
    /// recomputed: a "corrected" table would be a different driver.
    pub octave_down: [u32; 12],
}

impl Tables {
    /// Offset of the note-to-f-number table in the driver image.
    pub const FREQUENCY_AT: usize = 0x3614;
    /// Offset of the operator-pair table: modulator then carrier, nine channels.
    pub const OPERATORS_AT: usize = 0x35B9;
    /// Offset of the velocity curve, 64 entries indexed by `velocity >> 1`.
    pub const VELOCITY_AT: usize = 0x35D0;
    /// Offset of the halved-octave table, indexed backwards. Carries the
    /// driver's own rounding slip at entry 6, which is reproduced rather than
    /// corrected.
    pub const OCTAVE_DOWN_AT: usize = 0x37B0;
    /// Notes 12…114 — 103 entries, ending where an f-number would overflow.
    pub const NOTES: usize = 103;

    /// Reads the tables out of a driver image, e.g. `HMIMDRV.386`'s `0xA009`.
    ///
    /// Each failure names the table it was reading. The addresses are fixed, so
    /// a miss means this is not the driver these constants belong to — and
    /// which of the four is missing is the difference between "wrong device out
    /// of the archive" and "the archive is truncated".
    pub fn read(driver: &Driver) -> Result<Self> {
        let img = &driver.image;
        let have = img.len();
        let slice = |name: &'static str, at: usize, need: usize| -> Result<&[u8]> {
            img.get(at..at + need).ok_or(Error::MissingTable {
                name,
                at,
                need,
                have,
            })
        };
        let u32s = |name: &'static str, at: usize, n: usize| -> Result<Vec<u32>> {
            let b = slice(name, at, n * 4)?;
            Ok(b.as_chunks::<4>()
                .0
                .iter()
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect())
        };

        let frequency = u32s("frequency", Self::FREQUENCY_AT, Self::NOTES)?;
        let mut operators = [0u8; 18];
        operators.copy_from_slice(slice("operators", Self::OPERATORS_AT, 18)?);
        let mut velocity = [0u8; 64];
        velocity.copy_from_slice(slice("velocity", Self::VELOCITY_AT, 64)?);
        let mut octave_down = [0u32; 12];
        octave_down.copy_from_slice(&u32s("octave_down", Self::OCTAVE_DOWN_AT, 12)?);
        Ok(Self {
            frequency,
            operators,
            velocity,
            octave_down,
        })
    }

    /// The modulator's register offset for an OPL channel.
    fn modulator(&self, voice: usize) -> usize {
        self.operators[voice * 2] as usize
    }

    /// The carrier's register offset for an OPL channel.
    fn carrier(&self, voice: usize) -> usize {
        self.operators[voice * 2 + 1] as usize
    }
}

/// An instrument after the driver's conversion pass (`0x0CD6`).
///
/// The pass rewrites the thirty bytes **in place**, so the values sit at the
/// offsets of whichever field they were built from, and byte 2 of the bank
/// header is stamped `'H'` so it cannot run twice. The offsets are kept here
/// because reading them is what the note-on path does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Patch {
    /// `0x20 + op`, modulator then carrier: tremolo, vibrato, sustaining
    /// envelope, key-scale rate, frequency multiplier.
    pub am_vib: [u8; 2],
    /// `0x40 + op`: key-scale level and total level. The **carrier's is never
    /// written to the chip by the patch loader** — only into the shadow, from
    /// where the velocity calculation picks it up a moment later.
    pub level: [u8; 2],
    /// `0x60 + op`: attack and decay.
    pub attack_decay: [u8; 2],
    /// `0x80 + op`: sustain and release.
    pub sustain_release: [u8; 2],
    /// `0xE0 + op`: waveform, taken over unchanged.
    pub wave: [u8; 2],
    /// `0xC0 + channel`: feedback and connection, from the **modulator's**
    /// fields only. The carrier's two are discarded.
    pub feedback_connection: u8,
}

impl Patch {
    /// True when the two operators are added rather than chained. The velocity
    /// then reaches the modulator as well.
    pub fn additive(self) -> bool {
        self.feedback_connection & 1 != 0
    }

    /// Folds one thirty-byte record the way `0x0CD6` does.
    fn convert(r: &[u8; 30]) -> Self {
        Self {
            // (am << 7) | (vib << 6) | (sustaining << 5) | (ksr << 4) | mult
            am_vib: [
                (r[11] << 7) | (r[12] << 6) | (r[7] << 5) | (r[13] << 4) | r[3],
                (r[24] << 7) | (r[25] << 6) | (r[20] << 5) | (r[26] << 4) | r[16],
            ],
            level: [(r[2] << 6) | r[10], (r[15] << 6) | r[23]],
            attack_decay: [(r[5] << 4) | r[8], (r[18] << 4) | r[21]],
            sustain_release: [(r[6] << 4) | r[9], (r[19] << 4) | r[22]],
            wave: [r[28], r[29]],
            feedback_connection: (r[4] << 1) | r[14],
        }
    }

    /// The same offsets read out of a record the conversion never reached.
    ///
    /// This is what instruments 125…127 sound like: the driver reads the same
    /// eleven positions regardless, and finds the raw Ad Lib fields still in
    /// them. See fault 1 in the module documentation.
    fn unconverted(r: &[u8; 30]) -> Self {
        Self {
            am_vib: [r[11], r[24]],
            level: [r[2], r[15]],
            attack_decay: [r[5], r[18]],
            sustain_release: [r[6], r[19]],
            wave: [r[28], r[29]],
            feedback_connection: r[14],
        }
    }
}

/// Converts a whole bank, stopping where the driver stops.
///
/// `0x0D17` compares against `used - 2`, and `used` is the header's count,
/// which reads 127 in both shipped banks while 128 records are present. Three
/// records are therefore left raw.
pub fn convert_bank(bank: &Bank) -> Vec<Patch> {
    let converted = (bank.used as usize).saturating_sub(2);
    bank.raw
        .iter()
        .enumerate()
        .map(|(i, r)| {
            if i < converted {
                Patch::convert(r)
            } else {
                Patch::unconverted(r)
            }
        })
        .collect()
}

/// How many voices the driver has. Nine, on both banks at once.
pub const VOICES: usize = 9;
/// The highest operator offset in use, `0x15`, plus one.
const OPERATOR_SLOTS: usize = 0x16;

/// The driver's own memory, named after the addresses it lives at.
pub struct Fm {
    tables: Tables,
    melodic: Vec<Patch>,
    drums: Vec<Patch>,
    /// `DRUM.BNK`'s name table, byte `+2` of each entry: the note the record is
    /// *played* at, which is not the note that selected it.
    drum_pitch: Vec<u8>,

    /// `0x2C38` — program per MIDI channel.
    program: [u8; 16],
    /// `0x2D60` — controller 7.
    volume: [u8; 16],
    /// `0x2E68` — controller 10.
    pan: [u8; 16],
    /// `0x2C9C` — the pitch bend's high byte, 0x40 at rest.
    bend: [u8; 16],
    /// `0x2D1C` — controller 102, the bend range in semitones. Two at reset.
    bend_range: [u8; 16],
    /// `0x2CDC` — set once a channel has sent a bend. Voice stealing avoids
    /// channels that have, and note-on only recomputes a frequency for them.
    bent: [bool; 16],
    /// `0x2E04` — controller 64.
    sustain: [bool; 16],
    /// `0x2F28` — notes waiting for the sustain pedal, 32 to a channel.
    held: [Vec<u8>; 16],

    /// `0x2C78` — which MIDI channel each voice belongs to.
    owner: [u8; VOICES],
    /// `0x35A8` — the note each voice is playing, zero when idle.
    note: [u8; VOICES],
    /// `0x2DA0` — the velocity of that note, before the channel volume.
    velocity: [u8; VOICES],

    /// `0x3548` — the patch's `0x40` byte per operator offset. Thirty-two
    /// bytes, of which the twenty-two operator offsets use the front.
    shadow_40: [u8; OPERATOR_SLOTS],
    /// `0x3568` — the patch's `0x80` byte per operator offset, **and it is
    /// sixteen bytes long where twenty-two are needed**. See
    /// [`Fm::sustain_shadow`].
    shadow_80: [u8; 0x10],
    /// `0x3578` / `0x3581` — the last `0xA0` and `0xB0` per voice.
    shadow_a0: [u8; VOICES],
    shadow_b0: [u8; VOICES],

    out: Vec<Write>,
}

impl Fm {
    /// The device id of the OPL3 driver inside `HMIMDRV.386`.
    pub const DEVICE: u32 = 0xA009;
    /// And of the OPL2 one, for the record — a different driver, not built here.
    pub const DEVICE_OPL2: u32 = 0xA002;

    /// Builds the driver from its own image and the two banks the engine
    /// uploads into it (`0x850DB`, `0x8511C`, `0x8F7C4`).
    pub fn new(driver: &Driver, melodic: &Bank, drums: &Bank) -> Result<Self> {
        let tables = Tables::read(driver)?;
        let mut fm = Self {
            tables,
            melodic: convert_bank(melodic),
            drums: convert_bank(drums),
            drum_pitch: drums.names.iter().map(|n| n.key).collect(),
            program: [0; 16],
            volume: [0x7f; 16],
            pan: [0x40; 16],
            bend: [0x40; 16],
            bend_range: [2; 16],
            bent: [false; 16],
            sustain: [false; 16],
            held: Default::default(),
            owner: [0; VOICES],
            note: [0; VOICES],
            velocity: [0; VOICES],
            shadow_40: [0; OPERATOR_SLOTS],
            shadow_80: [0; 0x10],
            shadow_a0: [0; VOICES],
            shadow_b0: [0; VOICES],
            out: Vec::new(),
        };
        fm.reset();
        Ok(fm)
    }

    /// The writes made since the last call, and clears the list.
    pub fn take(&mut self) -> Vec<Write> {
        std::mem::take(&mut self.out)
    }

    /// The same, appended to a buffer the caller keeps.
    ///
    /// Both sides hold on to their allocation, which is what the audio thread
    /// needs: after the first few notes nothing here reaches the allocator.
    pub fn take_into(&mut self, out: &mut Vec<Write>) {
        out.append(&mut self.out);
    }

    /// The switch-on sequence: ordinal 4 (`0x1D1A`) and the silencing pass it
    /// calls (`0x1D8E`).
    ///
    /// `0x105 = 1` leaves OPL2 compatibility and `0x104 = 0` keeps every
    /// channel in two-operator mode — both on the **second** bank, which is
    /// where those two registers live. Then every `0xB0` is cleared on both
    /// banks so nothing is sounding, and `0xBD` gets `0xC0`: deep vibrato and
    /// deep tremolo, with the chip's rhythm bits left off for good. That last
    /// one goes to the **first bank only** (`0x1E4A`).
    pub fn reset(&mut self) {
        self.write(1, 0x05, 0x01);
        self.write(1, 0x04, 0x00);
        for v in 0..VOICES {
            self.shadow_b0[v] = 0;
            self.write(0, 0xb0 + v as u8, 0);
            self.write(1, 0xb0 + v as u8, 0);
        }
        self.write(0, 0xbd, 0xc0);
    }

    /// One MIDI message from the sequencer.
    pub fn send(&mut self, m: Message) {
        let ch = (m.channel & 0x0f) as usize;
        match m.kind {
            // A velocity of zero is not a note-off — the sequencer does not
            // send one and neither does the format.
            Kind::NoteOn { note, velocity } => self.note_on(ch, note, velocity),
            Kind::NoteOff { note } => self.note_off(ch, note),
            Kind::Control { controller, value } => self.control(ch, controller, value),
            Kind::Program(p) => self.program[ch] = p,
            Kind::PitchBend { msb, .. } => self.pitch_bend(ch, msb),
            // `0xA0`, `0xD0` and the `0xF0` class are read off the wire and
            // dropped (`0x1AAD`); the driver has no aftertouch.
            Kind::PolyPressure { .. } | Kind::ChannelPressure(_) => {}
        }
    }

    // ---------------------------------------------------------------- notes

    /// `0x0EF9` for the melodic channels, `0x12F4` for channel 9.
    fn note_on(&mut self, ch: usize, note: u8, velocity: u8) {
        let percussion = ch == 9;
        let voice = self.alloc(ch, note);
        self.stop(voice);
        self.owner[voice] = ch as u8;

        // Five times the same write: the original hammers the old voice's
        // release rate to the fastest setting so the envelope is down before
        // the new patch lands (`0x0F5F`, `0x1333`). Only the first of the five
        // changes anything.
        for _ in 0..5 {
            for op in [self.tables.carrier(voice), self.tables.modulator(voice)] {
                let v = self.sustain_shadow(op) | 0x0f;
                self.write(0, 0x80 + op as u8, v);
                self.write(1, 0x80 + op as u8, v);
            }
        }

        let patch = if percussion {
            self.drums.get(note as usize).copied().unwrap_or_default()
        } else {
            self.melodic
                .get(self.program[ch] as usize)
                .copied()
                .unwrap_or_default()
        };
        self.program_voice(voice, patch);

        self.velocity[voice] = velocity;
        // Controller 7 folded into the velocity, not sent to the chip as a
        // level of its own (`0x1081`). The `<< 7 / 0x7F` is the original's.
        let scaled = ((((self.volume[ch] as u32) << 7) / 0x7f) * velocity as u32) >> 7;
        let scaled = scaled as u8;

        // Fault 3: for percussion this reads the *melodic* patch of program
        // `program[9]`, not the drum record being played (`0x1487`).
        let connection_patch = if percussion {
            self.melodic
                .get(self.program[9] as usize)
                .copied()
                .unwrap_or_default()
        } else {
            patch
        };
        if connection_patch.additive() {
            let op = self.tables.modulator(voice);
            self.level(op, scaled, None);
        }
        let op = self.tables.carrier(voice);
        self.level(op, scaled, None);

        self.note[voice] = note;
        self.set_pan(ch, self.pan[ch]);

        let pitch = if percussion {
            self.drum_pitch.get(note as usize).copied().unwrap_or(note)
        } else {
            note
        };
        let packed = self.frequency(pitch);
        self.key_on(voice, packed);

        // Only a channel that has actually bent gets its frequency recomputed
        // (`0x12A7`); percussion never does.
        if !percussion && self.bent[ch] {
            let packed = self.bend_frequency(voice, note, self.bend[ch]);
            self.set_frequency(voice, packed, true);
        }
    }

    /// `0x1695`.
    fn note_off(&mut self, ch: usize, note: u8) {
        if self.sustain[ch] {
            // Thirty-two notes to a channel, and the thirty-third is dropped.
            if self.held[ch].len() < 32 {
                self.held[ch].push(note);
            }
            return;
        }
        for v in 0..VOICES {
            if self.note[v] == note && self.owner[v] as usize == ch {
                self.stop(v);
                self.note[v] = 0;
            }
        }
    }

    /// `0x2033`. A free voice, else one belonging to a channel that has never
    /// bent, else the channel number itself folded into range.
    fn alloc(&self, ch: usize, _note: u8) -> usize {
        if let Some(v) = (0..VOICES).find(|&v| self.note[v] == 0) {
            return v;
        }
        for c in 0..16 {
            if self.bent[c] {
                continue;
            }
            if let Some(v) = (0..VOICES).find(|&v| self.owner[v] as usize == c) {
                return v;
            }
        }
        if ch >= 9 { ch - 9 } else { ch }
    }

    /// `0x20D8`: clears the key-on bit and writes `0xB0`, nothing else. The
    /// obvious alternative — turning the level down — is not what happens, so
    /// a released note keeps its patch and its frequency.
    fn stop(&mut self, voice: usize) {
        if self.note[voice] == 0 {
            return;
        }
        self.shadow_b0[voice] &= 0xdf;
        let v = self.shadow_b0[voice];
        self.write(0, 0xb0 + voice as u8, v);
        self.write(1, 0xb0 + voice as u8, v);
    }

    // ------------------------------------------------------------ registers

    /// `0x2162`. Eleven registers, both banks, in this order.
    ///
    /// The **carrier's `0x40` is deliberately missing**: `0x23F7` stores it
    /// into the shadow and stops there, because the velocity calculation is
    /// about to write that register anyway. The modulator's is written
    /// (`0x2271`) and shadowed — for a chained patch nothing overwrites it, so
    /// the modulator keeps the level the instrument asks for.
    fn program_voice(&mut self, voice: usize, patch: Patch) {
        let (m, c) = (self.tables.modulator(voice), self.tables.carrier(voice));
        self.set_sustain_shadow(c, patch.sustain_release[1]);
        self.set_sustain_shadow(m, patch.sustain_release[0]);
        self.shadow_40[m] = patch.level[0];
        self.shadow_40[c] = patch.level[1];

        for (reg, value) in [
            (0x20 + m as u8, patch.am_vib[0]),
            (0x40 + m as u8, patch.level[0]),
            (0x60 + m as u8, patch.attack_decay[0]),
            (0x80 + m as u8, patch.sustain_release[0]),
            (0xc0 + voice as u8, patch.feedback_connection),
            (0xe0 + m as u8, patch.wave[0]),
            (0x20 + c as u8, patch.am_vib[1]),
            (0x60 + c as u8, patch.attack_decay[1]),
            (0x80 + c as u8, patch.sustain_release[1]),
            (0xe0 + c as u8, patch.wave[1]),
        ] {
            // The two output bits are the whole of this driver's stereo: bank 0
            // gets `0x20`, bank 1 gets `0x10`, so the same voice reaches both
            // sides as two independent copies.
            if reg & 0xf0 == 0xc0 {
                self.write(0, reg, value | 0x20);
                self.write(1, reg, value | 0x10);
            } else {
                self.write(0, reg, value);
                self.write(1, reg, value);
            }
        }
    }

    /// The velocity law (`0x10F2`, `0x11B7`, `0x2830`).
    ///
    /// ```text
    /// c = curve[velocity >> 1]
    /// t = (0x40 - c) * 2
    /// level = (0x2000 - (0x40 - patch_level) * t) >> 7
    /// ```
    ///
    /// The key-scale bits of the patch's own `0x40` byte are kept, the six
    /// level bits replaced. `side` names a single bank, which is what panning
    /// uses; `None` writes both, bank 1 first — the order the original goes in.
    ///
    /// Two guards here that the original does not have, both for values that
    /// can only arrive from a damaged song and both on the audio thread, where
    /// a panic is not a failed test but a dead process:
    ///
    /// - The curve table has 64 entries and is indexed by `velocity >> 1`, so a
    ///   velocity above 127 reads past it. MIDI velocity is seven bits and the
    ///   decoder does not mask, so the byte arrives as it was written. The
    ///   original would have read whatever followed the table in its own data
    ///   segment; there is no way to reproduce *that* faithfully from here, and
    ///   masking is the reading that keeps the note in range.
    /// - `0x40 - curve` underflows if a driver image ever held a curve entry
    ///   above 0x40. `HMIMDRV.386` does not, and `velocity_becomes_a_total_level`
    ///   checks the whole table falls; the saturation is for images this was
    ///   never built against.
    fn level(&mut self, op: usize, velocity: u8, side: Option<u8>) {
        let patch_level = (self.shadow_40[op] & 0x3f) as u32;
        let curve = self.tables.velocity[(velocity & 0x7f) as usize >> 1] as u32;
        let t = (0x40u32.saturating_sub(curve)) * 2;
        let attenuation = (0x2000u32.wrapping_sub((0x40 - patch_level) * t)) >> 7;
        let value = (self.shadow_40[op] & 0xc0) | (attenuation as u8);
        match side {
            Some(bank) => self.write(bank, 0x40 + op as u8, value),
            None => {
                self.write(1, 0x40 + op as u8, value);
                self.write(0, 0x40 + op as u8, value);
            }
        }
    }

    /// `0x276D`. Attenuates the copy on one side and leaves the other alone.
    fn set_pan(&mut self, ch: usize, pan: u8) {
        self.pan[ch] = pan;
        // Folded to a distance from whichever end is nearer, then doubled:
        // 126 at the center, 0 at either extreme.
        let weight = (if pan >= 0x40 { 0x7f - pan } else { pan } as u32) << 1;
        // Which bank is quietened. See the module documentation: read against
        // the chip's own bit assignment this is the wrong way round, and only
        // the assignment — not any recording — can settle it.
        let side = if pan < 0x40 { 1 } else { 0 };

        for v in 0..VOICES {
            if self.note[v] == 0 || self.owner[v] as usize != ch {
                continue;
            }
            let x = (self.volume[ch] as u32 * self.velocity[v] as u32) >> 7;
            let x = ((weight * x) >> 7) as u8;
            let c = self.tables.carrier(v);
            self.level(c, x, Some(side));
            // And the modulator too, when the patch adds rather than chains.
            // This is the same connection test the note-on path makes — and
            // the same fault: it always reads the *melodic* bank, so a drum
            // voice is judged by whatever program channel 9 was left on.
            let patch = self
                .melodic
                .get(self.program[self.owner[v] as usize & 0x0f] as usize)
                .copied()
                .unwrap_or_default();
            if patch.additive() {
                let m = self.tables.modulator(v);
                self.level(m, x, Some(side));
            }
        }
    }

    /// `0x2A0A`: sets the frequency, then keys off and on again, so a repeated
    /// note restarts its envelope instead of continuing.
    fn key_on(&mut self, voice: usize, packed: u32) {
        self.set_frequency(voice, packed, false);
        let on = self.shadow_b0[voice];
        self.write(0, 0xb0 + voice as u8, on & 0xdf);
        self.write(1, 0xb0 + voice as u8, on & 0xdf);
        self.write(0, 0xb0 + voice as u8, on);
        self.write(1, 0xb0 + voice as u8, on);
    }

    /// `0x2B26` when `keyed`: writes `0xA0` and `0xB0` without the pulse.
    fn set_frequency(&mut self, voice: usize, packed: u32, keyed: bool) {
        self.shadow_a0[voice] = packed as u8;
        self.shadow_b0[voice] = ((packed >> 8) as u8) | 0x20;
        let a = self.shadow_a0[voice];
        self.write(0, 0xa0 + voice as u8, a);
        self.write(1, 0xa0 + voice as u8, a);
        if keyed {
            let b = self.shadow_b0[voice];
            self.write(0, 0xb0 + voice as u8, b);
            self.write(1, 0xb0 + voice as u8, b);
        }
    }

    /// `(block << 10) | fnum` for a note, clamped to the table.
    fn frequency(&self, note: u8) -> u32 {
        let i = (note as usize).saturating_sub(12).min(Tables::NOTES - 1);
        self.tables.frequency[i]
    }

    // ------------------------------------------------------------ the bend

    /// `0x1B5F`. Interpolates in per-mille steps between the note and the note
    /// a bend range away.
    fn bend_frequency(&self, voice: usize, note: u8, bend: u8) -> u32 {
        let index = (note as usize).saturating_sub(12);
        let packed = self.frequency(note);
        let block = packed & 0x1c00;
        let fnum = packed & 0x3ff;
        let channel = self.owner[voice] as usize & 0x0f;

        if bend < 0x40 {
            let per_mille = ((0x40 - bend as u32) * 1000) >> 6;
            // Fault 2: the range is indexed by the voice here and by the MIDI
            // channel three instructions later (`0x1BCD` against `0x1BFA`).
            let range = self.bend_range[voice] as usize;
            let mut span = packed.wrapping_sub(self.table_at(index.wrapping_sub(range)));
            if span > 0x2cf {
                // The two notes sit in different blocks, so the difference of
                // the packed values is meaningless; take it between f-numbers
                // with the lower note expressed one block up.
                let range = self.bend_range[channel] as usize;
                span = fnum.wrapping_sub(self.tables.octave_down[range.saturating_sub(1).min(11)])
                    & 0x3ff;
            }
            packed.wrapping_sub(span * per_mille / 1000)
        } else {
            let per_mille = ((bend as u32 - 0x40) * 1000) >> 6;
            let range = self.bend_range[channel] as usize;
            let up = self.table_at(index + range);
            let mut span = up.wrapping_sub(packed);
            let mut packed = packed;
            if span > 0x2cf {
                let step = self.tables.octave_down[11 - index % 12];
                packed = (block + 0x400) | step;
                span = up.wrapping_sub(packed);
            }
            packed.wrapping_add(span * per_mille / 1000)
        }
    }

    fn table_at(&self, index: usize) -> u32 {
        self.tables.frequency.get(index).copied().unwrap_or(0)
    }

    /// `0x1A9E`.
    ///
    /// **Channel 9 is turned away at the door** (`0x1ABB`), and that has a
    /// consequence well beyond percussion not bending: `bent[9]` therefore
    /// never becomes true, so once every melodic channel has bent at least
    /// once, channel 9 is the only one voice stealing will take from. In a
    /// piece where the melodic parts all bend — and every one of the game's
    /// does — the drums are what gets cut off when the ninth voice runs out.
    fn pitch_bend(&mut self, ch: usize, msb: u8) {
        if ch == 9 {
            return;
        }
        self.bend[ch] = msb;
        self.bent[ch] = true;
        for v in 0..VOICES {
            if self.note[v] == 0 || self.owner[v] as usize != ch {
                continue;
            }
            let packed = self.bend_frequency(v, self.note[v], msb);
            self.set_frequency(v, packed, true);
        }
    }

    // ----------------------------------------------------------- controllers

    /// `0x1926`. Five controllers reach the chip; everything else is dropped.
    fn control(&mut self, ch: usize, controller: u8, value: u8) {
        match controller {
            7 => self.volume[ch] = value,
            10 => self.set_pan(ch, value),
            64 => {
                self.sustain[ch] = value != 0;
                if !self.sustain[ch] {
                    for note in std::mem::take(&mut self.held[ch]) {
                        self.note_off(ch, note);
                    }
                }
            }
            102 => self.bend_range[ch] = value,
            // 121 resets the controllers, 123 silences the channel. Both go
            // through the same per-channel defaults at `0x1875`.
            121 | 123 => {
                if controller == 123 {
                    for v in 0..VOICES {
                        if self.owner[v] as usize == ch && self.note[v] != 0 {
                            self.stop(v);
                            self.note[v] = 0;
                        }
                    }
                }
                self.volume[ch] = 0x7f;
                self.sustain[ch] = false;
                self.bend[ch] = 0x40;
                self.bend_range[ch] = 2;
                self.held[ch].clear();
            }
            _ => {}
        }
    }

    /// The `0x80` shadow, **including the original's overrun**.
    ///
    /// `0x3568` is sixteen bytes: the next array, the per-voice `0xA0` shadow,
    /// starts at `0x3578`. But the driver indexes it by operator offset, and
    /// those run to `0x15` — so operators `0x10` … `0x15`, which are the
    /// operators of voices 6, 7 and 8, read and write the `0xA0` shadows of
    /// voices 0 … 5 instead.
    ///
    /// The recording shows it plainly: at the first note ever played on voice
    /// 6, with nothing having touched its registers, the release-rate write
    /// goes out as `0xBF` — `0xB0 | 0x0F`, and `0xB0` is the low byte of the
    /// f-number voice 0 was playing at the time. Reading it is harmless noise
    /// on the first note of those three voices; writing it scribbles over
    /// another voice's f-number shadow, which the next frequency write
    /// overwrites anyway.
    fn sustain_shadow(&self, op: usize) -> u8 {
        match self.shadow_80.get(op) {
            Some(&v) => v,
            None => self.shadow_a0[op - 0x10],
        }
    }

    fn set_sustain_shadow(&mut self, op: usize, value: u8) {
        match self.shadow_80.get_mut(op) {
            Some(slot) => *slot = value,
            None => self.shadow_a0[op - 0x10] = value,
        }
    }

    /// `0x1E60` for bank 0, `0x1E97` for bank 1 — the same routine on
    /// `port`/`port + 1` and `port + 2`/`port + 3`, with a string of dummy
    /// reads between the two `out`s to give the chip its settling time.
    ///
    /// **It does not touch the shadows.** `0x3548` and `0x3568` are written
    /// only where a patch is programmed, so they keep the patch's own values
    /// and every later velocity or pan calculation starts from those rather
    /// than from what it last wrote. Feeding the written value back would make
    /// each note quieter than the one before it.
    fn write(&mut self, bank: u8, reg: u8, value: u8) {
        self.out.push(Write { bank, reg, value });
    }
}
