//! The music of the 32-bit engine's game: HMI sequences through the FM driver
//! `fmmidi3.com`.
//!
//! The chain is [`Sequencer`] -> [`Fm`] -> [`crate::Chip`], with [`Player`]
//! the three of them and a clock — the same shape the 16-bit stack has under
//! [`crate::m16`], over an entirely different driver and song format.

pub mod fm;
pub mod player;
pub mod sequencer;

pub use fm::Fm;
pub use player::Player;
pub use sequencer::{Kind, Message, Sequencer};
