//! Where the test suites find the games' files, and what they hold their
//! output against.
//!
//! No game data ships with this repository and none can, so every test that
//! needs a sprite or a song has to be told where a copy of the original
//! installation is. One lookup, here, rather than a copy in each suite: copies
//! drift, and a copy that reaches one directory too few answers "no data" on a
//! machine that has it — which reads exactly like a clean skip.
//!
//! Nine environment variables are read:
//!
//! - `MOTIONVM_GAMEDATA_DS2` — the directory holding Dunkle Schatten 2:
//!   `001.RSC` and friends. Falls back to `../games/DS2` beside the
//!   workspace, which is where a checkout next to an installed copy of the
//!   game finds it.
//! - `MOTIONVM_GAMEDATA_ENVIRO` — the directory holding Die Enviro-Kids
//!   greifen ein: `DATA.-1-` and `ENVIRO.EXE`. Falls back to
//!   `../games/ENVIRO` beside the workspace.
//! - `MOTIONVM_GAMEDATA_JEFFJET` — the directory holding Jeff Jet - Abenteuer
//!   InfoHighway: `DATA.-1-`, `DATA.-2-` and `HPPLAY.EXE`. Falls back to
//!   `../games/JEFFJET` beside the workspace.
//! - `MOTIONVM_GAMEDATA_HFA` — the directory holding Hilfe für Amajambere:
//!   `DATA.-1-`, `DATA.-2-` and `BMZ.EXE`. Falls back to `../games/HFA`
//!   beside the workspace.
//! - `MOTIONVM_GAMEDATA_VLOOMES` — the directory holding Victor Loomes:
//!   `DATA.-1-` and `LL.EXE`. Falls back to `../games/VLOOMES` beside the
//!   workspace.
//! - `MOTIONVM_GAMEDATA_EDDIEM` — the directory holding Falsches Spiel mit
//!   Eddie M.: `DATA.-1-`, `DATA.-2-`, `DATA.-3-` and `STERN.EXE`. Falls back
//!   to `../games/EDDIEM` beside the workspace.
//! - `MOTIONVM_GAMEDATA_CHECKER` — the directory holding Checker 2000:
//!   `001.RSC` and friends, `ENGINE.EXE` and `ENGINE.RSC`. Falls back to
//!   `../games/CHECKER` beside the workspace.
//! - `MOTIONVM_NO_GAMEDATA` — set to anything non-empty, every lookup here
//!   answers `None` before any of the others is consulted, so the suite runs
//!   the way CI runs it. Without it that cannot be reproduced on a machine
//!   that keeps the games beside the checkout: the fallback below is anchored
//!   at compile time through `CARGO_MANIFEST_DIR`, so no working directory
//!   escapes it and an empty `MOTIONVM_GAMEDATA_DS2` reaches the data anyway.
//!   The data-free path is the only thing CI proves, and a change that breaks
//!   it — a test that quietly starts needing a file, a skip that stops being a
//!   skip — is otherwise invisible until after a push. `just check-nodata`.
//! - `MOTIONVM_SAVES` — a directory holding a savegame. There is no fallback;
//!   savegames cannot be reconstructed, only played to. It has to be one of
//!   *this* engine's: the layouts are not interchangeable with the original's,
//!   which stores raw heap pointers where this stores handles.
//!
//! Seven games, seven variables, seven functions — rather than one variable
//! and a guess from the files it points at — because a test is written against
//! one game's modules and ids, and says which by the function it calls.
//!
//! Each game is probed for the file that is **its own**, not for its
//! container: the five 16-bit games all ship a `DATA.-1-`, so a container
//! probe would let `MOTIONVM_GAMEDATA_ENVIRO` accept a Jeff Jet directory and
//! then fail deep inside a suite instead of at the variable — and the two
//! 32-bit games both ship an `ENGINE.EXE`, so the 16-bit games are probed for
//! their player and the 32-bit ones for a file the other lacks.
//!
//! Beside the lookup, [`digest`]: the reference digests the suites hold their
//! scenes and their register streams against.
//!
//! **This crate may depend on the neutral layer and on nothing of the
//! family's.** `motionvm-motion-formats` dev-depends on it, so a dependency
//! the other way would be a cycle — which is what the rule protects, rather
//! than a count of dependencies for its own sake.

pub mod digest;

pub use digest::Digests;

use std::path::PathBuf;

