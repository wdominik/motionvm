//! Falsches Spiel mit Eddie M.: the files it ships, and the binary its kernel
//! table is lifted out of.
//!
//! Everything the game shares with the 16-bit engine's other titles — opening,
//! stepping, input, the location mechanism — is in [`super::motion16`]. It is
//! the one game whose modules reach two words no other game's do, and both
//! are answered by the engine as the generation's: `PLAYSAMPLE`, a digital
//! sound effect, at thirty-four sites, and `GIVEDATE`, the date, at one. The
//! other 151 words it uses are the ones its siblings use.

use std::path::Path;

use motionvm_motion_forth::m16;

use crate::Result;
use crate::game::{Game, LocationScheme};
use crate::titles::{Title, motion16};

/// What a directory must hold before the game can be opened.
///
/// Four files. The game is on **three** volumes, the most of any game here,
/// and every segment is packed: `DATA.-1-` keeps the scripts, the texts, the
/// per-location tables and the animation catalogs, `DATA.-2-` more than half
/// the artwork with the samples, the two songs, every palette, both fonts and
/// the font reference table, and `DATA.-3-` the rest of the artwork — so a
/// copy missing either later volume finds every script and only part of what
/// it draws. `STERN.EXE` is read,
/// not run: the 226-word kernel table is lifted out of it. It is the second
/// oldest of the five builds, with the shortest core table and a domain table
/// that binds at 102 as `LL.EXE`'s does, so the table has to come from this
/// game's own binary.
const REQUIRED: &[(&str, &str)] = &[
    (
        "DATA.-1-",
        "scripts, texts, the per-location tables and the animation catalogs",
    ),
    (
        "DATA.-2-",
        "more than half the artwork, the samples and songs, and every palette, font and font reference table",
    ),
    ("DATA.-3-", "the rest of the artwork"),
    ("STERN.EXE", "the kernel word table"),
];

/// The engine binary the kernel word table is read from.
pub(super) const ENGINE: &str = "STERN.EXE";

/// Where it keeps the location it is in and the one it is going to.
///
/// The scheme the four 1994–96 builds' games share, named here rather than
/// assumed: this game's module 601 declares all three variables, and
/// `STARTLOC` at 3 — the flat the game opens in.
pub(super) const LOCATION: LocationScheme = motion16::MODULE_601;

/// Which of the required files `dir` does not hold, as `(what, what for)`.
pub fn missing_data(dir: &Path) -> Vec<(&'static str, &'static str)> {
    motion16::missing_data(dir, REQUIRED)
}

/// Opens the game in `dir`.
pub fn open(dir: &Path) -> Result<Game<m16::Vm>> {
    motion16::open(
        dir,
        Title::FalschesSpielMitEddieM,
        ENGINE,
        REQUIRED,
        LOCATION,
    )
}
