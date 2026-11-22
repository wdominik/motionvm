//! The driver's tick: event streams in, OPL register writes out.
//!
//! The whole music player lives in `MUSADL.DRV` — a few thousand bytes
//! behind a `MUS\0` header, in the two builds [`Driver`] reads — and the
//! program hosting it holds only a loader, a far-jump table and a timer.
//! This is that driver's `0xa71`, read at the instruction level and rebuilt.
//!
//! File offsets in the comments are `MUSADL.DRV`'s unless marked otherwise.
//! The registers go through a shadow of all 256 (`0x5c4`, written through
//! `0x7df`/`0x7ec`): a value the shadow already holds is not sent again, so
//! the stream carries **changes only** — which is what makes it comparable,
//! byte for byte, against a DRO capture of the original.

use crate::driver::Driver;
use crate::format::Plx;
use crate::num;
use motionvm_opl::Write;

/// A call the driver makes through its callback cell (`[27Ah]`), which a
/// digital driver installed beside it fills with its table (`DMABLAST.DRV`
/// `0x2318`). The calls are made where the driver makes them; what the
/// digital side answers is [`Player`](crate::Player)'s to deliver, and the
/// row count the answers set (`[278h]`) goes with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Call {
    /// `bx = 0`: the song is over (`0xb66`) or stopped (`0xe2f`).
    Stop,
    /// `bx = 2` at the end of the priming (`0xda3`): a song starts at the
    /// level word `[280h]` as it stands.
    Start {
        /// The level, `0x100` from the play entry and 0 from the fade-in
        /// entry.
        level: u16,
    },
    /// The tick where the countdown reaches 0 (`0xa89`), and the tick a
    /// song loops (`0xb4b`): the row count down by one, and `bx = 4` when
    /// it reaches 0.
    Step,
    /// `bx = 6` with `ax = −1` from a fade tick: the level for every voice
    /// (`0xbae`, `0xc1e`, `0xc51`).
    Volume {
        /// The level: `0x100` at a fade's full end, 0 at its silent one,
        /// the fade's level word in between.
        level: u16,
    },
}

/// One channel of a song, as the driver keeps it.
#[derive(Debug, Clone, Copy, Default)]
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
#[derive(Debug)]
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
    /// Channel master volumes (`0x58c`), `0x100` throughout — nothing here
    /// calls the driver's volume entry.
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
    /// The callbacks made since they were last taken — `None` while the
    /// cell holds the file's own stub (`0x57e`, a bare `retf`), which a
    /// digital driver has not replaced.
    calls: Option<Vec<Call>>,
}

impl Sequencer {
    /// Takes a whole module — the song a host hands the driver — and stands
    /// at its first row, the shadow primed with the driver's defaults.
    ///
    /// The default writes themselves belong to the driver's *init*, not to
    /// a song: [`Sequencer::install_writes`] hands them out once, and a
    /// second song starts from the shadow the first one left, exactly as
    /// the original's driver stays installed from one song to the next.
    pub fn new(driver: &Driver, song: Plx, loops: i16) -> Self {
        Self::build(driver, song, loops, 0x100, false)
    }

    /// The first song of a freshly installed driver at `level` — `0x100` by
    /// the play entry, 0 by the fade-in entry, which sets it before priming
    /// (`0x4a5`, `0x378`) — with the callbacks recorded when `linked`.
    fn build(driver: &Driver, song: Plx, loops: i16, level: u16, linked: bool) -> Self {
        let mut shadow = [0xFFu8; 256];
        shadow.copy_from_slice(&driver.defaults);
        let mut seq = Self {
            song,
            pass: 1,
            last_pass: num::passes(loops),
            row: 0,
            countdown: 0,
            ticks_per_row: 1,
            period: 0x200,
            channels: [Channel::default(); 9],
            master: [0x100; 9],
            fade_pos: i32::from(level) << 8,
            fade_step: 0,
            shadow,
            mod_ops: driver.mod_ops,
            car_ops: driver.car_ops,
            notes: driver.notes,
            calls: linked.then(Vec::new),
        };
        seq.prime();
        seq
    }

