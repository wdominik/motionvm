//! Readers for the resource formats of *Dunkle Schatten* (1994), the game
//! N. Schmidt's PARSEC engine runs.
//!
//! PARSEC is not a virtual machine. The game is packed `XP00`/`XP01`/`XP02`
//! executables (compiled C: verbs, puzzles, inventory) plus `RESOURCE/*.DAT`
//! — FGROUP/RIB containers of pictures, rooms, hotspots, dialogues, sprites
//! and PSM 2 music. This crate reads both sides: the containers and what is
//! in them, and the executables' `PRSC` packing, which [`Unpacked`] undoes
//! into the load image the engine reads its tables and texts out of. The
//! puzzle logic is the engine's, expressed from the unpacked `XP01.EXE`.
//!
//! Everything here is derived from the shipped files and the unpacked
//! programs, their code and their strings. Each reader carries the evidence
//! for its format: the offsets it reads, and the shipped files the reading
//! is checked against.

// The readers are this crate's whole surface toward a file a player supplies,
// and a damaged one is refused, never crashed on — `tests/parsec_malformed.rs`
// hands them broken input to check it. These two lints hold that by
// construction rather than by the shipped files: an index has been
// bounds-checked or is a constant, and a sum says what it does at the edge.
// The tests are outside the rule, because a test indexes what it built.
#![cfg_attr(
    not(test),
    deny(clippy::indexing_slicing, clippy::arithmetic_side_effects)
)]

pub mod ani;
mod cursor;
pub mod detect;
pub mod dialogue;
pub mod error;
pub mod exe;
pub mod fgroup;
pub mod font;
pub mod hotspot;
pub mod lz;
pub mod omp;
pub mod p2l;
pub mod pal;
pub mod picture;
pub mod rdv;
pub mod rib;

pub use ani::Animations;
pub use detect::{Title, identify, resource_dir};
pub use dialogue::{Bank, Dialogue};
pub use error::{Error, Result};
pub use exe::Unpacked;
pub use fgroup::{Dat, Fgroup, Slot, open_dat};
pub use font::{Font, Glyph};
pub use hotspot::{Hotspot, HotspotTable, MouseObject};
pub use omp::{MapEntry, ObjectMap};
pub use p2l::P2l;
pub use picture::Picture;
pub use rdv::{MapDefaults, Overlay, Room, RoomFile, RoomLayer, RowParts, WalkGraph, WalkNode};
pub use rib::Rib;

pub(crate) use cursor::Cursor;

/// Four-byte magic of an inner RIB bundle, including the trailing NUL.
pub const MAGIC_RIB: [u8; 4] = *b"RIB\0";
/// Empty FGROUP slot payload. The original writes this literal, CR/LF and
/// all — nine bytes in every place Dunkle Schatten's files hold one; a
/// reader that treated any short slot as empty would also accept a
/// truncated RIB.
pub const NULLPTR: &[u8] = b"NULLPTR";

/// Finds `name` in `dir` whatever case it is stored in.
///
/// The game's files are named in upper case on the disc — `XP01.EXE`,
/// `DSROOM.DAT`, `PANELS.DAT` — but a copy that has been through a CD-ROM
/// driver, an archiver or a file manager very often arrives in lower case.
/// On a case-insensitive filesystem that costs nothing and is invisible; on a
/// case-sensitive one an exact-case `join` simply does not find the file.
///
/// Returns the path as it is actually spelled on disk. An exact match wins
/// over a differently-cased one. `None` when the directory cannot be read or
/// holds nothing by that name. The comparison is ASCII-only, which is all
/// these names are.
///
/// When several spellings are there and none is exact — `dsroom.dat` and
/// `DsRoom.DAT` on a case-sensitive disk — the first in byte order is taken,
/// so the answer is the same on every machine and every run rather than
/// whatever order the directory happens to list its entries in.
pub fn find_ci(dir: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    let exact = dir.join(name);
    if exact.exists() {
        return Some(exact);
    }
    let names = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(str::to_owned));
    first_ci(names, name).map(|n| dir.join(n))
}

