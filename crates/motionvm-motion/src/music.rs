//! The family's music, joined up for a platform: a sink for the game, a
//! source for the audio thread, and the channel between them.
//!
//! ```text
//! game thread                            audio thread
//!   Engine ─ MusicSink ──── mpsc ────▶  Player::fill
//!            Song::parse here             nothing but arithmetic
//! ```
//!
//! The split is where it is on purpose. Parsing a song means reading, checking
//! and allocating, and none of that belongs in an audio callback — so
//! the sink does it on the game thread and sends the finished song
//! across. What is left for the callback's side is a `try_recv` and the
//! synthesis.
//!
//! (The old song is dropped on the audio thread when a new one replaces it.
//! Handing it back over a second channel would avoid even that, but a location
//! change happens every few minutes, and a returning channel is more machinery
//! than the problem is worth.)
//!
//! Every game comes through here with the same shape and its generation's
//! player: Dunkle Schatten 2's HMI songs through the rebuilt MIDI driver,
//! the 16-bit games' PSM 2 tunes through the rebuilt `MUSADL.DRV` sequencer.
//! The device itself is the platform's: it opens the output, learns the rate,
//! and hands the rate in.

use motionvm_motion_audio::m16::Cue;
use motionvm_motion_audio::{Player, m16, m32};
use motionvm_motion_engine::MusicSink;
use motionvm_motion_engine::titles;
use motionvm_motion_formats::m16::psm::Plx;
use motionvm_motion_formats::m32::DriverArchive;
use motionvm_motion_formats::m32::bnk::Bank as InstrumentBank;
use motionvm_motion_formats::m32::hmi::Song;
use motionvm_playable::{AudioSource, Result};
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, channel};

/// What crosses from the game thread to the audio one. Boxed because a parsed
/// song is far larger than the rest of the enum and the channel would carry
/// that size on every message.
enum Command<S> {
    Start(Box<S>),
    Stop,
}

/// The audio thread's side: drain the command channel, then render.
///
/// Generic over the player, because that is the whole of the difference:
/// what a song is, and which driver turns it into registers, are both the
/// player's.
struct Backend<P: Player> {
    player: P,
    rx: Receiver<Command<P::Song>>,
}

impl<P: Player + Send + 'static> AudioSource for Backend<P>
where
    P::Song: Send + 'static,
{
    fn fill(&mut self, out: &mut [i16]) {
        while let Ok(command) = self.rx.try_recv() {
            match command {
                Command::Start(song) => self.player.start(*song),
                Command::Stop => self.player.stop(),
            }
        }
        self.player.fill(out);
    }
}

/// What the 32-bit game's engine talks to. Everything it is handed goes over
/// the channel.
///
/// The parse happens here, on the game thread: reading, checking and
/// allocating have no business in an audio callback.
struct Music {
    tx: Sender<Command<Song>>,
}

impl MusicSink for Music {
    fn start(&mut self, _handle: i32, tune: i32, _looping: bool, song: &[u8]) {
        match Song::parse(song) {
            Ok(song) => {
                let _ = self.tx.send(Command::Start(Box::new(song)));
            }
            // A song that will not parse is worth saying out loud — it means a
            // block the decoder does not understand — but not worth stopping
            // for.
            Err(e) => eprintln!("tune {tune} did not parse: {e}"),
        }
    }

    fn stop(&mut self, _handle: i32) {
        let _ = self.tx.send(Command::Stop);
    }
}

/// The 16-bit games' sink. `STARTTUNE`'s loop count is `-1` at every call
/// site in those games — endless — and the driver reads it unsigned, so the
/// bool comes back out as the count it stands for.
struct PsmMusic {
    tx: Sender<Command<Cue>>,
}

impl MusicSink for PsmMusic {
    fn start(&mut self, _handle: i32, tune: i32, looping: bool, song: &[u8]) {
        match Plx::parse(song) {
            Ok(song) => {
                let loops = if looping { -1 } else { 0 };
                let _ = self.tx.send(Command::Start(Box::new(Cue { song, loops })));
            }
            Err(e) => eprintln!("tune {tune} did not parse: {e}"),
        }
    }

    fn stop(&mut self, _handle: i32) {
        let _ = self.tx.send(Command::Stop);
    }
}

/// The music for the game in `dir`, at the device rate the platform learned:
/// a source for its audio thread and a sink for the game, already joined.
///
/// Which stack comes up is the roster's answer, not the caller's: the
/// directory's game is detected the way `titles::open` detects it, and its
/// generation names the format. The files are the game's own — `HMIMDRV.386`
/// and the two instrument banks for the 32-bit game, the same three files
/// `ENGINE.EXE` hands its MIDI layer; `MUSADL.DRV` for the 16-bit games,
/// the same file their player loads whole and installs. All four 16-bit
/// games ship that driver: the three later ones carry byte-identical copies,
/// Victor Loomes an older build with one entry fewer, and `m16::Driver`
/// reads either — so one opener serves them.
pub fn open_music(dir: &Path, rate: u32) -> Result<(Box<dyn AudioSource>, Box<dyn MusicSink>)> {
    let Some(title) = titles::detect(dir) else {
        return Err(format!("no game this family plays in {}", dir.display()).into());
    };
    // `find_ci` throughout: the files are looked up by name, and a copied
    // install is as likely to spell them in lower case as on the disc.
    let read = |name: &str| -> std::result::Result<Vec<u8>, String> {
        let path = motionvm_motion_formats::find_ci(dir, name)
            .ok_or_else(|| format!("{name}: not found"))?;
        std::fs::read(path).map_err(|e| format!("{name}: {e}"))
    };
    match title.generation() {
        titles::Generation::Motion32 => {
            let archive = read("HMIMDRV.386")?;
            let archive =
                DriverArchive::parse(&archive).map_err(|e| format!("HMIMDRV.386: {e}"))?;
            let driver = archive
                .device(m32::fm::Fm::DEVICE)
                .ok_or("HMIMDRV.386 has no OPL3 driver")?;
            let melodic = read("MELODIC.BNK")?;
            let drums = read("DRUM.BNK")?;
            let melodic =
                InstrumentBank::parse(&melodic).map_err(|e| format!("MELODIC.BNK: {e}"))?;
            let drums = InstrumentBank::parse(&drums).map_err(|e| format!("DRUM.BNK: {e}"))?;
            let player = m32::Player::new(rate, driver, &melodic, &drums)
                .map_err(|e| format!("the FM driver did not come up: {e}"))?;
            let (tx, rx) = channel();
            Ok((Box::new(Backend { player, rx }), Box::new(Music { tx })))
        }
        titles::Generation::Motion16 => {
            let driver = read("MUSADL.DRV")?;
            let player = m16::Player::new(rate, &driver)
                .map_err(|e| format!("the Ad Lib driver did not come up: {e}"))?;
            let (tx, rx) = channel();
            Ok((Box::new(Backend { player, rx }), Box::new(PsmMusic { tx })))
        }
    }
}
