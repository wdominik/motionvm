//! The music.
//!
//! The game's songs are [HMI](motionvm_formats::m32::hmi) sequences; this crate turns one
//! into a stream of MIDI messages at the song's own tick rate. The synthesis
//! that turns those messages into sound follows in its own module.
//!
//! The chain is [`Sequencer`] -> [`Fm`] -> [`Chip`], and [`Player`] is the three
//! of them with a clock. Everything but the chip is rebuilt from the original;
//! see [`chip`] for what the exception costs.

pub mod chip;
pub mod error;
pub mod opl;
pub mod player;
pub mod psm;
pub mod sequencer;

pub use chip::Chip;
pub use error::{Error, Result};
pub use opl::{Fm, Write};
pub use player::Player;
pub use sequencer::{Kind, Message, Sequencer};