/// Of `names`, the first in byte order that is `want` but for ASCII case.
///
/// The choice [`find_ci`] and its directory twin make, apart from the disk
/// so that it can be tested on a filesystem that cannot hold two spellings.
pub(crate) fn first_ci(names: impl Iterator<Item = String>, want: &str) -> Option<String> {
    names.filter(|n| n.eq_ignore_ascii_case(want)).min()
}

/// The refusal for a read of `need` bytes at `off` that `d` does not hold.
pub(crate) fn past_end(d: &[u8], off: usize, need: usize) -> Error {
    Error::Truncated {
        off,
        need,
        have: d.len(),
    }
}

/// `N` bytes at `off`, or `Err` if they would run past the end.
pub(crate) fn bytes<const N: usize>(d: &[u8], off: usize) -> Result<&[u8; N]> {
    d.get(off..)
        .and_then(|rest| rest.first_chunk())
        .ok_or_else(|| past_end(d, off, N))
}

/// `len` bytes at `off`, or `Err` if they would run past the end.
pub(crate) fn slice(d: &[u8], off: usize, len: usize) -> Result<&[u8]> {
    d.get(off..)
        .and_then(|rest| rest.get(..len))
        .ok_or_else(|| past_end(d, off, len))
}

/// Everything from `off` on — nothing, when `off` is the end — or `Err` if
/// `off` is past it.
pub(crate) fn tail(d: &[u8], off: usize) -> Result<&[u8]> {
    d.get(off..).ok_or_else(|| past_end(d, off, 0))
}

/// `count` records of `N` bytes each, back to back from `off`, or `Err` if
/// the last of them would run past the end.
pub(crate) fn records<const N: usize>(d: &[u8], off: usize, count: usize) -> Result<&[[u8; N]]> {
    d.get(off..)
        .and_then(|rest| rest.as_chunks().0.get(..count))
        .ok_or_else(|| past_end(d, off, count.saturating_mul(N)))
}

/// A record read whole, whose fields sit at constant offsets inside it.
///
/// A field is named by its offset and width as constants, and the pair is
/// held inside the record at compile time — which is the proof a bare index
/// into the array cannot give, since a slice index is checked at run time
/// whatever it was computed from.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Record<'a, const LEN: usize>(pub(crate) &'a [u8; LEN]);

impl<const LEN: usize> Record<'_, LEN> {
    /// The `N` bytes at `AT`, which the constants place inside the record.
    #[expect(
        clippy::indexing_slicing,
        clippy::arithmetic_side_effects,
        reason = "held inside the record by the assertion on the constants"
    )]
    pub(crate) fn bytes<const AT: usize, const N: usize>(self) -> [u8; N] {
        const {
            assert!(AT + N <= LEN, "a field lies inside its record");
        }
        std::array::from_fn(|i| self.0[AT + i])
    }

    pub(crate) fn u8<const AT: usize>(self) -> u8 {
        self.bytes::<AT, 1>()[0]
    }

    pub(crate) fn u16<const AT: usize>(self) -> u16 {
        u16::from_le_bytes(self.bytes::<AT, 2>())
    }

    pub(crate) fn i16<const AT: usize>(self) -> i16 {
        i16::from_le_bytes(self.bytes::<AT, 2>())
    }

    pub(crate) fn u32<const AT: usize>(self) -> u32 {
        u32::from_le_bytes(self.bytes::<AT, 4>())
    }

    /// A `u32` field as what it is used for next: an offset into, or a count
    /// of things in, the file it came out of.
    pub(crate) fn u32at<const AT: usize>(self) -> usize {
        wide(self.u32::<AT>())
    }
}

/// Reads a little-endian `u16` at `off`, or `Err` if it would run past the end.
pub(crate) fn u16le(d: &[u8], off: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(*bytes(d, off)?))
}

