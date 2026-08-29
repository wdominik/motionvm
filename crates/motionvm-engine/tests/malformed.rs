//! A damaged game directory is reported, not crashed on.
//!
//! `motionvm-formats` has a file of this shape already, and its header says
//! why: the rest of that crate's tests read the games' own files, which are
//! well-formed by construction, so the whole suite could pass while the
//! readers panicked on the first byte anyone else's copy had wrong. The same
//! held one layer up and had no counter-test. [`titles::open`] takes the same
//! untrusted bytes — it opens a container, lifts a kernel table out of an
//! executable and parses every script module behind it — and everything that
//! reaches it has been through a CD-ROM drive, an archiver or a download.
//!
//! What is asserted here is narrow on purpose: an `Err` that **names the
//! directory it was handed**, rather than an unwind, an abort, or a message
//! about a type. The person reading it has just copied a 1996 CD and has no
//! way to guess which of its thirty files mattered — and that holds for the
//! messages the readers below write as much as for the opener's own, because
//! `motionvm_engine::Error::Data` puts the path in front of them. Written the
//! other way round, the reader alone says "read of 4 bytes at 0x3c past end of
//! 17-byte buffer", which is true of every file in the directory.
//!
//! These need no game data. Every directory below is built here, which is the
//! point: like `motionvm-formats`' file, this one runs everywhere, and CI's
//! floor is the only thing CI proves.

use std::path::{Path, PathBuf};

use motionvm_engine::{Title, titles};

/// A directory of this test's own under `target/`, wiped before use.
///
/// Per test rather than shared: each one puts a different set of files in it,
/// and two tests sharing a directory would decide each other's outcome by
/// whichever ran first.
fn dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/test-dirs")
        .join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a test directory");
    dir
}

/// Writes one file into `dir`.
fn put(dir: &Path, name: &str, bytes: &[u8]) {
    std::fs::write(dir.join(name), bytes).expect("writing a test file");
}

/// The error `titles::open` answers with, as text — and a failure naming the
/// case when it opened something instead.
///
/// The message has to name the directory it was handed. That is the whole
/// difference between an error a player can act on and one they cannot, and
/// it holds for every case below, the ones a reader two crates down reports
/// included.
fn refused_naming_dir(dir: &Path, what: &str) -> String {
    match titles::open(dir) {
        Ok(_) => panic!("{what}: opened as a game"),
        Err(e) => {
            let text = e.to_string();
            assert!(
                text.contains(&dir.display().to_string()),
                "{what}: the message names no directory: {text}"
            );
            text
        }
    }
}

/// An `NNN.RSC` container with one slot per kind and every slot empty.
///
/// Valid and openable: six `u32` counts, twenty-four reserved bytes, then the
/// offset table, whose first entry has to land exactly where the table ends —
/// the container's own strongest self-check. Every offset is that same value,
/// so no slot has bytes and nothing behind the index is ever read. It exists
/// so a test can get *past* the container and fail on the next thing.
fn empty_rsc() -> Vec<u8> {
    let counts = [1u32; 6];
    let total = 2 * counts[0] + counts[1..].iter().sum::<u32>();
    let table_end = 0x30 + total * 4;
    let mut v = Vec::new();
    for c in counts {
        v.extend_from_slice(&c.to_le_bytes());
    }
    v.resize(0x30, 0);
    for _ in 0..total {
        v.extend_from_slice(&table_end.to_le_bytes());
    }
    v
}

/// A one-volume `DATA.-n-` over three GFX slots — an item, an empty slot, an
/// item — and nothing else. Valid and openable, for the same reason as
/// [`empty_rsc`]: it gets a test past the container.
fn minimal_dat() -> Vec<u8> {
    let mut v = vec![0u8; 0x26];
    v[0..2].copy_from_slice(&100u16.to_le_bytes()); // boot module
    v[2..4].copy_from_slice(&401u16.to_le_bytes()); // boot word
    v[4..6].copy_from_slice(&3u16.to_le_bytes()); // three GFX slots
    v[18..20].copy_from_slice(&1u16.to_le_bytes()); // one volume
    v.extend_from_slice(&[1u16, 0, 1].map(u16::to_le_bytes).concat());
    let first = (v.len() + 3 * 4) as u32;
    let items: [&[u8]; 2] = [&[1, 0, 1, 0, 0, 0], &[2, 0, 1, 0, 0, 0, 7, 8]];
    let second = first + items[0].len() as u32;
    v.extend_from_slice(&[first, second, second].map(u32::to_le_bytes).concat());
    v.extend_from_slice(items[0]);
    v.extend_from_slice(items[1]);
    v
}

