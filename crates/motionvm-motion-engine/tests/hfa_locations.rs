//! Hilfe für Amajambere, past the intro: `CTRL`'s frames in the locations.
//!
//! `RUN` plays the intro, enters location 20 with the game's menu up and parks
//! in its `ANIMPLAY`; from there every frame is `CTRL`'s, over a location's
//! modules and blocks. These tests drive that far and on, and need the game's
//! files (`MOTIONVM_GAMEDATA_HFA`); they skip without them.
//!
//! Twenty locations, more than either sibling, and two things make walking all
//! of them worth doing. The container is split by kind rather than by half —
//! every sprite and every palette is on the second volume — so a room that
//! comes up blank is a container fault and not a script one. And the game must
//! be clicked out of its menu first: `CTRL` only honours `NEXTLOC` while
//! `_INVMODE` is below 2, and `RUN` leaves it at exactly 2.
//!
//! The game this file drives is Hilfe für Amajambere (MOTION 16-bit).

mod common;

use motionvm_motion_engine::{Game, titles};
use motionvm_motion_forth::m16::Vm;
use motionvm_motion_testutil::{Digests, digest, gamedata_hfa};
use std::path::Path;

/// Words `CTRL`'s frames may walk past without effect: the sprite and text
/// status tables, which the 16-bit handlers keep for their own loader and
/// which a lazy loader has no use for. The same three as the sibling games',
/// because they are the engine's and not the game's.
const INERT: &[&str] = &["TXTSTAT", "XGFXSTAT", "XGFXSTAT+"];

/// The game's twenty locations. Every one has a scene module `100 + n`, a
/// macro `300 + n` and a click module `500 + n`, and the blocks `400 + n`,
/// `600 + n` and `800 + n` — and all but location 7 an item table `300 + n`.
const LOCATIONS: std::ops::RangeInclusive<i32> = 1..=20;

/// The one location the game ships no item table for.
///
/// `INCLLOC` loads block `300 + N` into `_LDITEM` unconditionally, and block
/// 307 is not in the container — flag and offsets agree that it was never
/// written. The original engine does not paper over it either: its `GET`
/// (`BMZ.EXE` `12bb:0e91`) tests the resolved block for null and, finding
/// none, calls its own error reporter with code `0xE` — *Fehler diverser Natur
/// (FDN)*, the last of the fifteen messages at file `0x20626` — and returns
/// with the destination unfilled. The room is reachable in play, from location
/// 11, so this is a fault in the shipped game rather than in the reading of
/// it; motionvm refuses the location by name where the original carries on
/// with a stale table.
const WITHOUT_ITEM_TABLE: i32 = 7;

/// `RUN` up to the first frame of the game's own loop: the intro clicked
/// through, location 20 entered, `CTRL` installed.
fn into_the_game(dir: &Path) -> Game<Vm> {
    let mut game = titles::hfa::open(dir).expect("opens");
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

/// The same, with the menu clicked away.
///
/// `RUN` ends on location 20 with `2 _INVMODE !` and the menu drawn, and
/// `CTRL`'s `_INVMODE @ 2 <` guard means no location can be asked for until it
/// is gone. The button that closes it is in the strip below the world
/// viewport, which is where `CTRL` computes `_IMX`/`_IMY` from the pointer.
fn out_of_the_menu(dir: &Path) -> Game<Vm> {
    let mut game = into_the_game(dir);
    assert_eq!(game.get_var(601, "_INVMODE"), Some(2), "the menu is up");
    for (press, frames) in [(false, 2), (true, 2), (false, 2)] {
        for _ in 0..frames {
            game.set_input(30, 185, press, false, 0).expect("input");
            game.step().expect("the menu takes the click");
        }
    }
    for frame in 1..=200 {
        game.set_input(2, 2, false, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("settling frame {frame}: {e}"));
    }
    let mode = game.get_var(601, "_INVMODE").expect("_INVMODE");
    assert!(mode < 2, "the menu is still up: _INVMODE is {mode}");
    game
}

/// This game's table of reference digests.
fn digests() -> Digests {
    common::digests("hfa")
}

/// Asks for `n` and runs frames until the game is standing in it.
fn walk_to(game: &mut Game<Vm>, n: i32) -> bool {
    game.request_location(n).expect("NEXTLOC");
    for frame in 1..=800 {
        game.set_input(2, 2, false, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("location {n}, frame {frame}: {e}"));
        if game.get_var(601, "ACTLOC") == Some(n) && game.get_var(601, "NEXTLOC") == Some(-1) {
            return true;
        }
    }
    false
}

#[test]
fn run_plays_through_the_intro_into_location_20() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    let mut game = into_the_game(&dir);
    // `RUN`'s own first location is 20 — the end of this game's numbering,
    // where Die Enviro-Kids greifen ein begins at 1 and Jeff Jet at 13.
    // Nothing in the engine assumes any of them.
    assert_eq!(game.get_var(601, "ACTLOC"), Some(20));
    assert!(common::lit(&mut game) > 40_000, "the room is drawn");
    // The room `RUN` leaves the game standing in, reached by the game's own
    // steps and by no shortcut, so the picture is the same on any machine
    // that has the game.
    digests().check("location_20", digest::frame(&game.render()));
    // Two screens, and between them the whole display: a room 960×544 seen
    // through a 320×165 window that scrolls, and the 320×35 strip the verbs
    // and the inventory stand on. 165 + 35 = 200. The same geometry as Jeff
    // Jet's, which is the engine's and not the game's.
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
fn every_location_with_an_item_table_is_entered_through_nextloc_and_draws() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    let mut game = out_of_the_menu(&dir);
    for n in LOCATIONS {
        if n == WITHOUT_ITEM_TABLE {
            continue;
        }
        assert!(walk_to(&mut game, n), "location {n} was never entered");
        assert!(
            common::lit(&mut game) > 40_000,
            "location {n} was entered but drew almost nothing"
        );
    }
}

#[test]
fn the_location_without_an_item_table_is_refused_by_name() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    let mut game = out_of_the_menu(&dir);
    game.request_location(WITHOUT_ITEM_TABLE).expect("NEXTLOC");
    let mut err = None;
    for _ in 1..=800 {
        game.set_input(2, 2, false, false, 0).expect("input");
        if let Err(e) = game.step() {
            err = Some(e.to_string());
            break;
        }
    }
    let err = err.expect("entering location 7 asks for a block the game does not ship");
    assert!(
        err.contains("307"),
        "the refusal should name the missing block: {err}"
    );
}

#[test]
fn walking_the_locations_reaches_no_word_the_engine_lacks() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    let mut game = out_of_the_menu(&dir);
    for n in LOCATIONS {
        if n == WITHOUT_ITEM_TABLE {
            continue;
        }
        walk_to(&mut game, n);
        for frame in 1..=400 {
            game.set_input(160, 100, false, false, 0).expect("input");
            game.step()
                .unwrap_or_else(|e| panic!("location {n}, frame {frame}: {e}"));
        }
    }
    // `SDNORM` besides the three: it is not called by any game's bytecode but
    // noted by the engine's own conversation path, where the original resets a
    // descriptor motionvm never gave a template to. `enviro_locations.rs`
    // allows it in the same way and for the same reason.
    let stubbed = game.engine.stubbed();
    let walked: Vec<&String> = stubbed
        .keys()
        .filter(|w| w != &"SDNORM" && !INERT.contains(&w.as_str()))
        .collect();
    assert!(
        walked.is_empty(),
        "words walked past without effect: {walked:?}"
    );
}