    /// [`Sequencer::new`] with its callbacks recorded from the start, as a
    /// driver whose cell a digital driver filled makes them.
    pub(crate) fn new_linked(driver: &Driver, song: Plx, loops: i16) -> Self {
        Self::build(driver, song, loops, 0x100, true)
    }

    /// [`Sequencer::new_fade`] with its callbacks recorded from the start.
    pub(crate) fn new_fade_linked(driver: &Driver, song: Plx, loops: i16, ms: u16) -> Self {
        let mut seq = Self::build(driver, song, loops, 0, true);
        seq.fade_in(ms);
        seq
    }

    /// The callback cell from here on holds a digital driver's table: the
    /// calls are recorded for [`Sequencer::take_calls`].
    pub(crate) fn link(&mut self) {
        self.calls.get_or_insert_with(Vec::new);
    }

    /// The callback cell back on the file's stub: no call is recorded from
    /// here on.
    pub(crate) fn unlink(&mut self) {
        self.calls = None;
    }

    /// The callbacks made since the last take, in order.
    pub(crate) fn take_calls(&mut self) -> Vec<Call> {
        self.calls.as_mut().map(std::mem::take).unwrap_or_default()
    }

    /// One callback, where the cell holds a table to make it through.
    fn call(&mut self, call: Call) {
        if let Some(calls) = &mut self.calls {
            calls.push(call);
        }
    }

    /// The init's register pass (`0xf9e` path at `0x105e`): every register
    /// whose default is not `0xFF`, ascending. What a fresh OPL has to hear
    /// once, before any song.
    pub fn install_writes(driver: &Driver) -> Vec<Write> {
        driver
            .defaults
            .iter()
            .zip(0u8..=u8::MAX)
            .filter(|&(&v, _)| v != 0xFF)
            .map(|(&v, reg)| Write {
                bank: 0,
                reg,
                value: v,
            })
            .collect()
    }

    /// `0xcb4`: the song's head into the driver state, the streams at their
    /// starts. The countdown comes out one under the speed, so the first
    /// row falls on the `speed`th tick. Its last act is the start callback
    /// with the level word as it stands (`0xda3`).
    fn prime(&mut self) {
        self.ticks_per_row = self.song.speed;
        self.period = period_of(self.song.tempo);
        self.countdown = i16::from(self.song.speed) - 1;
        self.row = 0;
        for (ch, chan) in self.channels.iter_mut().enumerate() {
            chan.at = usize::from(self.song.channels[ch]);
            chan.next_row = 0;
        }
        let level = num::word((self.fade_pos >> 8) & 0xFFFF);
        self.call(Call::Start { level });
    }

    /// The tick period this sequencer currently runs at, in PIT cycles.
    pub fn period(&self) -> u32 {
        u32::from(self.period)
    }

    /// Whether the song still plays; cleared when every channel has ended
    /// and the passes are spent, or by [`Sequencer::silence`].
    pub fn playing(&self) -> bool {
        self.pass != 0
    }

    /// `SetSong` and the play entry (`0x2f9`, `0x470`): notes stopped where
    /// they still ring, the new song in, the volume full with its fraction
    /// byte cleared (`0x4a5`, `0x4b6`), the first row `speed` ticks away.
    /// The register shadow stays — the driver keeps its chip state across
    /// songs, so whatever a song shares with the last one goes out only
    /// where it differs.
    pub fn play(&mut self, song: Plx, loops: i16, out: &mut Vec<Write>) {
        self.set_song(song, out);
        self.pass = 1;
        self.last_pass = num::passes(loops);
        self.fade_step = 0;
        self.fade_pos = 0x100 << 8;
        self.prime();
    }

    /// `SetSong` and the fade-in entry (`0x2f9`, `0x34a`): notes stopped
    /// where they still ring, the new song in at volume 0 with its fraction
    /// byte cleared (`0x378`, `0x389`), and the level rising to full over
    /// `ms` milliseconds — the fade-out's arithmetic with the step's sign
    /// turned round, so the same tick that closes the carriers at level 0
    /// opens them as the level climbs, and the full volume pass runs once
    /// it reaches `0x100` (`0xb96`).
    ///
    /// The entry starts a song only when none is playing (`0x35f`); `SetSong`
    /// has just stopped any, so here it always does.
    pub fn play_fade(&mut self, song: Plx, loops: i16, ms: u16, out: &mut Vec<Write>) {
        self.set_song(song, out);
        self.pass = 1;
        self.last_pass = num::passes(loops);
        self.fade_pos = 0;
        self.prime();
        self.fade_in(ms);
    }