/// Reads a little-endian `u32` at `off`, or `Err` if it would run past the end.
pub(crate) fn u32le(d: &[u8], off: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(*bytes(d, off)?))
}

/// Reads a little-endian `u32` at `off` as what it is used for next: an
/// offset into, or a count of things in, the file it came out of.
pub(crate) fn u32at(d: &[u8], off: usize) -> Result<usize> {
    u32le(d, off).map(wide)
}

/// The bytes from `off` up to the first NUL, or to the end when there is
/// none; nothing when `off` is 0 or past the end. A C string as the
/// readers' string pools store one, where offset 0 means "none".
pub(crate) fn c_string(d: &[u8], off: usize) -> &[u8] {
    if off == 0 {
        return &[];
    }
    let rest = d.get(off..).unwrap_or_default();
    rest.split(|&b| b == 0).next().unwrap_or_default()
}

/// A 32-bit offset or count read out of a file, as an index into it.
///
/// Lossless on every target this workspace builds for, all of which have a
/// `usize` at least 32 bits wide; `usize` has no `From<u32>` because Rust
/// does not promise that for every target it has.
#[expect(
    clippy::as_conversions,
    reason = "a widening on every target this builds for; `usize` has no `From<u32>`"
)]
pub(crate) fn wide(n: u32) -> usize {
    n as usize
}

/// A size or count, as the 64-bit value an [`Error::OutOfRange`] reports.
#[expect(
    clippy::as_conversions,
    reason = "a widening on every target this builds for; `u64` has no `From<usize>`"
)]
pub(crate) fn long(n: usize) -> u64 {
    n as u64
}

/// The low sixteen bits of a word: what a 16-bit register keeps of a sum
/// carried out in 32.
pub(crate) fn low_word(v: u32) -> u16 {
    let [a, b, ..] = v.to_le_bytes();
    u16::from_le_bytes([a, b])
}

/// Two words as the one 32-bit value an [`Error::OutOfRange`] reports for a
/// field that is a pair, `high` in the upper half.
pub(crate) fn pair(high: u16, low: u16) -> u64 {
    let [l0, l1] = low.to_le_bytes();
    let [h0, h1] = high.to_le_bytes();
    u64::from(u32::from_le_bytes([l0, l1, h0, h1]))
}

/// A 16-bit word read signed: the same bits, as the original's signed
/// arithmetic takes them.
pub(crate) fn sign16(word: u16) -> i16 {
    i16::from_le_bytes(word.to_le_bytes())
}

/// Decodes a NUL-terminated CP437 byte string into a Rust `String`.
///
/// The game's text is code page 437; German umlauts land in the 0x80..0xFF
/// range and would otherwise come out as mojibake.
pub fn cp437_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| cp437_char(b)).collect()
}

/// One byte in, one `char` out — always.
///
/// The mapping is one to one, so `chars().count()` of a decoded string equals
/// the number of bytes the original counted with `strlen`. What would *not*
/// work is `str::len()`, which is the UTF-8 length and counts every umlaut
/// twice.
#[expect(
    clippy::indexing_slicing,
    reason = "a byte less 0x80 is below 128, the table's length"
)]
pub fn cp437_char(b: u8) -> char {
    let Some(high) = b.checked_sub(0x80) else {
        return char::from(b);
    };
    const HIGH: [char; 128] = [
        'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', 'É', 'æ',
        'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ', 'á', 'í', 'ó', 'ú',
        'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»', '░', '▒', '▓', '│', '┤', '╡',
        '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐', '└', '┴', '┬', '├', '─', '┼', '╞', '╟',
        '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧', '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘',
        '┌', '█', '▄', '▌', '▐', '▀', 'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ',
        '∞', 'φ', 'ε', '∩', '≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²',
        '■', '\u{a0}',
    ];
    HIGH[usize::from(high)]
}