#[test]
fn a_path_that_is_not_a_directory_says_so_rather_than_listing_files() {
    // Listing three missing files for a directory that does not exist
    // describes the symptom and hides the cause, which is usually a typo.
    let missing = dir("no_such").join("nowhere");
    assert_eq!(titles::detect(&missing), None);
    let text = refused_naming_dir(&missing, "a path that is not there");
    assert!(
        text.contains("no such directory"),
        "should name the cause: {text}"
    );

    // A regular file handed in where a directory was meant reaches the same
    // answer, and must not be opened and read as one.
    let d = dir("a_file_not_a_directory");
    put(&d, "001.RSC", &empty_rsc());
    refused_naming_dir(&d.join("001.RSC"), "a file in place of a directory");
}

#[test]
fn a_directory_holding_none_of_the_games_lists_what_each_would_need() {
    let d = dir("empty");
    assert_eq!(titles::detect(&d), None);
    let text = refused_naming_dir(&d, "an empty directory");
    for name in ["001.RSC", "DATA.-1-", "HPPLAY.EXE", "ENVIRO.EXE", "BMZ.EXE"] {
        assert!(text.contains(name), "should name {name}: {text}");
    }
}

#[test]
fn a_container_beside_no_engine_binary_is_not_guessed_at() {
    // All three 16-bit games ship a `DATA.-1-`, and the engine binary beside it is
    // the only thing that tells them apart. Neither binary means neither
    // game, and naming one of them and then failing on its missing files
    // would be worse than saying so.
    let d = dir("dat_alone");
    put(&d, "DATA.-1-", &minimal_dat());
    assert_eq!(titles::detect(&d), None);
    refused_naming_dir(&d, "a DATA.-1- with no engine binary");
}

#[test]
fn a_32_bit_directory_missing_a_required_file_names_it() {
    // `detect` reads file names only, so a lone `001.RSC` is answered as the
    // 32-bit game; what it is missing has to come out of the opener.
    let d = dir("rsc_alone");
    put(&d, "001.RSC", &empty_rsc());
    assert_eq!(titles::detect(&d), Some(Title::DunkleSchatten2));
    let text = refused_naming_dir(&d, "a container with no engine binary");
    assert!(text.contains("ENGINE.EXE"), "should name it: {text}");
    assert!(text.contains("000.FRT"), "and the font table: {text}");

    assert_eq!(
        titles::ds2::missing_data(&d)
            .iter()
            .map(|(n, _)| *n)
            .collect::<Vec<_>>(),
        ["ENGINE.EXE", "000.FRT"]
    );
}

#[test]
fn a_broken_32_bit_container_is_refused_rather_than_indexed_into() {
    // Empty, shorter than the header, an implausible slot total, and an
    // offset table whose first entry does not land where the table ends —
    // the four shapes a truncated or half-written `NNN.RSC` arrives in.
    let implausible = {
        let mut v = vec![0u8; 0x30];
        v[0..4].copy_from_slice(&u32::MAX.to_le_bytes());
        v
    };
    let truncated = {
        let mut v = empty_rsc();
        v.truncate(0x38);
        v
    };
    let misplaced = {
        let mut v = empty_rsc();
        v[0x30..0x34].copy_from_slice(&0u32.to_le_bytes());
        v
    };
    for (name, bytes) in [
        ("empty", Vec::new()),
        ("short", vec![0u8; 0x20]),
        ("implausible", implausible),
        ("truncated", truncated),
        ("misplaced", misplaced),
    ] {
        let d = dir(&format!("rsc_{name}"));
        put(&d, "001.RSC", &bytes);
        put(&d, "ENGINE.EXE", b"not an executable");
        put(&d, "000.FRT", &[0u8; 512]);
        refused_naming_dir(&d, &format!("a {name} container"));
    }
}

#[test]
fn an_engine_binary_that_is_not_an_le_image_is_refused() {
    // The container opens; the kernel table has to come out of `ENGINE.EXE`,
    // and without it the bytecode's ordinals mean nothing. Three shapes: a
    // file that is not an executable at all, an empty one, and an MZ stub
    // with no LE image behind it — which is what an interrupted copy of a
    // 845 KB binary leaves.
    let mz_stub = {
        let mut v = vec![0u8; 0x40];
        v[0..2].copy_from_slice(b"MZ");
        v
    };
    for (name, exe) in [
        ("garbage", b"not an executable".to_vec()),
        ("empty", Vec::new()),
        ("mz_only", mz_stub),
    ] {
        let d = dir(&format!("engine_{name}"));
        put(&d, "001.RSC", &empty_rsc());
        put(&d, "ENGINE.EXE", &exe);
        put(&d, "000.FRT", &[0u8; 512]);
        refused_naming_dir(&d, &format!("a {name} ENGINE.EXE"));
    }
}

