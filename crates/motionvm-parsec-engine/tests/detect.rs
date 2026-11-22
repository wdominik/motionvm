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
