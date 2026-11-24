//! A save is kept whole or not at all.
//!
//! Every savegame file is replaced atomically on its own
//! ([`super::write_atomically`]), and that is not enough. A save is three
//! words — `PUT`, `PUTANIM`, `=>PUTAS`, in that order in every game that has
//! a save menu (DS2's `ICTRL` at 0x036c0, the 16-bit games' module 650 and
//! module 100) — and each writes its own file. A save that stops after the
//! first has already replaced the `.blk` and left the other two behind: a
//! slot that names one location and holds another's modules, and fails to
//! load. The slot the player was saving over is gone even though no file in
//! it is torn.
//!
//! The three need not even share a number. Victor Loomes saves `n 700 + PUT`
//! and then `n PUTANIM`, `n =>PUTAS` — slot A is `701.blk`, `001.anm` and
//! `001.FRZ` — and loads the same way. So what is kept together is the save,
//! not a slot id: a [`Journal`] lists every file the save replaces, as it
//! replaces it.
//!
//! * **Opening** ([`Journal::open`]), at `PUT`: an empty note, `save.journal`.
//! * **Enlisting** ([`Journal::enlist`]), before each of the three words
//!   writes: the file goes into the note, with whether it existed, and then,
//!   if it did, is renamed to `<name>.old`.
//! * The word then writes its file.
//! * **Closing** ([`Journal::close`]), once `=>PUTAS` has written: the note is
//!   removed — that removal is the moment the new save counts — and the
//!   `.old` files after it.
//! * **Settling** ([`settle`]), when a save directory is taken into use: a
//!   note still there is a save that never closed. Its `.old` files go back
//!   over whatever it wrote, and a file it wrote that did not exist before is
//!   removed, so a first save that never finished leaves no slot.
//!
//! A file goes into the note before it is moved. A stop between the two
//! leaves a file the note names with no `.old` beside it, and that file is
//! still the old one.
//!
//! A save word that fails ends the run: of the errors a word can meet, only a
//! division by zero is reported in a box and played on from
//! ([`crate::Engine::report_error`]). So a failed save is rolled back by the
//! next run, and nothing in between reads the save directory.
//!
//! The original has none of this. It writes the three straight through, and a
//! save interrupted there leaves exactly the mixed slot described above. The
//! words, their order and their files are the original's; what a stop between
//! them costs is not.

use std::fmt::Write as _;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// The note an open save keeps in the save directory.
const NOTE: &str = "save.journal";

/// What a note opens with, so that a file of that name that is not one is
/// refused rather than read as a list.
const HEAD: &str = "motionvm save journal 1";

/// The suffixes of the files a save writes, the only ones a note may name.
const SUFFIXES: [&str; 3] = ["blk", "anm", "FRZ"];

/// One file a save replaces, and whether there was one to replace.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    name: String,
    existed: bool,
}

/// A save in progress: the files it has replaced so far.
#[derive(Debug)]
pub(crate) struct Journal {
    dir: PathBuf,
    entries: Vec<Entry>,
}

fn old(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!("{name}.old"))
}

