//! The layer rule, held by a test: no crate without a family's name depends
//! on a crate with one, and no family's evidence is written into the neutral
//! layer's sources.
//!
//! The rule this enforces is the workspace's naming law: a crate that exists
//! for one engine family carries that family's name (`motionvm-motion-*`),
//! an unprefixed `motionvm-*` name is the neutral layer's, and the two meet
//! only at the contract. The one sanctioned crossing is the window's roster,
//! which names each family's front door — the crate called `motionvm-<family>`
//! with nothing after it — and nothing deeper.
//!
//! Needs no game data: it reads the workspace's own manifests and sources.
//! It lives in the rig crate — outside both layers, depending on nothing —
//! because no crate inside a layer can judge the layers impartially, and
//! because the evidence list below may one day carry several families'
//! strings.

use std::fs;
use std::path::{Path, PathBuf};

/// Every family the workspace holds. A new family adds its name here, which
/// is what lets the rules below tell its crates from the neutral ones.
const FAMILIES: &[&str] = &["motion"];

/// Strings that carry a family's evidence: its name, its games, the files
/// and words its documentation cites. None of them may appear in a neutral
/// crate's sources — a fact argued with these belongs on the family's side
/// of the contract.
const FAMILY_EVIDENCE: &[&str] = &[
    "ENGINE.EXE",
    "DATA.-1-",
    "MUSADL",
    "HMIMDRV",
    "?KEY",
    "ICTRL",
    "SCRCTRL",
    "Dunkle Schatten",
    "Enviro",
    "Amajambere",
    "Jeff Jet",
    "JeffJet",
    "Loomes",
];

/// What a crate's name says about it.
#[derive(Debug, PartialEq)]
enum Layer {
    /// `motionvm-<family>`: the family's front door, the one crate a roster
    /// line may name.
    FrontDoor,
    /// `motionvm-<family>-<part>`: the family's own.
    Member,
    /// An unprefixed `motionvm-*` name: the neutral layer's.
    Neutral,
}

/// The family whose name `name` carries, if any — exact segment match, so a
/// later family whose name extends an earlier one (`motion2`, say) is its
/// own and not the earlier family's.
fn family_of(name: &str) -> Option<&'static str> {
    FAMILIES
        .iter()
        .copied()
        .find(|f| name == format!("motionvm-{f}") || name.starts_with(&format!("motionvm-{f}-")))
}

/// The layer a workspace crate's name puts it in.
fn layer(name: &str) -> Layer {
    match family_of(name) {
        Some(family) if name == format!("motionvm-{family}") => Layer::FrontDoor,
        Some(_) => Layer::Member,
        None => Layer::Neutral,
    }
}

/// The workspace's `crates/` directory, from this crate's own manifest dir.
fn crates_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// The member paths, read out of the root manifest's own list — the walk
/// sees exactly what Cargo sees, wherever a member lives.
fn member_paths() -> Vec<PathBuf> {
    let root = crates_dir().join("..").join("Cargo.toml");
    let text = fs::read_to_string(&root).expect("the root manifest is readable");
    let mut paths = Vec::new();
    let mut in_members = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with("members") {
            in_members = true;
            continue;
        }
        if in_members {
            if line.starts_with(']') {
                break;
            }
            if let Some(path) = line.strip_prefix('"').and_then(|l| l.strip_suffix("\",")) {
                paths.push(crates_dir().join("..").join(path));
            }
        }
    }
    assert!(!paths.is_empty(), "the members list should parse");
    paths
}

