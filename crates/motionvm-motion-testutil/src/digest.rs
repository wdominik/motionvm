//! Reference digests: what a scene composed, or what a tune wrote, reduced to
//! sixteen hex digits that may live in the repository.
//!
//! The strongest check this project has ever run is a rendered frame held
//! against a capture of the original, pixel for pixel. It cannot be kept: the
//! baseline is a rendering of the game's own artwork, and the game's data may
//! not enter this repository in any form, including a fixture derived from it.
//! A **digest** of that frame is not a derived fixture — sixteen hex digits
//! reconstruct no picture and carry nothing of what was drawn — so it may be
//! checked in, and what it does is exactly what was missing: it notices a
//! change nobody thought to write an assertion for.
//!
//! What that buys, concretely. Every refactor claims to change no pixel and no
//! register. Without a baseline that claim is checked by rendering a scene
//! before and after on one machine and looking at the two pictures, which
//! nobody does twice. With one it is `just test`, it runs on every machine
//! that has the game, and a contributor's tree and this one agree or say where
//! they differ.
//!
//! What it does not buy. A digest says *the same* or *not the same*, and never
//! *what* moved: a one-pixel shift and a blank screen fail identically. It is
//! a tripwire, not a diff — the picture itself is still the thing to look at
//! once one goes off, and the suites keep their own assertions about what a
//! scene must contain, because those say what is *right* rather than only what
//! is *unchanged*.
//!
//! ## Determinism, which all of this rests on
//!
//! A digest is only a check if the same run gives the same number twice. The
//! engine reads no clock, `RANDOM` is a seeded LCG, there is no `HashMap` on
//! any drawing path and the resource directory is walked in sorted order — so
//! a scene reached the same way twice composes the same bytes twice. The one
//! thing that is not insensitive is the number of `RANDOM` calls made before
//! the frame: the generator is shared and consumed in call order, so a change
//! elsewhere that draws one more random number moves every later one.
//!
//! ## The table
//!
//! One file per game, `tests/digests/<slug>.txt` in the crate whose suite
//! produced it, one `name  hash` line per entry, sorted. The recipe
//! `just digests` rewrites it from a run and leaves the difference in the
//! working tree, so a deliberate change is a reviewed line in a diff and an
//! accidental one is a red test.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use motionvm_render::Framebuffer;

/// FNV-1a over 64 bits: the offset basis and the prime.
///
/// Chosen for being writable in a dozen lines with no dependency and no
/// ambiguity about what it computes — the values below are the published
/// constants, and any other implementation of FNV-1a gives the same answer for
/// the same bytes. Nothing here wants a cryptographic hash: the adversary is a
/// refactor, not a forger.
const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

/// A running FNV-1a digest.
///
/// Bytes go in, one number comes out. What a caller feeds it is the caller's
/// statement about what the digest covers, and that statement is worth making
/// carefully: a frame digest that left out the width would pass a frame that
/// had been reshaped, and a register digest that left out the bank would pass
/// a write that landed on the wrong half of the chip.
#[derive(Debug, Clone, Copy)]
pub struct Digest(u64);

impl Default for Digest {
    fn default() -> Self {
        Self::new()
    }
}

impl Digest {
    /// An empty digest.
    pub fn new() -> Self {
        Self(OFFSET_BASIS)
    }

    /// Folds one byte in.
    pub fn byte(&mut self, b: u8) -> &mut Self {
        self.0 = (self.0 ^ u64::from(b)).wrapping_mul(PRIME);
        self
    }

    /// Folds a run of bytes in, in order.
    pub fn bytes(&mut self, bytes: &[u8]) -> &mut Self {
        for &b in bytes {
            self.byte(b);
        }
        self
    }

    /// Folds a number in as eight bytes, least significant first.
    ///
    /// Always eight, whatever the number's own width, so that a value that
    /// grows a type later still digests the same.
    pub fn number(&mut self, n: u64) -> &mut Self {
        self.bytes(&n.to_le_bytes())
    }

    /// The digest so far.
    pub fn value(&self) -> u64 {
        self.0
    }
}

/// The digest of a composed frame: its size, then every index in it.
///
/// The size is in the digest and not merely implied by the pixel count,
/// because a frame reshaped from 640×480 to 480×640 holds the same bytes in
/// the same order and is not the same picture.
///
/// Indices rather than colors, for the reason the whole project renders in
/// indices: the palette is animated under the picture, so two frames that
/// differ only in a palette cycle are the same composition and should digest
/// the same. What a palette change breaks is caught where palettes are
/// asserted on, not here.
pub fn frame(f: &Framebuffer) -> u64 {
    let mut d = Digest::new();
    d.number(u64::from(f.width))
        .number(u64::from(f.height))
        .bytes(&f.pixels)
        .value()
}

/// One game's table of reference digests.
///
/// Built per suite from the calling crate's own directory, because the tables
/// live beside the tests that produce them: the engine's suites digest frames,
/// the audio suites digest register streams, and the two are different tables
/// for the same game.
#[derive(Debug)]
pub struct Digests {
    /// The checked-in table.
    table: PathBuf,
    /// What it holds, by name.
    entries: BTreeMap<String, u64>,
    /// Where a recording run appends its lines, or `None` for the ordinary
    /// checking run.
    record: Option<PathBuf>,
}

