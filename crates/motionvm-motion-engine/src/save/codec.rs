//! The grammar every savegame file is written in: a head with a magic, the
//! layout version and a checksum over the body; a body of tagged chunks; a
//! reader that refuses by name before a byte is used, and the writer it
//! reads back.

use super::VERSION;
use motionvm_motion_forth::cell;
use std::collections::BTreeMap;

// The body's checksum is `motionvm_render::crc32` — the reflected IEEE
// 802.3 polynomial PNG, zip and gzip use, which the render crate carries for
// its own files. One algorithm, one copy; it runs twice in the life of a
// savegame, once written and once read, over a few hundred kilobytes at most.
pub(super) use motionvm_render::crc32;

/// This module's own result, and the one place in the crate where an error is
/// a plain message.
///
/// Deliberate, and not the crate's [`crate::Error`] in miniature. Every caller
/// is a savegame word — `PUT`, `GET`, `PUTANIM`, `GETANIM`, `=>PUTAS`,
/// `=>GETAS` — reached through the machine, and the machine's own
/// `motionvm_motion_forth::Error::Unsupported` carries exactly a `String`. So
/// `words/saves.rs` converts with `.map_err(Error::Unsupported)` and nothing
/// else ever sees the value: an enum here would be flattened one call later,
/// into a variant that exists to carry a sentence.
///
/// The sentence is what matters. Every message opens with `what` — the file
/// and the word that asked for it, `GETANIM 3` or `701.FRZ` — and then says
/// what would not read: "module 907 appears twice, as record 4 and 11". The
/// person reading one has a savegame that will not load and needs to know
/// which part of it is wrong.
///
/// This carried no comment for three releases, because `missing_docs = "deny"`
/// does not reach `pub(crate)` items. The lint cannot be the only thing that
/// makes a comment appear.
pub(crate) type Result<T> = std::result::Result<T, Error>;

/// What can be wrong with a savegame file.
///
/// A struct and not six variants each repeating the context: every one of
/// these is "while reading *this*, *that* was wrong", and the two halves are
/// separable. `what` is the caller's name for the read — `GETANIM 701`, the
/// slot's path — and reaches the message as its prefix, which is where it was
/// when all six were built with `format!`.
///
/// The sentences are unchanged from when they were strings, and deliberately:
/// they are what a player sees when a slot is refused, and the suite pins
/// them.
#[derive(Debug)]
pub(crate) struct Error {
    /// What was being read, as the caller names it.
    what: String,
    /// What was wrong with it.
    kind: Kind,
}

/// The ways a savegame can fail to be one.
#[derive(Debug)]
pub(crate) enum Kind {
    /// The file does not open with the magic this layout writes. A different
    /// game's slot lands here, which is why the bytes are quoted.
    NotASavegame {
        /// The eight bytes that were there instead.
        got: [u8; 8],
    },
    /// A slot written by a different build of this program.
    Version {
        /// The version in the file.
        found: u32,
        /// The version this build writes.
        writes: u32,
    },
    /// A length that cannot be a length — a header field read as garbage.
    AbsurdLength,
    /// A read ran past the end of the file.
    Short {
        /// Bytes wanted.
        wanted: usize,
        /// Where the read started.
        at: usize,
        /// How long the file is.
        len: usize,
    },
    /// A string field that is not text.
    NotText {
        /// What the decoder said.
        why: std::string::FromUtf8Error,
    },
    /// Bytes after everything the layout accounts for: the writer and the
    /// reader disagree, which is worth saying out loud.
    Trailing {
        /// How many are left.
        left: usize,
    },
    /// The body does not match the checksum in the header, or does not
    /// reach the length the header claims. Either way the file was damaged
    /// after it was written.
    Torn {
        /// What the header says the body is.
        want: u32,
        /// What the bytes on disk actually come to.
        got: u32,
        /// Which of the two numbers disagreed.
        about: &'static str,
    },
    /// A slot belonging to another game. The directory keeps them apart, so
    /// this is a file that has been moved by hand — worth saying plainly
    /// rather than failing somewhere inside the body.
    WrongGame {
        /// The game named in the file.
        theirs: String,
        /// The game reading it.
        ours: &'static str,
    },
    /// A chunk this layout requires is not in the file.
    MissingChunk {
        /// Its four-byte tag.
        tag: &'static str,
    },
    /// One tag twice. A reader that took the second would silently ignore the
    /// first, so it says so instead.
    DuplicateChunk {
        /// The tag that appears twice.
        tag: String,
    },
    /// Two records for one module, which no writer of this layout produces.
    DuplicateModule {
        /// The module named twice.
        module: u32,
        /// The record that named it first.
        first: usize,
        /// The one that named it again.
        second: usize,
    },
}