    /// [`Sequencer::new`] by the fade-in entry rather than the play entry:
    /// the first song a freshly installed driver is handed, starting at
    /// volume 0 and rising over `ms` milliseconds as
    /// [`Sequencer::play_fade`] has it.
    pub fn new_fade(driver: &Driver, song: Plx, loops: i16, ms: u16) -> Self {
        let mut seq = Self::build(driver, song, loops, 0, false);
        seq.fade_in(ms);
        seq
    }

    /// The fade-in entry's own part, past the start (`0x378`–`0x3d8`): the
    /// level and its fraction at 0, and the step to full.
    fn fade_in(&mut self, ms: u16) {
        self.fade_pos = 0;
        self.fade_step = match fade_ticks(ms, self.period) {
            // `0x3ca`: no length, or one of two ticks or less, is full at
            // once.
            Ticks::Instant => {
                self.fade_pos = 0x100 << 8;
                0
            }
            // `0x3c1`: a quotient past the word, or past `0x8000` ticks,
            // climbs by the smallest step there is.
            Ticks::Endless => 1,
            Ticks::Some(ticks) => 0x10000 / ticks,
        };
    }

    /// `SetSong` (`0x2f9`): a song still playing is stopped first (`0x319`).
    fn set_song(&mut self, song: Plx, out: &mut Vec<Write>) {
        if self.pass != 0 {
            self.silence(out);
        }
        self.song = song;
    }

    /// Starts the fade entry (`0x3e2`) over `ms` milliseconds, the length
    /// its caller passes: `0x4a9` PIT cycles to a millisecond, over the
    /// tick's period. Which length a host passes, and when, is the host's
    /// to cite.
    ///
    /// The entry does nothing at level 0 (`0x3fe`); otherwise it clears the
    /// fraction byte and holds the level at most at full (`0x405`–`0x412`)
    /// before the step is worked out.
    pub fn fade_out(&mut self, ms: u16) {
        let level = num::word((self.fade_pos >> 8) & 0xFFFF);
        if level == 0 {
            return;
        }
        self.fade_pos = i32::from(level.min(0x100)) << 8;
        self.fade_step = match fade_ticks(ms, self.period) {
            // `0x44d`: no length, or one of two ticks or less, is silent at
            // once — the level 0, and a step no tick climbs back from.
            Ticks::Instant => {
                self.fade_pos = 0;
                -0x8000
            }
            // `0x444`: a quotient past the word, or past `0x8000` ticks,
            // falls by the smallest step there is.
            Ticks::Endless => -1,
            Ticks::Some(ticks) => -(0x10000 / ticks),
        };
    }

    /// The stop entry (`0x4c9` → `0xde2`): from channel 8 down, the key
    /// off where it is on and the release rate of both operators opened to
    /// its fastest, so what still sounds dies away; then the stop callback
    /// (`0xe2f`), which the driver reaches only from a song that plays.
    pub fn silence(&mut self, out: &mut Vec<Write>) {
        let playing = self.pass != 0;
        for ch in (0..9).rev() {
            let b0 = 0xb0 + num::reg(ch);
            let val = self.shadow[usize::from(b0)];
            if val & 0x20 != 0 {
                self.write(out, b0, val ^ 0x20);
            }
            self.write(out, 0x80 + self.car_ops[ch], 0x0F);
            self.write(out, 0x80 + self.mod_ops[ch], 0x0F);
        }
        self.pass = 0;
        self.fade_step = 0;
        if playing {
            self.call(Call::Stop);
        }
    }

