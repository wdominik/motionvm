//! The music.
//!
//! Two stacks, one per engine generation, because the games' songs are two
//! formats: the 32-bit game's are [HMI](motionvm_motion_formats::m32::hmi) sequences
//! played through the FM driver out of `HMIMDRV.386`, the 16-bit games' are
//! [PSM 2](motionvm_motion_formats::m16::psm) sections played through `MUSADL.DRV`.
//! They are under [`m32`] and [`m16`], and they share no code path — only the
//! [`Chip`] both end at and the [`Write`] they answer in.
//!
//! **No sound is computed in either driver.** Both turn a song into the
//! byte-for-byte register writes the original sends, and the chip turns those
//! into samples. Keeping the two apart is what makes a driver checkable
//! against a recording of the original; see [`chip`] for what the one piece
//! that is not rebuilt costs.

pub mod chip;
pub mod clock;
pub mod error;
pub mod m16;
pub mod m32;
mod num;

pub use chip::{Chip, Write};
pub use error::{Error, Result};

/// A song going in, samples coming out.
///
/// What the two stacks have in common once they are built, which is what a
/// caller feeding an audio device wants and all of it: the two players are
/// otherwise unrelated code over unrelated formats, and even making one takes
/// different files. So there is no constructor here, and the generation is
/// chosen once, by whoever opens the game's music files.
pub trait Player {
    /// What one song is on this stack — a sequence for the 32-bit game, a
    /// section and a repeat count for the 16-bit ones.
    type Song;

    /// The sample rate it was made with.
    fn rate(&self) -> u32;

    /// Starts `song`, in place of whatever was playing.
    fn start(&mut self, song: Self::Song);

    /// Stops what is playing. Whether that is immediate is the driver's own
    /// business: the 16-bit one fades first, and stays [`Player::playing`]
    /// while it does.
    fn stop(&mut self);

    /// Whether anything is still sounding.
    fn playing(&self) -> bool;

    /// Renders the next samples, running the sequencer as their time passes.
    ///
    /// Interleaved stereo, two `i16` per frame. Allocation-free by
    /// construction, because this is what an audio callback calls.
    fn fill(&mut self, out: &mut [i16]);
}