#[test]
fn a_broken_16_bit_container_is_refused_rather_than_indexed_into() {
    // The 16-bit shapes: shorter than its header, no slots at all, and slot
    // counts whose tables run past the end of the file — 4345 slots need
    // 26 120 bytes and this one has 38.
    let no_slots = vec![0u8; 0x26];
    let overrunning = {
        let mut v = vec![0u8; 0x26];
        for (i, c) in [2500u16, 1000, 700, 25, 10, 10, 100].iter().enumerate() {
            v[4 + 2 * i..6 + 2 * i].copy_from_slice(&c.to_le_bytes());
        }
        v
    };
    for (name, bytes) in [
        ("short", vec![0u8; 0x25]),
        ("no_slots", no_slots),
        ("overrunning", overrunning),
    ] {
        let d = dir(&format!("dat_{name}"));
        put(&d, "DATA.-1-", &bytes);
        put(&d, "ENVIRO.EXE", b"not an executable");
        assert_eq!(titles::detect(&d), Some(Title::DieEnviroKidsGreifenEin));
        refused_naming_dir(&d, &format!("a {name} DATA.-1-"));
    }
}

#[test]
fn a_16_bit_engine_binary_that_is_not_mz_is_refused() {
    // The container opens and the game is told apart correctly; the word
    // table still has to come out of the binary beside it. All three 16-bit
    // games, because each reads its own — the three builds hold 233, 232 and
    // 228 words and `HPPLAY.EXE`'s ordinals are shifted besides, so no game's
    // table can stand in for another's.
    for (exe, title, volumes) in [
        ("ENVIRO.EXE", Title::DieEnviroKidsGreifenEin, 1),
        ("HPPLAY.EXE", Title::JeffJet, 2),
        ("BMZ.EXE", Title::HilfeFuerAmajambere, 2),
    ] {
        let d = dir(&format!("mz_{exe}"));
        put(&d, "DATA.-1-", &minimal_dat());
        if volumes == 2 {
            put(&d, "DATA.-2-", &minimal_dat());
        }
        put(&d, exe, b"not an executable");
        assert_eq!(titles::detect(&d), Some(title));
        refused_naming_dir(&d, &format!("a {exe} that is not an MZ image"));
    }
}

#[test]
fn jeff_jet_without_its_second_volume_is_refused_by_name() {
    // Volume 2 holds every palette, both fonts and the font reference table,
    // so a copy without it would find every script and no colour. Saying so
    // beats starting and drawing nothing.
    let d = dir("jeffjet_one_volume");
    put(&d, "DATA.-1-", &minimal_dat());
    put(&d, "HPPLAY.EXE", b"not an executable");
    assert_eq!(titles::detect(&d), Some(Title::JeffJet));
    let text = refused_naming_dir(&d, "Jeff Jet with one volume");
    assert!(text.contains("DATA.-2-"), "should name it: {text}");

    assert_eq!(
        titles::jeffjet::missing_data(&d)
            .iter()
            .map(|(n, _)| *n)
            .collect::<Vec<_>>(),
        ["DATA.-2-"]
    );
}

#[test]
fn amajambere_without_its_second_volume_is_refused_by_name() {
    // The same rule, and this game leans on it harder: its second volume holds
    // *every* sprite as well as every palette, font and the font reference
    // table, so a copy without it would find every script and nothing at all
    // to draw.
    let d = dir("hfa_one_volume");
    put(&d, "DATA.-1-", &minimal_dat());
    put(&d, "BMZ.EXE", b"not an executable");
    assert_eq!(titles::detect(&d), Some(Title::HilfeFuerAmajambere));
    let text = refused_naming_dir(&d, "Hilfe für Amajambere with one volume");
    assert!(text.contains("DATA.-2-"), "should name it: {text}");

    assert_eq!(
        titles::hfa::missing_data(&d)
            .iter()
            .map(|(n, _)| *n)
            .collect::<Vec<_>>(),
        ["DATA.-2-"]
    );
}

#[test]
fn a_lower_cased_install_is_found_and_an_almost_container_is_not() {
    // A copy that has been through a CD-ROM driver, an archiver or a file
    // manager often arrives lower-cased, and on a case-sensitive filesystem
    // an exact-case join reports the files as absent while they sit right
    // there. It must reach the same error as the upper-cased copy — the
    // engine binary — and not "no game here".
    let d = dir("lower_case");
    put(&d, "001.rsc", &empty_rsc());
    put(&d, "engine.exe", b"not an executable");
    put(&d, "000.frt", &[0u8; 512]);
    assert_eq!(titles::detect(&d), Some(Title::DunkleSchatten2));
    refused_naming_dir(&d, "a lower-cased install");
    assert!(titles::ds2::missing_data(&d).is_empty());

    // The pattern is three digits and nothing else: the engine loads
    // `%03d.rsc` and a file named otherwise is not one of its containers.
    let d = dir("not_a_container");
    for name in ["1.RSC", "0001.RSC", "OLD.RSC", "001.RSC.bak"] {
        put(&d, name, &empty_rsc());
    }
    assert_eq!(titles::detect(&d), None);
}
