//! Readers for the resource formats of the MOTION engine (DigiTales, 1996) in
//! both of its generations: the 32-bit engine of "Im Netzwerk gefangen -
//! Dunkle Schatten 2" under [`m32`], the 16-bit engine of "Die Enviro-Kids
//! greifen ein" under [`m16`].
//!
//! The crate root holds only what the two generations share byte for byte or
//! decode to the same structure: the font reference table, the decoded font
//! and text table, the GFXCRUNCH LZW codec, the error type and the byte
//! helpers. A reader under `m32` or `m16` reads one generation's
//! layout and nothing else; a caller picks the generation.
//!
//! Everything here was derived from the shipped data files and the strings in
//! the engine binaries. Each reader carries the evidence for its format: the
//! offsets it reads, the address in the binary the layout was read at where
//! there is one, and the corpus the reading was checked against — which is
//! always one game's, and named.

// The readers are this crate's whole surface toward a file a player supplies,
// and a damaged one is refused, never crashed on — `tests/mutation.rs` cuts
// and flips the shipped files to check it. These two lints hold that by
// construction rather than by corpus: an index has been bounds-checked or is a
// constant, and a sum says what it does at the edge. The tests are outside
// the rule, because a test indexes what it built.
#![cfg_attr(
    not(test),
    deny(clippy::indexing_slicing, clippy::arithmetic_side_effects)
)]

mod cursor;
pub mod error;
pub mod font;
pub mod generation;
pub mod kernel;
pub mod lzw;
pub mod m16;
pub mod m32;
pub mod text;

pub use error::{Error, Result};
pub use generation::Generation;
pub use kernel::{Binding, Inline, KernelWord};
pub use text::TextTable;

/// Finds `name` in `dir` whatever case it is stored in.
///
/// The game's files are named in upper case on the disc — `ENGINE.EXE`,
/// `000.FRT`, `MELODIC.BNK` — but a copy that has been through a CD-ROM
/// driver, an archiver or a file manager very often arrives in lower case.
/// On a case-insensitive filesystem that costs nothing and is invisible; on a
/// case-sensitive one an exact-case `join` simply does not find the file, and
/// the game reports a directory it is standing in as "not a MOTION game
/// directory".
///
/// So every shipped file is looked up through here. The `NNN.RSC` containers
/// were already found this way — [`m32::rsc::Bank::open_dir`] scans and compares
/// with `eq_ignore_ascii_case` — and this is the same rule for the files that
/// are opened by name.
///
/// Returns the path as it is actually spelled on disk, so what gets opened is
/// what was found. An exact match wins over a differently-cased one, which
/// only matters on a filesystem holding both. `None` when the directory cannot
/// be read or holds nothing by that name.
///
/// The comparison is ASCII-only, which is all these names are.
pub fn find_ci(dir: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    let exact = dir.join(name);
    if exact.exists() {
        return Some(exact);
    }
    std::fs::read_dir(dir).ok()?.flatten().find_map(|e| {
        let p = e.path();
        p.file_name()
            .and_then(|f| f.to_str())
            .is_some_and(|f| f.eq_ignore_ascii_case(name))
            .then_some(p)
    })
}

/// An empty `Vec` reserved for `count` items, but never for more than `have`
/// bytes of input could describe at `per_item` bytes each.
///
/// Item counts come out of the file being read, so a damaged header can ask for
/// a hundred million records inside a five-kilobyte file. Every loop that
/// follows one of these checks its bounds as it goes and fails on the first
/// missing byte — reserving is only an optimization, and capping it is what
/// stops the reservation itself from aborting the process before the loop gets
/// a chance to report the real problem.
pub(crate) fn reserve<T>(count: usize, have: usize, per_item: usize) -> Vec<T> {
    let fit = have.checked_div(per_item).unwrap_or(have);
    Vec::with_capacity(count.min(fit.saturating_add(1)))
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

/// `N` little-endian `u16`s back to back at `off`.
pub(crate) fn words<const N: usize>(d: &[u8], off: usize) -> Result<[u16; N]> {
    let raw = records::<2>(d, off, N)?;
    let mut out = [0u16; N];
    for (word, bytes) in out.iter_mut().zip(raw) {
        *word = u16::from_le_bytes(*bytes);
    }
    Ok(out)
}

/// `N` little-endian `u32`s back to back at `off`.
pub(crate) fn dwords<const N: usize>(d: &[u8], off: usize) -> Result<[u32; N]> {
    let raw = records::<4>(d, off, N)?;
    let mut out = [0u32; N];
    for (word, bytes) in out.iter_mut().zip(raw) {
        *word = u32::from_le_bytes(*bytes);
    }
    Ok(out)
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

/// The bytes before the first NUL, or all of them when there is none: a
/// C string as the formats store one.
pub fn nul_terminated(bytes: &[u8]) -> &[u8] {
    bytes.split(|&b| b == 0).next().unwrap_or_default()
}

/// Reads one byte at `off`, or `Err` if that is past the end.
pub(crate) fn u8at(d: &[u8], off: usize) -> Result<u8> {
    Ok(bytes::<1>(d, off)?[0])
}

/// Reads a little-endian `u16` at `off`, or `Err` if it would run past the end.
pub(crate) fn u16le(d: &[u8], off: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(*bytes(d, off)?))
}

/// Reads a little-endian `u32` at `off`, or `Err` if it would run past the end.
pub(crate) fn u32le(d: &[u8], off: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(*bytes(d, off)?))
}

