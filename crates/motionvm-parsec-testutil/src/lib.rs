//! Where the test suites find the game's files.
//!
//! No game data ships with this repository and none can, so every test that
//! needs a room or a panel has to be told where a copy of the original
//! installation is. One lookup, here, rather than a copy in each suite: copies
//! drift, and a copy that reaches one directory too few answers "no data" on a
//! machine that has it — which reads exactly like a clean skip.
//!
//! These environment variables are read:
//!
//! - `MOTIONVM_GAMEDATA_DS1` — the directory holding *Dunkle Schatten*
//!   (the one with `XP01.EXE` and a `RESOURCE/` folder, or the files
//!   themselves), named for its slug. It falls back to `../games/DS1`
//!   beside the workspace, which is where a checkout next to an installed
//!   copy finds it. A machine without the game skips, it does not fail.
//! - `MOTIONVM_GAMEDATA_CAPZINS` — the same for *Captain Zins*, falling
//!   back to `../games/CAPZINS`.
//! - `MOTIONVM_SAVES` — the directory the games' own save directories are
//!   under: the `saves/` the program writes, not `saves/ds1/`, because that
//!   is what the engine is pointed at and it puts the game's slug on itself.
//!   The game's savegame is then in `ds1/` below it. There is no fallback;
//!   savegames cannot be reconstructed, only played to.
//! - `MOTIONVM_NO_GAMEDATA` — set to anything non-empty, every lookup here
//!   answers `None` before any of the others is consulted, so the suite runs
//!   the way CI runs it. Without it that cannot be reproduced on a machine
//!   that keeps the games beside the checkout: the fallbacks above are
//!   anchored at compile time through `CARGO_MANIFEST_DIR`, so no working
//!   directory escapes them and an empty `MOTIONVM_GAMEDATA_DS1`
//!   reaches the data anyway. The data-free path is the only thing CI
//!   proves, and a change that breaks it — a test that quietly starts
//!   needing a file, a skip that stops being a skip — is otherwise invisible
//!   until after a push. `just check-nodata`.
//!
//! A game is recognized by its own room container — `DSROOM.DAT`,
//! `CZROOM.DAT` — not by `XP01.EXE`: that is a name, not a signature.
//!
//! The reference digests the suites hold their register streams and the
//! location logic's cases to are in [`digest`].

pub mod digest;

pub use digest::Digests;

use std::path::{Path, PathBuf};

/// One title's install: what [`title`] needs to find it.
struct Install {
    /// The variable that names it.
    var: &'static str,
    /// Its folder in `../games`, relative to this crate.
    fallback: &'static str,
    /// The room container only this title ships.
    probe: &'static str,
    /// The title, for a message.
    name: &'static str,
}

/// The install `install` describes, or `None` when there is nothing to
/// test against.
///
/// **Panics when the variable names a directory without that title.**
/// Answering `None` there would let a mistyped path read as "this machine
/// has no game data", which is the failure this module exists to prevent:
/// a run that skips everything is indistinguishable from a run that passes
/// everything. No variable and no data beside the workspace is the one
/// case that is genuinely a skip.
///
/// An **empty** value counts as no variable rather than as a wrong path. A
/// recipe that forwards the setting cannot know whether the caller gave one,
/// and forwarding an empty string is how it says "nothing to pass on".
///
/// `MOTIONVM_NO_GAMEDATA` wins over all of it and answers `None`; see the
/// module header.
fn title(install: &Install) -> Option<PathBuf> {
    if no_gamedata() {
        return None;
    }
    let holds = |dir: &Path| find_ci(&resource_dir(dir), install.probe).is_some();
    match std::env::var(install.var).ok().filter(|s| !s.is_empty()) {
        Some(set) => {
            let dir = PathBuf::from(set);
            assert!(
                holds(&dir),
                "{} is set to {} but there is no {} in it (expected {} in RESOURCE/ \
                 or beside the EXEs). Point it at a directory holding the game's \
                 files, or unset it to let the tests skip themselves.",
                install.var,
                dir.display(),
                install.name,
                install.probe,
            );
            Some(dir)
        }
        None => {
            let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(install.fallback);
            holds(&dir).then_some(dir)
        }
    }
}

/// *Dunkle Schatten* (1994), or `None` when there is none to test.
/// Panics on a `MOTIONVM_GAMEDATA_DS1` that names another directory.
pub fn gamedata_ds1() -> Option<PathBuf> {
    title(&Install {
        var: "MOTIONVM_GAMEDATA_DS1",
        fallback: "../../../games/DS1",
        probe: "DSROOM.DAT",
        name: "Dunkle Schatten",
    })
}

