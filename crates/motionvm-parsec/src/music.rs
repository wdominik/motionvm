//! The family's sound, joined up for a platform: a sink for the game, a
//! source for the audio thread, and the channel between them.
//!
//! ```text
//! game thread                            audio thread
//!   Runtime ─ MusicSink ─── mpsc ────▶  Player::fill
//!             Plx, Sample, Mdh here      nothing but arithmetic
//! ```
//!
//! The game asks its drivers for something in the drivers' own entries: a
//! song arrives as the bare `PLX` section the program hands the Ad Lib
//! driver, a song's digital part as the module the DAC path assembles, an
//! effect as its `SM8` block. Reading them means checking and allocating,
//! which belongs on the game thread, so the sink does it and sends the
//! finished pieces across. The drivers are the game's own `MUSADL.DRV` and
//! `DMABLAST.DRV`, both played by
//! `motionvm_psm`'s [`Player`] on the audio thread. Every sample and every
//! song's digital part is read once and kept by the sink for the session,
//! so a mixer channel letting go of one never frees it on the audio thread.
//!
//! **Time.** The game asks for an effect on its own thread, a step at a
//! time, and the audio thread takes what was asked at the start of its next
//! buffer, before it renders a frame of it — the music's calls the same
//! way. From there the mixer's own clock decides when it is heard: a play
//! into a standing mixer is heard from the first frame of that buffer, a
//! play into a running one from the first byte of the block after the
//! next, 128 to 256 DMA bytes (6.1 to 12.3 ms) on, as the driver's double
//! buffer has it. Nothing comes back from the audio thread: what the game
//! does never depends on what the mixer did.
//!
//! The player starts a tick of the music driver every `period` PIT cycles,
//! its period following the song's own tempo events. The original's timer
//! module runs the PIT on the shortest of its base period, `952h` cycles
//! (500 a second, `0x5B29`), and the periods its clients ask for, and
//! counts a client's period down by each interrupt's (`0x1A115`). So a
//! driver tick there lands on the first interrupt at or after it — on the
//! base period, less than 2386 PIT cycles (2 ms) later than here. The
//! writes and their order are the same.

use motionvm_parsec_engine::MusicSink;
use motionvm_playable::{AudioSource, Result};
use motionvm_psm::{Cue, Mdh, Player, Plx, Sample};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

/// What crosses from the game thread to the audio one. A song is boxed
/// because it is far larger than the rest.
enum Command {
    Play(Box<Cue>),
    PlayFade(Box<Cue>, u16),
    FadeOut(u16),
    Stop,
    Reinstall,
    Install(u16),
    EffectVolume(u16),
    Effect(Arc<Sample>, u16),
    StopEffects,
    Digital(Option<Arc<motionvm_psm::Music>>),
}

/// The audio thread's side: drain the command channel, then render.
struct Backend {
    player: Player,
    /// `DMABLAST.DRV`, when the game has it: what an install reads.
    dac: Option<Vec<u8>>,
    rx: Receiver<Command>,
}

impl AudioSource for Backend {
    fn fill(&mut self, out: &mut [i16]) {
        while let Ok(command) = self.rx.try_recv() {
            match command {
                Command::Play(cue) => self.player.start(*cue),
                Command::PlayFade(cue, ms) => self.player.start_fade(*cue, ms),
                Command::FadeOut(ms) => self.player.fade_out(ms),
                // `Stop` lands at once: the hard stop's key-offs and release
                // rates before the next tick.
                Command::Stop => self.player.stop_after(0),
                // Both drivers go with the program; the next one's init
                // installs the digital driver again.
                Command::Reinstall => {
                    self.player.reinstall();
                    self.player.uninstall_mixer();
                }
                Command::Install(effects) => {
                    // The file was checked on the game thread when the sink
                    // was made, and reading its tables allocates nothing.
                    if let Some(file) = &self.dac {
                        let _ = self.player.install_mixer(file, effects);
                    }
                }
                Command::EffectVolume(v) => {
                    self.player.set_effect_volume(v);
                }
                Command::Effect(sample, repeats) => {
                    self.player.play_effect(&sample, repeats);
                }
                Command::StopEffects => self.player.stop_effects(),
                Command::Digital(music) => self.player.set_music(music),
            }
        }
        self.player.fill(out);
    }
}