impl Error {
    /// An error about `what`.
    pub(crate) fn new(what: impl Into<String>, kind: Kind) -> Self {
        Self {
            what: what.into(),
            kind,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: ", self.what)?;
        match &self.kind {
            Kind::NotASavegame { got } => write!(f, "not a savegame file ({got:02x?})"),
            Kind::Version { found, writes } => {
                write!(f, "savegame version {found}, this build writes {writes}")
            }
            Kind::AbsurdLength => write!(f, "absurd length"),
            Kind::Short { wanted, at, len } => {
                write!(
                    f,
                    "wanted {wanted} bytes at {at} but the file is {len} long"
                )
            }
            Kind::NotText { why } => write!(f, "{why}"),
            Kind::Torn { want, got, about } => write!(
                f,
                "damaged after it was written: the header says {about} {want}, \
                 the file has {got}"
            ),
            Kind::WrongGame { theirs, ours } => {
                write!(f, "a savegame of {theirs}, and this is {ours}")
            }
            Kind::MissingChunk { tag } => write!(f, "no {tag} section"),
            Kind::DuplicateChunk { tag } => write!(f, "two {tag} sections"),
            Kind::Trailing { left } => write!(f, "{left} bytes left over after reading"),
            Kind::DuplicateModule {
                module,
                first,
                second,
            } => write!(
                f,
                "module {module} appears twice, as record {first} and {second}"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// A little-endian reader that refuses to run off the end.
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
    what: String,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8], what: impl Into<String>) -> Self {
        Self {
            bytes,
            at: 0,
            what: what.into(),
        }
    }

    /// The eight bytes the file has to open with.
    pub(crate) fn magic(&mut self, want: &[u8; 8]) -> Result<()> {
        let got = self.take(8)?;
        if got != want {
            let got: [u8; 8] = got.try_into().unwrap_or_default();
            return Err(Error::new(&self.what, Kind::NotASavegame { got }));
        }
        Ok(())
    }

    /// The version after the magic, which has to be [`VERSION`]: there is
    /// one layout, and a file of any other version is refused by name.
    pub(crate) fn version(&mut self) -> Result<()> {
        let found = self.u32()?;
        if found == VERSION {
            return Ok(());
        }
        Err(Error::new(
            &self.what,
            Kind::Version {
                found,
                writes: VERSION,
            },
        ))
    }

    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .ok_or_else(|| Error::new(&self.what, Kind::AbsurdLength))?;
        let out = self.bytes.get(self.at..end).ok_or_else(|| {
            Error::new(
                &self.what,
                Kind::Short {
                    wanted: n,
                    at: self.at,
                    len: self.bytes.len(),
                },
            )
        })?;
        self.at = end;
        Ok(out)
    }

    /// The next `N` bytes as an array — what a fixed-size field is.
    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let bytes = self.take(N)?;
        // `take` answers exactly `N` bytes or an error, so the chunk is there;
        // the fallback is the shape the type wants, not a case that happens.
        Ok(bytes.first_chunk::<N>().copied().unwrap_or([0; N]))
    }

    pub(crate) fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub(crate) fn i32(&mut self) -> Result<i32> {
        Ok(cell::signed(self.u32()?))
    }

    pub(crate) fn u8(&mut self) -> Result<u8> {
        Ok(self.array::<1>()?[0])
    }

    /// A length-prefixed string, for the few descriptor fields that carry one.
    pub(crate) fn string(&mut self) -> Result<String> {
        let n = cell::index(self.u32()?);
        let bytes = self.take(n)?;
        String::from_utf8(bytes.to_vec())
            .map_err(|why| Error::new(&self.what, Kind::NotText { why }))
    }

    /// Whether everything has been read.
    fn done(&self) -> bool {
        self.at == self.bytes.len()
    }

    /// Whether the file has been consumed exactly. Anything left over means the
    /// writer and the reader disagree, which is worth saying out loud.
    pub(crate) fn finish(&self) -> Result<()> {
        if self.at != self.bytes.len() {
            return Err(Error::new(
                &self.what,
                Kind::Trailing {
                    left: self.bytes.len().saturating_sub(self.at),
                },
            ));
        }
        Ok(())
    }
}

/// A little-endian writer, the mirror of [`Reader`].
#[derive(Default)]
pub(crate) struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    pub(crate) fn u32(&mut self, v: u32) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    pub(crate) fn i32(&mut self, v: i32) {
        self.u32(cell::unsigned(v));
    }

    pub(crate) fn u8(&mut self, v: u8) {
        self.bytes.push(v);
    }

    pub(crate) fn string(&mut self, s: &str) {
        self.u32(cell::narrow(s.len()));
        self.bytes.extend_from_slice(s.as_bytes());
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Appends bytes that are already laid out.
    fn raw(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    /// One section of a body: a four-byte tag, its length, and its bytes.
    ///
    /// The length is what lets a reader step over a section it does not know,
    /// which is how a later build adds one without moving the version. Inside
    /// a section the layout is fixed for the version that defines it — a
    /// known section with bytes left over is a reader and a writer
    /// disagreeing, and says so — so growth is a new section and never a
    /// field appended to an old one.
    pub(super) fn chunk(&mut self, tag: &[u8; 4], write: impl FnOnce(&mut Writer)) {
        let mut inner = Writer::default();
        write(&mut inner);
        self.raw(tag);
        self.u32(cell::narrow(inner.bytes.len()));
        self.raw(&inner.bytes);
    }
}

/// The sections of a body, by tag, with the file's name for its errors.
pub(super) struct Chunks<'a> {
    what: String,
    sections: BTreeMap<[u8; 4], &'a [u8]>,
}

