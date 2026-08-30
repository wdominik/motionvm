//! Victor Loomes boots on the 16-bit machine.
//!
//! The same `RUN` as the sibling games', out of the same module 100 — but at
//! word id 449 rather than 401, and it does not fetch a module 601, because
//! this game has none. These tests drive that far and ask for the picture.
//! They need the game's files — `MOTIONVM_GAMEDATA_VLOOMES` — and skip
//! without them.
//!
//! What they are really for is the thing this game does that no other does:
//! it gives its three screens **three different controllers**. `RUN` hands
//! `CTRL` to the scene screen, another word to the caption screen and none at
//! all to the inventory screen, then selects the scene screen and enters
//! `ANIMPLAY`. Read as one controller per engine — which is how the later
//! games can be read, since each gives only one screen one — the last call
//! wins, that call is the inventory screen's `-1 SCRCTRL`, and the game ends
//! on the frame it starts: `ANIMPLAY` returns, `RUN` runs its teardown, and a
//! test that only asked "did it crash" would have said no.
//!
//! The game this file drives is Victor Loomes (MOTION 16-bit).

use motionvm_engine::{Title, titles};
use motionvm_testutil::gamedata_vloomes;

/// Words `CTRL`'s frames may walk past without effect: the sprite and text
/// status tables, which the 16-bit handlers keep for their own loader and
/// which a lazy loader has no use for. The sibling games' set, plus the
/// screen one — they are the engine's and not the game's.
const INERT: &[&str] = &["SCRSTAT", "TXTSTAT", "XGFXSTAT", "XGFXSTAT+"];

#[test]
fn the_directory_is_told_apart_by_its_engine_binary() {
    let Some(dir) = gamedata_vloomes() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    assert_eq!(titles::detect(&dir), Some(Title::VictorLoomes));
    let game = titles::open(&dir).expect("the game opens");
    assert_eq!(game.title(), Title::VictorLoomes);
    assert_eq!(game.title().name(), "Victor Loomes – Das Spiel");
    assert_eq!(game.display_size(), (320, 200));
    assert_eq!(game.pixel_aspect(), (6, 5));
}

#[test]
fn run_plays_the_intro_and_draws_it() {
    let Some(dir) = gamedata_vloomes() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    let mut game = titles::vloomes::open(&dir).expect("the game opens");
    game.start().expect("RUN starts");
    for _ in 0..1200 {
        game.pump().expect("pump");
        game.step().expect("step");
    }
    // The intro is module 610's, installed on its own screen, and it is a
    // page of text: the competition slide the game opens on. Whatever else
    // is uncertain about a still frame, an intro that drew nothing would be
    // a boot that only looked like one.
    let lit = game.render().pixels.iter().filter(|&&p| p != 0).count();
    assert!(lit > 500, "the intro drew {lit} pixels");
    assert!(!game.finished(), "RUN has not returned");
    // Still in the intro: `RUN` zeroes `AO` and only enters a location after
    // the intro is done with.
    assert_eq!(game.get_var(605, "AO"), Some(0));

    for w in game.engine.stubbed().keys() {
        assert!(
            INERT.contains(&w.as_str()),
            "{w} was reached and does nothing"
        );
    }
}

#[test]
fn the_intro_runs_on_the_screen_it_made_for_itself() {
    let Some(dir) = gamedata_vloomes() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    let mut game = titles::vloomes::open(&dir).expect("the game opens");
    game.start().expect("RUN starts");
    for _ in 0..1200 {
        game.pump().expect("pump");
        game.step().expect("step");
    }
    // `RUN` fetches module 610, and the intro makes a screen of its own and
    // hands it `ICTRL` — word id 567, the only controller running while the
    // intro is up. `RUN`'s own three screens come after it.
    let ctrls: Vec<i32> = game.engine.screens().iter().map(|s| s.controller).collect();
    assert_eq!(ctrls, [567], "the intro's screen runs ICTRL");
}