/// What the engine talks to. A song that will not read as a `PLX`, a
/// sample that will not read as an `SM8`, a module whose digital part will
/// not read are not played: the reader hands over only what carries the
/// tag, so it would mean a damaged library.
struct Music {
    tx: Sender<Command>,
    /// Every effect read so far, by its row of table A.
    effects: HashMap<u16, Arc<Sample>>,
    /// Every song's digital part read so far, by module.
    songs: HashMap<u16, Option<Arc<motionvm_psm::Music>>>,
}

impl Music {
    /// The cue for a song, or nothing for one that is not a `PLX`.
    fn cue(song: &[u8], loops: i16) -> Option<Box<Cue>> {
        let song = Plx::from_section(song.to_vec()).ok()?;
        Some(Box::new(Cue { song, loops }))
    }
}

/// A module's digital part, as `DMABLAST.DRV`'s entry 21 is handed it: the
/// `MDH` chunk at `+38h` (`0x606C`) and the eight sample words at `+14h`
/// (`0x6079`), each the offset the loader appended that sample at, 0 for
/// none (`0x142B1`–`0x1434C`).
fn digital_part(module: &[u8]) -> Option<motionvm_psm::Music> {
    let mdh = Mdh::parse(module.get(0x38..)?).ok()?;
    let mut samples = Vec::with_capacity(8);
    for slot in 0..8 {
        let at = 0x14 + 4 * slot;
        let word = u32::from_le_bytes(module.get(at..at + 4)?.try_into().ok()?);
        if word == 0 {
            samples.push(None);
            continue;
        }
        let block = module.get(usize::try_from(word).ok()?..)?;
        samples.push(Some(Arc::new(Sample::parse(block).ok()?)));
    }
    motionvm_psm::Music::new(mdh, samples).ok()
}

impl MusicSink for Music {
    fn play(&mut self, song: &[u8], loops: i16) {
        if let Some(cue) = Self::cue(song, loops) {
            let _ = self.tx.send(Command::Play(cue));
        }
    }

    fn play_fade(&mut self, song: &[u8], loops: i16, ms: u16) {
        if let Some(cue) = Self::cue(song, loops) {
            let _ = self.tx.send(Command::PlayFade(cue, ms));
        }
    }

    fn fade_out(&mut self, ms: u16) {
        let _ = self.tx.send(Command::FadeOut(ms));
    }

    fn stop(&mut self) {
        let _ = self.tx.send(Command::Stop);
    }

    fn reinstall(&mut self) {
        let _ = self.tx.send(Command::Reinstall);
    }

    fn install_digital(&mut self, effects: u16) {
        let _ = self.tx.send(Command::Install(effects));
    }

    fn effect_volume(&mut self, volume: u16) {
        let _ = self.tx.send(Command::EffectVolume(volume));
    }

    fn play_effect(&mut self, row: u16, sample: &[u8], repeats: u16) {
        let sample = match self.effects.get(&row) {
            Some(s) => Arc::clone(s),
            None => {
                let Ok(s) = Sample::parse(sample) else { return };
                let s = Arc::new(s);
                self.effects.insert(row, Arc::clone(&s));
                s
            }
        };
        let _ = self.tx.send(Command::Effect(sample, repeats));
    }

    fn stop_effects(&mut self) {
        let _ = self.tx.send(Command::StopEffects);
    }

    fn set_digital(&mut self, module: Option<(u16, &[u8])>) {
        let music = module.and_then(|(piece, bytes)| {
            self.songs
                .entry(piece)
                .or_insert_with(|| digital_part(bytes).map(Arc::new))
                .clone()
        });
        let _ = self.tx.send(Command::Digital(music));
    }
}

