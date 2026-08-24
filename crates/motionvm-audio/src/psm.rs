//! PSM 2: the Ad Lib music of Die Enviro-Kids greifen ein.
//!
//! The 16-bit game's whole music player lives in `MUSADL.DRV` — 4480 bytes
//! behind a `MUS\0` header — and `ENVIRO.EXE` holds only a loader, a far-jump
//! table and a timer. This module is that driver, read at the instruction
//! level and rebuilt: [`Driver`] carries the four data tables out of the
//! shipped file, [`Sequencer`] is the tick — event streams in, OPL register
//! writes out — and [`Player`] adds the clock and the chip, the shape
//! [`crate::Player`] has for the other game.
//!
//! File offsets in the comments are `MUSADL.DRV`'s unless marked otherwise.
//! The registers go through a shadow of all 256 (`0x5c4`, written through
//! `0x7df`/`0x7ec`): a value the shadow already holds is not sent again, so
//! the stream carries **changes only** — which is what makes it comparable,
//! byte for byte, against a DRO capture of the original.

use crate::error::{Error, Result};
use crate::opl::Write;
use motionvm_formats::m16::psm::Plx;

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
    /// version 1.00, fifteen entries, seven imports, and the `NS` trailer
    /// at the offset the header names.
    pub fn parse(file: &[u8]) -> Result<Self> {
        let head_ok = file.len() >= 0x796
            && &file[..4] == b"MUS\0"
            && file[4..6] == [0x00, 0x01]
            && file[6..8] == [0x0F, 0x00]
            && file[8..10] == [0x07, 0x00];
        if !head_ok {
            return Err(Error::Psm("not a MUS 1.00 driver with 15 entries"));
        }
        let trailer = u16::from_le_bytes([file[0x0a], file[0x0b]]) as usize;
        if file.get(trailer..trailer + 2) != Some(b"NS".as_ref()) {
            return Err(Error::Psm("the NS trailer is not where the header says"));
        }
        let mut defaults = [0u8; 256];
        defaults.copy_from_slice(&file[0x5c4..0x6c4]);
        let mut mod_ops = [0u8; 9];
        mod_ops.copy_from_slice(&file[0x6c4..0x6cd]);
        let mut car_ops = [0u8; 9];
        car_ops.copy_from_slice(&file[0x6cd..0x6d6]);
        let mut notes = [0u16; 96];
        for (i, n) in notes.iter_mut().enumerate() {
            *n = u16::from_le_bytes([file[0x6d6 + 2 * i], file[0x6d7 + 2 * i]]);
        }
        Ok(Self {
            defaults,
            mod_ops,
            car_ops,
            notes,
        })
    }
}

/// One channel of a song, as the driver keeps it.
#[derive(Clone, Copy, Default)]
struct Channel {
    /// Position in the section's bytes; 0 is a dead channel (`0x796`).
    at: usize,
    /// The row the next event falls on (`0x7a8`).
    next_row: u16,
    /// The last frequency word written (`0x5b2`).
    freq: u16,
    /// The event volume (`0x59e`): loudness 0–63 in the low byte, the
    /// carrier's KSL bits in the high.
    volume: u16,
}

/// The driver's playing state: the tick at `0xa71`, rebuilt.
pub struct Sequencer {
    song: Plx,
    /// Loop bookkeeping: the pass this play is on and the last pass wanted
    /// (`0x26a`, `0x26c`) — compared unsigned, `0xFFFF` plays forever.
    pass: u16,
    last_pass: u16,
    /// The row (`0x26e`) and the tick countdown to the next row (`0x5b1`).
    row: u16,
    countdown: i16,
    /// Ticks a row lasts (`0x5b0`) and the tick period in PIT cycles
    /// (`0x270`), `max(0x200, tempo)` at the default speed scale.
    ticks_per_row: u8,
    period: u16,
    channels: [Channel; 9],
    /// Channel master volumes (`0x58c`), `0x100` throughout — the game
    /// never calls the volume entry.
    master: [u16; 9],
    /// The fade: level in `1/256`ths above the fraction byte (`0x27f:0x280`
    /// as one number) and the signed step a tick adds (`0x272`).
    fade_pos: i32,
    fade_step: i32,
    /// The register shadow (`0x5c4`): what the chip last heard.
    shadow: [u8; 256],
    mod_ops: [u8; 9],
    car_ops: [u8; 9],
    notes: [u16; 96],
}

