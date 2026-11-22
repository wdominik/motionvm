//! What can go wrong between a directory on disk and a game that steps.
//!
//! The reader below this crate has its own type, [`motionvm_parsec_formats::Error`],
//! for a file that will not read as what it claims; this is the one at the
//! engine's boundary, which wraps it. It is `Send + Sync`, so a host may
//! carry it wherever it carries the game, and it has a variant per case a
//! caller may want to tell apart rather than a message to match on.
//!
//! **`Display` is the text a player reads** — in the window's dialog, on the
//! command line — and the suite pins it character for character. These are
//! read by a person who has just copied Dunkle Schatten's disks and has no way to guess
//! which of its files mattered. The enum exists so that a caller can *also*
//! branch on the case; it is not a license to reword the cases.

use std::fmt;

use motionvm_parsec_formats::Title;

/// The result of opening and driving a game.
pub type Result<T> = std::result::Result<T, Error>;

/// What went wrong opening or driving a game.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A directory that does not hold the game.
    ///
    /// `missing` is what [`crate::missing_data`] reported, in its order, as
    /// `(what, what for)`; it is empty when the caller only identified.
    Unrecognized {
        /// What the directory does not hold.
        missing: Vec<(&'static str, &'static str)>,
    },
    /// The game's directory with files missing from it.
    Incomplete {
        /// The game the directory holds.
        title: Title,
        /// What it does not hold, as `(what, what for)`, in the order the
        /// game lists its needs.
        missing: Vec<(&'static str, &'static str)>,
    },
    /// A shipped file that would not read as what it has to be.
    Data(motionvm_parsec_formats::Error),
    /// The file system refused: a file that would not open, a directory that
    /// could not be made.
    Io(std::io::Error),
    /// A file or the program image did not hold what this engine reads from
    /// it: a table that is not where the program's own code says, a row that
    /// does not end, a value outside the range the program allows. The
    /// message says which and where.
    Unexpected(String),
    /// A save directory inside the install, which is treated as read-only.
    SavesInsideGame,
    /// A second start: play begins once.
    AlreadyStarted,
    /// An arrival point the game's 16-bit entry word cannot hold.
    NoSuchEntry {
        /// The arrival point asked for.
        entry: i32,
    },
}

/// One `missing what (what for)` line per entry, each on a line of its own.
fn missing_lines(f: &mut fmt::Formatter<'_>, missing: &[(&str, &str)]) -> fmt::Result {
    for (what, why) in missing {
        write!(f, "\n  missing {what} ({why})")?;
    }
    Ok(())
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unrecognized { missing } => {
                f.write_str("not a PARSEC game directory")?;
                missing_lines(f, missing)
            }
            Error::Incomplete { title, missing } => {
                write!(f, "incomplete {} directory:", title.name())?;
                missing_lines(f, missing)
            }
            Error::Data(e) => write!(f, "{e}"),
            Error::Io(e) => write!(f, "{e}"),
            Error::Unexpected(msg) => f.write_str(msg),
            Error::SavesInsideGame => f.write_str("refusing a save directory inside the game data"),
            Error::AlreadyStarted => f.write_str("the game has already started"),
            Error::NoSuchEntry { entry } => write!(
                f,
                "there is no arrival point {entry}: a PARSEC game numbers them \
                 from -32768 to 32767"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Data(e) => Some(e),
            Error::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<motionvm_parsec_formats::Error> for Error {
    fn from(e: motionvm_parsec_formats::Error) -> Self {
        Error::Data(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

/// The reads of the program image and of the files state what they found
/// in a sentence of their own, and that sentence is the message.
impl From<String> for Error {
    fn from(msg: String) -> Self {
        Error::Unexpected(msg)
    }
}

/// A value read from the program that does not fit the width its use has:
/// the conversion's own words.
impl From<std::num::TryFromIntError> for Error {
    fn from(e: std::num::TryFromIntError) -> Self {
        Error::Unexpected(e.to_string())
    }
}

impl From<&str> for Error {
    fn from(msg: &str) -> Self {
        Error::Unexpected(msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compiles only for a type a host may keep anywhere it keeps the game.
    fn send_sync<T: Send + Sync + 'static>() {}

    #[test]
    fn the_error_may_cross_threads() {
        send_sync::<Error>();
    }

    /// The reader's and the file system's messages come through bare: the
    /// wrapper adds no prefix of its own.
    #[test]
    fn a_wrapped_error_reads_as_the_one_it_wraps() {
        let io = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        assert_eq!(Error::from(io).to_string(), "no such file");
        let data = motionvm_parsec_formats::Error::Truncated {
            off: 4,
            need: 2,
            have: 5,
        };
        let want = data.to_string();
        assert_eq!(Error::from(data).to_string(), want);
        assert_eq!(
            Error::from(format!("{:#x}: a rule row with no end", 0x40)).to_string(),
            "0x40: a rule row with no end"
        );
    }
}