/// The sound at the device rate the platform learned, through the driver
/// files the game ships — the Ad Lib driver and the digital one: a source
/// for the audio thread and a sink for the game, already joined. The digital
/// driver is checked here, so a file that will not read turns sound off with
/// its reason rather than failing later on the audio thread; the game's init
/// installs it.
pub(crate) fn open_music(
    driver: &[u8],
    dac: Option<Vec<u8>>,
    rate: u32,
) -> Result<(Box<dyn AudioSource>, Box<dyn MusicSink>)> {
    let player =
        Player::new(rate, driver).map_err(|e| format!("the Ad Lib driver did not come up: {e}"))?;
    if let Some(file) = &dac {
        motionvm_psm::DmaDriver::parse(file)
            .map_err(|e| format!("the digital driver did not come up: {e}"))?;
    }
    let (tx, rx) = channel();
    Ok((
        Box::new(Backend { player, dac, rx }),
        Box::new(Music {
            tx,
            effects: HashMap::new(),
            songs: HashMap::new(),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use motionvm_parsec_formats::detect::resource_dir;
    use motionvm_parsec_formats::{P2l, find_ci, p2l};

    /// The game's two drivers and its library, or `None` without the game.
    fn files() -> Option<(Vec<u8>, Vec<u8>, P2l)> {
        let dir = motionvm_parsec_testutil::gamedata_ds1()?;
        let res = resource_dir(&dir);
        let read = |n: &str| std::fs::read(find_ci(&res, n).expect(n)).expect(n);
        let drv = read("PSMDRV.DAT");
        Some((
            p2l::driver(&drv, p2l::AD_LIB).expect("MUSADL.DRV"),
            p2l::driver(&drv, p2l::DMABLAST).expect("DMABLAST.DRV"),
            P2l::parse(read("PSMP2L.DAT")).expect("PSMP2L.DAT"),
        ))
    }

    /// Renders `frames` frames and answers the loudest sample.
    fn peak(source: &mut dyn AudioSource, frames: usize) -> u16 {
        let mut buf = vec![0i16; 2 * frames];
        source.fill(&mut buf);
        buf.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0)
    }

    /// An effect played through the sink sounds once the game's init has
    /// installed the digital driver, with no song playing; without the
    /// digital driver's file it is silent.
    #[test]
    fn an_effect_sounds_through_the_installed_mixer() {
        let Some((musadl, dmablast, library)) = files() else {
            eprintln!("skipping: no Dunkle Schatten gamedata directory");
            return;
        };
        let effect = library.sample(2).expect("effect 2");
        let render = |dac: Option<Vec<u8>>, play: bool| {
            let (mut source, mut sink) = open_music(&musadl, dac, 44_100).expect("the sound");
            sink.install_digital(6);
            sink.effect_volume(0x50);
            sink.effect_volume(0x3C);
            if play {
                sink.play_effect(2, &effect, 0);
            }
            peak(source.as_mut(), 4410)
        };
        let quiet = render(Some(dmablast.clone()), false);
        assert!(
            render(Some(dmablast), true) > quiet + 1000,
            "the effect is heard"
        );
        assert_eq!(render(None, true), quiet, "no mixer, no effect");
    }

    /// A song's digital part, handed over before its play, adds its voices
    /// to the OPL's frames: the render differs from the same song without
    /// it.
    #[test]
    fn a_song_s_digital_part_is_heard_over_its_fm() {
        let Some((musadl, dmablast, library)) = files() else {
            eprintln!("skipping: no Dunkle Schatten gamedata directory");
            return;
        };
        let render = |digital: bool| {
            let (mut source, mut sink) =
                open_music(&musadl, Some(dmablast.clone()), 44_100).expect("the sound");
            sink.install_digital(6);
            if digital {
                sink.set_digital(Some((1, &library.module(1).expect("module 1"))));
            }
            sink.play(&library.song(1).expect("song 1"), -1);
            let mut buf = vec![0i16; 2 * 44_100];
            source.fill(&mut buf);
            buf
        };
        assert_ne!(render(true), render(false));
    }

    /// Bi-Fi 2's sound under the closing band (row 0, 10 629 bytes at a
    /// period of 157: a pass of 1.4 s), played with the four passes more
    /// the program asks for, still sounds where a single pass is over.
    #[test]
    fn an_effect_plays_the_passes_it_is_asked_for() {
        let Some(dir) = motionvm_parsec_testutil::gamedata_bifi2() else {
            eprintln!("skipping: no Bi-Fi 2 gamedata directory");
            return;
        };
        let res = resource_dir(&dir);
        let read = |n: &str| std::fs::read(find_ci(&res, n).expect(n)).expect(n);
        let drv = read("PSMDRV.DAT");
        let musadl = p2l::driver(&drv, p2l::AD_LIB).expect("MUSADL.DRV");
        let dmablast = p2l::driver(&drv, p2l::DMABLAST).expect("DMABLAST.DRV");
        let library = P2l::parse(read("PSMP2L.DAT")).expect("PSMP2L.DAT");
        let effect = library.sample(0).expect("row 0");
        let after_one_pass = |repeats: u16| {
            let (mut source, mut sink) =
                open_music(&musadl, Some(dmablast.clone()), 44_100).expect("the sound");
            sink.install_digital(6);
            sink.effect_volume(0x50);
            sink.effect_volume(0x3C);
            sink.play_effect(0, &effect, repeats);
            peak(source.as_mut(), 66_150);
            peak(source.as_mut(), 4410)
        };
        let (once, again) = (after_one_pass(0), after_one_pass(4));
        assert!(again > once + 1000, "once {once}, with four more {again}");
    }
}
