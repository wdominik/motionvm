//! Title identification, without needing every DAT to parse.
//!
//! `missing_data` is the existence check [`Game::open`][motionvm_parsec_engine::Game::open]
//! runs first. These tests pin the messages so a wrong directory names what
//! it is missing rather than panicking inside a reader.

use motionvm_parsec_engine::missing_data;
use std::path::Path;

/// An empty path is "not a PARSEC game", not a crash.
#[test]
fn an_empty_directory_is_reported() {
    let missing = missing_data(Path::new("/no/such/parsec/game"));
    assert!(!missing.is_empty());
    assert!(
        missing.iter().any(|(w, _)| w.contains("PARSEC")),
        "{missing:?}"
    );
}

/// Every figure set the program loads is looked for: a Captain Zins
/// install without its second figure file is named as missing it, though
/// a fresh game starts with the first.
#[test]
fn every_figure_set_the_program_loads_is_required() {
    let Some(src) = motionvm_parsec_testutil::gamedata_capzins() else {
        eprintln!("skipping: no Captain Zins gamedata directory");
        return;
    };
    assert!(
        missing_data(&src).is_empty(),
        "a full install misses nothing"
    );
    let dir = std::env::temp_dir().join(format!("motionvm-parsec-sets-{}", std::process::id()));
    let res = dir.join("RESOURCE");
    std::fs::create_dir_all(&res).expect("temp dir");
    for entry in std::fs::read_dir(&src).expect("install") {
        let p = entry.expect("entry").path();
        if p.is_file() {
            std::fs::copy(&p, dir.join(p.file_name().expect("name"))).expect("copy");
        }
    }
    for entry in std::fs::read_dir(src.join("RESOURCE")).expect("RESOURCE") {
        let p = entry.expect("entry").path();
        let name = p
            .file_name()
            .expect("name")
            .to_string_lossy()
            .to_uppercase();
        if name != "PLAY01.DAT" {
            std::fs::copy(&p, res.join(&name)).expect("copy");
        }
    }
    let missing = missing_data(&dir);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        missing.iter().map(|(w, _)| *w).collect::<Vec<_>>(),
        ["PLAY0%.DAT"],
        "{missing:?}"
    );
}
