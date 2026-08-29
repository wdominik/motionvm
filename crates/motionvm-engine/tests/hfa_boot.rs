//! Hilfe für Amajambere boots on the 16-bit machine.
//!
//! The same `RUN` as the sibling games', out of the same module 100 and the
//! same word id 401: load the library, play the intro, enter its `ANIMPLAY`.
//! These tests drive that far and ask for the picture. They need the game's
//! files — `MOTIONVM_GAMEDATA_HFA` — and skip without them.
//!
//! What they are really for is this game's own opening. `RUN` writes
//! `20 STARTLOC !` and enters location 20, and because that is the location it
//! starts in it also loads module 650 and puts the game's menu up with
//! `2 _INVMODE !` before parking. So a boot that "works" here means more than
//! a picture: it means the frame handler is installed, the menu is on screen,
//! and the game is waiting for a click rather than walking a room.
//!
//! The game this file drives is Hilfe für Amajambere (MOTION 16-bit).

use motionvm_engine::{Playable, Title, titles};
use motionvm_testutil::gamedata_hfa;

/// Words `CTRL`'s frames may walk past without effect: the sprite and text
/// status tables, which the 16-bit handlers keep for their own loader and
/// which a lazy loader has no use for. The same three as the sibling games',
/// because they are the engine's and not the game's.
const INERT: &[&str] = &["TXTSTAT", "XGFXSTAT", "XGFXSTAT+"];

#[test]
fn the_directory_is_told_apart_by_its_engine_binary() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    // All three 16-bit games ship a DATA.-1-; only this one ships BMZ.EXE.
    assert_eq!(titles::detect(&dir), Some(Title::HilfeFuerAmajambere));
    let game = titles::open(&dir).expect("opens");
    assert_eq!(game.title(), Title::HilfeFuerAmajambere);
    assert_eq!(game.title().name(), "Hilfe für Amajambere");
    assert_eq!(game.display_size(), (320, 200));
    assert_eq!(game.pixel_aspect(), (6, 5));
}

#[test]
fn a_directory_holding_a_container_but_no_engine_binary_is_not_a_game() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    let tmp = std::env::temp_dir().join("motionvm-hfa-detect");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("a scratch directory");
    std::fs::copy(dir.join("DATA.-1-"), tmp.join("DATA.-1-")).expect("the container copies");
    assert_eq!(titles::detect(&tmp), None);
    assert!(titles::open(&tmp).is_err());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn run_reaches_the_intro_loop_and_the_first_frames_draw_a_picture() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    let mut game = titles::hfa::open(&dir).expect("opens");
    game.start().expect("RUN starts");
    let mut frames = 0;
    while game.pump().expect("RUN runs to ANIMPLAY") {
        frames += 1;
        assert!(frames < 10_000, "RUN never reached ANIMPLAY");
    }
    assert!(game.engine.main_loop(), "ANIMPLAY was entered");
    assert!(
        game.engine.controller().is_some(),
        "the intro installed its controller with SCRCTRL"
    );
    assert!(
        game.palette().raw.iter().any(|&v| v != 0),
        "a palette was installed"
    );
    let mut lit_frames = 0;
    for frame in 1..=1500 {
        game.step()
            .unwrap_or_else(|e| panic!("the intro stopped at frame {frame}: {e}"));
        let picture = game.render();
        assert_eq!((picture.width, picture.height), (320, 200));
        if picture.pixels.iter().any(|&p| p != 0) {
            lit_frames += 1;
        }
    }
    assert!(
        lit_frames > 1000,
        "only {lit_frames} of 1500 frames drew anything"
    );
    // `TXTSTAT` is one of the three the 16-bit handlers keep for their own
    // loader and a lazy loader has no use for — the engine's, not this game's.
    // Anything else walked past would be a word the intro needs and does not
    // get.
    let walked: Vec<&String> = game
        .engine
        .stubbed()
        .keys()
        .filter(|w| !INERT.contains(&w.as_str()))
        .collect();
    assert!(
        walked.is_empty(),
        "words walked past without effect: {walked:?}"
    );
}

#[test]
fn run_enters_location_20_and_opens_the_games_own_menu() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    let mut game = titles::hfa::open(&dir).expect("opens");
    game.start().expect("RUN starts");
    while game.pump().expect("RUN runs to ANIMPLAY") {}
    // Before the intro is over `STARTLOC` still holds the 9 module 601
    // declares, and no location is active — so the game has not yet said where
    // it begins.
    assert_eq!(game.get_var(601, "STARTLOC"), Some(9));
    assert_eq!(game.get_var(601, "ACTLOC"), Some(-1));
    assert_eq!(game.start_location(), None);

    let mut left_intro = false;
    let mut settled = false;
    for frame in 1..=8000 {
        let key = if frame % 200 == 0 { 32 } else { 0 };
        game.set_input(160, 100, false, false, key).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
        if !game.engine.main_loop() {
            left_intro = true;
        } else if left_intro {
            settled = true;
            break;
        }
    }
    assert!(settled, "8000 frames and the intro never gave way to CTRL");
    // `RUN`'s own first location is 20 — the end of this game's numbering,
    // where Die Enviro-Kids greifen ein begins at 1 and Jeff Jet at 13.
    assert_eq!(game.get_var(601, "STARTLOC"), Some(20));
    assert_eq!(game.get_var(601, "ACTLOC"), Some(20));
    assert_eq!(game.start_location(), Some(20));
    // And because location 20 is where it starts, `RUN` also fetched module
    // 650 and put the menu up rather than handing the room straight over.
    assert_eq!(game.get_var(601, "_INVMODE"), Some(2), "the menu is up");
}
