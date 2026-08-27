//! Jeff Jet - Abenteuer InfoHighway: the files it ships, and the binary its
//! kernel table is lifted out of.
//!
//! Everything the game shares with the 16-bit engine's other title — opening,
//! stepping, input, the location mechanism — is in [`super::motion16`]. This
//! game asks nothing of the engine that Die Enviro-Kids greifen ein does not:
//! it uses 150 kernel words, all of them bound, and none that the other game's
//! build lacks.

use std::path::Path;

use motionvm_forth::m16::Vm;

use crate::game::{Game, Res};
use crate::titles::{Title, motion16};

/// What a directory must hold before the game can be opened.
///
/// Three files. The game is on **two** volumes — `DATA.-1-` keeps the scripts,
/// the texts, the music and half the artwork, `DATA.-2-` the other half and
/// every palette, both fonts and the font reference table — so a copy missing
/// the second does not run dimmer, it runs blind. `HPPLAY.EXE` is read, not
/// run: the 228-word kernel table is lifted out of it. It is an older build
/// than `ENVIRO.EXE`, and from ordinal 124 up its words sit four below their
/// namesakes there, so the table has to come from this game's own binary.
const REQUIRED: &[(&str, &str)] = &[
    ("DATA.-1-", "scripts, texts, music and half the artwork"),
    (
        "DATA.-2-",
        "the rest of the artwork, and every palette, font and font reference table",
    ),
    ("HPPLAY.EXE", "the kernel word table"),
];

/// The engine binary the kernel word table is read from.
pub(super) const ENGINE: &str = "HPPLAY.EXE";

/// Which of the required files `dir` does not hold, as `(what, what for)`.
pub fn missing_data(dir: &Path) -> Vec<(&'static str, &'static str)> {
    motion16::missing_data(dir, REQUIRED)
}

/// Opens the game in `dir`.
pub fn open(dir: &Path) -> Res<Game<Vm>> {
    motion16::open(dir, Title::JeffJet, ENGINE, REQUIRED)
}