impl<'a> Chunks<'a> {
    /// A section this layout requires.
    pub(super) fn take(&self, tag: &'static [u8; 4]) -> Result<Reader<'a>> {
        let name = std::str::from_utf8(tag).unwrap_or("????");
        let bytes = self
            .sections
            .get(tag)
            .ok_or_else(|| Error::new(&self.what, Kind::MissingChunk { tag: name }))?;
        Ok(Reader::new(bytes, format!("{}, {name}", self.what)))
    }

    /// A section this layout writes only sometimes.
    pub(super) fn option(&self, tag: &[u8; 4]) -> Option<Reader<'a>> {
        let name = std::str::from_utf8(tag).unwrap_or("????");
        self.sections
            .get(tag)
            .map(|bytes| Reader::new(bytes, format!("{}, {name}", self.what)))
    }
}

/// Lays a whole savegame file out: the head, then the body the chunks make up.
///
/// ```text
/// [8] magic          the generation's
/// u32 version        VERSION
/// u32 length         of the body
/// u32 crc32          of the body
/// --- body ---
/// string slug        the game whose slot this is
/// string wrote       the motionvm version that wrote it
/// chunks to the end
/// ```
///
/// The generation is in the magic and is not repeated below it: two fields
/// that say the same thing can disagree, and then a reader has to decide
/// which to believe. What the magic cannot say is *which game* — all five
/// 16-bit games write `ENVFRZ` — so the slug is in the body, where the
/// checksum covers it.
///
/// `wrote` is never read back. It is there for the person holding a slot that
/// will not load, who needs to know which build made it before anything else
/// can be worked out.
pub(super) fn lay_out(magic: &[u8; 8], slug: &str, chunks: &Writer) -> Vec<u8> {
    let mut body = Writer::default();
    body.string(slug);
    body.string(env!("CARGO_PKG_VERSION"));
    body.raw(chunks.bytes());

    let mut out = Writer::default();
    out.raw(magic);
    out.u32(VERSION);
    out.u32(cell::narrow(body.bytes.len()));
    out.u32(crc32(body.bytes()));
    out.raw(body.bytes());
    out.bytes
}

/// Opens a file past its magic and version: checks that it is whole and that
/// it is this game's, and answers its sections.
///
/// The order is the order the checks are worth doing in. The length and the
/// checksum come first, because a torn file can say anything at all about
/// itself; the game's name second, because a slot from another game is a
/// mistake with an obvious remedy and everything after it would be noise.
pub(super) fn open_body<'a>(
    r: &mut Reader<'a>,
    what: &str,
    slug: &'static str,
) -> Result<Chunks<'a>> {
    let length = cell::index(r.u32()?);
    let want = r.u32()?;
    let body = r.take(length)?;
    r.finish()?;
    let got = crc32(body);
    if got != want {
        return Err(Error::new(
            what,
            Kind::Torn {
                want,
                got,
                about: "a checksum of",
            },
        ));
    }

    let mut b = Reader::new(body, what);
    let theirs = b.string()?;
    if theirs != slug {
        return Err(Error::new(what, Kind::WrongGame { theirs, ours: slug }));
    }
    let _wrote = b.string()?;

    let mut sections = BTreeMap::new();
    while !b.done() {
        let tag: [u8; 4] = b.array()?;
        let n = cell::index(b.u32()?);
        let bytes = b.take(n)?;
        if sections.insert(tag, bytes).is_some() {
            return Err(Error::new(
                what,
                Kind::DuplicateChunk {
                    tag: String::from_utf8_lossy(&tag).into_owned(),
                },
            ));
        }
    }
    Ok(Chunks {
        what: what.to_owned(),
        sections,
    })
}

/// An optional value, written as a present flag and then the value.
impl Writer {
    pub(crate) fn option_i32(&mut self, v: Option<i32>) {
        self.u8(u8::from(v.is_some()));
        self.i32(v.unwrap_or(0));
    }
}

impl Reader<'_> {
    pub(crate) fn option_i32(&mut self) -> Result<Option<i32>> {
        let present = self.u8()? != 0;
        let value = self.i32()?;
        Ok(present.then_some(value))
    }
}

pub(super) fn r_u16(r: &mut Reader<'_>) -> Result<u16> {
    Ok(cell::low16(r.i32()?))
}

pub(super) fn r_i16(r: &mut Reader<'_>) -> Result<i16> {
    Ok(cell::short(r.i32()?))
}