/// Reads a little-endian `i16` at `off` — a field the format keeps signed.
pub(crate) fn i16le(d: &[u8], off: usize) -> Result<i16> {
    Ok(i16::from_le_bytes(*bytes(d, off)?))
}

/// Reads a little-endian `u32` at `off` as what it is used for next: an
/// offset into, or a count of things in, the file it came out of.
pub(crate) fn u32at(d: &[u8], off: usize) -> Result<usize> {
    u32le(d, off).map(wide)
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

/// An offset inside a file the reader holds whole, as the 32-bit number the
/// format writes it as. Every file these readers open is far smaller than
/// 4 GiB — the largest container in the corpus is 7.6 MB — so the value is
/// the offset and not a truncation of it.
#[expect(
    clippy::as_conversions,
    reason = "an offset inside a file held in memory, smaller than 4 GiB by the corpus"
)]
pub(crate) fn narrow(n: usize) -> u32 {
    n as u32
}

/// A 16-bit cell read signed: the machine's own reading of an inline operand
/// or a field the format keeps signed.
pub(crate) fn sign16(cell: u16) -> i16 {
    i16::from_le_bytes(cell.to_le_bytes())
}

/// The low sixteen bits of a word: a code out of a bit buffer.
pub(crate) fn low_word(v: u32) -> u16 {
    let [a, b, ..] = v.to_le_bytes();
    u16::from_le_bytes([a, b])
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
/// Relied on outside this crate: `GDTEXTLEN` answers with `chars().count()` of
/// a decoded string and that has to equal the number of bytes the original
/// counted with `strlen`. It does, because the mapping is one to one. What
/// would *not* work is `str::len()`, which is the UTF-8 length and counts every
/// umlaut twice. `a_decoded_string_has_one_char_per_byte` holds this down.
///
/// Maps one CP437 byte to its Unicode code point.
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

#[cfg(test)]
mod tests {
    /// One byte of CP437 is one `char`, umlauts included.
    ///
    /// The property `GDTEXTLEN` stands on: the original measures a line with
    /// `strlen` over CP437 bytes, and this rebuild measures it as `chars()` of
    /// the decoded string. Those agree only because the decode is one to one,
    /// and the speech durations `TSX` works out from that length would drift by
    /// one tick per umlaut if it ever stopped being true.
    #[test]
    fn a_decoded_string_has_one_char_per_byte() {
        // Every byte, not a sample: the mapping has no gaps and no pairs.
        let all: Vec<u8> = (0..=255u8).collect();
        let decoded = super::cp437_to_string(&all);
        assert_eq!(decoded.chars().count(), all.len());
        // And the trap this protects against: UTF-8 length is not the same
        // number, because the high half does not fit in one byte.
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

    /// A directory of empty files with the given names, under `target/`.
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

    /// The case the disc uses, which is also the case the code asks for.
    #[test]
    fn an_exact_name_is_found() {
        let dir = dir_with("exact", &["ENGINE.EXE", "000.FRT"]);
        assert!(find_ci(&dir, "ENGINE.EXE").is_some());
        assert!(find_ci(&dir, "000.FRT").is_some());
    }

    /// The case a copied install very often has — and the one that only a
    /// case-sensitive filesystem can catch, which is why it is asserted
    /// rather than trusted.
    #[test]
    fn a_lowercased_install_is_found_too() {
        let dir = dir_with("lower", &["engine.exe", "000.frt", "melodic.bnk"]);
        for want in ["ENGINE.EXE", "000.FRT", "MELODIC.BNK"] {
            let got = find_ci(&dir, want)
                .unwrap_or_else(|| panic!("{want} was not found among lowercase files"));
            // What comes back is the real spelling, so opening it works.
            assert!(got.exists(), "{} does not exist", got.display());
        }
    }

    /// Mixed case counts too — archivers produce this as readily as either.
    #[test]
    fn any_mixture_of_case_is_found() {
        let dir = dir_with("mixed", &["Engine.Exe"]);
        assert!(find_ci(&dir, "ENGINE.EXE").is_some());
    }

    #[test]
    fn a_name_that_is_not_there_is_none() {
        let dir = dir_with("absent", &["ENGINE.EXE"]);
        assert!(find_ci(&dir, "000.FRT").is_none());
    }

    #[test]
    fn a_directory_that_is_not_there_is_none() {
        let dir = dir_with("gone", &[]);
        assert!(find_ci(&dir.join("nope"), "ENGINE.EXE").is_none());
    }
}