/// Removes a file that may not be there.
fn remove(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Makes the renames in `dir` durable before the next step depends on them.
///
/// The order of the journal's steps is what makes it work, and a rename is
/// not on the device until its directory is. Unix opens a directory as a file
/// for this; Windows does not, and orders its metadata writes itself.
fn sync(dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    std::fs::File::open(dir)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = dir;
    Ok(())
}

/// Whether `name` is one a save writes: three digits, a dot, one of the
/// three suffixes. A note naming anything else was not written here.
fn is_save_file(name: &str) -> bool {
    name.split_once('.').is_some_and(|(id, suffix)| {
        id.len() == 3 && id.bytes().all(|b| b.is_ascii_digit()) && SUFFIXES.contains(&suffix)
    })
}

impl Journal {
    /// Opens a save in `dir`. A save a previous run left open there is
    /// rolled back first.
    pub(crate) fn open(dir: &Path) -> std::io::Result<Self> {
        settle(dir)?;
        let journal = Journal {
            dir: dir.to_path_buf(),
            entries: Vec::new(),
        };
        journal.write_note()?;
        Ok(journal)
    }

    /// Takes `name` into the save before a word replaces it: noted first,
    /// then the old file moved aside.
    ///
    /// Only the names a save writes go into a note, since only those are
    /// read back from one.
    pub(crate) fn enlist(&mut self, name: &str) -> std::io::Result<()> {
        if !is_save_file(name) {
            return Err(std::io::Error::new(
                ErrorKind::InvalidInput,
                format!("{name} is not a name a save writes"),
            ));
        }
        if self.entries.iter().any(|e| e.name == name) {
            return Ok(());
        }
        let path = self.dir.join(name);
        let existed = path.exists();
        self.entries.push(Entry {
            name: name.to_string(),
            existed,
        });
        self.write_note()?;
        if existed {
            std::fs::rename(&path, old(&self.dir, name))?;
            sync(&self.dir)?;
        }
        Ok(())
    }

    /// Closes the save: what it wrote is now what the directory holds.
    pub(crate) fn close(self) -> std::io::Result<()> {
        sync(&self.dir)?;
        remove(&self.dir.join(NOTE))?;
        sync(&self.dir)?;
        for entry in &self.entries {
            remove(&old(&self.dir, &entry.name))?;
        }
        Ok(())
    }

    /// Rolls the save back now, in this run.
    pub(crate) fn abandon(self) -> std::io::Result<()> {
        settle(&self.dir).map(|_| ())
    }

    fn write_note(&self) -> std::io::Result<()> {
        let mut text = format!("{HEAD}\n");
        for entry in &self.entries {
            let state = if entry.existed { "old" } else { "new" };
            let _ = writeln!(text, "{state} {}", entry.name);
        }
        super::write_atomically(&self.dir.join(NOTE), text.as_bytes())?;
        sync(&self.dir)
    }
}

/// Rolls back a save that was opened in `dir` and never closed, and answers
/// whether there was one.
///
/// Without a note, an `.old` file left beside a save file is from a save that
/// did close and stopped before tidying up, and is removed.
pub(crate) fn settle(dir: &Path) -> std::io::Result<bool> {
    let path = dir.join(NOTE);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            for entry in std::fs::read_dir(dir)? {
                let name = entry?.file_name();
                if let Some(base) = name.to_str().and_then(|n| n.strip_suffix(".old"))
                    && is_save_file(base)
                {
                    remove(&dir.join(&name))?;
                }
            }
            return Ok(false);
        }
        Err(e) => return Err(e),
    };
    let entries = read_note(&text).ok_or_else(|| {
        std::io::Error::new(
            ErrorKind::InvalidData,
            format!("{} is not a note this program wrote", path.display()),
        )
    })?;
    for entry in entries {
        let moved = old(dir, &entry.name);
        if entry.existed {
            // Not moved yet when the save stopped: still the old file.
            if moved.exists() {
                std::fs::rename(&moved, dir.join(&entry.name))?;
            }
        } else {
            remove(&dir.join(&entry.name))?;
        }
    }
    sync(dir)?;
    remove(&path)?;
    Ok(true)
}

