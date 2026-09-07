//! Falsches Spiel mit Eddie M. boots on the 16-bit machine.
//!
//! The same `RUN` as the sibling games', out of the same module 100 — at word
//! id 411 rather than their 401 — and the same shape: load the library, play
//! the intro through `STARTINTRO`, enter a location and run `ANIMPLAY`. The
//! intro is two `ANIMPLAY`s of its own with a poll loop between them, and the
//! second gives way to the game's own loop in location 3, the flat, with the
//! opening scene running under `INTROON`. These tests drive that far and ask
//! for the picture, and for what the music was asked. They need the game's
//! files — `MOTIONVM_GAMEDATA_EDDIEM` — and skip without them.
//!
//! What they are really for is the one thing this game does that no other
//! does: it checks the engine. `CTRL` opens every frame on `DUP 10 !=`, against
//! the ten constants `RUN` pushes before `TOGFX` and never pops, and prints the
//! top of the stack with `.` and `EMIT` when the check fails — so a kernel
//! word that leaves a cell behind is a word this game reports, where the
//! others carry the cell silently.
//!
//! That this directory is told apart and opens as this game is asked in
//! `titles_detected.rs`, one row per game; here the game is already open.
//!
//! The game this file drives is Falsches Spiel mit Eddie M. (MOTION 16-bit).

mod common;

use motionvm_motion_engine::{Game, MusicSink, titles};
use motionvm_motion_forth::m16::Vm;
use motionvm_motion_testutil::{Digests, digest, digest::Digest, gamedata_eddiem};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Words `CTRL`'s frames may walk past without effect: the sprite and text
/// status tables, which the 16-bit handlers keep for their own loader and
/// which a lazy loader has no use for. The sibling games' three and the text
/// one, because they are the engine's and not the game's.
const INERT: &[&str] = &["TXTSTAT", "XGFXSTAT", "XGFXSTAT+", "XTXTSTAT"];

/// The words a run walked past that are not inert.
fn walked_past(game: &Game<Vm>) -> Vec<String> {
    game.engine
        .stubbed()
        .keys()
        .filter(|w| !INERT.contains(&w.as_str()))
        .cloned()
        .collect()
}

/// A sink that writes down what it was asked for.
#[derive(Default, Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl MusicSink for Log {
    fn start(&mut self, _handle: i32, tune: i32, looping: bool, _song: &[u8]) {
        self.0
            .lock()
            .unwrap()
            .push(format!("start {tune} {looping}"));
    }
    fn stop(&mut self, _handle: i32) {
        self.0.lock().unwrap().push("stop".into());
    }
    fn cut(&mut self, _handle: i32) {
        self.0.lock().unwrap().push("cut".into());
    }
    fn start_sample(&mut self, _handle: i32, _wav: &[u8], _volume: u16, _loops: i32) {}
    fn stop_sample(&mut self, _handle: i32) {}
    fn music_volume(&mut self, _volume: u16) {}
    fn sample(&mut self, block: i32, _sample: &[u8]) {
        self.0.lock().unwrap().push(format!("sample {block}"));
    }
}

/// The game open with a recording sink, `RUN` started and parked in the
/// intro's first `ANIMPLAY`.
fn into_the_intro(dir: &Path) -> (Game<Vm>, Log) {
    let mut game = titles::eddiem::open(dir).expect("opens");
    let log = Log::default();
    game.engine.set_music(Box::new(log.clone()));
    game.start().expect("RUN starts");
    let mut frames = 0;
    while game.pump().expect("RUN runs to ANIMPLAY") {
        frames += 1;
        assert!(frames < 10_000, "RUN never reached ANIMPLAY");
    }
    (game, log)
}

/// This game's table of reference digests.
fn digests() -> Digests {
    common::digests("eddiem")
}

