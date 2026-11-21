//! Every stop by address is on the ledger.
//!
//! CONTRIBUTING's rule: where a branch of a handler has not been followed,
//! the engine answers `Error::Unread` naming the binary and the address, and
//! the open questions carry the same address — or the departures ledger
//! does, where the branch is read and the stop is the choice. A rule kept by
//! memory drifts; this reads every `Error::Unread` in the sources, takes the
//! addresses out of its `at:` and asks the two ledgers for each.
//!
//! Needs no game data: it reads the sources and the documentation.

use std::fs;
use std::path::{Path, PathBuf};

/// The workspace root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// Every `.rs` file under `dir`, recursively, in a stable order.
fn sources(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    for entry in entries.map(|e| e.expect("a directory entry")) {
        let path = entry.path();
        if path.is_dir() {
            out.extend(sources(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out.sort();
    out
}

/// The addresses in a string: `0x…` hex, and `seg:off` pairs of four hex
/// digits each.
fn addresses(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in text.split(|c: char| !(c.is_ascii_hexdigit() || c == 'x' || c == ':')) {
        let word = raw.trim_matches(':');
        if let Some(hex) = word.strip_prefix("0x") {
            if !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit()) {
                out.push(word.to_ascii_lowercase());
            }
        } else if let Some((seg, off)) = word.split_once(':') {
            let four = |s: &str| s.len() == 4 && s.chars().all(|c| c.is_ascii_hexdigit());
            if four(seg) && four(off) {
                out.push(word.to_ascii_lowercase());
            }
        }
    }
    out
}

/// The `at:` string of every `Error::Unread { … }` in `text`, with the line
/// the error opens on.
fn unread_sites(text: &str) -> Vec<(usize, String)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if !line.contains("Error::Unread {") {
            continue;
        }
        let at = lines
            .iter()
            .skip(i)
            .take(16)
            .find_map(|l| l.trim().strip_prefix("at: \""))
            .unwrap_or_else(|| panic!("an Error::Unread at line {} with no `at:`", i + 1));
        out.push((i + 1, at.trim_end_matches("\",").to_string()));
    }
    out
}

#[test]
fn every_unread_address_is_on_a_ledger() {
    let root = root();
    let docs = ["docs/motion/open-questions.md", "docs/motion/departures.md"]
        .iter()
        .map(|p| fs::read_to_string(root.join(p)).unwrap_or_else(|e| panic!("{p}: {e}")))
        .collect::<String>()
        .to_ascii_lowercase();

    let mut sites = 0;
    let mut missing = Vec::new();
    let crates = fs::read_dir(root.join("crates")).expect("the crates directory");
    let mut dirs: Vec<PathBuf> = crates
        .map(|e| e.expect("a directory entry").path().join("src"))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for dir in dirs {
        for file in sources(&dir) {
            let text = fs::read_to_string(&file).expect("a source file");
            for (line, at) in unread_sites(&text) {
                sites += 1;
                for address in addresses(&at) {
                    if !docs.contains(&address) {
                        missing.push(format!("{}:{line} {address}", file.display()));
                    }
                }
            }
        }
    }
    assert!(
        sites >= 10,
        "only {sites} Error::Unread sites found — the walk is reading the wrong tree"
    );
    assert!(
        missing.is_empty(),
        "addresses an Error::Unread names that neither ledger carries:\n{}",
        missing.join("\n")
    );
}

#[test]
fn addresses_are_told_from_prose() {
    assert_eq!(
        addresses("0x76dcc, 0x77a2b"),
        ["0x76dcc", "0x77a2b"],
        "two flat addresses"
    );
    assert_eq!(
        addresses("05f1:105d (0362:2111, 0362:2007)"),
        ["05f1:105d", "0362:2111", "0362:2007"],
        "segmented addresses"
    );
    assert!(
        addresses("no such word in its kernel table").is_empty(),
        "prose has no address"
    );
}
