//! Jeff Jet - Abenteuer InfoHighway, past the intro: `CTRL`'s frames in the
//! locations.
//!
//! `RUN` plays the intro, enters the game's first location and parks in its
//! `ANIMPLAY`; from there every frame is `CTRL`'s, over a location's modules
//! and blocks. These tests drive that far and on, and need the game's files
//! (`MOTIONVM_GAMEDATA_JEFFJET`); they skip without them.
//!
//! Thirteen locations, and the point of walking all of them is the container:
//! every backdrop is packed and half the artwork sits on the second volume, so
//! a room that comes up blank is a container fault and not a script one.
//!
//! The game this file drives is Jeff Jet (MOTION 16-bit).

use motionvm_engine::{Game, Playable, titles};
use motionvm_forth::m16::Vm;
use motionvm_testutil::gamedata_jeffjet;
use std::path::Path;

/// Words `CTRL`'s frames may walk past without effect: the sprite and text
/// status tables, which the 16-bit handlers keep for their own loader and
/// which a lazy loader has no use for. The same three as the other 16-bit
/// game's, because they are the engine's and not the game's.
const INERT: &[&str] = &["TXTSTAT", "XGFXSTAT", "XGFXSTAT+"];

/// The game's thirteen locations. Every one has a scene module `100 + n`, a
/// macro `300 + n`, a click module `500 + n` and the four blocks `200 + n`,
/// `400 + n`, `600 + n`, `800 + n`.
const LOCATIONS: std::ops::RangeInclusive<i32> = 1..=13;

/// `RUN` up to the first frame of the game's own loop: the intro clicked
/// through, the first location entered, `CTRL` installed.
fn into_the_game(dir: &Path) -> Game<Vm> {
    let mut game = titles::jeffjet::open(dir).expect("opens");
    game.start().expect("RUN starts");
    while game.pump().expect("RUN runs to the intro's ANIMPLAY") {}
    assert!(game.engine.main_loop(), "the intro's ANIMPLAY was entered");
    // The intro ends on its own or on a key; one every 200 frames gets through
    // it either way. A key rather than a click, because a click still latched
    // when `CTRL` takes over would land on whatever hotspot is under the
    // pointer.
    let mut left_intro = false;
    for frame in 1..=8000 {
        let key = if frame % 200 == 0 { 32 } else { 0 };
        game.set_input(160, 100, false, false, key).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
        if !game.engine.main_loop() {
            left_intro = true;
        } else if left_intro {
            return game;
        }
    }
    panic!("8000 frames and the intro never gave way to CTRL");
}

/// The same, with the arrival played out: `RUN` enters location 13 itself and
/// the scene there runs its course before anything else can be asked for.
fn settled_in_the_game(dir: &Path) -> Game<Vm> {
    let mut game = into_the_game(dir);
    for frame in 1..=1000 {
        // No clicks: the scene runs itself out.
        game.set_input(2, 2, false, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("arrival frame {frame} stopped: {e}"));
        if game.get_var(601, "ACTLOC") == Some(13) && game.get_var(601, "NEXTLOC") == Some(-1) {
            return game;
        }
    }
    panic!("the arrival never settled");
}

fn lit(game: &mut Game<Vm>) -> usize {
    game.render().pixels.iter().filter(|&&p| p != 0).count()
}

#[test]
fn run_plays_through_the_intro_into_the_first_location() {
    let Some(dir) = gamedata_jeffjet() else {
        eprintln!("skipping: no Jeff Jet gamedata directory");
        return;
    };
    let mut game = settled_in_the_game(&dir);
    // `RUN`'s own first location is 13, not 1 — the game begins at the end of
    // the numbering, where Die Enviro-Kids greifen ein begins at 1. Nothing in
    // the engine assumes either.
    assert_eq!(game.get_var(601, "ACTLOC"), Some(13));
    assert!(lit(&mut game) > 40_000, "the room is drawn");
    // Two screens, and between them the whole display: a room 960×544 seen
    // through a 320×165 window that scrolls, and the 320×35 strip the verbs
    // and the inventory stand on. 165 + 35 = 200.
    let screens: Vec<((u16, u16), (u16, u16))> = game
        .engine
        .screens()
        .iter()
        .map(|s| (s.size, s.view))
        .collect();
    assert_eq!(screens, [((960, 544), (320, 165)), ((320, 35), (320, 35))]);
    assert_eq!(screens.iter().map(|(_, v)| v.1).sum::<u16>(), 200);
}

#[test]
fn every_location_is_entered_through_nextloc_and_draws() {
    let Some(dir) = gamedata_jeffjet() else {
        eprintln!("skipping: no Jeff Jet gamedata directory");
        return;
    };
    let mut game = settled_in_the_game(&dir);
    for n in LOCATIONS {
        // The way the game moves itself: `CTRL` runs
        // `NEXTLOC @ -1 != IF NEXTLOC @ INCLLOC -1 NEXTLOC ! THEN` every frame,
        // and this is what `--loc` writes.
        game.request_location(n).expect("NEXTLOC takes the request");
        let mut entered = false;
        for frame in 1..=600 {
            game.set_input(2, 2, false, false, 0).expect("input");
            game.step()
                .unwrap_or_else(|e| panic!("location {n}, frame {frame}: {e}"));
            if game.get_var(601, "ACTLOC") == Some(n) && game.get_var(601, "NEXTLOC") == Some(-1) {
                entered = true;
                break;
            }
        }
        assert!(entered, "location {n} was never entered");
        // Every backdrop of this game is a packed item and about half of them
        // are on the second volume, so an empty room here is the container
        // failing, not the script.
        assert!(
            lit(&mut game) > 40_000,
            "location {n} came up all but empty"
        );
    }
}

#[test]
fn walking_the_locations_reaches_no_word_the_engine_lacks() {
    let Some(dir) = gamedata_jeffjet() else {
        eprintln!("skipping: no Jeff Jet gamedata directory");
        return;
    };
    let mut game = settled_in_the_game(&dir);
    for n in LOCATIONS {
        game.request_location(n).expect("NEXTLOC takes the request");
        for _ in 1..=400 {
            game.set_input(2, 2, false, false, 0).expect("input");
            game.step().expect("a frame");
            if game.get_var(601, "ACTLOC") == Some(n) && game.get_var(601, "NEXTLOC") == Some(-1) {
                break;
            }
        }
    }
    // The kernel is bound out of HPPLAY.EXE, whose ordinals run four below
    // ENVIRO.EXE's from 124 up. Anything walked past that is not one of the
    // three the engine deliberately ignores would be a word bound to the wrong
    // name.
    let unexpected: Vec<&String> = game
        .engine
        .stubbed()
        .keys()
        .filter(|w| !INERT.contains(&w.as_str()))
        .collect();
    assert!(
        unexpected.is_empty(),
        "words without effect: {unexpected:?}"
    );
}
