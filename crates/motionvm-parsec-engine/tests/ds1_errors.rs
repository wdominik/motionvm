//! What a player reads when a directory cannot be played.
//!
//! These are the messages the window shows in its dialog and the command
//! line prints, so they are pinned character for character: a person who
//! has just copied Dunkle Schatten's disks has no way to guess which of its files
//! mattered, and the message is all they get.
//!
//! The first three build their directories in scratch space; the last two
//! need the game's files — `MOTIONVM_GAMEDATA_DS1` — and skip without them.
//!
//! The game this file drives is Dunkle Schatten (PARSEC).

use motionvm_parsec_engine::{Game, Runtime, locations};
use std::path::{Path, PathBuf};

/// A scratch directory holding empty files of these names.
fn holding(name: &str, files: &[&str]) -> PathBuf {
    let dir = motionvm_parsec_testutil::saves_dir(&format!("ds1-errors-{name}"));
    for f in files {
        std::fs::write(dir.join(f), b"").expect("a scratch file");
    }
    dir
}

fn open_err(dir: &Path) -> String {
    match Game::open(dir) {
        Ok(_) => panic!("{} opened", dir.display()),
        Err(e) => e.to_string(),
    }
}

#[test]
fn a_directory_without_a_game_names_what_one_needs() {
    let dir = holding("empty", &[]);
    assert_eq!(
        open_err(&dir),
        "not a PARSEC game directory\n  \
         missing a PARSEC game (XP00.EXE / XP01.EXE and RESOURCE/*.DAT \
         (DSROOM.DAT or CZROOM.DAT))"
    );
}

#[test]
fn an_incomplete_install_lists_every_missing_file() {
    let dir = holding("incomplete", &["DSROOM.DAT", "PANELS.DAT"]);
    assert_eq!(
        open_err(&dir),
        "incomplete Dunkle Schatten directory:\n  \
         missing CMBMOU.DAT (the font and the pointers)\n  \
         missing PLAY00.DAT (the player's cels and their palette bank)"
    );
}

#[test]
fn the_location_listing_refuses_a_directory_without_a_game() {
    let dir = holding("listing", &[]);
    let e = locations(&dir).expect_err("no game");
    assert_eq!(e.to_string(), "not a PARSEC game directory");
}

#[test]
fn a_save_directory_inside_the_install_is_refused() {
    let Some(dir) = motionvm_parsec_testutil::gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("the install opens");
    let e = game.set_saves(&dir).expect_err("refused");
    assert_eq!(
        e.to_string(),
        "refusing a save directory inside the game data"
    );
}

#[test]
fn a_game_starts_once() {
    let Some(dir) = motionvm_parsec_testutil::gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("the install opens");
    game.set_saves(&motionvm_parsec_testutil::saves_dir("ds1-errors-twice"))
        .expect("a save directory outside the install");
    game.start().expect("the first start");
    let e = game.start().expect_err("a second start");
    assert_eq!(e.to_string(), "the game has already started");
}
