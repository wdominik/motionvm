//! The PARSEC runtime: rooms, verbs, inventory and the game loop.
//!
//! PARSEC is not a bytecode VM. The puzzle logic is compiled code in the
//! game's `XP01.EXE`; this crate expresses it in Rust, on top of a loader
//! that reads FGROUP/RIB, `RDV1.00` rooms, hotspot tables and `II`
//! dialogue. The executables themselves are treated as data: their text and
//! table rows are read from the unpacked load image, never transcribed into
//! literals and never executed. The game is *Dunkle Schatten*.

mod dac;
mod draw;
mod error;
mod exe;
mod game;
mod keys;
mod listing;
mod mouse;
mod music_sink;
mod num;
mod runtime;
mod titles;

pub use error::{Error, Result};
pub use game::{Game, missing_data};
pub use listing::{Location, locations, rules};
pub use motionvm_parsec_formats::Title;
pub use music_sink::MusicSink;
pub use runtime::Runtime;
pub use titles::Start;
pub use titles::xp1994::Rule;
