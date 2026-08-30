//! What every MOTION 32-bit game needs to be opened: the resource bank, the
//! kernel table out of `ENGINE.EXE`, and every script module in the
//! containers.
//!
//! None of it is one game's. `NNN.RSC` containers beside an `ENGINE.EXE` is
//! the generation's shape, and MOTION made more games in it than the one this
//! engine plays today — Checker 2000 ships exactly that, with its own
//! `ENGINE.EXE` V0.04.15/R78. What a game's own module says is which files it
//! must have, which words tell its container apart from another's, and what
//! its bootstrap is called; [`super::ds2`] is that for Dunkle Schatten 2.
//!
//! The 16-bit counterpart is [`super::motion16`], and this file is shaped like
//! it on purpose: two generations that read the same way are the property
//! CONTRIBUTING's "Two generations" section asks for.

use std::path::Path;

use motionvm_formats::m32::{Kind, ScrModule, rsc::Bank};
use motionvm_forth::Machine;
use motionvm_forth::m32::Vm;

use crate::Engine;
use crate::Error;
use crate::Result;
use crate::game::Game;
use crate::titles::Title;

/// Whether `dir` holds a resource container at all.
///
/// Named by pattern rather than by number because how many a game ships is
/// the game's business: Dunkle Schatten 2 has three, and the loader merges
/// however many it finds. The engine loads `%03d.rsc` from `RSCPATH`, so the
/// stem is three digits and nothing else — `OLD.RSC` and `001.RSC.bak` are
/// not containers. Case-insensitively, because a copied install often arrives
/// lower-cased.
pub(super) fn has_container(dir: &Path) -> bool {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries.flatten().any(|e| {
                let p = e.path();
                p.extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|x| x.eq_ignore_ascii_case("rsc"))
                    && p.file_stem()
                        .and_then(|s| s.to_str())
                        .is_some_and(|s| s.len() == 3 && s.bytes().all(|b| b.is_ascii_digit()))
            })
        })
        .unwrap_or(false)
}

/// Which of `required` the directory `dir` does not hold, as `(what, what
/// for)`.
///
/// An entry whose name begins with "a resource container" is answered by
/// [`has_container`]; every other is a file name, looked for
/// case-insensitively like the container scan, because reporting a file as
/// missing while it sits in the directory is worse than not finding it at all.
///
/// An existence check, not a validity one — a truncated `ENGINE.EXE` still
/// fails later, and says so itself.
pub(super) fn missing_data(
    dir: &Path,
    required: &[(&'static str, &'static str)],
) -> Vec<(&'static str, &'static str)> {
    let container = has_container(dir);
    required
        .iter()
        .filter(|(name, _)| match *name {
            n if n.starts_with("a resource container") => !container,
            n => motionvm_formats::find_ci(dir, n).is_none(),
        })
        .copied()
        .collect()
}

/// Opens the bank, binds the kernel out of `ENGINE.EXE`, loads every script
/// module in the containers, and checks that the container is the game the
/// caller named.
///
/// Checks the required files first. Not for safety — the opens below would
/// fail anyway — but because of *what* they fail with: a bare
/// `Os { code: 2, kind: NotFound }` names neither the file nor the fact that a
/// game directory was expected at all, and the person reading it has just
/// copied a 1996 CD and has no way to guess which of its thirty files
/// mattered.
///
/// `signature` is how a container says which game it is. A `NNN.RSC` beside an
/// `ENGINE.EXE` says MOTION 32-bit and nothing more, and the words a bootstrap
/// names do not say it either: they come from the authoring template, so
/// another MOTION game has them too. Checker 2000 exports `STARTUP`, `START`
/// and `INCLLOC` from modules 3, 4 and 5 exactly as Dunkle Schatten 2 does.
/// What is a game's own is its module 2 script variables — see
/// [`super::ds2::SIGNATURE`] — and Checker 2000's module 2 has none of Dunkle
/// Schatten 2's.
///
/// Without the check the template's words would bind, run against another
/// game's data, and fail somewhere inside the VM — under the wrong game's
/// name, which is the part that misleads. The 16-bit opener answers the same
/// question by the engine binary beside the container; here the binary is
/// `ENGINE.EXE` in every game, so the answer has to come from the data.
///
/// The other directory this catches is a copy of the right game missing the
/// container its script is in, which reaches exactly the same state — hence a
/// message that names the two cases rather than deciding between them, which
/// the data cannot do.
pub(super) fn open(
    dir: &Path,
    title: Title,
    required: &[(&'static str, &'static str)],
    signature: &[&str],
) -> Result<Game<Vm>> {
    // A path that is not there at all gets its own answer. Listing three
    // missing files for a directory that does not exist describes the symptom
    // and hides the cause, which is usually a typo.
    if !dir.is_dir() {
        return Err(Error::NoSuchDirectory {
            dir: dir.to_path_buf(),
        });
    }
    let missing = missing_data(dir, required);
    if !missing.is_empty() {
        return Err(Error::Incomplete {
            dir: dir.to_path_buf(),
            title: title.short(),
            missing: missing.iter().map(|(n, _)| *n).collect(),
        });
    }
    let bank = Bank::open_dir(dir).map_err(|e| Error::data(dir, e))?;
    let engine_exe = motionvm_formats::find_ci(dir, "ENGINE.EXE")
        .ok_or_else(|| Error::missing_file(dir, "ENGINE.EXE"))?;
    let img = motionvm_formats::m32::le::Image::open(&engine_exe)
        .map_err(|e| Error::data(&engine_exe, e))?;
    let kernel = motionvm_formats::m32::le::kernel_words(&img);
    let mut vm = Vm::new(&kernel);

    for (_, id) in bank.present(Kind::Script) {
        // `present` reads the index, `item` reads the data behind it, and a
        // truncated container can index an item it does not hold. Skipping
        // for the same reason a module that will not parse is skipped: one
        // bad entry should not stop the game from starting.
        let Some(item) = bank
            .item(Kind::Script, id)
            .map_err(|e| Error::data(dir, e))?
        else {
            continue;
        };
        // A module that will not parse is skipped rather than fatal: the set
        // of modules is large and one bad entry should not stop the game from
        // starting. A missing module announces itself loudly later, when
        // something calls into it.
        if let Ok(parsed) = ScrModule::parse(item) {
            vm.load(item, &parsed);
        }
    }

    if let Some(name) = signature
        .iter()
        .find(|name| vm.word_address(2, name).is_none())
    {
        return Err(Error::NotThisGame {
            dir: dir.to_path_buf(),
            title: title.short(),
            word: (*name).to_string(),
        });
    }

    let engine = Engine::new().with_bank(dir, bank);
    Ok(Game {
        vm,
        engine,
        title,
        running: false,
        frame_controllers: Vec::new(),
        ending: false,
        over: false,
        parked: None,
    })
}
