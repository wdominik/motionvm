//! Victor Loomes – Das Spiel: the files it ships, and the binary its kernel
//! table is lifted out of.
//!
//! Everything the game shares with the 16-bit engine's other titles — opening,
//! stepping, input — is in [`super::motion16`]. Two things are its own. Its
//! container uses the earlier of the two framings
//! ([`motionvm_formats::m16::Generation::One`]), which the reader tells from
//! the file. And it moves between locations through `NAO` and `AO` in module
//! 605 rather than `NEXTLOC` in module 601, which the later games use and
//! this one has no module for; see [`super::motion16`]'s
//! `request_location`.
//!
//! It asks nothing of the machine the others do not: 100 of its 124 domain
//! words are called, all of them bound, and its kernel is a strict subset of
//! `ENVIRO.EXE`'s — the inventory, walk, order and digital-sound words the
//! later builds added are not in it, and the game does that work in its own
//! scripts instead.

use std::path::Path;

use motionvm_forth::m16::Vm;

use crate::Result;
use crate::game::Game;
use crate::titles::{Title, motion16};

/// What a directory must hold before the game can be opened.
///
/// Two files. The game is on **one** volume: `LL.EXE` holds the literal
/// `data.-1-` where the later builds hold the `DATA.-#i-` name former, so it
/// cannot open a second one whatever its container's header says — and the
/// header says two. `LL.EXE` is read, not run: the 204-word kernel table is
/// lifted out of it. It is the oldest build of the player, and its domain
/// table binds three ordinals below the later ones', so the table has to come
/// from this game's own binary.
///
/// `GFX.INF` ships beside these and is not required: it says how big every
/// sprite is without unpacking it, which is a question the container answers
/// on its own here — see [`motionvm_formats::m16::gfxinf`].
const REQUIRED: &[(&str, &str)] = &[
    ("DATA.-1-", "the whole game"),
    ("LL.EXE", "the kernel word table"),
];

/// The engine binary the kernel word table is read from.
pub(super) const ENGINE: &str = "LL.EXE";

/// The module its location variables live in.
///
/// `AO` is the location the game is in and `NAO` the one it has been asked
/// for; `CTRL` ends every frame with `NAO @ IF NAO @ INCLORT NAO 0! THEN`,
/// which is this game's spelling of the `NEXTLOC` poll the later ones run.
pub(super) const LOCATION_MODULE: u32 = 605;

/// Which of the required files `dir` does not hold, as `(what, what for)`.
pub fn missing_data(dir: &Path) -> Vec<(&'static str, &'static str)> {
    motion16::missing_data(dir, REQUIRED)
}

/// Opens the game in `dir`.
pub fn open(dir: &Path) -> Result<Game<Vm>> {
    motion16::open(dir, Title::VictorLoomes, ENGINE, REQUIRED)
}