impl Sequencer {
    /// Takes a whole module — the block `STARTTUNE` names — and stands at
    /// its first row, the shadow primed with the driver's defaults.
    ///
    /// The default writes themselves belong to the driver's *init*, not to
    /// a song: [`Sequencer::install_writes`] hands them out once, and a
    /// second song starts from the shadow the first one left, exactly as
    /// the original's driver stays installed across `STARTTUNE`s.
    pub fn new(driver: &Driver, song: Plx, loops: i16) -> Self {
        let mut shadow = [0xFFu8; 256];
        shadow.copy_from_slice(&driver.defaults);
        let mut seq = Self {
            song,
            pass: 1,
            last_pass: loops as u16,
            row: 0,
            countdown: 0,
            ticks_per_row: 1,
            period: 0x200,
            channels: [Channel::default(); 9],
            master: [0x100; 9],
            fade_pos: 0x100 << 8,
            fade_step: 0,
            shadow,
            mod_ops: driver.mod_ops,
            car_ops: driver.car_ops,
            notes: driver.notes,
        };
        seq.prime();
        seq
    }

    /// The init's register pass (`0xf9e` path at `0x105e`): every register
    /// whose default is not `0xFF`, ascending. What a fresh OPL has to hear
    /// once, before any song.
    pub fn install_writes(driver: &Driver) -> Vec<Write> {
        driver
            .defaults
            .iter()
            .enumerate()
            .filter(|&(_, &v)| v != 0xFF)
            .map(|(reg, &v)| Write {
                bank: 0,
                reg: reg as u8,
                value: v,
            })
            .collect()
    }

    /// `0xcb4`: the song's head into the driver state, the streams at their
    /// starts. The countdown comes out one under the speed, so the first
    /// row falls on the `speed`th tick.
    fn prime(&mut self) {
        self.ticks_per_row = self.song.speed;
        self.period = period_of(self.song.tempo);
        self.countdown = self.song.speed as i16 - 1;
        self.row = 0;
        for (ch, chan) in self.channels.iter_mut().enumerate() {
            chan.at = self.song.channels[ch] as usize;
            chan.next_row = 0;
        }
    }

    /// The tick period this sequencer currently runs at, in PIT cycles.
    pub fn period(&self) -> u32 {
        self.period as u32
    }

    /// Whether the song still plays; cleared when every channel has ended
    /// and the passes are spent, or by [`Sequencer::silence`].
    pub fn playing(&self) -> bool {
        self.pass != 0
    }

    /// `SetSong` and the play entry (`0x2f9`, `0x470`): notes stopped where
    /// they still ring, the new song in, the volume full, the first row
    /// `speed` ticks away. The register shadow stays — the driver keeps its
    /// chip state across songs, so whatever a song shares with the last one
    /// goes out only where it differs.
    pub fn play(&mut self, song: Plx, loops: i16, out: &mut Vec<Write>) {
        if self.pass != 0 {
            self.silence(out);
        }
        self.song = song;
        self.pass = 1;
        self.last_pass = loops as u16;
        self.fade_step = 0;
        self.fade_pos = (0x100 << 8) | (self.fade_pos & 0xFF);
        self.prime();
    }
    /// Starts the 2000 ms fade `ENDTUNE` starts (`0x3e2` with `0x7d0`).
    pub fn fade_out(&mut self) {
        let ticks = (0x4a9 * 0x7d0) / self.period as i32;
        self.fade_step = -(0x10000 / ticks.max(1));
    }