    /// One driver tick (`0xa71`): a row when the countdown runs out, the
    /// fade every tick while one runs, the volume pass after every row.
    pub fn tick(&mut self, out: &mut Vec<Write>) {
        if self.pass == 0 {
            return;
        }
        self.countdown -= 1;
        if self.countdown >= 0 {
            if self.countdown == 0 {
                self.call(Call::Step);
            }
            if self.fade_step != 0 {
                self.fade_tick(out);
            }
            return;
        }
        self.countdown += i16::from(self.ticks_per_row);

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
                self.call(Call::Stop);
                return;
            }
            // The pass counter holds at the endless mark rather than
            // wrapping past it, as the handler's compare-and-skip does.
            self.pass = self.pass.saturating_add(1);
            self.prime();
            // `0xb4b`: the fresh start's row count is stepped in this tick.
            self.call(Call::Step);
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
                self.instrument(ch, usize::from(instrument), out);
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
                    u16::from((volume & 0x3F) ^ 0x3F) | (u16::from(volume & 0xC0) << 8);
            }
            if flags & 0x04 != 0 {
                // `0x97a`: only a key that is on goes off.
                let b0 = 0xb0 + num::reg(ch);
                let val = self.shadow[usize::from(b0)];
                if val & 0x20 != 0 {
                    self.write(out, b0, val & !0x20);
                }
            }
            if flags & 0x38 != 0 {
                // `0x9a5`: the key bit is the flag's **or the register's** —
                // a note without bit 5 keeps a key that is already down —
                // and the stored frequency word carries it in its high byte.
                let key = (u16::from(self.shadow[0xb0 + ch] | flags) & 0x20) << 8;
                let word = if flags & 0x08 != 0 {
                    let Some(note) = self.u8_operand(ch) else {
                        self.channel_end(ch, out);
                        return;
                    };
                    // The note byte is twice the semitone: a byte offset
                    // into the word table (`0x9be`).
                    let freq = self
                        .notes
                        .get(usize::from(note) / 2)
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
                // nothing here moves it — so the word goes out as it stands.
                self.write(out, 0xa0 + num::reg(ch), num::lo(word));
                self.write(out, 0xb0 + num::reg(ch), num::hi(word));
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
        self.channels[ch].next_row = self.channels[ch].next_row.wrapping_add(u16::from(delay));
    }

    /// `0x868`: the stream is over; the channel keys off and goes dead.
    fn channel_end(&mut self, ch: usize, out: &mut Vec<Write>) {
        let b0 = 0xb0 + num::reg(ch);
        let val = self.shadow[usize::from(b0)] & !0x20;
        self.write(out, b0, val);
        self.channels[ch].at = 0;
    }

    /// The instrument load (`0x898`–`0x9e0`): eleven raw register values,
    /// one byte past the stored offset, written in the handler's order —
    /// the carrier's level byte is not sent, it becomes the event volume.
    fn instrument(&mut self, ch: usize, offset: usize, out: &mut Vec<Write>) {
        let Some(&r) = self
            .song
            .bytes
            .get(offset + 1..)
            .and_then(|rest| rest.first_chunk::<11>())
        else {
            return;
        };
        let (m, c) = (self.mod_ops[ch], self.car_ops[ch]);
        self.write(out, 0xc0 + num::reg(ch), r[0]);
        for (i, base) in [0x20u8, 0x40, 0x60, 0x80, 0xe0].iter().enumerate() {
            self.write(out, base + m, r[1 + i]);
        }
        self.write(out, 0x20 + c, r[6]);
        // `0x91e`: loudness inverted out of the level byte, the KSL bits
        // kept beside it.
        self.channels[ch].volume = u16::from(0x3F - (r[7] & 0x3F)) | (u16::from(r[7] & 0xC0) << 8);
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
            let loud = num::lo(self.channels[ch].volume & 0x3F);
            let v = if master >= 0x100 {
                loud
            } else {
                num::byte((u32::from(master) * u32::from(loud)) >> 8)
            };
            let reg = 0x40 + self.car_ops[ch];
            let val = (v ^ 0x3F) | num::hi(self.channels[ch].volume);
            self.write(out, reg, val);
        }
    }

    /// One tick of a running fade (`0xb77`): a fraction byte under a level
    /// word moves by the step; past 0x100 the fade is done and the full
    /// pass runs, at zero the carriers close, in between every live channel
    /// follows the level — the handler's byte arithmetic kept as it is,
    /// truncations and all.
    ///
    /// Past `0x100` the fade ends (`0xb96`): at full where the level less
    /// `0x100` reads non-negative (`0xba8`), with the full pass and the
    /// callback at `0x100`; and where it reads negative — a fade-out whose
    /// step went below zero — at 0, because `0xb96` clears the level before
    /// the sign of that comparison sends it to the store (`0xb9e` →
    /// `0xbc0`): the zero level is kept, the carriers close and the one
    /// callback goes out at 0 (`0xc26`, `0xc49`), and the rows after it run
    /// no volume pass, so the carriers stay closed.
    fn fade_tick(&mut self, out: &mut Vec<Write>) {
        self.fade_pos += self.fade_step;
        let mut level = num::word((self.fade_pos >> 8) & 0xFFFF);
        if level >= 0x100 {
            self.fade_step = 0;
            if num::signed(level.wrapping_sub(0x100)) >= 0 {
                self.call(Call::Volume { level: 0x100 });
                self.fade_pos = 0x100 << 8;
                self.volume_pass(out);
                return;
            }
            level = 0;
        }
        let ended = self.fade_step == 0;
        self.fade_pos = (i32::from(level) << 8) | (self.fade_pos & 0xFF);
        if level == 0 {
            // `0xc26`: every carrier closed, channel 8 down to 0.
            for ch in (0..9).rev() {
                let reg = 0x40 + self.car_ops[ch];
                self.write(out, reg, 0xFF);
            }
            self.call(Call::Volume { level: 0 });
            return;
        }
        for ch in (0..9).rev() {
            if self.channels[ch].at == 0 {
                continue;
            }
            let scaled = num::byte((u32::from(self.master[ch]) * u32::from(level)) >> 8);
            let loud = num::lo(self.channels[ch].volume & 0x3F);
            let v = num::byte((u32::from(scaled) * u32::from(loud)) >> 8);
            let reg = 0x40 + self.car_ops[ch];
            let val = (v | num::hi(self.channels[ch].volume)) ^ 0x3F;
            self.write(out, reg, val);
        }
        if !ended {
            self.call(Call::Volume { level });
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
        if self.shadow[usize::from(register)] == value {
            return;
        }
        self.shadow[usize::from(register)] = value;
        out.push(Write {
            bank: 0,
            reg: register,
            value,
        });
    }
}

