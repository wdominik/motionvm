//! The music of the 16-bit engine's games: PSM 2, played through `MUSADL.DRV`.
//!
//! The driver and the format are the [`motionvm_psm`] component's, re-exported
//! here under the names this family's code reaches them by. What is the
//! engine's own is how it hosts the driver, and that is the [`Player`](crate::Player)
//! answer below: which call of the driver each of the engine's music words
//! makes, and what the engine does around it. The engine's own checks of
//! the driver file are the same ones [`Driver::parse`] makes — the manager in
//! `ENVIRO.EXE` verifies the tag, the version, the counts and the trailer
//! before it installs the file (`198c:0078`), and its loader relocates a
//! module's nine section offsets in place (`1696:017e`). Two builds of the
//! file ship: the 1995/96 games all carry the same 4480-byte one with fifteen
//! entries, and Victor Loomes carries the 3915-byte build with fourteen.

pub use motionvm_psm::{Cue, Driver, PIT_HZ, Player, Sequencer, Voice, time_constant};

impl crate::Player for Player {
    type Song = Cue;
    type Sample = motionvm_psm::Sample;

    fn rate(&self) -> u32 {
        Player::rate(self)
    }

    /// A fade-out still counts as playing; the silence after it does not. A
    /// sample still running counts too.
    fn playing(&self) -> bool {
        Player::playing(self)
    }

    /// `PLAYSAMPLE`'s hand-over (`STERN.EXE` `15e5:0425`), after the
    /// manager's `StopAll`: the driver's one channel takes the new sample and
    /// whatever was playing on it stops where it stands. One channel because
    /// `STERN.EXE` installs its digital driver with one — the sixth install
    /// argument is what the sound manager's trampoline leaves in that slot,
    /// the caller's `DI`, which is 1 at the one call site (`1058:01bd`).
    fn sample(&mut self, sample: motionvm_psm::Sample) {
        self.play_sample(sample);
    }

    /// The driver's `StopAll` on its one channel. No 16-bit word reaches it
    /// for a sample alone — `PLAYSAMPLE` replaces, and nothing stops — so
    /// this answers the trait and no game.
    fn stop_sample(&mut self, _handle: i32) {
        Player::stop_sample(self);
    }

    /// `MUSADL.DRV` has no master volume among the entries the games
    /// install, and no 16-bit word asks for one: the call changes nothing.
    fn music_volume(&mut self, _volume: u16) {}

    /// `STARTTUNE`'s path, `SetSong` then `Play`, with the count the games
    /// pass: `-1` at every call site, endless.
    fn start(&mut self, cue: Cue) {
        Player::start(self, cue);
    }

    /// The way `ENDTUNE` does it — the games' one stop site (`ENVIRO.EXE`
    /// `1696:02fd`): the 2000 ms fade-out, `0x7d0`, begins at once and the
    /// hard stop lands 500 ms in, so what still sounds dies away under the
    /// fade's last level.
    fn stop(&mut self) {
        self.fade_out(0x7d0);
        self.stop_after(u64::from(self.rate()) / 2);
    }

    /// `PLAYSAMPLE`'s way (`STERN.EXE` `15e5:035d`): no fade — the tune
    /// plays on at full volume through the half second the script waits —
    /// and the same hard stop lands when it is up.
    fn cut(&mut self) {
        self.stop_after(u64::from(self.rate()) / 2);
    }

    fn fill(&mut self, out: &mut [i16]) {
        Player::fill(self, out);
    }
}