/// Every member crate as (name, workspace dependencies), read straight out
/// of the manifests.
fn members() -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    for dir in member_paths() {
        let manifest = dir.join("Cargo.toml");
        let text = fs::read_to_string(&manifest)
            .unwrap_or_else(|_| panic!("{} names no manifest", dir.display()));
        let name = text
            .lines()
            .find_map(|l| l.trim().strip_prefix("name = "))
            .expect("every manifest names its package")
            .trim_matches('"')
            .to_string();
        let mut deps = Vec::new();
        let mut in_deps = false;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                // Every table of dependencies counts — regular, dev, build,
                // and the target-specific ones: a crossing in any of them is
                // a crossing. The section form `[dependencies.NAME]` names
                // its dependency in the header itself.
                in_deps = line.ends_with("dependencies]");
                if let Some(header) = line.strip_suffix(']')
                    && let Some(i) = header.find("dependencies.")
                {
                    deps.push(header[i + "dependencies.".len()..].to_string());
                }
                continue;
            }
            if in_deps && line.starts_with("motionvm") {
                let dep = line
                    .split(['.', ' ', '='])
                    .next()
                    .expect("a dependency line starts with its name");
                deps.push(dep.to_string());
            }
        }
        out.push((name, deps));
    }
    out
}

#[test]
fn no_neutral_crate_depends_on_a_family() {
    let members = members();
    assert_eq!(
        members.len(),
        member_paths().len(),
        "every member's manifest should have been read"
    );
    for (name, deps) in &members {
        if layer(name) != Layer::Neutral {
            continue;
        }
        for dep in deps {
            match layer(dep) {
                Layer::Neutral => {}
                Layer::FrontDoor => assert_eq!(
                    name, "motionvm-app",
                    "{name} depends on the family front door {dep}; only the \
                     window's roster may"
                ),
                Layer::Member => panic!(
                    "{name} depends on {dep}: a neutral crate reaches a \
                     family only through its front door, and only from the \
                     window's roster"
                ),
            }
        }
    }
}

#[test]
fn no_family_reaches_into_another() {
    for (name, deps) in members() {
        let Some(family) = family_of(&name) else {
            continue;
        };
        for dep in deps.iter().filter(|d| layer(d) != Layer::Neutral) {
            assert_eq!(
                family_of(dep),
                Some(family),
                "{name} depends on {dep}, which is another family's"
            );
        }
    }
}

/// The window names a family's crates in its roster and nowhere else — the
/// other half of the front-door rule, which the dependency check alone
/// cannot see.
#[test]
fn the_window_names_a_family_only_in_its_roster() {
    for family in FAMILIES {
        let ident = format!("motionvm_{family}");
        for file in sources(&crates_dir().join("motionvm-app").join("src")) {
            if file.ends_with("roster.rs") {
                continue;
            }
            let text = fs::read_to_string(&file).expect("source is readable");
            assert!(
                !text.contains(&ident),
                "{} names {ident}: the roster is the one place a family is named",
                file.display()
            );
        }
    }
}

#[test]
fn the_neutral_sources_carry_no_family_evidence() {
    for (name, _) in members() {
        if layer(&name) != Layer::Neutral {
            continue;
        }
        // `src/` only, on purpose: this very file sits in a neutral crate's
        // `tests/` and carries the evidence list above, so the scan stops at
        // the sources a build ships.
        for file in sources(&crates_dir().join(&name).join("src")) {
            // The roster is the sanctioned place a family is named.
            if name == "motionvm-app" && file.ends_with("roster.rs") {
                continue;
            }
            let text = fs::read_to_string(&file).expect("source is readable");
            for token in FAMILY_EVIDENCE {
                assert!(
                    !text.contains(token),
                    "{} speaks of {token}: that evidence belongs with its family",
                    file.display()
                );
            }
            // The family's name itself, told apart from the product's
            // `motionvm`/`MOTIONVM_*`: any bare `MOTION` is the family's.
            for (i, _) in text.match_indices("MOTION") {
                assert!(
                    text[i + 6..].starts_with("VM"),
                    "{} names the MOTION family",
                    file.display()
                );
            }
        }
    }
}

/// Every `.rs` file under `dir`, recursively.
fn sources(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.filter_map(|e| e.ok()) {
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