impl Digests {
    /// The table for `slug` belonging to the crate rooted at `crate_dir`,
    /// which a suite names with `env!("CARGO_MANIFEST_DIR")`.
    ///
    /// The manifest directory is passed in rather than read here because
    /// `env!` in this file would answer *this* crate's directory, and the
    /// table belongs to the caller's.
    ///
    /// A missing table is not an error: a game whose digests have never been
    /// recorded reports every scene as missing, which says what to do, rather
    /// than failing to open a file, which does not.
    pub fn of(crate_dir: &str, slug: &str) -> Self {
        let dir = Path::new(crate_dir);
        let table = dir
            .join("tests")
            .join("digests")
            .join(format!("{slug}.txt"));
        let entries = read_table(&table);
        let record = recording().then(|| {
            let crate_name = dir
                .file_name()
                .and_then(|n| n.to_str())
                .expect("a crate directory has a name");
            dir.join("../../target/digests")
                .join(format!("{crate_name}.{slug}.lines"))
        });
        Self {
            table,
            entries,
            record,
        }
    }

    /// Holds one digest against the table, or records it.
    ///
    /// `name` identifies the thing digested within its game — a scene, a tune
    /// — and is what appears in the table's left column, so it wants to be
    /// stable and to read as itself in a diff.
    ///
    /// **Panics** when the table disagrees, and when it has no line for
    /// `name`: a digest nothing was held against is not a check, and reporting
    /// it as a pass would make an empty table look like a green suite.
    pub fn check(&self, name: &str, digest: u64) {
        if let Some(path) = &self.record {
            append_line(path, name, digest);
            return;
        }
        match self.entries.get(name) {
            Some(&want) => assert!(
                want == digest,
                "{name}: this is not what {} says it was.\n  \
                 the table says {want:#018x}, this run made {digest:#018x}\n  \
                 If the change was meant, `just digests` rewrites the table and \
                 the diff is the review. If it was not, something on the way to \
                 this scene moved.",
                self.table.display()
            ),
            None => panic!(
                "{name}: {} has no line for it, so there is nothing to hold it \
                 against.\n  Run `just digests` on a machine that has the game \
                 to write one.",
                self.table.display()
            ),
        }
    }
}

/// Whether this run is recording rather than checking.
///
/// `MOTIONVM_DIGESTS=record`, which `just digests` sets and nothing else does.
/// A spelled-out value rather than mere presence, so that a stray empty
/// variable in an environment cannot quietly turn every assertion in the
/// suite into a write.
fn recording() -> bool {
    std::env::var("MOTIONVM_DIGESTS").is_ok_and(|v| v == "record")
}

/// Reads a table, or answers empty when there is none yet.
///
/// **Panics** on a line that does not parse and on a name that appears twice.
/// The second is worth the strictness: two lines for one name is what a
/// recording run produces when a scene digested differently on two passes,
/// which is nondeterminism — the one thing that would make every digest in
/// the file meaningless, and the one thing a silent last-wins read would hide.
fn read_table(path: &Path) -> BTreeMap<String, u64> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let mut out = BTreeMap::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (name, hash) = line
            .split_once(char::is_whitespace)
            .unwrap_or_else(|| panic!("{}:{}: no digest on the line", path.display(), n + 1));
        let hash = hash.trim();
        let value = u64::from_str_radix(hash.trim_start_matches("0x"), 16)
            .unwrap_or_else(|_| panic!("{}:{}: {hash} is not a digest", path.display(), n + 1));
        if out.insert(name.to_string(), value).is_some() {
            panic!(
                "{}: two lines for {name}. A recording run wrote two different \
                 digests for one thing, which means it is not deterministic.",
                path.display()
            );
        }
    }
    out
}

/// Appends one recorded line.
///
/// Opened in append mode for every line rather than held open, because the
/// suite's tests run in parallel threads and its binaries in parallel
/// processes: a short append is the one write several writers can make to one
/// file without arranging anything between them. `just digests` sorts what
/// comes out.
fn append_line(path: &Path, name: &str, digest: u64) {
    use std::io::Write as _;
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    writeln!(f, "{name}  {digest:#018x}").unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The published FNV-1a 64 test vectors, so that "FNV-1a" in the header is
    /// a claim someone else's implementation can be held to.
    #[test]
    fn the_digest_is_fnv_1a() {
        assert_eq!(Digest::new().value(), 0xcbf2_9ce4_8422_2325);
        assert_eq!(Digest::new().bytes(b"a").value(), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(
            Digest::new().bytes(b"foobar").value(),
            0x8594_4171_f739_67e8
        );
    }

    #[test]
    fn a_frame_digests_its_size_as_well_as_its_pixels() {
        let wide = Framebuffer {
            width: 4,
            height: 2,
            pixels: vec![1, 2, 3, 4, 5, 6, 7, 8],
        };
        let tall = Framebuffer {
            width: 2,
            height: 4,
            pixels: wide.pixels.clone(),
        };
        assert_ne!(frame(&wide), frame(&tall));
    }

    #[test]
    fn one_moved_pixel_moves_the_digest() {
        let mut a = Framebuffer::new(8, 8);
        let before = frame(&a);
        a.set(3, 3, 1);
        assert_ne!(frame(&a), before);
    }
}
