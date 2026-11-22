//! The rebuilt music path under a sample clock.
//!
//! File offsets in the comments are `MUSADL.DRV`'s unless marked otherwise.

use crate::dma::DmaDriver;
use crate::driver::{Driver, PIT_HZ};
use crate::error::{Error, Result};
use crate::format::{Plx, Sample};
use crate::mixer::{Mixer, Music};
use crate::num;
use crate::output::Output;
use crate::sequencer::{Call, Sequencer};
use crate::voice::Voice;
use motionvm_opl::{Chip, Write};
use std::sync::Arc;

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
/// it writes to, the PIT arithmetic between them — and beside them the
/// digital side of `DMABLAST.DRV`: either the one [`Voice`] its direct path
/// plays, or, installed with [`Player::install_mixer`], its [`Mixer`] with
/// the effects and a song's digital voices, both added into the same frames
/// the way the card's two outputs met on its mixer.
///
/// With the mixer installed, the sequencer's callbacks reach it in the tick
/// they are made in, and the row count they answer (`[278h]`) is kept here;
/// the OPL's register stream is the same with or without it.
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
    /// The digital driver's mixer and its transfer, once installed.
    digital: Option<Output>,
    /// Rows until the next step callback (`[278h]`): what the start and
    /// step callbacks answered, counted down by the sequencer's steps. 0
    /// while the cell holds the file's stub, whose `retf` hands back the
    /// `ax` it was called with — the fast-forward flag, 0.
    rows: u16,
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
            .field("mixer", &self.digital.as_ref().map(Output::playing))
            .field("rows", &self.rows)
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
            digital: None,
            rows: 0,
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
                if let Some(digital) = &mut self.digital {
                    digital.render(frames);
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
        self.deliver();
    }

    /// The callbacks the sequencer made, to the mixer, in order.
    fn deliver(&mut self) {
        let Some(seq) = &mut self.seq else { return };
        let calls = seq.take_calls();
        let Some(digital) = &mut self.digital else {
            return;
        };
        for call in calls {
            match call {
                Call::Stop => digital.with(Mixer::music_stop),
                Call::Start { level } => self.rows = digital.with(|m| m.music_start(0, level)),
                Call::Step => {
                    self.rows = self.rows.wrapping_sub(1);
                    if self.rows == 0 {
                        self.rows = digital.with(|m| m.music_step(0));
                    }
                }
                Call::Volume { level } => {
                    digital.with(|m| m.music_volume(level));
                }
            }
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
        self.deliver();
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
        self.seq.as_ref().is_some_and(Sequencer::playing)
            || self.voice.is_some()
            || self.digital.as_ref().is_some_and(Output::playing)
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
            None if self.digital.is_some() => {
                self.seq = Some(Sequencer::new_linked(&self.driver, cue.song, cue.loops));
            }
            None => self.seq = Some(Sequencer::new(&self.driver, cue.song, cue.loops)),
        }
        for w in self.writes.drain(..) {
            self.chip.write(w);
        }
        self.clock = 0;
        self.deliver();
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
            None if self.digital.is_some() => {
                self.seq = Some(Sequencer::new_fade_linked(
                    &self.driver,
                    cue.song,
                    cue.loops,
                    ms,
                ));
            }
            None => self.seq = Some(Sequencer::new_fade(&self.driver, cue.song, cue.loops, ms)),
        }
        for w in self.writes.drain(..) {
            self.chip.write(w);
        }
        self.clock = 0;
        self.deliver();
    }

    /// The driver loaded and installed afresh, as a host's next program does
    /// when it brings its own copy up: the sequencer gone with its register
    /// shadow, and the init's register pass (`0x1062`) written again — every
    /// default the file carries, to the chip whatever it holds, because a
    /// fresh copy's shadow is the file's and says nothing of what the last
    /// program left on the card. A sample still playing is the digital
    /// driver's and stays, and so does its mixer, to which the fresh copy's
    /// songs are linked as the first copy's were — a host whose programs
    /// take the digital driver down with them says so with
    /// [`Player::uninstall_mixer`] and installs the next copy with
    /// [`Player::install_mixer`], whose announcement links it.
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
        self.deliver();
        self.seq = None;
        self.rows = 0;
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

    /// `DMABLAST.DRV` installed beside the music driver with `effects` mixer
    /// channels (entry 0, clamped to 1–8): the mixer standing, and the
    /// music driver's callbacks reaching it from here on (the driver
    /// announces itself on install, `DMABLAST.DRV` `0x0f02`–`0x0f25`, and
    /// `MUSADL.DRV` takes its table into the cell, `0xf62`). A song already
    /// playing keeps the row count the stub left it, 0.
    pub fn install_mixer(&mut self, driver_file: &[u8], effects: u16) -> Result<()> {
        let driver = DmaDriver::parse(driver_file)?;
        let mixer = Mixer::new(&driver, effects);
        self.digital = Some(Output::new(mixer, driver.time_constant, self.rate));
        if let Some(seq) = &mut self.seq {
            seq.link();
        }
        Ok(())
    }

    /// `DMABLAST.DRV` taken down: the mixer gone with whatever it played,
    /// and the music driver's cell back on its stub (`MUSADL.DRV` `0xf54`,
    /// on the driver's `bl = 0x0a` announcement), so a song started after
    /// it steps no digital part.
    pub fn uninstall_mixer(&mut self) {
        self.digital = None;
        self.rows = 0;
        if let Some(seq) = &mut self.seq {
            seq.unlink();
        }
    }

    /// The mixer's play entry (17) with mode 0: see [`Mixer::play`]. `None`
    /// without the mixer installed, too.
    pub fn play_effect(&mut self, sample: &Arc<Sample>) -> Option<usize> {
        self.digital.as_mut()?.with(|m| m.play(sample))
    }

    /// The mixer's `StopAll` (entry 18): see [`Mixer::stop_all`].
    pub fn stop_effects(&mut self) {
        if let Some(digital) = &mut self.digital {
            digital.with(Mixer::stop_all);
        }
    }

    /// The mixer's default volume (entry 22): see [`Mixer::set_volume`].
    /// Answers the old one, or `None` without the mixer.
    pub fn set_effect_volume(&mut self, volume: u16) -> Option<u16> {
        Some(self.digital.as_mut()?.with(|m| m.set_volume(volume)))
    }

    /// The song's digital part for the next start (entry 21): see
    /// [`Mixer::set_music`]. A host hands it over before it starts the song
    /// it belongs to.
    pub fn set_music(&mut self, music: Option<Arc<Music>>) {
        if let Some(digital) = &mut self.digital {
            digital.with(|m| m.set_music(music));
        }
    }

    /// The mixer, to look at.
    pub fn mixer(&self) -> Option<&Mixer> {
        self.digital.as_ref().map(Output::mixer)
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

    /// The mixer installed with no digital part for the song changes
    /// nothing: the frames are the OPL's, bit for bit.
    #[test]
    fn a_mixer_without_a_digital_song_adds_nothing() {
        let render = |mixer: bool| {
            let mut p = player();
            if mixer {
                p.install_mixer(&crate::dma::tests::file(), 6)
                    .expect("the mixer");
            }
            p.start(Cue {
                song: song(),
                loops: -1,
            });
            let mut buf = vec![0i16; 16_000];
            p.fill(&mut buf);
            buf
        };
        assert_eq!(render(true), render(false));
    }

    /// A song's digital voice sounds over the OPL from its first step, and
    /// an effect plays beside it.
    #[test]
    fn a_digital_song_and_an_effect_sound_over_the_music() {
        use crate::format::{Mdh, MdhVoice};
        let mut p = player();
        p.install_mixer(&crate::dma::tests::file(), 6)
            .expect("the mixer");
        let tone = Arc::new(Sample {
            volume: 0x100,
            period: 100,
            pcm: (0..4000)
                .map(|i| if i % 8 < 4 { 0xc0 } else { 0x40 })
                .collect(),
        });
        let mdh = Mdh {
            voices: vec![MdhVoice {
                priority: 0x40,
                sample: 0,
                levels: [0xff; 4],
            }],
            events: vec![0x00, 0xbf, 0x20, 0x00],
        };
        p.set_music(Some(Arc::new(
            Music::new(mdh, vec![Some(tone.clone())]).unwrap(),
        )));
        // At one tick a row the countdown never stands at 0, so the stream
        // would never step.
        let mut slow = song();
        slow.speed = 3;
        p.start(Cue {
            song: slow,
            loops: -1,
        });
        assert!(p.mixer().is_some_and(Mixer::running));
        assert_eq!(p.mixer().unwrap().mixed(), 8);
        // The first step is the second tick, 9.6 ms in: 200 frames on, the
        // voice's sample plays.
        let mut buf = vec![0i16; 400];
        p.fill(&mut buf);
        assert!(
            p.mixer().unwrap().channel(7).unwrap().playing,
            "the voice event came"
        );
        assert_eq!(p.play_effect(&tone), Some(1));
        p.stop_effects();
        assert!(!p.mixer().unwrap().channel(7).unwrap().playing);
    }
}
