//! What can go wrong between a directory on disk and a game that steps.
//!
//! The three crates under this one each have an error type of their own —
//! `motionvm_motion_formats::Error` for a file that will not read as what it claims,
//! `motionvm_motion_forth::Error` for a machine that cannot go on,
//! `motionvm_motion_audio::Error` for the sound path. This crate flattened all of it
//! into `Box<dyn std::error::Error>` and `String` at its own boundary, so a
//! caller could not tell "this directory holds no game" from "this sprite is
//! truncated" without matching on message text.
//!
//! Two channels, and they are different things:
//!
//! - **This type** is the *opener's and driver's*: everything
//!   [`crate::titles::open`], [`crate::Game`] and [`crate::Driven`] answer
//!   with. It is what the family's front door boxes into the contract's
//!   error, and its `Display` output is the message a player reads.
//! - **`motionvm_motion_forth::Error`** is the *machine's*, and stays the machine's.
//!   Every kernel-word handler under `words/` returns it, because that is the
//!   `Host` contract: a word that cannot run has failed on the machine's
//!   terms, not the opener's. [`Error::Machine`] is where the two meet.
//!
//! **Every message here is the one that was written before there was a type
//! to hang it on, character for character.** That is deliberate. These are
//! read by a person who has just copied a 1996 CD and has no way to guess
//! which of its thirty files mattered, and several of them are asserted on by
//! the test suite. The enum exists so that a caller can *also* branch on the
//! case; it is not a license to reword the cases.

use std::fmt;
use std::path::{Path, PathBuf};

/// The opener's and the driver's result.
pub type Result<T> = std::result::Result<T, Error>;

/// What went wrong opening or driving a game.
#[derive(Debug)]
pub enum Error {
    /// The path is not a directory — usually a typo. Listing the files a game
    /// needs would describe the symptom and hide the cause.
    NoSuchDirectory {
        /// The path as it was given.
        dir: PathBuf,
    },
    /// A game directory of the right shape with something missing from it.
    Incomplete {
        /// The directory.
        dir: PathBuf,
        /// The game it was taken for, as a window would show it.
        title: &'static str,
        /// What it does not hold, in the order the game lists its needs.
        missing: Vec<&'static str>,
    },
    /// A container of the right generation holding another game's script.
    ///
    /// Only the 32-bit opener can reach this: a `NNN.RSC` beside an
    /// `ENGINE.EXE` says MOTION 32-bit and no more, so which game it is has to
    /// come out of the data. The 16-bit games are told apart by the engine
    /// binary beside the container.
    NotThisGame {
        /// The directory.
        dir: PathBuf,
        /// The game it was opened as.
        title: &'static str,
        /// The signature word module 2 does not define.
        word: String,
    },
    /// A directory holding none of the games, with what each would need.
    Unrecognized {
        /// The directory.
        dir: PathBuf,
    },
    /// A file the opener asked for by name and did not find.
    ///
    /// Distinct from [`Error::Incomplete`], which is the check made up front:
    /// this is the same file gone between that check and the open, or one the
    /// check does not cover.
    MissingFile {
        /// The directory.
        dir: PathBuf,
        /// The file's name, as the game ships it.
        name: String,
    },
    /// A shipped file that would not read as what it has to be.
    ///
    /// The path is carried because the reader's own message does not have it:
    /// "RSC container: file is only 0 bytes" says what is wrong and not what
    /// it is wrong about.
    Data {
        /// The file, or the directory the reader was pointed at.
        path: PathBuf,
        /// What the reader said.
        source: motionvm_motion_formats::Error,
    },
    /// The container's header names a boot module it does not hold.
    EmptyBootModule {
        /// The container, as it names itself.
        source: String,
        /// The module number in the header.
        module: u16,
    },
    /// The container's boot word is bound to no module the opener loaded.
    UnboundBootWord {
        /// The word id in the header.
        word: u16,
    },
    /// A word the caller named is not in the module it named.
    NoWord {
        /// The module.
        module: u32,
        /// The word.
        name: String,
    },
    /// A variable the caller named is not in the module it named.
    NoVariable {
        /// The module.
        module: u32,
        /// The variable.
        name: String,
    },
    /// A location was asked for of a game whose locations cannot be asked
    /// for: its scripts enter them from a story list, not from a variable a
    /// caller could write.
    NoLocationRequest {
        /// The game, in its short form.
        title: &'static str,
    },
    /// A 16-bit entry point was reached with no container behind it.
    NoContainer,
    /// A word was stepped to the frame budget without finishing.
    Unfinished,
    /// The save directory was refused, or could not be made.
    Saves(String),
    /// The machine could not go on. See the module header: the word handlers'
    /// own channel, surfacing here because the driver stepped them.
    Machine(motionvm_motion_forth::Error),
}

