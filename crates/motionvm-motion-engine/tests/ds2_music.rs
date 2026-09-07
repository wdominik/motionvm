//! `STARTTUNE` and `ENDTUNE` as the game uses them.
//!
//! Each location's scene macro ends with `NN -1 STARTTUNE _ACTMUSIC !`, and the
//! only stop in the whole game is in `INCLLOC`:
//! `_ACTMUSIC @ IF _ACTMUSIC @ ENDTUNE 0 _ACTMUSIC ! THEN`. So the two words
//! are exercised by simply entering locations.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_motion_engine::{Game, MusicSink};
use motionvm_motion_forth::m32::Vm;
use motionvm_motion_testutil::gamedata_ds2;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Call {
    Start {
        handle: i32,
        tune: i32,
        looping: bool,
        bytes: usize,
    },
    Stop {
        handle: i32,
    },
    Cut {
        handle: i32,
    },
    Sample {
        block: i32,
    },
}

/// A sink that only writes down what it was asked for.
#[derive(Default, Clone)]
struct Log(Arc<Mutex<Vec<Call>>>);

impl MusicSink for Log {
    fn start(&mut self, handle: i32, tune: i32, looping: bool, song: &[u8]) {
        self.0.lock().unwrap().push(Call::Start {
            handle,
            tune,
            looping,
            bytes: song.len(),
        });
    }
    fn stop(&mut self, handle: i32) {
        self.0.lock().unwrap().push(Call::Stop { handle });
    }
    fn cut(&mut self, handle: i32) {
        self.0.lock().unwrap().push(Call::Cut { handle });
    }
    fn start_sample(&mut self, _handle: i32, _wav: &[u8], _volume: u16, _loops: i32) {}
    fn stop_sample(&mut self, _handle: i32) {}
    fn music_volume(&mut self, _volume: u16) {}
    fn sample(&mut self, block: i32, _sample: &[u8]) {
        self.0.lock().unwrap().push(Call::Sample { block });
    }
}

fn game_with_music(dir: &std::path::Path) -> (Game<Vm>, Log) {
    let mut game = Game::open(dir).expect("game opens");
    let log = Log::default();
    game.engine.set_music(Box::new(log.clone()));
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    (game, log)
}

/// Entering a location starts its own tune, and leaving stops that handle.
///
/// The tune number is checked, not just "something started": the mapping is
/// location `N` → module `3NN` → tune `N`, and a rebuild that passed the two
/// arguments the other way round would ask for tune −1 and still look busy.
#[test]
fn a_location_starts_its_own_tune_and_the_next_one_stops_it() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let (mut game, log) = game_with_music(&dir);

    game.set_var(2, "_NEXTLOC", 1).expect("the classroom");
    for _ in 0..400 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame");
        if game.get_var(2, "_ACTLOC") == Some(1) && !game.engine.in_transition() {
            break;
        }
    }
    assert_eq!(game.get_var(2, "_ACTLOC"), Some(1));

    let calls = log.0.lock().unwrap().clone();
    let start = calls
        .iter()
        .rev()
        .find_map(|c| match c {
            Call::Start {
                handle,
                tune,
                looping,
                bytes,
            } => Some((*handle, *tune, *looping, *bytes)),
            _ => None,
        })
        .expect("the classroom starts a tune");
    assert_eq!(start.1, 1, "location 1 plays tune 1");
    assert!(start.2, "the game always asks for a loop");
    assert!(
        start.3 > 1000,
        "the whole block is handed over, got {} bytes",
        start.3
    );
    assert_eq!(
        game.get_var(2, "_ACTMUSIC"),
        Some(start.0),
        "_ACTMUSIC holds the handle STARTTUNE answered with"
    );

    // And the next location stops exactly that handle before starting its own.
    let handle = start.0;
    let before = log.0.lock().unwrap().len();
    game.set_var(2, "_NEXTLOC", 2).expect("the park");
    for _ in 0..600 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame");
        if game.get_var(2, "_ACTLOC") == Some(2) && !game.engine.in_transition() {
            break;
        }
    }
    let calls = log.0.lock().unwrap().clone();
    assert!(
        calls[before..].contains(&Call::Stop { handle }),
        "leaving stops the handle it was given: {:?}",
        &calls[before..]
    );
    assert!(
        calls[before..]
            .iter()
            .any(|c| matches!(c, Call::Start { tune: 2, .. })),
        "and the park starts tune 2"
    );
}

/// A location without music starts nothing.
///
/// Locations 12 and 16 carry no `STARTTUNE` at all — `INCLLOC` has already
/// stopped the previous tune, so the game is deliberately silent there. A
/// rebuild that fell back to "play tune `_ACTLOC`" would sound plausible and be
/// wrong.
#[test]
fn a_location_without_music_starts_nothing() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let (mut game, log) = game_with_music(&dir);
    game.set_var(2, "_NEXTLOC", 16).expect("location 16");
    for _ in 0..600 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame");
        if game.get_var(2, "_ACTLOC") == Some(16) && !game.engine.in_transition() {
            break;
        }
    }
    assert_eq!(game.get_var(2, "_ACTLOC"), Some(16));
    let calls = log.0.lock().unwrap().clone();
    assert!(
        !calls
            .iter()
            .any(|c| matches!(c, Call::Start { tune: 16, .. })),
        "location 16 has no tune of its own: {calls:?}"
    );
    assert_eq!(
        game.get_var(2, "_ACTMUSIC"),
        Some(0),
        "and nothing is playing"
    );
}

/// `STARTTUNE` leaves exactly one value behind, with or without sound.
///
/// The handler has a single exit (`0x7FB24`) with one `push()`. Counting
/// pushes statically gives two, because the handler branches and both arms
/// carry one — but only one arm runs. Answering with two leaves a stray zero on
/// the stack at every location change, growing it for the whole session.
#[test]
fn the_tune_words_leave_the_stack_as_they_found_it() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    for with_music in [false, true] {
        let mut game = Game::open(&dir).expect("game opens");
        if with_music {
            game.engine.set_music(Box::new(Log::default()));
        }
        game.start().expect("4:START");
        while game.pump().expect("startup runs") {}

        let settled = |g: &Game<Vm>| g.vm.data.len();
        game.set_var(2, "_NEXTLOC", 1).expect("the classroom");
        for _ in 0..400 {
            game.set_input(0, 0, false, false, 0).expect("input");
            game.step().expect("a frame");
            if game.get_var(2, "_ACTLOC") == Some(1) && !game.engine.in_transition() {
                break;
            }
        }
        let after_one = settled(&game);

        for target in [2, 3, 5] {
            game.set_var(2, "_NEXTLOC", target)
                .expect("another location");
            for _ in 0..600 {
                game.set_input(0, 0, false, false, 0).expect("input");
                game.step().expect("a frame");
                if game.get_var(2, "_ACTLOC") == Some(target) && !game.engine.in_transition() {
                    break;
                }
            }
        }
        assert_eq!(
            settled(&game),
            after_one,
            "three more location changes with music={with_music} moved the stack"
        );
    }
}
