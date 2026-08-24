//! Why the music could not be started.
//!
//! Only reachable at set-up: once a [`Player`](crate::Player) exists, nothing
//! on the audio thread returns an error, because there is nobody there to
//! report one to. What that path does instead is documented on the functions
//! that do it — a malformed song loses a track, an out-of-range velocity is
//! masked, and both are recorded rather than raised.

use std::fmt;

/// What stopped the FM driver from coming up.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// One of the driver's four tables is not inside the image.
    ///
    /// The addresses are fixed constants read out of `HMIMDRV.386`, so this
    /// means the image is not the driver it was taken to be — a different
    /// version, or a different device out of the same archive.
    MissingTable {
        /// Which table: `"frequency"`, `"operators"`, `"velocity"`,
        /// `"octave_down"`.
        name: &'static str,
        /// Where it should have been.
        at: usize,
        /// How many bytes it needs.
        need: usize,
        /// How large the image is.
        have: usize,
    },
    /// A PSM 2 file — the music driver or a module — is not what it was
    /// taken to be. The message says which check refused it.
    Psm(&'static str),
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
            Self::MissingTable {
                name,
                at,
                need,
                have,
            } => write!(
                f,
                "the driver's {name} table needs {need} bytes at {at:#x}, \
                 but the image is only {have:#x} bytes"
            ),
            Self::Psm(what) => write!(f, "{what}"),
            Self::ZeroRate => write!(f, "a sample rate of zero"),
        }
    }
}

impl std::error::Error for Error {}

/// This crate's result type.
pub type Result<T> = std::result::Result<T, Error>;
