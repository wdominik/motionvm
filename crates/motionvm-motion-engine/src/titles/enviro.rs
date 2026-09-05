//! Die Enviro-Kids greifen ein: the files it ships, and the binary its kernel
//! table is lifted out of.
//!
//! Everything the game shares with the 16-bit engine's other titles — opening,
//! stepping, input, the location mechanism — is in [`super::motion16`].

use std::path::Path;

use motionvm_motion_forth::m16;

use crate::Result;
use crate::game::{Game, LocationScheme};
use crate::titles::{Title, motion16};

/// What a directory must hold before the game can be opened.
///
/// Two files. `DATA.-1-` is the whole game — scripts, artwork, texts, music,
/// fonts, palettes, all in one container — and `ENVIRO.EXE` is read, not run:
/// the 233-word kernel table is lifted out of it, and the bytecode's ordinals
/// mean nothing without it. The sound setup and the drivers are the original
/// player's; nothing here opens them.
const REQUIRED: &[(&str, &str)] = &[
    (
        "DATA.-1-",
        "the whole game: scripts, artwork, texts, music, fonts, palettes",
    ),
    ("ENVIRO.EXE", "the kernel word table"),
];

/// The engine binary the kernel word table is read from.
pub(super) const ENGINE: &str = "ENVIRO.EXE";

/// Where it keeps the location it is in and the one it is going to.
///
/// The scheme the four 1994–96 builds' games share, named here rather than
/// assumed: this game's module 601 declares all three variables.
pub(super) const LOCATION: LocationScheme = motion16::MODULE_601;

/// Which of the required files `dir` does not hold, as `(what, what for)`.
pub fn missing_data(dir: &Path) -> Vec<(&'static str, &'static str)> {
    motion16::missing_data(dir, REQUIRED)
}

/// Opens the game in `dir`.
pub fn open(dir: &Path) -> Result<Game<m16::Vm>> {
    motion16::open(
        dir,
        Title::DieEnviroKidsGreifenEin,
        ENGINE,
        REQUIRED,
        LOCATION,
    )
}
