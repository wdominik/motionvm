//! Every spelling of a save directory inside the install is refused, and
//! nothing is created there.
//!
//! The install is treated as read-only, so `set_saves` refuses a directory
//! inside it — and the refusal is only worth something if it holds for a path
//! that does not exist yet, spelled relative to the working directory, through
//! a `..`, or through a symlink. Each of those is a path a plain comparison
//! with the install's absolute name lets through, and each is followed here
//! by a look at the install for anything the attempt left behind.
//!
//! One test, alone in its file: a relative path means the working directory,
//! which belongs to the whole process, and this binary runs nothing beside it.
//! The install is a scratch directory of symlinks to the game's files, so
//! that a refusal that fails creates its directory in scratch space and not
//! among the game's data. Needs `MOTIONVM_GAMEDATA_DS1`; skips without it.
//!
//! The game this file drives is Dunkle Schatten (PARSEC).

// Symlinks build the scratch install, and only unix has them as the
// standard library spells them.
#![cfg(unix)]

use motionvm_parsec_engine::{Game, Runtime};
use std::path::Path;

/// The save directory each attempt names: one the install does not hold. A
/// name it does hold — Dunkle Schatten's own `SAVES`, say, on a file system
/// that ignores case — would resolve through the scratch install's symlink to
/// the game's real directory, which is not the install under test.
const NEW: &str = "motionvm-test-saves";

/// What the install directory holds, by name.
fn names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("the install lists")
        .map(|e| {
            e.expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

#[test]
fn a_save_directory_inside_the_install_is_refused_however_it_is_spelled() {
    let Some(original) = motionvm_parsec_testutil::gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let scratch = motionvm_parsec_testutil::saves_dir("ds1-save-paths");
    let install = scratch.join("install");
    std::fs::create_dir(&install).expect("the install's directory");
    for entry in std::fs::read_dir(&original).expect("the game lists") {
        let entry = entry.expect("an entry");
        std::os::unix::fs::symlink(entry.path(), install.join(entry.file_name()))
            .expect("a link to one of the game's files");
    }
    let before = names(&install);
    assert!(
        !before.iter().any(|n| n.eq_ignore_ascii_case(NEW)),
        "the install holds no {NEW}"
    );
    let outside = scratch.join("outside");
    std::fs::create_dir(&outside).expect("a directory beside the install");
    std::os::unix::fs::symlink(&install, outside.join("link")).expect("a link into the install");

    let cwd = std::env::current_dir().expect("a working directory");
    std::env::set_current_dir(&install).expect("into the install");
    let mut game = Game::open(&install).expect("the install opens");
    let refused = [
        (
            "relative to the working directory",
            Path::new(NEW).to_path_buf(),
        ),
        ("absolute", install.join(NEW)),
        ("through ..", install.join("..").join("install").join(NEW)),
        ("through a symlink", outside.join("link").join(NEW)),
    ];
    for (how, dir) in &refused {
        let e = game.set_saves(dir).expect_err(how);
        assert_eq!(
            e.to_string(),
            "refusing a save directory inside the game data",
            "{how}"
        );
        assert_eq!(names(&install), before, "{how}: nothing was created");
    }

    // And a relative path that is not inside is taken, from the working
    // directory.
    std::env::set_current_dir(&outside).expect("beside the install");
    game.set_saves(Path::new(NEW))
        .expect("a save directory outside");
    std::env::set_current_dir(&cwd).expect("back");
    let at = game.saves().expect("a save directory").to_path_buf();
    assert!(
        at.ends_with(Path::new("outside").join(NEW).join("ds1")),
        "the game's own directory under the relative one: {}",
        at.display()
    );
    assert!(at.is_dir(), "and it exists");
    let _ = std::fs::remove_dir_all(&scratch);
}