    /// The stop entry (`0x4c9` → `0xde2`): from channel 8 down, the key
    /// off where it is on and the release rate of both operators opened to
    /// its fastest, so what still sounds dies away.
    pub fn silence(&mut self, out: &mut Vec<Write>) {
        for ch in (0..9).rev() {
            let b0 = 0xb0 + ch as u8;
            let val = self.shadow[b0 as usize];
            if val & 0x20 != 0 {
                self.write(out, b0, val ^ 0x20);
            }
            self.write(out, 0x80 + self.car_ops[ch], 0x0F);
            self.write(out, 0x80 + self.mod_ops[ch], 0x0F);
        }
        self.pass = 0;
        self.fade_step = 0;
    }

    /// One driver tick (`0xa71`): a row when the countdown runs out, the
    /// fade every tick while one runs, the volume pass after every row.
    pub fn tick(&mut self, out: &mut Vec<Write>) {
        if self.pass == 0 {
            return;
        }
        self.countdown -= 1;
        if self.countdown >= 0 {
            if self.fade_step != 0 {
                self.fade_tick(out);
            }
            return;
        }
        self.countdown += self.ticks_per_row as i16;

        loop {
            let mut live = 0;
            for ch in (0..9).rev() {
                // `0xadf`: every event that is due runs — a zero delay
                // chains events inside one row — and a channel that dies
                // under its events no longer counts as live.
                while self.channels[ch].at != 0 && self.channels[ch].next_row <= self.row {
                    self.event(ch, out);
                }
                if self.channels[ch].at != 0 {
                    live += 1;
                }
            }

            let (row, wrapped) = self.row.overflowing_add(1);
            self.row = row;
            if live != 0 && !wrapped {
                break;
            }
            // `0xb26`: the song is over. The passes are compared unsigned,
            // so `-1` loops forever — and the loop is gapless: the fresh
            // first row runs in this same tick (`0xb63` jumps back up).
            if self.pass > self.last_pass {
                self.pass = 0;
                return;
            }
            // The pass counter holds at the endless mark rather than
            // wrapping past it, as the handler's compare-and-skip does.
            self.pass = self.pass.saturating_add(1);
            self.prime();
        }
        if self.fade_step != 0 {
            self.fade_tick(out);
        } else if self.fade_pos >> 8 != 0 {
            self.volume_pass(out);
        }
    }

    /// One event off a channel's stream — the decoder at `0x890`.
    fn event(&mut self, ch: usize, out: &mut Vec<Write>) {
        let Some(&flags) = self.song.bytes.get(self.channels[ch].at) else {
            self.channel_end(ch, out);
            return;
        };
        self.channels[ch].at += 1;
        if flags == 0 {
            self.channel_end(ch, out);
            return;
        }
        if flags != 0x80 {
            if flags & 0x01 != 0 {
                let Some(instrument) = self.u16_operand(ch) else {
                    self.channel_end(ch, out);
                    return;
                };
                self.instrument(ch, instrument as usize, out);
            }
            if flags & 0x02 != 0 {
                // `0x960`: the byte is a raw carrier level — loudness
                // inverted out of it, the KSL bits kept beside it, exactly
                // like the instrument's own level byte.
                let Some(volume) = self.u8_operand(ch) else {
                    self.channel_end(ch, out);
                    return;
                };
                self.channels[ch].volume =
                    ((volume & 0x3F) ^ 0x3F) as u16 | ((volume & 0xC0) as u16) << 8;
            }
            if flags & 0x04 != 0 {
                // `0x97a`: only a key that is on goes off.
                let b0 = 0xb0 + ch as u8;
                let val = self.shadow[b0 as usize];
                if val & 0x20 != 0 {
                    self.write(out, b0, val & !0x20);
                }
            }
            if flags & 0x38 != 0 {
                // `0x9a5`: the key bit is the flag's **or the register's** —
                // a note without bit 5 keeps a key that is already down —
                // and the stored frequency word carries it in its high byte.
                let key = ((self.shadow[0xb0 + ch] | flags) as u16 & 0x20) << 8;
                let word = if flags & 0x08 != 0 {
                    let Some(note) = self.u8_operand(ch) else {
                        self.channel_end(ch, out);
                        return;
                    };
                    // The note byte is twice the semitone: a byte offset
                    // into the word table (`0x9be`).
                    let freq = self
                        .notes
                        .get(note as usize / 2)
                        .copied()
                        .unwrap_or_default();
                    freq | key
                } else {
                    let mut word = self.channels[ch].freq;
                    if flags & 0x10 != 0 {
                        let Some(raw) = self.u16_operand(ch) else {
                            self.channel_end(ch, out);
                            return;
                        };
                        word = raw;
                    }
                    word | key
                };
                self.channels[ch].freq = word;
                // The pitch scale (`0x9e7`) stays at its default of 0x100 —
                // nothing in the game moves it — so the word goes out as it
                // stands.
                self.write(out, 0xa0 + ch as u8, (word & 0xFF) as u8);
                self.write(out, 0xb0 + ch as u8, (word >> 8) as u8);
            }
            let rest = flags & 0xC0;
            if rest == 0x40 {
                let Some(tempo) = self.u16_operand(ch) else {
                    self.channel_end(ch, out);
                    return;
                };
                self.period = period_of(tempo);
            } else if rest != 0 {
                // `0xa6f`: a flag the decoder does not know ends the
                // channel — no stream of the shipped songs carries one.
                self.channel_end(ch, out);
                return;
            }
        }
        let Some(&delay) = self.song.bytes.get(self.channels[ch].at) else {
            self.channel_end(ch, out);
            return;
        };
        self.channels[ch].at += 1;
        self.channels[ch].next_row = self.channels[ch].next_row.wrapping_add(delay as u16);
    }

