//! A quit whose ending cannot be read ends the game and says why.
//!
//! What `XP01.EXE` shows after its main loop comes out of
//! `RESOURCE/EXTROS.DAT`. When that file is not there the run ends at once,
//! which in a window looks like the game closing after the last answer —
//! so the reason has to reach the host as a diagnostic, naming the file,
//! and not stay inside the engine.
//!
//! The install is a scratch directory of symlinks to the game's files with
//! `EXTROS.DAT` left out. The quit is played through the public interface
//! from the options at the start: *Spiel beenden*, Tab to the first answer
//! and a click, Tab to *Nein!* at the question whether to save and a click.
//! Needs `MOTIONVM_GAMEDATA_DS1`; skips without it.
//!
//! The game this file drives is Dunkle Schatten (PARSEC).

// Symlinks build the scratch install, and only unix has them as the
// standard library spells them.
#![cfg(unix)]

use motionvm_parsec_engine::{Game, Runtime, Start};
use motionvm_playable::{Button, Key, KeyPress};
use std::path::Path;

/// The install at `original` without `RESOURCE/EXTROS.DAT`, as symlinks
/// under `at`.
fn without_extros(original: &Path, at: &Path) {
    use std::os::unix::fs::symlink;
    std::fs::create_dir(at).expect("the install's directory");
    for entry in std::fs::read_dir(original).expect("the game lists") {
        let entry = entry.expect("an entry");
        let name = entry.file_name();
        if !name.eq_ignore_ascii_case("RESOURCE") {
            symlink(entry.path(), at.join(&name)).expect("a link");
            continue;
        }
        let resource = at.join(&name);
        std::fs::create_dir(&resource).expect("RESOURCE");
        for file in std::fs::read_dir(entry.path()).expect("RESOURCE lists") {
            let file = file.expect("an entry");
            if !file.file_name().eq_ignore_ascii_case("EXTROS.DAT") {
                symlink(file.path(), resource.join(file.file_name())).expect("a link");
            }
        }
    }
}

fn tab() -> KeyPress {
    KeyPress {
        key: Key::Tab,
        text: None,
        shift: false,
        ctrl: false,
        alt: false,
    }
}

#[test]
fn a_quit_without_its_ending_says_which_file_is_missing() {
    let Some(original) = motionvm_parsec_testutil::gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let scratch = motionvm_parsec_testutil::saves_dir("ds1-ending-unread");
    let install = scratch.join("install");
    without_extros(&original, &install);

    let mut game = Game::open(&install).expect("the install opens");
    game.set_saves(&scratch.join("saves"))
        .expect("a save directory");
    game.skip_title(Start::New);
    game.start().expect("the game starts");
    let mut ended = None;
    for step in 0..1500 {
        if [450, 650].contains(&step) {
            game.key_down(&tab());
        }
        if [300, 500, 700].contains(&step) {
            game.button(Button::Left, true);
        }
        if [301, 501, 701].contains(&step) {
            game.button(Button::Left, false);
        }
        game.step().expect("a step");
        if game.finished() {
            ended = Some(step);
            break;
        }
    }
    assert!(ended.is_some(), "the quit ends the game");
    let said: Vec<String> = game.diagnostics().iter().map(ToString::to_string).collect();
    assert!(
        said.iter()
            .any(|d| d.starts_with("the ending: ") && d.contains("EXTROS.DAT")),
        "the diagnostics name the missing file: {said:?}"
    );
    let _ = std::fs::remove_dir_all(&scratch);
}