impl Error {
    /// [`Error::MissingFile`], for a name the caller has as a `&str`.
    pub(crate) fn missing_file(dir: &Path, name: &str) -> Self {
        Error::MissingFile {
            dir: dir.to_path_buf(),
            name: name.to_string(),
        }
    }

    /// [`Error::Data`], for a reader that was pointed at `path`.
    pub(crate) fn data(path: &Path, source: motionvm_motion_formats::Error) -> Self {
        Error::Data {
            path: path.to_path_buf(),
            source,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::NoSuchDirectory { dir } => write!(f, "{}: no such directory", dir.display()),
            Error::Incomplete {
                dir,
                title,
                missing,
            } => write!(
                f,
                "{} is not a complete {title} directory\n  missing: {}\n  \
                 This needs the files of an original installation; \
                 see \"What a game needs\" in the README.",
                dir.display(),
                missing.join(", "),
            ),
            Error::NotThisGame { dir, title, word } => write!(
                f,
                "{} does not hold {title}'s script\n  \
                 module 2 has no {word}, a variable this game's own compiler named\n  \
                 This is another MOTION 32-bit game, or an incomplete copy of this one; \
                 see \"What a game needs\" in the README for the files a copy needs.",
                dir.display(),
            ),
            Error::Unrecognized { dir } => {
                writeln!(f, "{} is not a game motionvm can open", dir.display())?;
                for title in crate::Title::ALL {
                    writeln!(f, "  {} needs {}", title.short(), title.needs())?;
                }
                write!(
                    f,
                    "  Another MOTION game, or an incomplete copy of one of these; \
                     see \"What a game needs\" in the README."
                )
            }
            Error::MissingFile { dir, name } => write!(f, "{}: no {name}", dir.display()),
            Error::Data { path, source } => write!(f, "{}: {source}", path.display()),
            Error::EmptyBootModule { source, module } => {
                write!(f, "{source}: the boot module {module} is empty")
            }
            Error::UnboundBootWord { word } => {
                write!(f, "the boot word {word} is bound to no loaded module")
            }
            Error::NoWord { module, name } => write!(f, "module {module} has no word {name}"),
            Error::NoVariable { module, name } => {
                write!(f, "module {module} has no variable {name}")
            }
            Error::NoLocationRequest { title } => {
                write!(
                    f,
                    "{title} enters its locations from a story list; there is no location to ask for"
                )
            }
            Error::NoContainer => write!(f, "the engine holds no 16-bit container"),
            Error::Unfinished => write!(f, "the word never finished"),
            Error::Saves(what) => write!(f, "{what}"),
            Error::Machine(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Data { source, .. } => Some(source),
            Error::Machine(e) => Some(e),
            _ => None,
        }
    }
}

/// The machine's failures reach the driver through `?` and stay themselves.
///
/// There is no `From<motionvm_motion_formats::Error>` beside it on purpose: a reader
/// failing says nothing about *which file*, and the whole point of
/// [`Error::Data`] is to add the path. A bare `?` on a reader would drop it
/// silently, so the conversion has to be written out at every call site.
impl From<motionvm_motion_forth::Error> for Error {
    fn from(e: motionvm_motion_forth::Error) -> Self {
        Error::Machine(e)
    }
}