    /// `0x868`: the stream is over; the channel keys off and goes dead.
    fn channel_end(&mut self, ch: usize, out: &mut Vec<Write>) {
        let b0 = 0xb0 + ch as u8;
        let val = self.shadow[b0 as usize] & !0x20;
        self.write(out, b0, val);
        self.channels[ch].at = 0;
    }

    /// The instrument load (`0x898`–`0x9e0`): eleven raw register values,
    /// one byte past the stored offset, written in the handler's order —
    /// the carrier's level byte is not sent, it becomes the event volume.
    fn instrument(&mut self, ch: usize, offset: usize, out: &mut Vec<Write>) {
        let Some(r) = self.song.bytes.get(offset + 1..offset + 12) else {
            return;
        };
        let r: [u8; 11] = r.try_into().expect("eleven bytes just sliced");
        let (m, c) = (self.mod_ops[ch], self.car_ops[ch]);
        self.write(out, 0xc0 + ch as u8, r[0]);
        for (i, base) in [0x20u8, 0x40, 0x60, 0x80, 0xe0].iter().enumerate() {
            self.write(out, base + m, r[1 + i]);
        }
        self.write(out, 0x20 + c, r[6]);
        // `0x91e`: loudness inverted out of the level byte, the KSL bits
        // kept beside it.
        self.channels[ch].volume = (0x3F - (r[7] & 0x3F)) as u16 | ((r[7] & 0xC0) as u16) << 8;
        for (i, base) in [0x60u8, 0x80, 0xe0].iter().enumerate() {
            self.write(out, base + c, r[8 + i]);
        }
    }

    /// The per-row volume pass (`0xc5e`): every live channel's carrier
    /// level, channel 8 down to 0, from the master and event volumes,
    /// through the shadow so only changes go out.
    fn volume_pass(&mut self, out: &mut Vec<Write>) {
        for ch in (0..9).rev() {
            if self.channels[ch].at == 0 {
                continue;
            }
            let master = self.master[ch];
            let loud = (self.channels[ch].volume & 0x3F) as u8;
            let v = if master >= 0x100 {
                loud
            } else {
                ((master as u32 * loud as u32) >> 8) as u8
            };
            let reg = 0x40 + self.car_ops[ch];
            let val = (v ^ 0x3F) | (self.channels[ch].volume >> 8) as u8;
            self.write(out, reg, val);
        }
    }