/// The CP437 bytes of a string, `?` where the code page has no slot.
///
/// The engine keeps the game's text as Rust strings because that is what a
/// status line and a hotspot name are, but the font is indexed by the byte
/// the original had — including the control byte `01`, which selects an ink
/// mid-line and survives the round trip because every byte below `80h` maps
/// to the `char` of the same value.
pub fn to_cp437(s: &str) -> Vec<u8> {
    s.chars().map(|c| cp437_byte(c).unwrap_or(b'?')).collect()
}

/// Maps a `char` back to the CP437 byte, if it has one.
pub fn cp437_byte(ch: char) -> Option<u8> {
    if let Ok(low) = u8::try_from(ch)
        && low < 0x80
    {
        return Some(low);
    }
    (0x80u8..=0xff).find(|&b| cp437_char(b) == ch)
}

#[cfg(test)]
mod tests {
    /// Two spellings of one name and no exact one: the same is taken in
    /// whatever order the directory lists them.
    #[test]
    fn several_spellings_answer_the_first_in_byte_order() {
        let names = ["dsroom.dat", "PANELS.DAT", "DsRoom.DAT", "DSROOM.DAt"];
        let mut orders = vec![names.to_vec()];
        orders.push(names.iter().rev().copied().collect());
        orders.push(vec![names[2], names[0], names[3], names[1]]);
        for order in orders {
            let got = super::first_ci(order.into_iter().map(str::to_owned), "DSROOM.DAT");
            assert_eq!(got.as_deref(), Some("DSROOM.DAt"));
        }
        assert_eq!(super::first_ci(std::iter::empty(), "DSROOM.DAT"), None);
    }

    /// One byte of CP437 is one `char`, umlauts included.
    #[test]
    fn a_decoded_string_has_one_char_per_byte() {
        let all: Vec<u8> = (0..=255u8).collect();
        let decoded = super::cp437_to_string(&all);
        assert_eq!(decoded.chars().count(), all.len());
        assert!(
            decoded.len() > all.len(),
            "if these were equal the test would be proving nothing"
        );

        let german = super::cp437_to_string(b"Gr\x81\xe1e, sch\x94n!");
        assert_eq!(german, "Grüße, schön!");
        assert_eq!(german.chars().count(), 13, "thirteen bytes, thirteen chars");
        assert_eq!(
            german.len(),
            16,
            "but sixteen bytes of UTF-8: three of them are two-byte"
        );
    }
}

#[cfg(test)]
mod find_ci_tests {
    use super::find_ci;

    fn dir_with(name: &str, files: &[&str]) -> std::path::PathBuf {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/find-ci")
            .join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a test directory");
        for f in files {
            std::fs::write(dir.join(f), b"").expect("a test file");
        }
        dir
    }

    #[test]
    fn an_exact_name_is_found() {
        let dir = dir_with("exact", &["XP01.EXE", "DSROOM.DAT"]);
        assert!(find_ci(&dir, "XP01.EXE").is_some());
        assert!(find_ci(&dir, "DSROOM.DAT").is_some());
    }

    #[test]
    fn a_lowercased_install_is_found_too() {
        let dir = dir_with("lower", &["xp01.exe", "dsroom.dat", "panels.dat"]);
        for want in ["XP01.EXE", "DSROOM.DAT", "PANELS.DAT"] {
            let got = find_ci(&dir, want)
                .unwrap_or_else(|| panic!("{want} was not found among lowercase files"));
            assert!(got.exists(), "{} does not exist", got.display());
        }
    }

    #[test]
    fn any_mixture_of_case_is_found() {
        let dir = dir_with("mixed", &["Xp01.Exe"]);
        assert!(find_ci(&dir, "XP01.EXE").is_some());
    }

    #[test]
    fn a_name_that_is_not_there_is_none() {
        let dir = dir_with("absent", &["XP01.EXE"]);
        assert!(find_ci(&dir, "DSROOM.DAT").is_none());
    }

    #[test]
    fn a_directory_that_is_not_there_is_none() {
        let dir = dir_with("gone", &[]);
        assert!(find_ci(&dir.join("nope"), "XP01.EXE").is_none());
    }
}
