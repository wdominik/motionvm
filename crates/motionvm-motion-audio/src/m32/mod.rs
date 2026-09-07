//! The music of the 32-bit engine's games: HMI sequences through the FM
//! driver `fmmidi3.com`, and the WAV samples of the sound layer's digital
//! side over them.
//!
//! The chain is [`Sequencer`] -> [`Fm`] -> [`crate::Chip`], with [`Player`]
//! the three of them and a clock — the same shape the 16-bit stack has under
//! [`crate::m16`], over an entirely different driver and song format — and a
//! [`Voice`] mixed into the frames the chip fills, as the 16-bit player mixes
//! its own.

pub mod fm;
pub mod player;
pub mod sequencer;
pub mod voice;

pub use fm::Fm;
pub use player::Player;
pub use sequencer::{Kind, Message, Sequencer};
pub use voice::{Sample, Voice};
