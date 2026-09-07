//! Checker 2000: what the engine has to know about this one game to open it
//! and run it — the files it ships, the words its bootstrap names, the
//! script variables its state goes through.
//!
//! Everything here is the game's, not the engine's. What the game shares
//! with any other MOTION 32-bit title — the resource bank, the kernel table
//! out of `ENGINE.EXE`, loading the script modules, and the check that the
//! container is this game's — is in [`super::motion32`], as the 16-bit games'
//! shared half is in [`super::motion16`].
//!
//! The game runs on the *older* of the two shipped builds of the engine,
//! `ENGINE.EXE` V0.04.15/R78 of 1996-02-24, eight months before Dunkle
//! Schatten 2's R109. Nothing in this file knows that: the kernel's 375
//! words are scanned out of whatever binary sits beside the containers, and
//! the ordinal base the older build binds its domain table at — 934, where
//! R109 binds at 1039 — is read out of the same binary's start-up code
//! (`motionvm_motion_formats::m32::le::binding_of`).

use std::path::Path;

use motionvm_motion_forth::m32;

use crate::Result;
use crate::game::{Game, LocationScheme};
use crate::titles::{Title, motion32};

/// What a directory must hold before the game can be opened.
///
/// The same three things as Dunkle Schatten 2 and for the same reasons; see
/// [`super::ds2`] for how each was established. This game ships four
/// numbered containers, and the engine's own `ENGINE.RSC` beside them, which
/// holds the system font and palette as items 0 where the other game has
/// the loose `000.FNT` and `000.PAL`. It is not required: every string the
/// game draws names a font of its own, as the other game's do, and
/// `Resources::system_font` reads it when it is there — though without it
/// `TOGFX` has no system palette to install, the arrow's two colors resolve
/// to index 0, and the pointer is not seen. `SMPPATH` and the
/// `WAVS` directory are not required either: without them `->STARTSAMPLE`
/// finds nothing and the game plays on without speech, where the original
/// would wait for the disc.
const REQUIRED: &[(&str, &str)] = &[
    (
        "a resource container (NNN.RSC)",
        "scripts, artwork, texts, music, samples, fonts",
    ),
    ("ENGINE.EXE", "the kernel word table"),
    (
        "000.FRT",
        "the font reference table, without which no text is drawn",
    ),
];

/// The words that make a MOTION 32-bit container *this* game.
///
/// Both are module 4's, because module 2 cannot tell the two 32-bit games
/// apart: Dunkle Schatten 2's globals module is the same template grown —
/// every one of this game's 92 module-2 words, `_CHKM`, `SAVEBUF` and the
/// high-score table among them, is in the other game's 288. What is this
/// game's own is how the story is driven. `TASK_START` is what `START` calls
/// after `STARTUP`: it copies `_STASK` into `_TASK` and runs `TASK_CTRL`
/// once. `TASK_CTRL` is the story: a task counter `_TASK` walked by the
/// frame controller `ICTRL`, each step an `INCLLOC` of the location that
/// story step plays in (module 4, `0x002d0`). Dunkle Schatten 2's module 4
/// has `DO_INVSEL`, `CALLMENU` and `SHOW_DOC` where these are, and neither
/// of these.
pub(super) const SIGNATURE: &[(u32, &str)] = &[(4, "TASK_START"), (4, "TASK_CTRL")];

/// Where it keeps the location it is in.
///
/// `_ACTLOC` in module 2 is the location `INCLLOC` last entered, 0 before
/// any was (module 5, `0x000b4`: written from the argument after the old
/// location's modules are freed). There is no location to *ask* for: the
/// story is the task list in `TASK_CTRL`, and a location is entered when the
/// step that names it comes round — so a request has nowhere to go, and the
/// scheme says so with no variable to write.
pub(super) const LOCATION: LocationScheme = LocationScheme {
    module: 2,
    next: None,
    fallback: Some(("_ACTLOC", "_ACTLOC")),
    unset_below: Some(1),
};

/// Where the game keeps its shell variables: the story step `TASK_CTRL`
/// walks, and the location's own task counter.
///
/// The mechanism behind them is the generation's, in [`super::motion32`];
/// only the names are this game's.
pub(super) const SHELL: motion32::Shell = motion32::Shell {
    module: 2,
    task: "_TASK",
    task_phase: "_LOCTASK",
};

/// Which of the required files `dir` does not hold, as `(what, what for)`.
pub fn missing_data(dir: &Path) -> Vec<(&'static str, &'static str)> {
    motion32::missing_data(dir, REQUIRED)
}

/// Opens Checker 2000 in `dir`.
pub fn open(dir: &Path) -> Result<Game<m32::Vm>> {
    motion32::open(
        dir,
        Title::Checker2000,
        REQUIRED,
        SIGNATURE,
        LOCATION,
        SHELL,
    )
}