/// Dunkle Schatten 2's game directory, or `None` when there is nothing to
/// test against. Probed for the loose `000.PAL`, the system palette this game
/// ships as a file where Checker 2000 keeps it in `ENGINE.RSC` — a container
/// or an `ENGINE.EXE` would let the variable accept the other 32-bit game.
///
/// **Panics when `MOTIONVM_GAMEDATA_DS2` names a directory without
/// `000.PAL`.** Answering `None` there would let a mistyped path read as "this
/// machine has no game data", which is the failure this module exists to
/// prevent: a run that skips everything is indistinguishable from a run that
/// passes everything. No variable and no data beside the workspace is the one
/// case that is genuinely a skip.
///
/// An **empty** value counts as no variable rather than as a wrong path. A
/// recipe that forwards the setting cannot know whether the caller gave one,
/// and forwarding an empty string is how it says "nothing to pass on".
///
/// `MOTIONVM_NO_GAMEDATA` wins over all of it and answers `None`; see the
/// module header.
pub fn gamedata_ds2() -> Option<PathBuf> {
    game("MOTIONVM_GAMEDATA_DS2", "../../../games/DS2", "000.PAL")
}

/// Die Enviro-Kids greifen ein's game directory, or `None` when there is
/// nothing to test against. The same rules as [`gamedata_ds2`], probing for
/// `ENVIRO.EXE` and falling back to `../games/ENVIRO`.
pub fn gamedata_enviro() -> Option<PathBuf> {
    game(
        "MOTIONVM_GAMEDATA_ENVIRO",
        "../../../games/ENVIRO",
        "ENVIRO.EXE",
    )
}

/// Jeff Jet - Abenteuer InfoHighway's game directory, or `None` when there is
/// nothing to test against. The same rules as [`gamedata_ds2`], probing for
/// `HPPLAY.EXE` and falling back to `../games/JEFFJET`.
pub fn gamedata_jeffjet() -> Option<PathBuf> {
    game(
        "MOTIONVM_GAMEDATA_JEFFJET",
        "../../../games/JEFFJET",
        "HPPLAY.EXE",
    )
}

/// Hilfe für Amajambere's game directory, or `None` when there is nothing to
/// test against. The same rules as [`gamedata_ds2`], probing for `BMZ.EXE`
/// and falling back to `../games/HFA`.
pub fn gamedata_hfa() -> Option<PathBuf> {
    game("MOTIONVM_GAMEDATA_HFA", "../../../games/HFA", "BMZ.EXE")
}

/// Victor Loomes' game directory, or `None` when there is nothing to test
/// against. The same rules as [`gamedata_ds2`], probing for `LL.EXE` and
/// falling back to `../games/VLOOMES`.
pub fn gamedata_vloomes() -> Option<PathBuf> {
    game(
        "MOTIONVM_GAMEDATA_VLOOMES",
        "../../../games/VLOOMES",
        "LL.EXE",
    )
}

/// Falsches Spiel mit Eddie M.'s game directory, or `None` when there is
/// nothing to test against. The same rules as [`gamedata_ds2`], probing for
/// `STERN.EXE` and falling back to `../games/EDDIEM`.
pub fn gamedata_eddiem() -> Option<PathBuf> {
    game(
        "MOTIONVM_GAMEDATA_EDDIEM",
        "../../../games/EDDIEM",
        "STERN.EXE",
    )
}

/// Checker 2000's game directory, or `None` when there is nothing to test
/// against. The same rules as [`gamedata_ds2`], probing for `ENGINE.RSC` —
/// the system font and palette container only this game ships, where Dunkle
/// Schatten 2 has the loose `000.FNT` and `000.PAL` — and falling back to
/// `../games/CHECKER`.
pub fn gamedata_checker() -> Option<PathBuf> {
    game(
        "MOTIONVM_GAMEDATA_CHECKER",
        "../../../games/CHECKER",
        "ENGINE.RSC",
    )
}

/// Whether the caller asked for CI's floor: no game data, whatever is on
/// this machine.
///
/// Read before anything else, and deliberately not overridable by the
/// per-game variables — the point is a run with *no* data, and a single
/// switch that seven functions honour is one thing to get right rather than
/// seven. See the module header for why the fallback makes this necessary.
fn no_gamedata() -> bool {
    std::env::var("MOTIONVM_NO_GAMEDATA").is_ok_and(|v| !v.is_empty())
}

