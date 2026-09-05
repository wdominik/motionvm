//! Dunkle Schatten 2: what the engine has to know about this one game to
//! open it and run it — the files it ships, the words its bootstrap names,
//! the script variables its input goes through.
//!
//! Everything here is the game's, not the engine's: module 2's `_STARTLOC`,
//! `_NEXTLOC`, `_LTHANDLER`, `_MLK`, `_MRK`, `_AKTKEY`, `_LOCTASK` and
//! `_LOCTASKPHA` are names the game's own compiler gave its variables, and
//! another MOTION game has others. The bootstrap words `STARTUP`, `START`
//! and `INCLLOC` are *not* here: they come from the authoring template — the
//! generation's, with the evidence in [`super::motion32`] — and so does
//! everything that runs them.
//!
//! Everything the game shares with any other MOTION 32-bit title — the
//! resource bank, the kernel table out of `ENGINE.EXE`, loading the script
//! modules, and the check that the container is this game's — is in
//! [`super::motion32`], as the 16-bit games' shared half is in
//! [`super::motion16`].

use std::path::Path;

use motionvm_motion_forth::m32;

use crate::Result;
use crate::game::{Game, LocationScheme};
use crate::titles::{Title, motion32};

/// What a directory must hold before [`Game::open`] can do anything with it.
///
/// Only three entries, and each was established by taking it away and watching
/// what broke, not by reading the loader:
///
/// - **A resource container.** Everything the game *is* lives in the `NNN.RSC`
///   files — the script modules, the artwork, the texts, the music, the fonts,
///   the palettes. This one ships three; how many a game has is the game's
///   business, and `motion32::has_container` looks for the pattern.
/// - **`ENGINE.EXE`.** The 356-word kernel table is read out of the LE image.
///   Without it the VM has no primitives to bind the bytecode's ordinals to.
/// - **`000.FRT`.** Character to glyph. Text rendering leaves early without it,
///   so a game that has everything else draws its pictures and not one word.
///
/// `000.FNT` is deliberately *not* here. It is read when present, but only as
/// the fallback for text that names no font of its own — and every string this
/// game draws names one, so removing it changes nothing on screen. The three
/// sound files are not here either: the frontend reports their absence and
/// plays on in silence.
const REQUIRED: &[(&str, &str)] = &[
    (
        "a resource container (NNN.RSC)",
        "scripts, artwork, texts, music, fonts",
    ),
    ("ENGINE.EXE", "the kernel word table"),
    (
        "000.FRT",
        "the font reference table, without which no text is drawn",
    ),
];

/// The words that make a MOTION 32-bit container *this* game.
///
/// Both are module 2 variables the game's own compiler named, and both are
/// read from here: `_STARTLOC` is the location a run begins at,
/// [`crate::Driven::start_location`], and `_NEXTLOC` is the cell a script
/// writes to ask for the next one, which `ICTRL` acts on inside its own frame.
/// A container that does not define them is not a container this code can
/// drive, whatever else it holds.
pub(super) const SIGNATURE: &[&str] = &["_STARTLOC", "_NEXTLOC"];

/// Where it keeps the location it is in and the one it is going to.
///
/// `_STARTLOC` in module 2 is what a run begins at and what a location
/// request writes; a move between locations goes through `_NEXTLOC`, which
/// `ICTRL` reads and consumes inside one frame, so nothing outside the game
/// ever sees it set. There is nothing to fall back to and no value that
/// means "none" — the variable exists only once the container is open, and
/// `None` before that is the lookup failing rather than a sentinel.
pub(super) const LOCATION: LocationScheme = LocationScheme {
    module: 2,
    next: "_STARTLOC",
    fallback: None,
    unset_below: None,
};

/// Which of the required files `dir` does not hold, as `(what, what for)`.
///
/// Empty means [`Game::open`] will get as far as parsing.
pub fn missing_data(dir: &Path) -> Vec<(&'static str, &'static str)> {
    motion32::missing_data(dir, REQUIRED)
}

/// Opens Dunkle Schatten 2 in `dir` — the free spelling of
/// [`Game::<m32::Vm>::open`](Game::open), so the roster's five arms read alike.
pub fn open(dir: &Path) -> Result<Game<m32::Vm>> {
    Game::<m32::Vm>::open(dir)
}

impl Game<m32::Vm> {
    /// Opens Dunkle Schatten 2 in `dir`.
    ///
    /// An associated function rather than a free one, unlike the 16-bit
    /// games': there is one 32-bit game today, so `Game::<m32::Vm>::open` names its
    /// machine unambiguously, and it is the name the tests already use. The
    /// work is `motion32::open`'s; what this hands it is what makes the
    /// container this game's — `REQUIRED` and `SIGNATURE`.
    pub fn open(dir: &Path) -> Result<Self> {
        motion32::open(dir, Title::DunkleSchatten2, REQUIRED, SIGNATURE, LOCATION)
    }
}

/// Where the game keeps its shell variables — the module its compiler put
/// them in and the names it gave them.
///
/// The *mechanism* behind them — which pair the intro's progress is read
/// from — is the generation's and lives in [`super::motion32`], the way
/// [`LocationScheme`] splits the same pair for locations. Only the names are
/// this game's.
pub(super) const SHELL: motion32::Shell = motion32::Shell {
    module: 2,
    task: "_LOCTASK",
    task_phase: "_LOCTASKPHA",
};