/// How many ticks a fade of some length lasts, as both fade entries decide
/// it (`0x398`–`0x3c1`, `0x419`–`0x444`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ticks {
    /// No length, or two ticks or less: the fade is over at once.
    Instant,
    /// Longer than the step arithmetic can follow: the smallest step.
    Endless,
    /// This many.
    Some(i32),
}

/// `ms` milliseconds in ticks of `period` PIT cycles: `0x4a9` cycles to a
/// millisecond, multiplied into a double word and divided by the period —
/// where a high word at or past the period would overflow the division,
/// which the entries test for first (`cmp cx, dx` / `jae`).
fn fade_ticks(ms: u16, period: u16) -> Ticks {
    if ms == 0 {
        return Ticks::Instant;
    }
    let cycles = 0x4a9 * u32::from(ms);
    if cycles >> 16 >= u32::from(period) {
        return Ticks::Endless;
    }
    match u16::try_from(cycles / u32::from(period)) {
        Ok(0..=2) => Ticks::Instant,
        Ok(ticks @ 3..=0x8000) => Ticks::Some(i32::from(ticks)),
        _ => Ticks::Endless,
    }
}

/// `0xd29`: the tick period from a tempo word, at the default speed scale
/// of `0x100` — the driver multiplies and rounds, and nothing here moves
/// the scale.
fn period_of(tempo: u16) -> u16 {
    tempo.max(0x200)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// The standard two-operator layout: modulator and carrier slot per
    /// channel, as the shipped driver's tables hold them.
    const MOD_OPS: [u8; 9] = [0x00, 0x01, 0x02, 0x08, 0x09, 0x0a, 0x10, 0x11, 0x12];
    const CAR_OPS: [u8; 9] = [0x03, 0x04, 0x05, 0x0b, 0x0c, 0x0d, 0x13, 0x14, 0x15];

    /// A driver file of the fifteen-entry build whose defaults touch only
    /// the operators' level registers — `0x3F`, as the shipped file has
    /// them — with the standard operator layout and a rising note table.
    pub(crate) fn driver() -> Driver {
        let mut f = vec![0u8; 4480];
        f[..4].copy_from_slice(b"MUS\0");
        f[4..10].copy_from_slice(&[0x00, 0x01, 15, 0x00, 0x07, 0x00]);
        f[0x0a..0x0c].copy_from_slice(&0x1165u16.to_le_bytes());
        f[0x1165..0x1167].copy_from_slice(b"NS");
        f[0x5c4..0x6c4].fill(0xFF);
        for op in MOD_OPS.iter().chain(&CAR_OPS) {
            f[0x5c4 + 0x40 + usize::from(*op)] = 0x3F;
        }
        f[0x6c4..0x6cd].copy_from_slice(&MOD_OPS);
        f[0x6cd..0x6d6].copy_from_slice(&CAR_OPS);
        for i in 0..96u16 {
            let at = 0x6d6 + 2 * usize::from(i);
            f[at..at + 2].copy_from_slice(&(0x157 + i).to_le_bytes());
        }
        Driver::parse(&f).expect("a driver of the build that is read")
    }

    /// One channel at tempo `0x1662`, speed 1: an instrument whose carrier
    /// level byte is 0 (full loudness), keyed on a note, then held for 510
    /// rows before the stream ends.
    pub(crate) fn song() -> Plx {
        let mut b = Vec::new();
        b.extend_from_slice(b"PLX\0");
        b.push(1);
        b.extend_from_slice(&0x1662u16.to_le_bytes());
        let mut channels = [0u16; 9];
        channels[0] = 0x19 + 12;
        for c in channels {
            b.extend_from_slice(&c.to_le_bytes());
        }
        // `+0x19`: the instrument, eleven register values a byte in.
        b.extend_from_slice(&[
            0, 0x01, 0x21, 0x10, 0xf0, 0x77, 0x00, 0x21, 0x00, 0xf0, 0x77, 0x00,
        ]);
        // Instrument, note and key; then a held row; then the end.
        b.extend_from_slice(&[0x01 | 0x08 | 0x20, 0x19, 0x00, 0x30, 0xff]);
        b.extend_from_slice(&[0x80, 0xff, 0x00]);
        Plx::from_section(b).expect("a PLX section")
    }

    /// The carrier level register of channel 0, as the writes leave it.
    fn carrier_level(writes: &[Write]) -> Option<u8> {
        writes
            .iter()
            .rev()
            .find(|w| w.reg == 0x40 + CAR_OPS[0])
            .map(|w| w.value)
    }

    #[test]
    fn a_fade_lasts_its_milliseconds_in_ticks_of_the_period() {
        assert_eq!(fade_ticks(0, 0x1662), Ticks::Instant);
        // 1193 cycles over 512 are two ticks: over at once.
        assert_eq!(fade_ticks(1, 0x200), Ticks::Instant);
        // 0x4a9 × 2000 / 0x1662 = 416.4.
        assert_eq!(fade_ticks(2000, 0x1662), Ticks::Some(416));
        assert_eq!(fade_ticks(1500, 0x1662), Ticks::Some(312));
        // The high word of 0x4a9 × 65535 is 1192, past a period of 0x200.
        assert_eq!(fade_ticks(u16::MAX, 0x200), Ticks::Endless);
    }

    /// The fade-in: silent through its first ticks, rising row by row, and
    /// at full volume once the level reaches `0x100` — about
    /// `0x10000 / 157` ticks for 2000 ms at `0x1662`.
    #[test]
    fn a_fade_in_rises_from_silence_to_full() {
        let d = driver();
        let mut seq = Sequencer::new_fade(&d, song(), -1, 2000);
        let mut out = Vec::new();
        seq.tick(&mut out);
        assert_eq!(carrier_level(&out), Some(0xFF), "the first tick is silent");
        let mut levels = Vec::new();
        let mut full_at = None;
        for t in 2..=440 {
            out.clear();
            seq.tick(&mut out);
            if let Some(v) = carrier_level(&out) {
                levels.push(v);
                if v == 0x00 && full_at.is_none() {
                    full_at = Some(t);
                }
            }
        }
        assert!(
            levels.windows(2).all(|w| w[1] <= w[0]),
            "the attenuation only falls: {levels:x?}"
        );
        let full_at = full_at.expect("the fade reaches full");
        assert!(
            (410..=420).contains(&full_at),
            "full after {full_at} ticks, not 0x10000 / 157"
        );
    }

    /// The play entry by contrast: full at the first row.
    #[test]
    fn a_play_is_full_at_once() {
        let d = driver();
        let mut seq = Sequencer::new(&d, song(), -1);
        let mut out = Vec::new();
        seq.tick(&mut out);
        assert_eq!(carrier_level(&out), Some(0x00));
    }

    /// `SetSong` then the fade-in over a song that plays: the old song is
    /// stopped first, then the new one comes up silent.
    #[test]
    fn a_fade_in_over_a_playing_song_stops_it_first() {
        let d = driver();
        let mut seq = Sequencer::new(&d, song(), -1);
        let mut out = Vec::new();
        for _ in 0..10 {
            seq.tick(&mut out);
        }
        out.clear();
        seq.play_fade(song(), -1, 2000, &mut out);
        let b0 = out.iter().find(|w| w.reg == 0xb0).expect("the key off");
        assert_eq!(b0.value & 0x20, 0, "the old note is keyed off");
        out.clear();
        seq.tick(&mut out);
        assert_eq!(carrier_level(&out), Some(0xFF));
    }

    /// The fade-out entry does nothing at level 0 and starts from whole
    /// levels: its fraction byte is cleared, so two fades from one level
    /// fall alike whatever the level's fraction was.
    #[test]
    fn a_fade_out_clears_the_fraction_and_needs_a_level() {
        let d = driver();
        let mut seq = Sequencer::new(&d, song(), -1);
        seq.fade_pos = (0x80 << 8) | 0x7f;
        seq.fade_out(2000);
        assert_eq!(seq.fade_pos, 0x80 << 8);
        assert_eq!(seq.fade_step, -(0x10000 / 416));
        seq.fade_pos = 0x7f;
        seq.fade_step = 0;
        seq.fade_out(2000);
        assert_eq!(seq.fade_step, 0, "level 0 has nothing to fade");
    }

    /// The play entry clears the fraction a fade left.
    #[test]
    fn a_play_starts_from_a_whole_level() {
        let d = driver();
        let mut seq = Sequencer::new(&d, song(), -1);
        seq.fade_pos = (0x40 << 8) | 0x33;
        seq.play(song(), -1, &mut Vec::new());
        assert_eq!(seq.fade_pos, 0x100 << 8);
    }

    /// The callbacks fall where the driver makes them: the start at the
    /// priming, a step in the tick the countdown reaches 0 — the tick before
    /// the row — a start and a step in the tick a song loops, the stop when
    /// its passes are spent; and the register stream is the same with them
    /// as without.
    #[test]
    fn the_callbacks_come_where_the_driver_makes_them() {
        let d = driver();
        let mut slow = song();
        slow.speed = 3;
        let mut linked = Sequencer::new_linked(&d, slow.clone(), 1);
        let mut plain = Sequencer::new(&d, slow, 1);
        assert_eq!(linked.take_calls(), [Call::Start { level: 0x100 }]);
        let (mut a, mut b) = (Vec::new(), Vec::new());
        let mut calls = Vec::new();
        for t in 1..=3100 {
            linked.tick(&mut a);
            plain.tick(&mut b);
            calls.extend(linked.take_calls().into_iter().map(|c| (t, c)));
        }
        assert_eq!(a, b, "the register stream is untouched");
        assert!(plain.take_calls().is_empty());
        // Rows 0 to 510 at ticks 3, 6, …, 1533, the steps a tick before
        // each; row 510 ends the stream, so the loop is in tick 1533 and its
        // fresh row 0 with it; the second pass ends in tick 3063.
        assert_eq!(calls[0], (2, Call::Step));
        assert_eq!(calls[1], (5, Call::Step));
        let at = |c: Call| {
            calls
                .iter()
                .filter(|(_, k)| *k == c)
                .map(|(t, _)| *t)
                .collect::<Vec<_>>()
        };
        assert_eq!(at(Call::Start { level: 0x100 }), [1533]);
        assert_eq!(at(Call::Stop), [3063]);
        let loop_tick: Vec<_> = calls
            .iter()
            .filter(|(t, _)| *t == 1533)
            .map(|(_, c)| *c)
            .collect();
        assert_eq!(loop_tick, [Call::Start { level: 0x100 }, Call::Step]);
        assert_eq!(at(Call::Step).len(), 511 + 1 + 510);
    }

    /// A fade-out calls the volume every tick with the falling level, and its
    /// end with 0; a fade-in starts its song at level 0.
    #[test]
    fn a_fade_calls_the_volume_every_tick() {
        let d = driver();
        let mut seq = Sequencer::new_linked(&d, song(), -1);
        seq.take_calls();
        let mut out = Vec::new();
        seq.tick(&mut out);
        seq.fade_out(100);
        let mut levels = Vec::new();
        for _ in 0..40 {
            seq.tick(&mut out);
            for c in seq.take_calls() {
                if let Call::Volume { level } = c {
                    levels.push(level);
                }
            }
        }
        // Falling to 0, and once more 0 from the tick that falls past it
        // (`0xb96`–`0xb9e`); then the fade is over.
        assert!(levels.len() > 3, "{levels:x?}");
        assert!(
            levels.windows(2).all(|w| w[1] < w[0] || w[1] == 0),
            "{levels:x?}"
        );
        assert_eq!(levels[levels.len() - 2..], [0, 0]);
        let mut fade = Sequencer::new_fade_linked(&d, song(), -1, 2000);
        assert_eq!(fade.take_calls(), [Call::Start { level: 0 }]);
        fade.play_fade(song(), -1, 2000, &mut out);
        assert_eq!(fade.take_calls(), [Call::Stop, Call::Start { level: 0 }]);
    }

    /// A fade-out run past its end leaves the handler's level word at 0,
    /// and the next loop starts the digital part at that level.
    #[test]
    fn a_loop_after_a_spent_fade_starts_at_zero() {
        let d = driver();
        let mut seq = Sequencer::new_linked(&d, song(), -1);
        seq.take_calls();
        let mut out = Vec::new();
        seq.tick(&mut out);
        seq.fade_out(100);
        let mut starts = Vec::new();
        for _ in 0..600 {
            seq.tick(&mut out);
            starts.extend(
                seq.take_calls()
                    .into_iter()
                    .filter(|c| matches!(c, Call::Start { .. })),
            );
        }
        assert_eq!(starts, [Call::Start { level: 0 }], "the loop at row 510");
    }

    /// A fade-out whose step goes below zero ends at 0 (`0xb96` → `0xbc0`
    /// → `0xc26`): every carrier closed, and no row after it writes a
    /// carrier level again — the rows run no volume pass at level 0.
    #[test]
    fn a_fade_out_past_its_end_keeps_the_carriers_closed() {
        let d = driver();
        let mut seq = Sequencer::new(&d, song(), -1);
        let mut out = Vec::new();
        seq.tick(&mut out);
        seq.fade_out(100);
        let carriers: Vec<u8> = d.car_ops.iter().map(|op| 0x40 + op).collect();
        let mut closed_at = None;
        for t in 0..600 {
            out.clear();
            seq.tick(&mut out);
            if closed_at.is_none() {
                if out
                    .iter()
                    .any(|w| carriers.contains(&w.reg) && w.value == 0xFF)
                {
                    closed_at = Some(t);
                }
                continue;
            }
            assert!(
                !out.iter().any(|w| carriers.contains(&w.reg)),
                "tick {t} reopened a carrier: {out:x?}"
            );
        }
        assert!(closed_at.is_some(), "the fade never closed the carriers");
        assert!(seq.playing(), "level 0 does not stop the song");
    }
}
