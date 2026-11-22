//! PSM 2 music: the module format, the Ad Lib driver `MUSADL.DRV` that plays
//! it, and the digital sample the `DMA*.DRV` drivers play beside it.
//!
//! A component: a third party's program games ship as a file of its own,
//! rebuilt from that file, and owned by no engine family — a family whose
//! games play through it uses it from here. What a program hosting the
//! driver does around it — which song it names, when it fades, how long it
//! waits before it stops — is that program's, and is said on its family's
//! side.
//!
//! The chain is [`Sequencer`] -> [`motionvm_opl::Chip`], with [`Player`] the
//! two of them and a clock, and beside them the one digital [`Voice`]. Every
//! driver file is read, not embedded: [`Driver::parse`] takes the bytes of
//! `MUSADL.DRV`, wherever a family found them.
//!
//! File offsets in the comments are `MUSADL.DRV`'s unless marked otherwise.

pub mod driver;
pub mod error;
pub mod format;
mod num;
pub mod player;
mod read;
pub mod sequencer;
pub mod voice;

pub use driver::{Driver, PIT_HZ};
pub use error::{Error, Result};
pub use format::{Plx, Sample};
pub use player::{Cue, Player};
pub use sequencer::Sequencer;
pub use voice::{Voice, time_constant};
