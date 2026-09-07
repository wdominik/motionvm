//! The rebuilt music path under a sample clock.
//!
//! File offsets in the comments are `MUSADL.DRV`'s unless marked otherwise.

use crate::chip::{Chip, Write};
use crate::error::{Error, Result};
use crate::m16::driver::{Driver, PIT_HZ};
use crate::m16::sequencer::Sequencer;
use crate::m16::voice::Voice;
use crate::num;
use motionvm_motion_formats::m16::psm::{Plx, Sample};

/// A section and how many times to play it, which is what `STARTTUNE` asks
/// for: `-1` at every call site in the games, which the driver reads unsigned
/// and takes as endless.
#[derive(Debug)]
pub struct Cue {
    /// The section to play.
    pub song: Plx,
    /// How many times, `-1` for endlessly.
    pub loops: i16,
}

/// The rebuilt music path under a sample clock: the [`Sequencer`], the OPL2
/// it writes to, the PIT arithmetic between them — and beside them the one
/// digital [`Voice`] the direct-DMA path of `DMABLAST.DRV` plays, mixed into
/// the same frames the way the card's two outputs met on its mixer.
///
/// The original's tick is the sound host's timer service: `MUSADL.DRV` asks
/// for its tempo word as a period in PIT cycles and the host programs timer
/// channel 0 with it. Here a rendered frame is worth `PIT_HZ` cycles times
/// the output rate, a tick falls due every `rate × period` of those, and the
/// remainder carries over — the same integer scheme as [`crate::m32::Player`], so
/// the music cannot drift against the device whatever its rate is.
pub struct Player {
    chip: Chip,
    rate: u32,
    driver: Driver,
    seq: Option<Sequencer>,
    writes: Vec<Write>,
    /// The clock, in PIT cycles times the output rate.
    clock: u64,
    /// Frames left until a pending stop lands — see [`crate::Player::stop`].
    stop_in: Option<u64>,
    /// The sample playing, if one is: the driver as the games install it has
    /// one channel, and `PLAYSAMPLE` stops it before it plays the next.
    voice: Option<Voice>,
}

impl std::fmt::Debug for Player {
    /// The clock and what is playing; the driver's tables and the chip are
    /// the original's bytes and a third party's core.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Player")
            .field("rate", &self.rate)
            .field("playing", &self.seq.is_some())
            .field("clock", &self.clock)
            .field("stop_in", &self.stop_in)
            .field("voice", &self.voice.is_some())
            .finish_non_exhaustive()
    }
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
        let mut chip = Chip::new(rate);
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
            voice: None,
        })
    }

    /// Fills `out` with interleaved stereo frames.
    ///
    /// An odd length is a caller's mistake; the last lone sample is left
    /// alone rather than turning the two channels around after it.
    fn render(&mut self, out: &mut [i16]) {
        let frames = out.len() / 2;
        let mut done = 0;
        while done < frames {
            let step = crate::clock::frames_to_tick(self.clock, self.tick_period(), PIT_HZ)
                .min(frames - done);
            if step > 0 {
                let frames = &mut out[done * 2..(done + step) * 2];
                self.chip.render(frames);
                // The sample over the music, frame for frame — the DAC and
                // the OPL shared the card's output.
                if let Some(voice) = &mut self.voice
                    && !voice.mix(frames)
                {
                    self.voice = None;
                }
                done += step;
                self.clock += num::frames(step) * u64::from(PIT_HZ);
                self.count_down_stop(num::frames(step));
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

impl crate::Player for Player {
    type Song = Cue;
    type Sample = Sample;

    fn rate(&self) -> u32 {
        self.rate
    }

    /// A fade-out still counts as playing; the silence after it does not. A
    /// sample still running counts too.
    fn playing(&self) -> bool {
        self.seq.as_ref().is_some_and(Sequencer::playing) || self.voice.is_some()
    }

    /// `PLAYSAMPLE`'s hand-over (`STERN.EXE` `15e5:0425`), after the
    /// manager's `StopAll`: the driver's one channel takes the new sample and
    /// whatever was playing on it stops where it stands.
    fn sample(&mut self, sample: Sample) {
        self.voice = Some(Voice::new(sample, self.rate));
    }

    /// The driver's `StopAll` on its one channel. No 16-bit word reaches it
    /// for a sample alone — `PLAYSAMPLE` replaces, and nothing stops — so
    /// this answers the trait and no game.
    fn stop_sample(&mut self, _handle: i32) {
        self.voice = None;
    }

    /// `MUSADL.DRV` has no master volume among the entries the games
    /// install, and no 16-bit word asks for one: the call changes nothing.
    fn music_volume(&mut self, _volume: u16) {}

    /// `STARTTUNE`'s path, `SetSong` then `Play`. The first song builds the
    /// sequencer; every later one goes through [`Sequencer::play`], keeping
    /// the register shadow, exactly as the resident driver keeps its state
    /// from tune to tune. The clock starts over — `Play` re-registers the host
    /// timer (`0xcb4`).
    fn start(&mut self, cue: Cue) {
        self.stop_in = None;
        self.writes.clear();
        match &mut self.seq {
            Some(seq) => seq.play(cue.song, cue.loops, &mut self.writes),
            None => self.seq = Some(Sequencer::new(&self.driver, cue.song, cue.loops)),
        }
        for w in self.writes.drain(..) {
            self.chip.write(w);
        }
        self.clock = 0;
    }

    /// The way `ENDTUNE` does it — the games' one stop site (`ENVIRO.EXE`
    /// `1696:02fd`): the 2000 ms fade-out begins at once and the hard stop
    /// (`0xde2`: key-offs and both operators' release rate opened to `0x0F`)
    /// lands 500 ms in, so what still sounds dies away under the fade's last
    /// level.
    fn stop(&mut self) {
        let Some(seq) = &mut self.seq else { return };
        if !seq.playing() {
            return;
        }
        seq.fade_out();
        self.stop_in = Some(u64::from(self.rate) / 2);
    }

    /// `PLAYSAMPLE`'s way (`STERN.EXE` `15e5:035d`): no fade — the tune
    /// plays on at full volume through the half second the script waits —
    /// and the same hard stop lands when it is up.
    fn cut(&mut self) {
        let Some(seq) = &mut self.seq else { return };
        if !seq.playing() {
            return;
        }
        self.stop_in = Some(u64::from(self.rate) / 2);
    }

    fn fill(&mut self, out: &mut [i16]) {
        self.render(out);
    }
}