/// The lookup the seven games share: nothing at all when
/// [`no_gamedata`] says so, else the variable, else the fallback beside the
/// workspace; a set-but-wrong path panics, a missing fallback skips.
fn game(var: &str, fallback: &str, probe: &str) -> Option<PathBuf> {
    if no_gamedata() {
        return None;
    }
    match std::env::var(var).ok().filter(|s| !s.is_empty()) {
        Some(set) => {
            let dir = PathBuf::from(set);
            assert!(
                find_ci(&dir, probe).is_some(),
                "{var} is set to {} but there is no {probe} in it. \
                 Point it at a directory holding the game's files, or unset it \
                 to let the tests skip themselves.",
                dir.display()
            );
            Some(dir)
        }
        None => {
            let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(fallback);
            find_ci(&dir, probe).is_some().then_some(dir)
        }
    }
}

/// One of the game's files, found whatever case it is spelled in.
///
/// The shipped names are upper case, but a copy that has been through a CD-ROM
/// driver, an archiver or a file manager often arrives lower-cased. On a
/// case-sensitive filesystem an exact-case `join` finds nothing, so every test
/// that opens a shipped file by name goes through here — otherwise the suite
/// would accept such an install (see [`gamedata_ds2`]) and then fail reading the
/// very files it just found.
///
/// Written out rather than calling `motionvm_motion_formats::find_ci`, which is
/// the same rule: `motionvm-motion-formats` already dev-depends on this crate,
/// so this crate cannot depend on it — see the module header.
pub fn find_ci(dir: &std::path::Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.is_file() {
        return Some(exact);
    }
    std::fs::read_dir(dir).ok()?.flatten().find_map(|e| {
        let p = e.path();
        p.is_file()
            .then(|| e.file_name())
            .and_then(|f| f.to_str().map(str::to_owned))
            .is_some_and(|f| f.eq_ignore_ascii_case(name))
            .then_some(p)
    })
}

/// [`find_ci`], but a test that asked for a shipped file wants it to be there.
///
/// Panics with the name rather than answering `None`: reaching here means the
/// suite already decided this directory holds the game.
pub fn game_file(dir: &std::path::Path, name: &str) -> PathBuf {
    find_ci(dir, name).unwrap_or_else(|| panic!("{} holds no {name} in any case", dir.display()))
}

/// The save directory to hand the engine, if one was named, together with the
/// first of `slots` that `slug`'s subdirectory of it holds.
///
/// The variable names what a game is *pointed at* — the directory holding one
/// per game — because that is what the engine is given and what it puts the
/// game's own name under.
///
/// Unlike [`gamedata_ds2`] this has no fallback and does not panic on a wrong
/// path: a savegame is made by playing the game to a particular place, so
/// there is no directory a checkout could be expected to have.
/// `MOTIONVM_NO_GAMEDATA` answers `None` here too: a savegame is a game's
/// data as much as its container is, and CI's floor has neither.
/// Slots are `i32` because that is what they are on the stack the moment they
/// reach `GET`: a slot number is an ordinary Forth cell, not a separate kind of
/// thing.
pub fn savegame_slot(slug: &str, slots: &[i32]) -> Option<(PathBuf, i32)> {
    // An empty value counts as absent, not as the working directory: the
    // recipe that runs the suite passes the variable through whether or not it
    // was given one, and `PathBuf::from("")` would silently look for `701.FRZ`
    // wherever cargo happened to be standing.
    if no_gamedata() {
        return None;
    }
    let set = std::env::var("MOTIONVM_SAVES")
        .ok()
        .filter(|s| !s.is_empty())?;
    let dir = PathBuf::from(set);
    // What is handed back is the directory the *engine* is pointed at, which
    // puts the game's slug on itself — so the slots are looked for one level
    // down and the caller passes on what it was given.
    let slot = slots
        .iter()
        .copied()
        .find(|s| dir.join(slug).join(format!("{s}.FRZ")).is_file())?;
    Some((dir, slot))
}

/// A save directory of this test's own, under `target/`, wiped before use.
///
/// Per test rather than shared: the save words are order-dependent — `PUT`
/// over an existing slot is a different path from `PUT` into an empty one —
/// so two tests sharing a directory would decide each other's outcome by
/// whichever ran first.
pub fn saves_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-saves")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a save directory");
    dir
}
