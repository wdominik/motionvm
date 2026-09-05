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
    /// `what` names the reader that gave up — "RSC container", "script module",
    /// "font", "LE image" — because this variant is shared between all of them.
    /// A message that named one reader for all of them would be actively
    /// misleading: it would send someone chasing a container when the font is
    /// what failed.
    Corrupt {
        /// The kind of file being read.
        what: &'static str,
        /// What was wrong with it.
        detail: String,
    },
    /// A field that describes the shape of the data holds a value the format
    /// does not allow.
    ///
    /// Separate from [`Error::Corrupt`] because these are the fields that get
    /// used as a shift distance or an allocation size before anything else is
    /// read, so they have to be rejected on the spot rather than survive as far
    /// as the first bounds check.
    OutOfRange {
        /// The field's name in the format documentation.
        field: &'static str,
        /// What it held.
        value: u64,
        /// What it is allowed to hold, as prose: "11 or 12", "at most 0x2000".
        allowed: &'static str,
    },
    /// A resource id was outside the slot range declared by the header.
    IdOutOfRange {
        /// The resource kind, as the container names it.
        kind: &'static str,
        /// The id that was asked for.
        id: usize,
        /// How many slots that kind declares.
        count: usize,
    },
    /// A sprite did not carry the `32BITGFX` marker at the expected offset.
    MissingGfxMagic {
        /// The eight bytes that were there instead.
        found: [u8; 8],
    },
    /// The LZW stream ended or desynchronized before producing the declared size.
    Lzw(lzw::LzwError),
    /// A sprite's inner dimensions disagreed with its declared unpacked size.
    GfxSizeMismatch {
        /// Width from the inner header.
        width: u16,
        /// Height from the same.
        height: u16,
        /// The unpacked size the outer header declared.
        declared: usize,
    },
    /// An instrument bank did not carry the `ADLIB-` signature at offset 2.
    MissingBankSignature {
        /// The six bytes that were there instead.
        found: [u8; 6],
    },
    /// A bank's two table offsets do not bracket a table inside the file.
    BankTablesOutOfRange {
        /// Declared offset of the name table.
        names: usize,
        /// Declared offset of the instrument records.
        data: usize,
        /// The file's actual length.
        have: usize,
    },
    /// A `.386` archive's header claimed a size other than 44 bytes.
    DriverArchiveHeader {
        /// The size it claimed.
        size: usize,
    },
    /// Walking a `.386` archive's record chain did not land on the file end.
    DriverArchiveChain {
        /// Where the chain ended up.
        end: usize,
        /// Where the file ends.
        have: usize,
    },
    /// A song did not carry `HMI-MIDISONG061595`.
    MissingSongMagic {
        /// The eighteen bytes that were there instead.
        found: [u8; 18],
    },
    /// A track offset did not point at `HMI-MIDITRACK`.
    MissingTrackMagic {
        /// The offset the track header was expected at.
        at: usize,
        /// The thirteen bytes that were there instead.
        found: [u8; 13],
    },
    /// The event stream held a status the sequencer does not implement.
    UnknownSongEvent {
        /// Where it sat, from the start of the track record.
        at: usize,
        /// The status byte.
        status: u8,
        /// Its sub-status, for the statuses that carry one.
        sub: u8,
    },
    /// The file could not be read at all.
    Io(std::io::Error),
}

/// The LZW variant the game's graphics are packed with.
///
/// Not the standard one: see [`lzw::LzwError::BadWidth`] and the note in
/// `gfx.rs` on the bump code.
pub mod lzw {
    use std::fmt;

    /// What can go wrong unpacking one stream.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LzwError {
        /// Input exhausted before `expected` bytes had been produced.
        UnexpectedEof {
            /// Bytes unpacked before the input ran out.
            produced: usize,
            /// Bytes the header declared.
            expected: usize,
        },
        /// A code arrived that is neither in the dictionary nor the next free slot.
        BadCode {
            /// The code that was read.
            code: u16,
            /// The next code the table would have handed out.
            next: usize,
            /// Bytes unpacked so far.
            produced: usize,
        },
        /// Decoding produced more bytes than declared.
        Overrun {
            /// Bytes unpacked.
            produced: usize,
            /// Bytes the header declared.
            expected: usize,
        },
        /// The header's dictionary ceiling is not a width this codec can use.
        ///
        /// Checked before anything is allocated, because the value arrives from
        /// the file and is used as a shift distance: `1 << max_bits` with a
        /// `max_bits` of 40 asks for a terabyte, and with 64 it is not a shift
        /// at all. GFXCRUNCH uses 11 or 12 and the initial code width is 9, so
        /// nothing outside that range was ever written by the original.
        BadWidth {
            /// The width the stream asked for. Anything past 16 would shift a
            /// code out of its own type, so it is refused rather than clamped.
            max_bits: u32,
        },
    }

    impl fmt::Display for LzwError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::UnexpectedEof { produced, expected } => {
                    write!(
                        f,
                        "LZW input exhausted after {produced} of {expected} bytes"
                    )
                }
                Self::BadCode {
                    code,
                    next,
                    produced,
                } => write!(
                    f,
                    "LZW code {code} out of range (next free is {next}) after {produced} bytes"
                ),
                Self::Overrun { produced, expected } => {
                    write!(f, "LZW produced {produced} bytes, expected {expected}")
                }
                Self::BadWidth { max_bits } => {
                    write!(
                        f,
                        "LZW dictionary ceiling of {max_bits} bits, expected 9 to 12"
                    )
                }
            }
        }
    }

    impl std::error::Error for LzwError {}
}

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
            Self::IdOutOfRange { kind, id, count } => {
                write!(f, "{kind} id {id} outside 0..{count}")
            }
            Self::MissingGfxMagic { found } => {
                write!(f, "expected \"32BITGFX\" marker, found {found:02x?}")
            }
            Self::Lzw(e) => write!(f, "{e}"),
            Self::GfxSizeMismatch {
                width,
                height,
                declared,
            } => write!(
                f,
                "sprite is {width}x{height} but stream declared {declared} bytes"
            ),
            Self::MissingBankSignature { found } => {
                write!(f, "expected \"ADLIB-\" signature, found {found:02x?}")
            }
            Self::BankTablesOutOfRange { names, data, have } => write!(
                f,
                "bank tables at {names:#x} and {data:#x} do not fit a {have}-byte file"
            ),
            Self::DriverArchiveHeader { size } => {
                write!(f, "a .386 archive header is {size} bytes, expected 44")
            }
            Self::DriverArchiveChain { end, have } => write!(
                f,
                "the .386 record chain ended at {end:#x}, but the file is {have:#x} bytes"
            ),
            Self::MissingSongMagic { found } => write!(
                f,
                "expected \"HMI-MIDISONG061595\", found {:?}",
                String::from_utf8_lossy(found)
            ),
            Self::MissingTrackMagic { at, found } => write!(
                f,
                "no \"HMI-MIDITRACK\" at {at:#x}, found {:?}",
                String::from_utf8_lossy(found)
            ),
            Self::UnknownSongEvent { at, status, sub } => {
                write!(f, "unknown song event {status:#04x}/{sub:#04x} at {at:#x}")
            }
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {
    /// The wrapped cause, for the two variants that have one.
    ///
    /// Without this an `Error::Io` printed through a chain-walking reporter
    /// loses the `io::Error` underneath it, which is the half that says whether
    /// the file was missing or unreadable.
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Lzw(e) => Some(e),
            _ => None,
        }
    }
}

impl From<lzw::LzwError> for Error {
    fn from(e: lzw::LzwError) -> Self {
        Self::Lzw(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
