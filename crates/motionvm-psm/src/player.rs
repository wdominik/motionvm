//! The rebuilt music path under a sample clock.
//!
//! File offsets in the comments are `MUSADL.DRV`'s unless marked otherwise.

use crate::driver::{Driver, PIT_HZ};
use crate::error::{Error, Result};
use crate::format::{Plx, Sample};
use crate::num;
use crate::sequencer::Sequencer;
use crate::voice::Voice;
use motionvm_opl::{Chip, Write};

/// A section and how many times to play it — what the driver's `SetSong`
/// and `Play` are handed. The count is read unsigned, so `-1` plays
/// endlessly.
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
/// remainder carries over — [`motionvm_opl::clock`]'s integer scheme, so the
/// music cannot drift against the device whatever its rate is.
pub struct Player {
    chip: Chip,
    rate: u32,
    driver: Driver,
    seq: Option<Sequencer>,
    writes: Vec<Write>,
    /// The clock, in PIT cycles times the output rate.
    clock: u64,
    /// Frames left until a pending stop lands — see [`Player::stop_after`].
    stop_in: Option<u64>,
    /// The sample playing, if one is: the driver installed with one channel
    /// drops the sample on it when the next one is played.
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
            let step = motionvm_opl::clock::frames_to_tick(self.clock, self.tick_period(), PIT_HZ)
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

impl Player {
    /// The sample rate it was made with.
    pub fn rate(&self) -> u32 {
        self.rate
    }

    /// Whether anything still sounds. A fade-out still counts as playing;
    /// the silence after it does not. A sample still running counts too.
    pub fn playing(&self) -> bool {
        self.seq.as_ref().is_some_and(Sequencer::playing) || self.voice.is_some()
    }

