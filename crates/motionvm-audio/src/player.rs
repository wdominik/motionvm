//! The whole chain in one place: a song goes in, samples come out.
//!
//! ```text
//! Song ─▶ Sequencer ─ MIDI ─▶ Fm ─ registers ─▶ Chip ─▶ stereo samples
//!         120 Hz              the driver        OPL3     the device's rate
//! ```
//!
//! The three parts are the three pieces of the original: the HMI sequencer
//! linked into `ENGINE.EXE`, the `fmmidi3.com` driver, and the chip. What this
//! module adds is the one thing none of them carries — a clock that says when
//! the next sequencer tick falls, counted in output samples.
//!
//! **It is written to run in an audio callback.** After the first few notes it
//! does not allocate: the message and register buffers are kept and reused, and
//! the sequencer's own queue is unlinked in place. The two things that do
//! allocate are starting a song (the caller hands over a parsed [`Song`], and
//! the previous one is dropped here) and a song's loop point, which happens
//! once every half minute or so.
//!
//! That claim rests on which events the songs contain, because the sequencer
//! copies the event it is about to dispatch and three of
//! [`Event`](motionvm_formats::m32::hmi::Event)'s variants own a `Vec`. Counted
//! rather than assumed: across the shipped songs there are 58 468 events, of
//! which **no `SysEx` and no `Branch`** — the two that would allocate per note —
//! and 134 loop markers, which are the per-loop allocation named above.
//! `the_shipped_songs_hold_no_event_that_allocates_per_note` is that count, and
//! it fails if a song ever turns up that breaks it.
//!
//! **Nothing on this path panics.** There is nobody to report to from the audio
//! thread, so the two values that could arrive out of range from a damaged song
//! are handled where they are read instead: a velocity above 127 is masked
//! (see `Fm::level`) and a track that chains events without end is stopped and
//! recorded (see [`Sequencer::runaway`](crate::Sequencer::runaway)).

use crate::chip::Chip;
use crate::error::{Error, Result};
use crate::opl::{Fm, Write};
use crate::sequencer::{Message, Sequencer};
use motionvm_formats::m32::bnk::Bank;
use motionvm_formats::m32::drv::Driver;
use motionvm_formats::m32::hmi::Song;

/// A song, the driver above it and the chip below, filling a sample buffer.
///
/// This is what the audio callback owns. Everything it needs is inside it: no
/// locks, no shared state, and one channel from the game thread carrying whole
/// songs in.
pub struct Player {
    chip: Chip,
    fm: Fm,
    seq: Option<Sequencer>,
    rate: u32,
    /// How many master ticks one sequencer tick lasts. See [`Player::master_ticks`].
    period: u32,
    /// The clock, counted in PIT cycles. Each rendered frame adds the PIT's own
    /// frequency; a tick falls due at `rate * 795 * period` and that much is
    /// taken off again. Integers throughout and nothing is thrown away, so the
    /// music cannot drift against the device however awkward its rate is.
    clock: u64,
    ticks: u64,
    messages: Vec<Message>,
    writes: Vec<Write>,
}

impl Player {
    /// The PC's timer chip, in Hz. The engine divides it (`0x8CF13`), and it
    /// writes the constant as 1 193 180 rather than the 1 193 182 the hardware
    /// really runs at — a rounding of its own, kept here because it is the one
    /// the game's arithmetic uses.
    pub const PIT_HZ: u32 = 1_193_180;
    /// What the sound layer asks its master timer for (`0x8F80D`).
    pub const MASTER_HZ: u32 = 1500;
    /// And what it gets: the divisor is truncated, so the master really runs at
    /// `1 193 180 / 795 = 1500.86 Hz`.
    pub const MASTER_DIVISOR: u32 = Self::PIT_HZ / Self::MASTER_HZ;

    /// The rate a song asks for when none is loaded. Every shipped song asks
    /// for 120, and gets [`Player::master_ticks`] of them.
    const DEFAULT_TICK_HZ: u32 = 120;

    /// How many master ticks one sequencer tick lasts — **and this is why the
    /// music does not run at the rate its own header names.**
    ///
    /// A song's deltas are counted in `1/[0xD4]` of a second, 120 in every
    /// shipped song. But the sequencer does not get its own timer: the sound
    /// layer's master is already installed at 1500 Hz, and a second timer
    /// asking for a slower rate rides on it by counting master ticks
    /// (`0x8CF23` divides one against the other in 16.16 fixed point instead of
    /// reprogramming the chip). 1500.86 / 120 is 12.507 master ticks, which is
    /// not a whole number — and the original takes **13**, so the music really
    /// runs at `1500.86 / 13 = 115.45 Hz`, four per cent slow.
    ///
    /// That the two constants are 1500 and 1 193 180 is read from the code. That
    /// the remainder is dropped rather than carried is **measured**: against the
    /// recording, 13 master ticks leaves a residual of −41…+11 ms over eight
    /// seconds where the header's 120 Hz runs 334 ms ahead and 12 ticks 711 ms
    /// behind. The fixed-point arithmetic inside the timer service that makes it
    /// 13 has not been read.
    pub fn master_ticks(tick_hz: u32) -> u32 {
        let master_num = Self::PIT_HZ;
        let master_den = Self::MASTER_DIVISOR;
        // ceil(master / tick_hz) with the master as a fraction, so nothing is
        // rounded twice.
        (master_num).div_ceil(master_den * tick_hz.max(1)).max(1)
    }

