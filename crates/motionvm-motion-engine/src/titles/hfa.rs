//! Hilfe für Amajambere: the files it ships, and the binary its kernel table
//! is lifted out of.
//!
//! Everything the game shares with the 16-bit engine's other titles — opening,
//! stepping, input, the location mechanism — is in [`super::motion16`]. This
//! game asks nothing of the engine the other two do not: it uses 143 kernel
//! words, all of them bound, and the only two neither of the others calls —
//! `&` and `GFXVFLIP` — are implemented already.

use std::path::Path;

use motionvm_motion_forth::m16::Vm;

use crate::Result;
use crate::game::{Game, LocationScheme};
use crate::titles::{Title, motion16};

/// What a directory must hold before the game can be opened.
///
/// Three files. The game is on **two** volumes — `DATA.-1-` keeps the scripts,
/// the texts and the music, `DATA.-2-` every sprite, every palette, all seven
/// fonts and the font reference table — so a copy missing the second finds
/// every script and nothing to draw. Unlike Jeff Jet's, these two volumes
/// store their items plainly: two volumes and packed items are independent
/// choices, and this game makes them differently. `BMZ.EXE` is read, not run:
/// the 232-word kernel table is lifted out of it. It is a build between the
/// other two, one word short of `ENVIRO.EXE`'s table, so the table has to come
/// from this game's own binary.
const REQUIRED: &[(&str, &str)] = &[
    ("DATA.-1-", "scripts, texts and music"),
    (
        "DATA.-2-",
        "every sprite, palette and font, and the font reference table",
    ),
    ("BMZ.EXE", "the kernel word table"),
];

/// The engine binary the kernel word table is read from.
pub(super) const ENGINE: &str = "BMZ.EXE";

/// Where it keeps the location it is in and the one it is going to.
///
/// The scheme its build's games share, named here rather than assumed: this
/// game's module 601 declares all three variables.
pub(super) const LOCATION: LocationScheme = motion16::MODULE_601;

/// Which of the required files `dir` does not hold, as `(what, what for)`.
pub fn missing_data(dir: &Path) -> Vec<(&'static str, &'static str)> {
    motion16::missing_data(dir, REQUIRED)
}

/// Opens the game in `dir`.
pub fn open(dir: &Path) -> Result<Game<Vm>> {
    motion16::open(dir, Title::HilfeFuerAmajambere, ENGINE, REQUIRED, LOCATION)
}