/// The files a note names, or `None` for a file that is not a note.
fn read_note(text: &str) -> Option<Vec<Entry>> {
    let mut lines = text.lines();
    if lines.next() != Some(HEAD) {
        return None;
    }
    lines
        .map(|line| {
            let (state, name) = line.split_once(' ')?;
            let existed = match state {
                "old" => true,
                "new" => false,
                _ => return None,
            };
            is_save_file(name).then(|| Entry {
                name: name.to_string(),
                existed,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The files of a save in the order the words write them, numbered the
    /// way Victor Loomes numbers them: the block apart from the other two.
    const FILES: [&str; 3] = ["701.blk", "001.anm", "001.FRZ"];

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("motionvm-journal-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        dir
    }

    fn write(dir: &Path, generation: &str, names: &[&str]) {
        for name in names {
            std::fs::write(dir.join(name), format!("{generation} {name}")).expect("writes");
        }
    }

    /// Every file in the directory with what it holds.
    fn listing(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .expect("lists")
            .map(|e| {
                e.expect("an entry")
                    .file_name()
                    .into_string()
                    .expect("utf-8")
            })
            .collect();
        names.sort();
        names
            .into_iter()
            .map(|n| {
                let body = std::fs::read_to_string(dir.join(&n)).expect("reads");
                format!("{n}: {body}")
            })
            .collect()
    }

    fn generation(generation: &str) -> Vec<String> {
        let mut names = FILES.map(|n| format!("{n}: {generation} {n}")).to_vec();
        names.sort();
        names
    }

    /// The words of a save, the first `written` of them run: each enlists its
    /// file and writes it.
    fn save(dir: &Path, written: usize) -> Journal {
        let mut journal = Journal::open(dir).expect("opens");
        for name in FILES.get(..written).expect("a prefix") {
            journal.enlist(name).expect("enlists");
            write(dir, "new", &[name]);
        }
        journal
    }

    /// A save stopped after any number of its three words leaves the old
    /// files, whole, once the directory is next taken into use.
    #[test]
    fn a_save_stopped_after_any_word_settles_back_to_the_old_one() {
        for written in 0..=FILES.len() {
            let dir = scratch(&format!("stop-{written}"));
            write(&dir, "old", &FILES);
            drop(save(&dir, written));
            assert!(settle(&dir).expect("settles"));
            assert_eq!(listing(&dir), generation("old"), "after {written}");
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    /// A file is noted before it is moved, and a stop between the two is
    /// settled the same: a noted file without an `.old` is still the old one.
    #[test]
    fn a_save_stopped_between_noting_and_moving_settles_back() {
        let dir = scratch("between");
        write(&dir, "old", &FILES);
        let mut journal = save(&dir, 1);
        journal.entries.push(Entry {
            name: "001.anm".into(),
            existed: true,
        });
        journal.write_note().expect("noted, not moved");
        drop(journal);
        assert!(settle(&dir).expect("settles"));
        assert_eq!(listing(&dir), generation("old"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A closed save is what the directory holds, with nothing beside it.
    #[test]
    fn a_closed_save_is_the_new_one() {
        let dir = scratch("closed");
        write(&dir, "old", &FILES);
        save(&dir, FILES.len()).close().expect("closes");
        assert!(!settle(&dir).expect("settles"));
        assert_eq!(listing(&dir), generation("new"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A first save that never closed leaves nothing, so the menu does not
    /// offer a slot that cannot load.
    #[test]
    fn an_unfinished_first_save_leaves_nothing() {
        let dir = scratch("first");
        drop(save(&dir, 2));
        assert!(settle(&dir).expect("settles"));
        assert!(listing(&dir).is_empty(), "{:?}", listing(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A save abandoned in the run that opened it is rolled back at once.
    #[test]
    fn an_abandoned_save_is_rolled_back_in_the_same_run() {
        let dir = scratch("abandoned");
        write(&dir, "old", &FILES);
        save(&dir, 2).abandon().expect("rolls back");
        assert_eq!(listing(&dir), generation("old"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A save that closed but stopped before removing the old files has them
    /// removed, and its own kept. A file that only looks like one is left.
    #[test]
    fn old_files_without_a_note_are_left_over_and_removed() {
        let dir = scratch("tidy");
        write(&dir, "new", &FILES);
        std::fs::write(old(&dir, "001.anm"), "old 001.anm").expect("a left-over");
        std::fs::write(dir.join("notes.old"), "a player's").expect("not ours");
        assert!(!settle(&dir).expect("settles"));
        let mut expected = generation("new");
        expected.push("notes.old: a player's".into());
        assert_eq!(listing(&dir), expected);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A name no save writes is refused before anything is noted or moved.
    #[test]
    fn a_name_no_save_writes_is_not_enlisted() {
        let dir = scratch("refused");
        let mut journal = Journal::open(&dir).expect("opens");
        for name in ["1701.blk", "-01.blk", "701.txt", "../701.blk"] {
            assert!(journal.enlist(name).is_err(), "{name}");
        }
        journal.close().expect("closes");
        assert!(listing(&dir).is_empty(), "{:?}", listing(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A file under the note's name that this program did not write, or one
    /// naming a file no save writes, is refused, and nothing moves.
    #[test]
    fn a_note_that_is_not_one_is_refused() {
        for text in ["old 701.blk\n", &format!("{HEAD}\nold ../elsewhere\n")] {
            let dir = scratch("foreign");
            write(&dir, "old", &FILES);
            std::fs::write(dir.join(NOTE), text).expect("a stray file");
            assert!(settle(&dir).is_err(), "{text:?}");
            let mut expected = generation("old");
            expected.push(format!("{NOTE}: {text}"));
            expected.sort();
            assert_eq!(listing(&dir), expected);
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}
