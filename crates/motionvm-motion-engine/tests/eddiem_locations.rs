//! Falsches Spiel mit Eddie M., past the intro: `CTRL`'s frames in the
//! locations.
//!
//! `RUN` plays the intro, enters location 3 with the opening scene running
//! and parks in its `ANIMPLAY`; from there every frame is `CTRL`'s, over a
//! location's modules and blocks. These tests drive that far and on, and need
//! the game's files (`MOTIONVM_GAMEDATA_EDDIEM`); they skip without them.
//!
//! Fifteen locations, every one with all four of its blocks, so there is no
//! room to refuse — what walking them all is for is the artwork split over
//! three volumes: a room whose sprites are on the third volume comes up blank
//! if the reader stops at two, and a room whose sprites the occupancy word
//! names by bit comes up blank if the word is read as a count. And the walk
//! is where the game's sound effects are asked for: `PLAYSAMPLE` is reached
//! in the rooms, not in the intro, and the blocks it names go to the sink.
//!
//! The game this file drives is Falsches Spiel mit Eddie M. (MOTION 16-bit).

mod common;

use motionvm_motion_engine::{Game, MusicSink, titles};
use motionvm_motion_forth::m16::Vm;
use motionvm_motion_testutil::gamedata_eddiem;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// A sink that writes down the samples it was handed.
#[derive(Default, Clone)]
struct Samples(Arc<Mutex<Vec<i32>>>);

impl MusicSink for Samples {
    fn start(&mut self, _handle: i32, _tune: i32, _looping: bool, _song: &[u8]) {}
    fn stop(&mut self, _handle: i32) {}
    fn cut(&mut self, _handle: i32) {}
    fn sample(&mut self, block: i32, _sample: &[u8]) {
        self.0.lock().unwrap().push(block);
    }
}

/// Words `CTRL`'s frames may walk past without effect: the sprite and text
/// status tables, which the 16-bit handlers keep for their own loader and
/// which a lazy loader has no use for.
const INERT: &[&str] = &["TXTSTAT", "XGFXSTAT", "XGFXSTAT+", "XTXTSTAT"];

/// The game's fifteen locations. Every one has a scene module `100 + n`, a
/// macro `300 + n`, a click module `500 + n`, and the four blocks `200 + n`,
/// `400 + n`, `600 + n` and `800 + n`.
const LOCATIONS: std::ops::RangeInclusive<i32> = 1..=15;

/// `RUN` up to the first frame of the game's own loop: the intro played and
/// ended with a key, location 3 entered, `CTRL` installed — with a sink that
/// keeps the samples.
fn into_the_game(dir: &Path) -> (Game<Vm>, Samples) {
    let mut game = titles::eddiem::open(dir).expect("opens");
    let samples = Samples::default();
    game.engine.set_music(Box::new(samples.clone()));
    game.start().expect("RUN starts");
    while game.pump().expect("RUN runs to the intro's ANIMPLAY") {}
    assert!(game.engine.main_loop(), "the intro's ANIMPLAY was entered");
    let mut left_intro = false;
    for frame in 1..=2000 {
        let key = if frame % 200 == 0 { 32 } else { 0 };
        game.set_input(160, 100, false, false, key).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
        if !game.engine.main_loop() {
            left_intro = true;
        } else if left_intro && game.get_var(601, "ACTLOC") == Some(3) {
            return (game, samples);
        }
    }
    panic!("2000 frames and the intro never gave way to CTRL");
}

/// Asks for `n` and runs frames until the game is standing in it.
///
/// `CTRL` honours `NEXTLOC` under the opening scene as well as in play —
/// both branches of its `INTROON` split poll it — so nothing has to be
/// clicked away first.
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

/// The words a run walked past that are not inert.
fn walked_past(game: &Game<Vm>) -> Vec<String> {
    game.engine
        .stubbed()
        .keys()
        .filter(|w| !INERT.contains(&w.as_str()))
        .cloned()
        .collect()
}

#[test]
fn every_location_is_entered_through_nextloc_and_draws() {
    let Some(dir) = gamedata_eddiem() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
        return;
    };
    let (mut game, _) = into_the_game(&dir);
    for n in LOCATIONS {
        assert!(walk_to(&mut game, n), "location {n} was never entered");
        assert!(
            common::lit(&mut game) > 40_000,
            "location {n} was entered but drew almost nothing"
        );
    }
}

#[test]
fn walking_the_locations_reaches_no_word_the_engine_lacks() {
    let Some(dir) = gamedata_eddiem() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
        return;
    };
    let (mut game, samples) = into_the_game(&dir);
    for n in LOCATIONS {
        walk_to(&mut game, n);
        common::frames(&mut game, 300, &format!("location {n}"));
    }
    assert!(
        walked_past(&game).is_empty(),
        "words walked past without effect: {:?}",
        walked_past(&game)
    );
    // The scripted scenes in the rooms ask for their sound effects, and every
    // block they name is one of the thirteen samples the game ships.
    let asked = samples.0.lock().unwrap().clone();
    assert!(!asked.is_empty(), "the walk reached no PLAYSAMPLE at all");
    let shipped = [1, 9, 10, 12, 13, 14, 15, 17, 18, 20, 21, 22, 23];
    assert!(
        asked.iter().all(|b| shipped.contains(b)),
        "a block that is no sample was asked for: {asked:?}"
    );
    // And the ten constants are still the whole stack: fifteen rooms of
    // scripts left nothing behind for `CTRL`'s check to print.
    assert_eq!(game.vm.data, (1..=10).collect::<Vec<i32>>());
}