/// *Captain Zins* (1994), or `None` when there is none to test.
/// Panics on a `MOTIONVM_GAMEDATA_CAPZINS` that names another directory.
pub fn gamedata_capzins() -> Option<PathBuf> {
    title(&Install {
        var: "MOTIONVM_GAMEDATA_CAPZINS",
        fallback: "../../../games/CAPZINS",
        probe: "CZROOM.DAT",
        name: "Captain Zins",
    })
}

/// One of the game's files, found whatever case it is spelled in.
///
/// The shipped names are upper case, but a copy that has been through a CD-ROM
/// driver, an archiver or a file manager often arrives lower-cased. On a
/// case-sensitive filesystem an exact-case `join` finds nothing, so every test
/// that opens a shipped file by name goes through here.
///
/// Written out rather than calling `motionvm_parsec_formats::find_ci`, which is the
/// same rule: this crate has no dependencies on purpose, and `motionvm-parsec-formats`
/// already dev-depends on *it*.
///
/// Several spellings and no exact one — possible on a case-sensitive disk —
/// answer the first in byte order, so the choice does not depend on the
/// order the directory lists its entries in.
pub fn find_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.is_file() {
        return Some(exact);
    }
    let names = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.file_name().to_str().map(str::to_owned));
    first_ci(names, name).map(|n| dir.join(n))
}

/// [`find_ci`], but a test that asked for a shipped file wants it to be there.
///
/// Panics with the name rather than answering `None`: reaching here means the
/// suite already decided this directory holds the game.
pub fn game_file(dir: &Path, name: &str) -> PathBuf {
    find_ci(dir, name).unwrap_or_else(|| panic!("{} holds no {name} in any case", dir.display()))
}

/// The `RESOURCE/` subdirectory when the install uses one, otherwise `dir`
/// itself.
///
/// *Dunkle Schatten* keeps every DAT file under `RESOURCE/`. A flattened
/// copy that put them next to the EXEs still works: both places are looked
/// in.
pub fn resource_dir(dir: &Path) -> PathBuf {
    find_ci_dir(dir, "RESOURCE").unwrap_or_else(|| dir.to_path_buf())
}

fn find_ci_dir(dir: &Path, name: &str) -> Option<PathBuf> {
    let exact = dir.join(name);
    if exact.is_dir() {
        return Some(exact);
    }
    let names = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(str::to_owned));
    first_ci(names, name).map(|n| dir.join(n))
}

/// Of `names`, the first in byte order that is `want` but for ASCII case:
/// the choice both lookups above make, apart from the disk.
fn first_ci(names: impl Iterator<Item = String>, want: &str) -> Option<String> {
    names.filter(|n| n.eq_ignore_ascii_case(want)).min()
}

/// A game's own save directory under `MOTIONVM_SAVES` — its slug below
/// the directory named, `ds1/` for *Dunkle Schatten* — if one was named.
///
/// Unlike [`gamedata_ds1`] this has no fallback and does not panic on a
/// wrong path: a savegame is made by playing the game to a particular place, so
/// there is no directory a checkout could be expected to have.
/// `MOTIONVM_NO_GAMEDATA` answers `None` here too: a savegame is a game's
/// data as much as its container is, and CI's floor has neither.
pub fn saves_dir_named(slug: &str) -> Option<PathBuf> {
    if no_gamedata() {
        return None;
    }
    let set = std::env::var("MOTIONVM_SAVES")
        .ok()
        .filter(|s| !s.is_empty())?;
    Some(PathBuf::from(set).join(slug))
}

/// Whether `MOTIONVM_NO_GAMEDATA` asks for a run with no game data at all.
///
/// One switch rather than a convention of emptying the other variables —
/// the point is a run with *no* data, and a single switch that every lookup
/// honors is one thing to get right rather than several. See the module header
/// for why the fallbacks make this necessary.
fn no_gamedata() -> bool {
    std::env::var("MOTIONVM_NO_GAMEDATA").is_ok_and(|v| !v.is_empty())
}

/// A save directory of this test's own, under `target/`, wiped before use.
pub fn saves_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-saves")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a save directory");
    dir
}

#[cfg(test)]
mod tests {
    use super::first_ci;

    /// Two spellings of one name: the same one is taken in whatever order
    /// the directory lists them.
    #[test]
    fn several_spellings_answer_the_first_in_byte_order() {
        let names = ["dsroom.dat", "PANELS.DAT", "DsRoom.DAT", "DSROOM.DAt"];
        let mut orders = vec![names.to_vec()];
        orders.push(names.iter().rev().copied().collect());
        orders.push(vec![names[2], names[0], names[3], names[1]]);
        for order in orders {
            let got = first_ci(order.into_iter().map(str::to_owned), "DSROOM.DAT");
            assert_eq!(got.as_deref(), Some("DSROOM.DAt"));
        }
        assert_eq!(first_ci(std::iter::empty(), "DSROOM.DAT"), None);
    }
}
