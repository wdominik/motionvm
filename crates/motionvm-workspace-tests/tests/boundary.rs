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
//! Needs no game data: it asks Cargo what the workspace is and reads the
//! sources.
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

/// Runs `cargo` with these arguments and answers its lines.
///
/// Asking Cargo rather than reading the manifests, and the difference is not
/// tidiness: a hand-written scanner sees the shape of the file it was written
/// against. The one here missed a `[dependencies.name]` section header until
/// it was taught about them, would have missed a `[target.'cfg(unix)'
/// .dependencies]` table, and could not have seen a dependency renamed with
/// `package =`. Cargo has already resolved all of that by the time a test
/// runs, so this asks it.
///
/// `CARGO` is set for anything cargo runs, which is how a test finds the same
/// toolchain that built it. Still no dependency: the output is lines, and
/// lines are what this reads.
fn cargo(args: &[&str]) -> Vec<String> {
    let exe = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let out = std::process::Command::new(exe)
        .args(args)
        .current_dir(crates_dir().join(".."))
        .output()
        .expect("cargo runs");
    assert!(
        out.status.success(),
        "cargo {}: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("cargo speaks UTF-8")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The crate a `cargo tree` line names — the first word of it.
fn crate_of(line: &str) -> &str {
    line.split_whitespace().next().unwrap_or("")
}

/// Every member of the workspace, by name.
fn members() -> Vec<String> {
    cargo(&["tree", "--workspace", "--depth", "0", "--prefix", "none"])
        .iter()
        .map(|l| crate_of(l).to_owned())
        .collect()
}

/// The workspace crates `name` depends on, in any table — normal, build or
/// dev. A crossing in any of them is a crossing.
fn dependencies(name: &str) -> Vec<String> {
    cargo(&[
        "tree",
        "-p",
        name,
        "--depth",
        "1",
        "--prefix",
        "none",
        "-e",
        "normal,build,dev",
    ])
    .iter()
    .map(|l| crate_of(l).to_owned())
    .filter(|d| d != name && d.starts_with("motionvm"))
    .collect()
}

/// What the two checks below rest on: Cargo really is answering, and the
/// answers really do carry dependencies.
///
/// A rule enforced by a walk over an empty list passes for the wrong reason,
/// and rewriting this file's source of truth is exactly the change that could
/// have made it empty. So the walk is held to what the workspace is known to
/// contain.
#[test]
fn cargo_answers_what_the_workspace_is() {
    let members = members();
    assert!(
        members.len() >= 11,
        "cargo named {} members: {members:?}",
        members.len()
    );
    for expected in ["motionvm-app", "motionvm-playable", "motionvm-motion"] {
        assert!(members.iter().any(|m| m == expected), "no {expected}");
    }
    // The front door is the crate that knows both halves, so it is the one
    // whose dependencies must be visible for the rules to mean anything.
    let front = dependencies("motionvm-motion");
    for expected in ["motionvm-motion-engine", "motionvm-motion-audio"] {
        assert!(
            front.iter().any(|d| d == expected),
            "motionvm-motion should depend on {expected}, got {front:?}"
        );
    }
    // And a dev-dependency is seen, which is the table a hand-written scanner
    // is likeliest to miss.
    assert!(
        dependencies("motionvm-motion-formats")
            .iter()
            .any(|d| d == "motionvm-motion-testutil"),
        "a dev-dependency should be visible"
    );
}

#[test]
fn no_neutral_crate_depends_on_a_family() {
    let members = members();
    assert!(members.len() > 1, "cargo should name every member");
    for name in &members {
        if layer(name) != Layer::Neutral {
            continue;
        }
        for dep in &dependencies(name) {
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
    for name in members() {
        let Some(family) = family_of(&name) else {
            continue;
        };
        for dep in dependencies(&name)
            .iter()
            .filter(|d| layer(d) != Layer::Neutral)
        {
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
    for name in members() {
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
                    text.get(i + 6..).is_some_and(|rest| rest.starts_with("VM")),
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

/// Every game on the roster has a module of its own, named for its slug.
///
/// The third roster, and the one nothing else could see: `Title::ALL` is the
/// list, `motion16::GAMES` is what `detect` walks, and `titles/<slug>.rs` is
/// where a game's own files and words live. A game with a line in the first
/// two and no file here would open when named and have nothing to open with.
///
/// Read off the directory rather than from the engine, because this crate
/// depends on nothing — which is also why the slugs are spelled out. They are
/// the names of files in a tree, and a test that took them from the enum could
/// not fail when the enum was the thing that was wrong.
#[test]
fn every_game_has_a_module_of_its_own() {
    let dir = crates_dir()
        .join("motionvm-motion-engine")
        .join("src")
        .join("titles");
    for slug in ["ds2", "enviro", "hfa", "jeffjet", "vloomes"] {
        let path = dir.join(format!("{slug}.rs"));
        assert!(
            path.is_file(),
            "{} is on the roster and has no module",
            path.display()
        );
    }
    // And nothing else in there is a game module masquerading as one: what is
    // left is the two generations' shared halves and the module list.
    let mut extra: Vec<String> = sources(&dir)
        .iter()
        .filter_map(|p| p.file_stem()?.to_str().map(str::to_owned))
        .filter(|n| !["ds2", "enviro", "hfa", "jeffjet", "vloomes"].contains(&n.as_str()))
        .collect();
    extra.sort();
    assert_eq!(
        extra,
        ["mod", "motion16", "motion32"],
        "an unexpected module sits among the games'"
    );
}
