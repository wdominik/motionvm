//! Why a PSM 2 file could not be read, or the music could not come up.
//!
//! Only reachable at set-up and when a song is read: once a
//! [`Player`](crate::Player) exists, nothing on the audio thread returns an
//! error, because there is nobody there to report one to.

use std::fmt;

/// What stopped a PSM 2 file from being read or the driver from coming up.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The driver file is not what it was taken to be. The message says
    /// which check refused it.
    Psm(&'static str),
    /// A read ran past the end of the bytes it was handed.
    Truncated {
        /// Where the read started.
        off: usize,
        /// How many bytes it wanted.
        need: usize,
        /// How many were left.
        have: usize,
    },
    /// The bytes are there and do not say what the format requires.
    Corrupt {
        /// The kind of file being read.
        what: &'static str,
        /// What was wrong with it.
        detail: String,
    },
    /// A sample rate of zero was asked for.
    ///
    /// Its own variant because it is not a damaged file but a caller mistake,
    /// and because the consequence is specific: the tick period works out to
    /// zero and the mixing loop makes no progress, which is a hang rather than
    /// a failure.
    ZeroRate,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Psm(what) => write!(f, "{what}"),
            Self::Truncated { off, need, have } => {
                write!(
                    f,
                    "read of {need} bytes at {off:#x} past end of {have}-byte buffer"
                )
            }
            Self::Corrupt { what, detail } => write!(f, "{what}: {detail}"),
            Self::ZeroRate => write!(f, "a sample rate of zero"),
        }
    }
}

impl std::error::Error for Error {}

/// This crate's result type.
pub type Result<T> = std::result::Result<T, Error>;
