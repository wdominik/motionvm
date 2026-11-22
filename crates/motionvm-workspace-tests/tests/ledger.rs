//! Every stop by address is on the ledger.
//!
//! CONTRIBUTING's rule: where a branch of a handler has not been followed,
//! the engine answers `Error::Unread` naming the binary and the address, and
//! the open questions carry the same address — or the departures ledger
//! does, where the branch is read and the stop is the choice. A rule kept by
//! memory drifts; this reads every `Error::Unread` in the sources, takes the
//! addresses out of its `at:` and asks the two ledgers for each.
//!
//! The ledgers are the owner's: a family's crates answer to the family's
//! tree, a component's to its own, and the neutral layer stops nowhere by an
//! address, because it has no original to read.
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

/// Whose ledgers a crate answers to, by its name: a family's crates —
/// the front door and every `-<part>` — or a component. A new family or
/// component goes into `FAMILIES` or `COMPONENTS` (`tests/boundary.rs`) as
/// well.
const LEDGERS: &[(&str, &str)] = &[
    ("motionvm-motion", "docs/motion"),
    ("motionvm-psm", "docs/psm"),
];

/// The documentation tree whose ledgers `name` answers to, if any.
fn tree_of(name: &str) -> Option<&'static str> {
    LEDGERS
        .iter()
        .find(|(owner, _)| name == *owner || name.starts_with(&format!("{owner}-")))
        .map(|(_, tree)| *tree)
}

/// The two ledgers of a tree, lowercased and run together — or why there
/// are none to read.
fn ledgers(root: &Path, tree: &str) -> Result<String, String> {
    ["open-questions.md", "departures.md"]
        .iter()
        .map(|page| {
            let path = root.join(tree).join(page);
            fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
        })
        .collect::<Result<String, String>>()
        .map(|text| text.to_ascii_lowercase())
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
    let mut sites = 0;
    let mut missing = Vec::new();
    let crates = fs::read_dir(root.join("crates")).expect("the crates directory");
    let mut dirs: Vec<PathBuf> = crates
        .map(|e| e.expect("a directory entry").path())
        .filter(|p| p.join("src").is_dir())
        .collect();
    dirs.sort();
    for dir in dirs {
        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .expect("a crate directory's name")
            .to_owned();
        // Read only where a stop is: a tree with nothing to record keeps no
        // empty ledger for this test's sake.
        let mut docs: Option<String> = None;
        for file in sources(&dir.join("src")) {
            let text = fs::read_to_string(&file).expect("a source file");
            for (line, at) in unread_sites(&text) {
                sites += 1;
                let Some(tree) = tree_of(&name) else {
                    missing.push(format!(
                        "{}:{line} stops by address in {name}, which answers to no ledger",
                        file.display()
                    ));
                    continue;
                };
                let docs = match &docs {
                    Some(docs) => docs,
                    None => match ledgers(&root, tree) {
                        Ok(read) => docs.insert(read),
                        Err(e) => {
                            missing.push(format!(
                                "{}:{line} has no ledger to answer to: {e}",
                                file.display()
                            ));
                            continue;
                        }
                    },
                };
                for address in addresses(&at) {
                    if !docs.contains(&address) {
                        missing.push(format!("{}:{line} {address} ({tree})", file.display()));
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
        "addresses an Error::Unread names that neither of its owner's ledgers carries:\n{}",
        missing.join("\n")
    );
}

/// A crate answers to its owner's tree: a family's parts by the family's
/// prefix, a component by its own name, the neutral layer to none.
#[test]
fn a_crate_answers_to_its_owners_ledgers() {
    assert_eq!(tree_of("motionvm-motion-engine"), Some("docs/motion"));
    assert_eq!(tree_of("motionvm-motion"), Some("docs/motion"));
    assert_eq!(tree_of("motionvm-psm"), Some("docs/psm"));
    assert_eq!(tree_of("motionvm-render"), None);
    assert_eq!(
        tree_of("motionvm-motionless"),
        None,
        "a prefix is a whole segment"
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