    /// One tick of a running fade (`0xb77`): a fraction byte under a level
    /// word moves by the step; past 0x100 the fade is done and the full
    /// pass runs, at zero the carriers close, in between every live channel
    /// follows the level — the handler's byte arithmetic kept as it is,
    /// truncations and all.
    fn fade_tick(&mut self, out: &mut Vec<Write>) {
        self.fade_pos += self.fade_step;
        let level = ((self.fade_pos >> 8) & 0xFFFF) as u16;
        if level >= 0x100 {
            self.fade_step = 0;
            if (level as i16) >= 0 {
                self.fade_pos = 0x100 << 8;
                self.volume_pass(out);
                return;
            }
            // A step that overshot below zero leaves the wrapped level
            // standing, as the handler does (`0xb9e`); the game stops a
            // fade-out long before this could be reached.
        }
        self.fade_pos = ((level as i32) << 8) | (self.fade_pos & 0xFF);
        if level == 0 {
            // `0xc26`: every carrier closed, channel 8 down to 0.
            for ch in (0..9).rev() {
                let reg = 0x40 + self.car_ops[ch];
                self.write(out, reg, 0xFF);
            }
            return;
        }
        for ch in (0..9).rev() {
            if self.channels[ch].at == 0 {
                continue;
            }
            let scaled = ((self.master[ch] as u32 * level as u32) >> 8) as u8;
            let loud = (self.channels[ch].volume & 0x3F) as u8;
            let v = ((scaled as u32 * loud as u32) >> 8) as u8;
            let reg = 0x40 + self.car_ops[ch];
            let val = (v | (self.channels[ch].volume >> 8) as u8) ^ 0x3F;
            self.write(out, reg, val);
        }
    }

    fn u8_operand(&mut self, ch: usize) -> Option<u8> {
        let v = self.song.bytes.get(self.channels[ch].at).copied();
        self.channels[ch].at += 1;
        v
    }

    fn u16_operand(&mut self, ch: usize) -> Option<u16> {
        let at = self.channels[ch].at;
        let bytes = self.song.bytes.get(at..at + 2)?;
        self.channels[ch].at += 2;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// One register write, through the shadow (`0x7df`): a value the chip
    /// already holds is not sent again.
    fn write(&mut self, out: &mut Vec<Write>, register: u8, value: u8) {
        if self.shadow[register as usize] == value {
            return;
        }
        self.shadow[register as usize] = value;
        out.push(Write {
            bank: 0,
            reg: register,
            value,
        });
    }
}

/// `0xd29`: the tick period from a tempo word, at the default speed scale
/// of `0x100` — the driver multiplies and rounds, and the game never moves
/// the scale.
fn period_of(tempo: u16) -> u16 {
    tempo.max(0x200)
}

/// The rebuilt music path under a sample clock: the [`Sequencer`], the OPL2
/// it writes to, and the PIT arithmetic between them.
///
/// The original's tick is the sound host's timer service: `MUSADL.DRV` asks
/// for its tempo word as a period in PIT cycles and the host programs timer
/// channel 0 with it. Here a rendered frame is worth `PIT_HZ` cycles times
/// the output rate, a tick falls due every `rate × period` of those, and the
/// remainder carries over — the same integer scheme as [`crate::Player`], so
/// the music cannot drift against the device whatever its rate is.
pub struct Player {
    chip: crate::chip::Chip,
    rate: u32,
    driver: Driver,
    seq: Option<Sequencer>,
    writes: Vec<Write>,
    /// The clock, in PIT cycles times the output rate.
    clock: u64,
    /// Frames left until a pending stop lands — see [`Player::stop`].
    stop_in: Option<u64>,
}

impl Player {
    /// Builds the player for one output rate from the shipped `MUSADL.DRV`,
    /// and puts the chip in the driver's install state: every register
    /// default the file carries, in ascending order, as the init entry
    /// writes them (`0xf9e` → `0x1062`).
    pub fn new(rate: u32, driver_file: &[u8]) -> Result<Self> {
        if rate == 0 {
            return Err(Error::ZeroRate);
        }
        let driver = Driver::parse(driver_file)?;
        let mut chip = crate::chip::Chip::new(rate);
        for w in Sequencer::install_writes(&driver) {
            chip.write(w);
        }
        Ok(Self {
            chip,
            rate,
            driver,
            seq: None,
            writes: Vec::new(),
            clock: 0,
            stop_in: None,
        })
    }