#[test]
fn run_reaches_the_intro_loop_and_the_first_frames_draw_a_picture() {
    let Some(dir) = gamedata_eddiem() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
        return;
    };
    let (mut game, log) = into_the_intro(&dir);
    assert!(game.engine.main_loop(), "ANIMPLAY was entered");
    assert!(
        game.engine.controller().is_some(),
        "the intro installed its controller with SCRCTRL"
    );
    assert!(
        game.palette().raw.iter().any(|&v| v != 0),
        "a palette was installed"
    );
    // Before the intro is over no location is active, and `STARTLOC` holds
    // the 3 module 601 declares — the game never stores it.
    assert_eq!(game.get_var(601, "STARTLOC"), Some(3));
    assert_eq!(game.get_var(601, "ACTLOC"), Some(-1));
    assert_eq!(game.start_location(), None);

    // Three hundred frames: the title animation's `ANIMPLAY`, the poll loop
    // that waits for a key or a click under the intro's tune — a key at
    // frame 200 ends it — and the second `ANIMPLAY` the credits run in.
    // Every frame folded into one digest, because the intro is an animation
    // and a still from its end would say nothing about the pictures before.
    let mut lit_frames = 0;
    let mut intro = Digest::new();
    for frame in 1..=300 {
        let key = if frame % 200 == 0 { 32 } else { 0 };
        game.set_input(160, 100, false, false, key).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("the intro stopped at frame {frame}: {e}"));
        let picture = game.render();
        assert_eq!((picture.width, picture.height), (320, 200));
        intro.number(digest::frame(&picture));
        if picture.pixels.iter().any(|&p| p != 0) {
            lit_frames += 1;
        }
    }
    assert!(
        lit_frames > 200,
        "only {lit_frames} of 300 frames drew anything"
    );
    digests().check("intro", intro.value());
    // The intro's tune, block 24, started looping when the title animation
    // gave way to the poll loop, and stopped — with the fade, not cut — when
    // the key ended the loop. `MAC` is 0, so the digital rendition of the
    // same module the script would play instead is never asked for.
    let calls = log.0.lock().unwrap().clone();
    assert_eq!(calls, ["start 24 true", "stop"]);
    assert!(
        walked_past(&game).is_empty(),
        "words walked past without effect: {:?}",
        walked_past(&game)
    );
}

#[test]
fn run_enters_location_3_with_the_opening_scene_running() {
    let Some(dir) = gamedata_eddiem() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
        return;
    };
    let (mut game, _log) = into_the_intro(&dir);
    // The intro's second `ANIMPLAY` returns on its own once the credits are
    // over; `RUN` then sets `1 INTROON !`, enters `STARTLOC` and runs the
    // game's `ANIMPLAY`. Seen from outside that is the main-loop flag
    // dropping and rising again, with a location active the second time.
    let mut left_intro = false;
    let mut settled = false;
    for frame in 1..=2000 {
        let key = if frame % 200 == 0 { 32 } else { 0 };
        game.set_input(160, 100, false, false, key).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
        if !game.engine.main_loop() {
            left_intro = true;
        } else if left_intro && game.get_var(601, "ACTLOC") == Some(3) {
            settled = true;
            break;
        }
    }
    assert!(settled, "2000 frames and the intro never gave way to CTRL");
    // Location 3 is the flat — the middle of this game's numbering, where
    // Die Enviro-Kids greifen ein begins at 1, Jeff Jet at 13 and Hilfe für
    // Amajambere at 20. Nothing in the engine assumes any of them.
    assert_eq!(game.get_var(601, "STARTLOC"), Some(3));
    assert_eq!(game.get_var(601, "ACTLOC"), Some(3));
    assert_eq!(game.start_location(), Some(3));
    // The opening scene: `RUN` raised `INTROON` before entering the flat, and
    // `CTRL` runs the location's scripted sequence under it rather than the
    // player's clicks.
    assert_eq!(game.get_var(607, "INTROON"), Some(1));
    common::settle(&mut game);
    assert!(common::lit(&mut game) > 40_000, "the flat is drawn");
    digests().check("location_3", digest::frame(&game.render()));
    // Two screens, and between them the whole display: a room 960×280 seen
    // through a 320×165 window, and the 320×35 strip the verbs and the
    // inventory stand on. 165 + 35 = 200, the geometry of the three later
    // games.
    let screens: Vec<((u16, u16), (u16, u16))> = game
        .engine
        .screens()
        .iter()
        .map(|s| (s.size, s.view))
        .collect();
    assert_eq!(screens, [((960, 280), (320, 165)), ((320, 35), (320, 35))]);
    // The check this game runs on the engine, passed: the ten constants are
    // still the whole stack, so `CTRL`'s `DUP 10 !=` never printed.
    assert_eq!(game.vm.data, (1..=10).collect::<Vec<i32>>());
    assert!(
        walked_past(&game).is_empty(),
        "words walked past without effect: {:?}",
        walked_past(&game)
    );
}
