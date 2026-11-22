//! Reference digests: what a tune wrote, or what the location logic did in
//! one case, reduced to sixteen hex digits that may live in the repository.
//!
//! A register stream held against a capture of the original is the strongest
//! check the music has, and neither side can be kept here: the capture is a
//! recording of the original, our stream a rendering of the game's own songs,
//! and the game's data may not enter this repository in any form. A
//! **digest** of the stream is not a derived fixture — sixteen hex digits
//! reconstruct no song — so it may be checked in, and it notices a change
//! nobody thought to write an assertion for: every refactor that claims to
//! move no register is then `just test` on every machine that has the game.
//!
//! The location logic is held the same way. What the original's routines do
//! on a case's inputs — every call, every byte changed — is derived from the
//! game's program and cannot be kept here either; a digest of it can, and
//! the run of this engine's logic on the same inputs has to make the same
//! one.
//!
//! What it does not buy: a digest says *the same* or *not the same*, never
//! *what* moved. The suites keep their own assertions about what a stream
//! must contain, because those say what is right rather than only what is
//! unchanged.
//!
//! ## The table
//!
//! One file per game and subject, `tests/digests/<slug>.txt` (or
//! `<slug>-<subject>.txt`) in the crate whose suite produced it, one
//! `name  hash` line per entry, sorted. The recipe
//! `just digests` rewrites it from a run and leaves the difference in the
//! working tree, so a deliberate change is a reviewed line in a diff and an
//! accidental one is a red test.
//!
//! ## Anchored lines
//!
//! A line may end in `anchored: <what>`: the thing it digests has been held
//! against the original — a capture of it, or its routine traced on the same
//! inputs — and agreed with it. `just digests` will
//! not move such a line on its own: a run that digests it differently leaves
//! the old value in place and fails, and `REANCHOR=1 just digests` takes the
//! new value with the anchor kept — after which the comparison against the
//! original has to pass again before the anchor means anything.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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

/// One game's table of reference digests.
///
/// Built per suite from the calling crate's own directory, because the tables
/// live beside the tests that produce them.
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
        let (hash, rest) = hash.split_once(char::is_whitespace).unwrap_or((hash, ""));
        let rest = rest.trim();
        assert!(
            rest.is_empty() || rest.starts_with("anchored:"),
            "{}:{}: after the digest only `anchored: …` may follow, not {rest}",
            path.display(),
            n + 1
        );
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
/// processes: a short append made in one write is the one write several
/// writers can make to one file without arranging anything between them.
/// `just digests` sorts what comes out.
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
    // One write for the whole line. `writeln!` on an unbuffered file hands
    // the pieces of its format to the system one by one, and a writer in
    // another process lands between them — which is how a table once came
    // out with two names and a doubled `0x` on one line.
    let line = format!("{name}  {digest:#018x}\n");
    f.write_all(line.as_bytes())
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line may carry its anchor after the digest, and nothing else.
    #[test]
    fn an_anchored_line_reads_its_digest() {
        let dir = std::env::temp_dir().join("motionvm-parsec-digest-anchor");
        let _ = std::fs::create_dir_all(&dir);
        let table = dir.join("t.txt");
        std::fs::write(
            &table,
            "# a table\nplain  0x0000000000000001\nheld  0x0000000000000002  anchored: a capture\n",
        )
        .expect("writes");
        let t = read_table(&table);
        assert_eq!(t.get("plain"), Some(&1));
        assert_eq!(t.get("held"), Some(&2));
        let _ = std::fs::remove_dir_all(&dir);
    }

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
}