    /// The sample rate it was made with.
    pub fn rate(&self) -> u32 {
        self.rate
    }

    /// Whether a song is running — a fade-out still counts, the silence
    /// after it does not.
    pub fn playing(&self) -> bool {
        self.seq.as_ref().is_some_and(Sequencer::playing)
    }

    /// Starts a song: `STARTTUNE`'s path, `SetSong` then `Play`. The first
    /// song builds the sequencer; every later one goes through
    /// [`Sequencer::play`], keeping the register shadow, exactly as the
    /// resident driver keeps its state from tune to tune. The clock starts
    /// over — `Play` re-registers the host timer (`0xcb4`).
    pub fn start(&mut self, song: Plx, loops: i16) {
        self.stop_in = None;
        self.writes.clear();
        match &mut self.seq {
            Some(seq) => seq.play(song, loops, &mut self.writes),
            None => self.seq = Some(Sequencer::new(&self.driver, song, loops)),
        }
        for w in self.writes.drain(..) {
            self.chip.write(w);
        }
        self.clock = 0;
    }

    /// Stops the song the way `ENDTUNE` does — the game's one stop site
    /// (`ENVIRO.EXE` `1696:02fd`): the 2000 ms fade-out begins at once and
    /// the hard stop (`0xde2`: key-offs and both operators' release rate
    /// opened to `0x0F`) lands 500 ms in, so what still sounds dies away
    /// under the fade's last level.
    pub fn stop(&mut self) {
        let Some(seq) = &mut self.seq else { return };
        if !seq.playing() {
            return;
        }
        seq.fade_out();
        self.stop_in = Some(self.rate as u64 / 2);
    }

    /// Fills `out` with interleaved stereo frames.
    ///
    /// An odd length is a caller's mistake; the last lone sample is left
    /// alone rather than turning the two channels around after it.
    pub fn fill(&mut self, out: &mut [i16]) {
        let frames = out.len() / 2;
        let mut done = 0;
        while done < frames {
            let step = self.frames_to_tick().min(frames - done);
            if step > 0 {
                self.chip.render(&mut out[done * 2..(done + step) * 2]);
                done += step;
                self.clock += step as u64 * u64::from(PIT_HZ);
                self.count_down_stop(step as u64);
            }
            let period = self.tick_period();
            if self.clock >= period {
                self.clock -= period;
                self.tick();
            }
        }
    }

    /// A tick lasts `period` PIT cycles and a rendered frame is worth
    /// `PIT_HZ / rate` of them — multiplied through by `rate` so every side
    /// stays a whole number.
    fn tick_period(&self) -> u64 {
        let period = self.seq.as_ref().map_or(0x200, Sequencer::period);
        u64::from(self.rate) * u64::from(period)
    }

    /// How many frames may be rendered before the next tick falls due.
    fn frames_to_tick(&self) -> usize {
        let left = self.tick_period().saturating_sub(self.clock);
        // Round up: the tick belongs to the frame that reaches the mark,
        // not to the one before it.
        left.div_ceil(u64::from(PIT_HZ)) as usize
    }

    /// One sequencer tick, and whatever registers it turns into.
    fn tick(&mut self) {
        let Some(seq) = &mut self.seq else { return };
        seq.tick(&mut self.writes);
        for w in self.writes.drain(..) {
            self.chip.write(w);
        }
    }

    /// Runs the pending stop down by `frames` rendered, and lands it when
    /// its 500 ms are over.
    fn count_down_stop(&mut self, frames: u64) {
        let Some(left) = &mut self.stop_in else {
            return;
        };
        if *left > frames {
            *left -= frames;
            return;
        }
        self.stop_in = None;
        if let Some(seq) = &mut self.seq {
            seq.silence(&mut self.writes);
            for w in self.writes.drain(..) {
                self.chip.write(w);
            }
        }
    }
}