    /// The rate the sequencer is really clocked at, in Hz.
    pub fn tick_hz(&self) -> f64 {
        Self::PIT_HZ as f64 / Self::MASTER_DIVISOR as f64 / self.period as f64
    }

    /// Builds the chain. `driver` is `HMIMDRV.386`'s `0xA009` entry, the two
    /// banks are `MELODIC.BNK` and `DRUM.BNK` — the same three things
    /// `ENGINE.EXE` hands its MIDI layer.
    pub fn new(rate: u32, driver: &Driver, melodic: &Bank, drums: &Bank) -> Result<Self> {
        // Rejected here rather than defended against below: with a rate of zero
        // the tick period works out to zero, `frames_to_tick` answers zero
        // forever, and the mixing loop makes no progress. That is a hang on the
        // audio thread, which is worse than anything it could be mistaken for.
        if rate == 0 {
            return Err(Error::ZeroRate);
        }
        let mut fm = Fm::new(driver, melodic, drums)?;
        let mut chip = Chip::new(rate);
        // `Fm::new` has already produced the switch-on sequence; it only has to
        // reach the chip.
        for w in fm.take() {
            chip.write(w);
        }
        Ok(Self {
            chip,
            fm,
            seq: None,
            rate,
            period: Self::master_ticks(Self::DEFAULT_TICK_HZ),
            clock: 0,
            ticks: 0,
            messages: Vec::with_capacity(64),
            writes: Vec::with_capacity(512),
        })
    }

    /// The sample rate it was made with.
    pub fn rate(&self) -> u32 {
        self.rate
    }

    /// Whether a song is loaded and has not run out.
    pub fn playing(&self) -> bool {
        self.seq.is_some()
    }

    /// How many sequencer ticks have been run since the current song started.
    ///
    /// Only the clock is observable from outside otherwise, and it is the one
    /// part of this module that is neither the sequencer's nor the driver's nor
    /// the chip's — so it is the one part that needs its own check.
    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    /// Starts a song, stopping whatever was playing first.
    ///
    /// The game never overlaps two tunes — `INCLLOC` stops the old one before
    /// the location macro starts the new one — but doing it here as well means
    /// a stray `STARTTUNE` cannot leave notes hanging.
    pub fn start(&mut self, song: Song) {
        self.stop();
        self.period = Self::master_ticks(song.tick_hz as u32);
        self.clock = 0;
        self.ticks = 0;
        self.seq = Some(Sequencer::new(song));
    }

    /// Stops the song and silences what it left sounding, the way the
    /// original's stop path does — see [`Sequencer::all_notes_off`].
    pub fn stop(&mut self) {
        let Some(seq) = &mut self.seq else { return };
        self.messages.clear();
        seq.all_notes_off(&mut self.messages);
        for m in self.messages.drain(..) {
            self.fm.send(m);
        }
        self.fm.take_into(&mut self.writes);
        for w in self.writes.drain(..) {
            self.chip.write(w);
        }
        self.seq = None;
        self.period = Self::master_ticks(Self::DEFAULT_TICK_HZ);
    }

    /// Hands one MIDI message straight to the driver, past the sequencer.
    ///
    /// This is the driver's own way in — ordinal 2 of `fmmidi3.com` takes
    /// exactly this and nothing else. Nothing in the game uses it, because the
    /// game only ever plays songs; it is here so a single note can be sounded
    /// without inventing a song around it.
    pub fn send(&mut self, message: Message) {
        self.fm.send(message);
        self.fm.take_into(&mut self.writes);
        for w in self.writes.drain(..) {
            self.chip.write(w);
        }
    }

    /// Fills `out` with interleaved stereo frames.
    ///
    /// An odd length is a caller's mistake; the last lone sample is left alone
    /// rather than turning the two channels around for everything after it.
    pub fn fill(&mut self, out: &mut [i16]) {
        let frames = out.len() / 2;
        let mut done = 0;
        while done < frames {
            let step = self.frames_to_tick().min(frames - done);
            if step > 0 {
                self.chip.render(&mut out[done * 2..(done + step) * 2]);
                done += step;
                self.clock += step as u64 * Self::PIT_HZ as u64;
            }
            let period = self.tick_period();
            if self.clock >= period {
                self.clock -= period;
                self.tick();
            }
        }
    }

    /// One sequencer tick, in PIT cycles times the output rate.
    ///
    /// A tick lasts `period` master ticks, a master tick lasts `795` PIT
    /// cycles, and a rendered frame is worth `PIT_HZ / rate` of them — so
    /// multiplying through by `rate` keeps every side a whole number.
    fn tick_period(&self) -> u64 {
        self.rate as u64 * Self::MASTER_DIVISOR as u64 * self.period as u64
    }

    /// How many frames may be rendered before the next tick falls due.
    fn frames_to_tick(&self) -> usize {
        let left = self.tick_period().saturating_sub(self.clock);
        // Round up: the tick belongs to the frame that reaches the mark, not to
        // the one before it.
        left.div_ceil(Self::PIT_HZ as u64) as usize
    }

    /// One sequencer tick, and whatever registers it turns into.
    fn tick(&mut self) {
        let Some(seq) = &mut self.seq else { return };
        self.ticks += 1;
        self.messages.clear();
        seq.tick(&mut self.messages);
        for m in self.messages.drain(..) {
            self.fm.send(m);
        }
        self.fm.take_into(&mut self.writes);
        for w in self.writes.drain(..) {
            self.chip.write(w);
        }
    }
}
