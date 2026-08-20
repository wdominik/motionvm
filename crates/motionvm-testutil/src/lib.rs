//! Where the test suites find the game's files.
//!
//! No game data ships with this repository and none can, so every test that
//! needs a sprite or a song has to be told where a copy of the original
//! installation is. One lookup, here, rather than a copy in each suite: copies
//! drift, and a copy that reaches one directory too few answers "no data" on a
//! machine that has it — which reads exactly like a clean skip.
//!
//! Two environment variables are read:
//!
//! - `MOTIONVM_GAMEDATA` — the directory holding `001.RSC`. Falls back to
//!   `../gamedata` beside the workspace, which is where a checkout next to an
//!   installed copy of the game finds it.
//! - `MOTIONVM_SAVES` — a directory holding a savegame. There is no fallback;
//!   savegames cannot be reconstructed, only played to. It has to be one of
//!   *this* engine's: the layouts are not interchangeable with the original's,
//!   which stores raw heap pointers where this stores handles.

use std::path::PathBuf;

/// The game data directory, or `None` when there is nothing to test against.
///
/// **Panics when `MOTIONVM_GAMEDATA` names a directory without `001.RSC`.**
/// Answering `None` there would let a mistyped path read as "this machine has
/// no game data", which is the failure this module exists to prevent: a run
/// that skips everything is indistinguishable from a run that passes
/// everything. No variable and no data beside the workspace is the one case
/// that is genuinely a skip.
///
/// An **empty** value counts as no variable rather than as a wrong path. A
/// recipe that forwards the setting cannot know whether the caller gave one,
/// and forwarding an empty string is how it says "nothing to pass on".
pub fn gamedata() -> Option<PathBuf> {
    match std::env::var("MOTIONVM_GAMEDATA")
        .ok()
        .filter(|s| !s.is_empty())
    {
        Some(set) => {
            let dir = PathBuf::from(set);
            assert!(
                has_container(&dir),
                "MOTIONVM_GAMEDATA is set to {} but there is no 001.RSC in it. \
                 Point it at a directory holding the game's files, or unset it \
                 to let the tests skip themselves.",
                dir.display()
            );
            Some(dir)
        }
        None => {
            let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../gamedata");
            has_container(&dir).then_some(dir)
        }
    }
}

/// One of the game's files, found whatever case it is spelled in.
///
/// The shipped names are upper case, but a copy that has been through a CD-ROM
/// driver, an archiver or a file manager often arrives lower-cased. On a
/// case-sensitive filesystem an exact-case `join` finds nothing, so every test
/// that opens a shipped file by name goes through here — otherwise the suite
/// would accept such an install (see [`gamedata`]) and then fail reading the
/// very files it just found.
///
/// Written out rather than calling `motionvm_formats::find_ci`, which is the
/// same rule: this crate has no dependencies on purpose, and `motionvm-formats`
/// already dev-depends on *it*.
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

/// Whether `dir` holds `001.RSC`, in whatever case it is spelled.
fn has_container(dir: &std::path::Path) -> bool {
    find_ci(dir, "001.RSC").is_some()
}

/// A directory holding one of this engine's savegames, if one was named,
/// together with the first slot number found in it.
///
/// Unlike [`gamedata`] this has no fallback and does not panic on a wrong
/// path: a savegame is made by playing the game to a particular place, so
/// there is no directory a checkout could be expected to have.
/// Slots are `i32` because that is what they are on the stack the moment they
/// reach `GET`: a slot number is an ordinary Forth cell, not a separate kind of
/// thing.
pub fn savegame_slot(slots: &[i32]) -> Option<(PathBuf, i32)> {
    // An empty value counts as absent, not as the working directory: the
    // recipe that runs the suite passes the variable through whether or not it
    // was given one, and `PathBuf::from("")` would silently look for `701.FRZ`
    // wherever cargo happened to be standing.
    let set = std::env::var("MOTIONVM_SAVES")
        .ok()
        .filter(|s| !s.is_empty())?;
    let dir = PathBuf::from(set);
    let slot = slots
        .iter()
        .copied()
        .find(|s| dir.join(format!("{s}.FRZ")).is_file())?;
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
