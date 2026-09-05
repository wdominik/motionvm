//! The music of the 16-bit engine's games: PSM 2, played through `MUSADL.DRV`.
//!
//! The chain is [`Sequencer`] -> [`crate::Chip`], with [`Player`] the two of
//! them and a clock — the same shape the 32-bit stack has under
//! [`crate::m32`], over an entirely different driver and song format — and
//! beside it the one digital [`Voice`] the sound effects play through.

pub mod driver;
pub mod player;
pub mod sequencer;
pub mod voice;

pub use driver::{Driver, PIT_HZ};
pub use player::{Cue, Player};
pub use sequencer::Sequencer;
pub use voice::{Voice, time_constant};
