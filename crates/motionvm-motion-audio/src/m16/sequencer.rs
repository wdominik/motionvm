//! The driver's tick: event streams in, OPL register writes out.
//!
//! The 16-bit game's whole music player lives in `MUSADL.DRV` — 4480 bytes
//! behind a `MUS\0` header — and `ENVIRO.EXE` holds only a loader, a far-jump
//! table and a timer. This is that driver's `0xa71`, read at the instruction
//! level and rebuilt.
//!
//! File offsets in the comments are `MUSADL.DRV`'s unless marked otherwise.
//! The registers go through a shadow of all 256 (`0x5c4`, written through
//! `0x7df`/`0x7ec`): a value the shadow already holds is not sent again, so
//! the stream carries **changes only** — which is what makes it comparable,
//! byte for byte, against a DRO capture of the original.

use crate::chip::Write;
use crate::m16::driver::Driver;
use crate::num;
use motionvm_motion_formats::m16::psm::Plx;

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
            last_pass: num::passes(loops),
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
    /// row falls on the `speed`th tick.
    fn prime(&mut self) {
        self.ticks_per_row = self.song.speed;
        self.period = period_of(self.song.tempo);
        self.countdown = i16::from(self.song.speed) - 1;
        self.row = 0;
        for (ch, chan) in self.channels.iter_mut().enumerate() {
            chan.at = usize::from(self.song.channels[ch]);
            chan.next_row = 0;
        }
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
        self.last_pass = num::passes(loops);
        self.fade_step = 0;
        self.fade_pos = (0x100 << 8) | (self.fade_pos & 0xFF);
        self.prime();
    }
    /// Starts the 2000 ms fade `ENDTUNE` starts (`0x3e2` with `0x7d0`).
    pub fn fade_out(&mut self) {
        let ticks = (0x4a9 * 0x7d0) / i32::from(self.period);
        self.fade_step = -(0x10000 / ticks.max(1));
    }

    /// The stop entry (`0x4c9` → `0xde2`): from channel 8 down, the key
    /// off where it is on and the release rate of both operators opened to
    /// its fastest, so what still sounds dies away.
    pub fn silence(&mut self, out: &mut Vec<Write>) {
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
                // nothing in the game moves it — so the word goes out as it
                // stands.
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
    fn fade_tick(&mut self, out: &mut Vec<Write>) {
        self.fade_pos += self.fade_step;
        let level = num::word((self.fade_pos >> 8) & 0xFFFF);
        if level >= 0x100 {
            self.fade_step = 0;
            if num::signed(level) >= 0 {
                self.fade_pos = 0x100 << 8;
                self.volume_pass(out);
                return;
            }
            // A step that overshot below zero leaves the wrapped level
            // standing, as the handler does (`0xb9e`); the game stops a
            // fade-out long before this could be reached.
        }
        self.fade_pos = (i32::from(level) << 8) | (self.fade_pos & 0xFF);
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
            let scaled = num::byte((u32::from(self.master[ch]) * u32::from(level)) >> 8);
            let loud = num::lo(self.channels[ch].volume & 0x3F);
            let v = num::byte((u32::from(scaled) * u32::from(loud)) >> 8);
            let reg = 0x40 + self.car_ops[ch];
            let val = (v | num::hi(self.channels[ch].volume)) ^ 0x3F;
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

/// `0xd29`: the tick period from a tempo word, at the default speed scale
/// of `0x100` — the driver multiplies and rounds, and the game never moves
/// the scale.
fn period_of(tempo: u16) -> u16 {
    tempo.max(0x200)
}
