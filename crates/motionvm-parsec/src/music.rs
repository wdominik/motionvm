//! The family's music, joined up for a platform: a sink for the game, a
//! source for the audio thread, and the channel between them.
//!
//! ```text
//! game thread                            audio thread
//!   Runtime ─ MusicSink ─── mpsc ────▶  Player::fill
//!             Plx::from_section here     nothing but arithmetic
//! ```
//!
//! The game asks its Ad Lib driver for something in the driver's own
//! entries, and a song arrives as the bare `PLX` section the program hands
//! over; reading it means checking and allocating, which belongs on the
//! game thread, so the sink does it and sends the finished song across.
//! The driver is the game's own `MUSADL.DRV`, played by `motionvm_psm`'s
//! [`Player`] on the audio thread.
//!
//! The player starts a tick of the driver every `period` PIT cycles, its
//! period following the song's own tempo events. The original's timer
//! module runs the PIT on the shortest of its base period, `952h` cycles
//! (500 a second, `0x5B29`), and the periods its clients ask for, and
//! counts a client's period down by each interrupt's (`0x1A115`). So a
//! driver tick there lands on the first interrupt at or after it — on the
//! base period, less than 2386 PIT cycles (2 ms) later than here. The
//! writes and their order are the same.

use motionvm_parsec_engine::MusicSink;
use motionvm_playable::{AudioSource, Result};
use motionvm_psm::{Cue, Player, Plx};
use std::sync::mpsc::{Receiver, Sender, channel};

/// What crosses from the game thread to the audio one. A song is boxed
/// because it is far larger than the rest.
enum Command {
    Play(Box<Cue>),
    PlayFade(Box<Cue>, u16),
    FadeOut(u16),
    Stop,
    Reinstall,
}

/// The audio thread's side: drain the command channel, then render.
struct Backend {
    player: Player,
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
                Command::Reinstall => self.player.reinstall(),
            }
        }
        self.player.fill(out);
    }
}

/// What the engine talks to. A song that will not read as a `PLX` is not
/// played: the reader hands over only what carries the tag, so it would
/// mean a damaged library.
struct Music {
    tx: Sender<Command>,
}

impl Music {
    /// The cue for a song, or nothing for one that is not a `PLX`.
    fn cue(song: &[u8], loops: i16) -> Option<Box<Cue>> {
        let song = Plx::from_section(song.to_vec()).ok()?;
        Some(Box::new(Cue { song, loops }))
    }
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
}

/// The music at the device rate the platform learned, through the driver
/// file the game ships: a source for the audio thread and a sink for the
/// game, already joined.
pub(crate) fn open_music(
    driver: &[u8],
    rate: u32,
) -> Result<(Box<dyn AudioSource>, Box<dyn MusicSink>)> {
    let player =
        Player::new(rate, driver).map_err(|e| format!("the Ad Lib driver did not come up: {e}"))?;
    let (tx, rx) = channel();
    Ok((Box::new(Backend { player, rx }), Box::new(Music { tx })))
}