    /// The driver's `SetSong` then `Play`. The first song builds the
    /// sequencer; every later one goes through [`Sequencer::play`], keeping
    /// the register shadow, exactly as the resident driver keeps its state
    /// from tune to tune. The clock starts over — `Play` re-registers the host
    /// timer (`0xcb4`).
    pub fn start(&mut self, cue: Cue) {
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

    /// The driver's `SetSong` then its fade-in entry: the song starts at
    /// volume 0 and rises to full over `ms` milliseconds
    /// ([`Sequencer::play_fade`]). The first song builds the sequencer
    /// silent and fades it in from there; the clock starts over as it does
    /// for [`Player::start`], the same `0xcb4` re-registering the timer.
    pub fn start_fade(&mut self, cue: Cue, ms: u16) {
        self.stop_in = None;
        self.writes.clear();
        match &mut self.seq {
            Some(seq) => seq.play_fade(cue.song, cue.loops, ms, &mut self.writes),
            None => self.seq = Some(Sequencer::new_fade(&self.driver, cue.song, cue.loops, ms)),
        }
        for w in self.writes.drain(..) {
            self.chip.write(w);
        }
        self.clock = 0;
    }

    /// The driver loaded and installed afresh, as a host's next program does
    /// when it brings its own copy up: the sequencer gone with its register
    /// shadow, and the init's register pass (`0x1062`) written again — every
    /// default the file carries, to the chip whatever it holds, because a
    /// fresh copy's shadow is the file's and says nothing of what the last
    /// program left on the card. A sample still playing is the digital
    /// driver's and stays.
    ///
    /// A stop still pending lands first: the old copy goes down after
    /// what its host last asked of it.
    pub fn reinstall(&mut self) {
        if self.stop_in.take().is_some()
            && let Some(seq) = &mut self.seq
        {
            seq.silence(&mut self.writes);
            for w in self.writes.drain(..) {
                self.chip.write(w);
            }
        }
        self.seq = None;
        for w in Sequencer::install_writes(&self.driver) {
            self.chip.write(w);
        }
        self.clock = 0;
    }

    /// Begins the driver's fade-out over `ms` milliseconds
    /// ([`Sequencer::fade_out`]) if a song is playing, and leaves it sounding
    /// while the level falls.
    pub fn fade_out(&mut self, ms: u16) {
        let Some(seq) = &mut self.seq else { return };
        if !seq.playing() {
            return;
        }
        seq.fade_out(ms);
    }

    /// Lands the driver's hard stop (`0xde2`: key-offs and both operators'
    /// release rate opened to `0x0F`) once `frames` more have been rendered,
    /// if a song is playing — so what still sounds dies away from wherever
    /// its level stands then. How long a host waits before it stops is the
    /// host's.
    pub fn stop_after(&mut self, frames: u64) {
        let Some(seq) = &self.seq else { return };
        if !seq.playing() {
            return;
        }
        self.stop_in = Some(frames);
    }

    /// Plays `sample` on the digital driver's one channel: whatever was
    /// playing on it stops where it stands.
    pub fn play_sample(&mut self, sample: Sample) {
        self.voice = Some(Voice::new(sample, self.rate));
    }

    /// The digital driver's `StopAll` on its one channel.
    pub fn stop_sample(&mut self) {
        self.voice = None;
    }

    /// Renders the next frames, interleaved stereo, running the sequencer as
    /// their time passes.
    pub fn fill(&mut self, out: &mut [i16]) {
        self.render(out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequencer::tests::{driver, song};

    /// A player over the test driver: the defaults of
    /// `sequencer::tests::driver` written back out as a file the player
    /// reads.
    fn player() -> Player {
        let d = driver();
        let mut f = vec![0u8; 4480];
        f[..4].copy_from_slice(b"MUS\0");
        f[4..10].copy_from_slice(&[0x00, 0x01, 15, 0x00, 0x07, 0x00]);
        f[0x0a..0x0c].copy_from_slice(&0x1165u16.to_le_bytes());
        f[0x1165..0x1167].copy_from_slice(b"NS");
        f[0x5c4..0x6c4].copy_from_slice(&d.defaults);
        f[0x6c4..0x6cd].copy_from_slice(&d.mod_ops);
        f[0x6cd..0x6d6].copy_from_slice(&d.car_ops);
        for (i, n) in d.notes.iter().enumerate() {
            f[0x6d6 + 2 * i..0x6d8 + 2 * i].copy_from_slice(&n.to_le_bytes());
        }
        Player::new(8_000, &f).expect("the driver comes up")
    }

    fn peak(p: &mut Player, frames: usize) -> u16 {
        let mut buf = vec![0i16; frames * 2];
        p.fill(&mut buf);
        buf.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0)
    }

    /// A fade-in is quieter in its first half second than a play.
    #[test]
    fn a_fade_in_starts_quiet() {
        let mut played = player();
        played.start(Cue {
            song: song(),
            loops: -1,
        });
        let mut faded = player();
        faded.start_fade(
            Cue {
                song: song(),
                loops: -1,
            },
            2000,
        );
        let (loud, quiet) = (peak(&mut played, 4000), peak(&mut faded, 4000));
        assert!(loud > 0 && quiet < loud / 2, "play {loud}, fade-in {quiet}");
    }

    /// A stop asked for and a reinstall in one breath: the stop lands
    /// before the copy goes, keying the note off with its release opened,
    /// so a second later it is fainter than under a reinstall alone, which
    /// leaves it keyed under the defaults' closed levels.
    #[test]
    fn a_reinstall_lands_a_pending_stop_first() {
        let after = |stop: bool| {
            let mut p = player();
            p.start(Cue {
                song: song(),
                loops: -1,
            });
            assert!(peak(&mut p, 4000) > 0);
            if stop {
                p.stop_after(0);
            }
            p.reinstall();
            assert!(!p.playing());
            peak(&mut p, 8000);
            peak(&mut p, 4000)
        };
        let (stopped, left) = (after(true), after(false));
        assert!(stopped < left, "stopped {stopped}, left keyed {left}");
    }
}
