//! What can go wrong reading the game's files.
//!
//! Every variant is reached from data on disk, which is data this project does
//! not control: a truncated copy, a bad transfer, a file that is not what its
//! extension says. None of them is a bug in the caller, and none of them may
//! panic — a reader that panics on malformed input turns a damaged install into
//! a crash report about this crate.

use std::fmt;

/// The result of every parse in this crate.
pub type Result<T> = std::result::Result<T, Error>;

/// What can go wrong reading the game's files.
///
/// `#[non_exhaustive]`: the set grows as formats are read, and a caller that
/// matched every variant would break on each new one. Match what you handle
/// and let the rest fall to a catch-all.
#[non_exhaustive]
#[derive(Debug)]
pub enum Error {
    /// A read ran past the end of the buffer.
    Truncated {
        /// Where the read started.
        off: usize,
        /// How many bytes it wanted.
        need: usize,
        /// How many were left.
        have: usize,
    },
    /// A structure that is present but does not hold together.
    ///
    /// `what` names the reader that gave up — "RIB", "FGROUP", "RDV room",
    /// "hotspot table" — because this variant is shared between all of them.
    /// A message that named one reader for all of them would be actively
    /// misleading: it would send someone chasing a container when the room is
    /// what failed.
    Corrupt {
        /// The kind of file being read.
        what: &'static str,
        /// What was wrong with it.
        detail: String,
    },
    /// A field that describes the shape of the data holds a value the format
    /// does not allow.
    OutOfRange {
        /// The field's name in the format documentation.
        field: &'static str,
        /// What it held.
        value: u64,
        /// What it is allowed to hold, as prose: "at most 320", "16 or 32".
        allowed: &'static str,
    },
    /// The RIB LZ decoder ran out of input, wrote past the buffer, or asked
    /// for a match that does not fit.
    Lz(LzError),
    /// A picture or cel declared dimensions that do not match its payload.
    SizeMismatch {
        /// Width from the header.
        width: u16,
        /// Height from the same.
        height: u16,
        /// Bytes the blob actually held.
        have: usize,
        /// Bytes `4+w*h` or `8+w*h` asked for.
        need: usize,
    },
    /// The file could not be read at all.
    Io(std::io::Error),
}

/// What can go wrong unpacking one RIB LZ stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LzError {
    /// The compressed payload was consumed before the output pointer met it.
    UnexpectedEof {
        /// Bytes still unwritten when input ran out.
        remaining: usize,
    },
    /// A match distance reached before the start of the buffer.
    BadDistance {
        /// The distance the token asked for.
        dist: usize,
        /// Where the output pointer sat.
        dst: usize,
    },
    /// A token asked for a run that would write before the start of the buffer.
    Underrun {
        /// Bytes the run wanted to write.
        count: usize,
        /// Where the output pointer sat.
        dst: usize,
    },
}

impl fmt::Display for LzError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedEof { remaining } => {
                write!(
                    f,
                    "RIB LZ input exhausted with {remaining} bytes of output still unwritten"
                )
            }
            Self::BadDistance { dist, dst } => {
                write!(
                    f,
                    "RIB LZ match distance {dist} at output {dst} reaches before the buffer"
                )
            }
            Self::Underrun { count, dst } => {
                write!(
                    f,
                    "RIB LZ run of {count} bytes at output {dst} writes before the buffer"
                )
            }
        }
    }
}

impl std::error::Error for LzError {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated { off, need, have } => {
                write!(
                    f,
                    "read of {need} bytes at {off:#x} past end of {have}-byte buffer"
                )
            }
            Self::Corrupt { what, detail } => write!(f, "{what}: {detail}"),
            Self::OutOfRange {
                field,
                value,
                allowed,
            } => write!(f, "{field} is {value}, which is not {allowed}"),
            Self::Lz(e) => write!(f, "{e}"),
            Self::SizeMismatch {
                width,
                height,
                have,
                need,
            } => write!(
                f,
                "image is {width}x{height} ({need} bytes) but the blob is {have} bytes"
            ),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Lz(e) => Some(e),
            _ => None,
        }
    }
}

impl From<LzError> for Error {
    fn from(e: LzError) -> Self {
        Self::Lz(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
