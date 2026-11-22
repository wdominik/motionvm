//! The OPL3 a music driver writes to.
//!
//! A component: the chip every rebuilt music driver writes to, owned by no
//! engine family. What is here is the [`Chip`] every rebuilt driver ends at,
//! the [`Write`] they all answer in, the [`clock`] that runs a driver's
//! sequencer against an output rate, and the [`num`] rules by which a value a
//! driver holds becomes a byte a register takes.
//!
//! **No sound is computed in a driver.** A driver turns a song into the
//! byte-for-byte register writes its original sends, and the chip turns
//! those into samples. Keeping the two apart is what makes a driver
//! checkable against a recording of the original; see [`chip`] for what the
//! one piece that is not rebuilt costs.

pub mod chip;
pub mod clock;
pub mod num;

pub use chip::{Chip, Write};
