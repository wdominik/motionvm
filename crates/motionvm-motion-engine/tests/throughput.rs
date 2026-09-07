//! How fast the machines run. Not a test — a measurement rig.
//!
//! It asserts nothing about the numbers, because a rate is a property of the
//! machine it was measured on. What it is for is that a claim about the cost
//! of the interpreter can be reproduced and argued with, and that a change to
//! how a word is dispatched or how the two machines share their primitives is
//! measured before and after rather than reasoned about. Plausible arguments
//! about where the time goes are routinely wrong in both directions.
//!
//! Three numbers per game, and each is a different kind of thing:
//!
//! - **cells per second** — the interpreter's own rate, over every cell the
//!   game executed: calls, returns, primitives and host words alike.
//! - **host words per second**, and the share of cells they are. These are the
//!   cells that leave the interpreter for the engine, and today the only ones
//!   dispatched by *name*; a change to that dispatch shows up here or nowhere.
//! - **microseconds per frame**, split into the step and the draw. A frame is
//!   both, and the two move independently — the drawer's cost follows the
//!   scene, the interpreter's follows the script. The draw is measured the way
//!   a window does it, through `frame`.
//!
//! Cells and host words come off the machine's own counters rather than being
//! timed, so they are the same on any machine and the rates are the only part
//! that is not. Which makes the ratio between them the durable number: cells
//! per host word is what a dispatch change moves and a faster laptop does not.
//!
//! `#[ignore]`d, so `just check` never pays for it. `just bench` runs it, in
//! release — a debug build measures the optimizer, not the code.
//!
//! Every game the workspace plays is here, because the two machines are
//! different interpreters, the two 32-bit games run the same one over two
//! builds' kernels, and the five 16-bit games exercise the other over five
//! sets of scripts. A change that speeds one up and slows another down is
//! the interesting case and would be invisible from one game.

use motionvm_motion_engine::{Game, titles};
use motionvm_motion_forth::{Counters, Machine as _};
use motionvm_motion_testutil::{
    gamedata_checker, gamedata_ds2, gamedata_eddiem, gamedata_enviro, gamedata_hfa,
    gamedata_jeffjet, gamedata_vloomes,
};
use std::time::{Duration, Instant};

/// A machine counter as the float the rates are worked out in. Two thousand
/// frames of the busiest game execute a few million cells, nowhere near where
/// `f64` stops counting whole numbers.
#[expect(
    clippy::as_conversions,
    reason = "a counter of a few million, as a float for a rate"
)]
fn real(n: u64) -> f64 {
    n as f64
}

/// Frames to measure over, once the game is past its own startup.
///
/// Enough that a scheduler hiccup does not decide the answer, few enough that
/// the whole rig runs in seconds. The number is part of what is being
/// reported, so it is one constant and not five.
const FRAMES: u32 = 2_000;

/// What one game's run came to.
struct Run {
    counters: Counters,
    stepping: Duration,
    drawing: Duration,
}

/// Steps a game for [`FRAMES`] frames with no input, timing the step and the
/// draw apart, and answers what the machine did in that time.
///
/// A macro rather than a function generic over the machine, which is what this
/// wants to be: `set_input` is each generation's own inherent method — the two
/// write different shell variables — and `step` and `render` are bounded on a
/// trait the engine does not export, because nothing outside the crate is
/// meant to implement it. So the one thing the rig has to do for both machines
/// is written once here and expanded twice, rather than written twice.
///
/// No input, because input would make the run depend on what the pointer
/// happened to be over, and a measurement has to be the same run twice.
macro_rules! measure {
    ($game:expr) => {{
        let game = &mut $game;
        let before = game.vm.counters();
        let (mut stepping, mut drawing) = (Duration::ZERO, Duration::ZERO);
        for frame in 1..=FRAMES {
            game.set_input(0, 0, false, false, 0).expect("input");
            let t = Instant::now();
            game.step()
                .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
            stepping += t.elapsed();

            let t = Instant::now();
            // `frame`, not `render`: this is the window's path, and the two
            // differ by a copy. `render` composes and then hands the picture
            // over owned, which is what a test that keeps one wants; `frame`
            // composes into the buffer the engine keeps and lends it out,
            // which is what a window that shows one wants.
            std::hint::black_box(game.frame().pixels.width);
            drawing += t.elapsed();
        }
        let after = game.vm.counters();
        Run {
            counters: Counters {
                cells: after.cells - before.cells,
                host_words: after.host_words - before.host_words,
            },
            stepping,
            drawing,
        }
    }};
}

/// Prints one game's line, in the shape every game's line has.
fn report(name: &str, run: &Run) {
    let Run {
        counters,
        stepping,
        drawing,
    } = run;
    let secs = stepping.as_secs_f64();
    let per_frame = |d: Duration| d.as_secs_f64() * 1e6 / f64::from(FRAMES);
    println!("\n--- {name}, {FRAMES} frames ---");
    println!(
        "cells            {:>12}   {:>9.2} M/s",
        counters.cells,
        real(counters.cells) / secs / 1e6
    );
    println!(
        "host words       {:>12}   {:>9.2} M/s   one in {:.1} cells",
        counters.host_words,
        real(counters.host_words) / secs / 1e6,
        real(counters.cells) / real(counters.host_words.max(1))
    );
    println!("step()           {:>12.1} us/frame", per_frame(*stepping));
    println!("render()         {:>12.1} us/frame", per_frame(*drawing));
}

/// Dunkle Schatten 2, in the classroom the game starts a new game in.
///
/// Reached by the game's own path — `START`, then `_NEXTLOC` — and then given
/// four hundred frames to arrive, because a measurement taken while a location
/// is still loading is a measurement of the loader.
#[test]
#[ignore = "a measurement rig, not a test: `just bench`"]
fn dunkle_schatten_2() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(2, "_NEXTLOC", 1).expect("the classroom");
    for _ in 0..400 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame on the way in");
    }
    report("Dunkle Schatten 2 (32-bit)", &measure!(game));
}

/// Checker 2000, on the registration board its shell opens with.
///
/// Reached by the game's own path — `START` runs to `ANIMPLAY`, and the
/// board is up sixty frames on — and measured there because it is the one
/// scene every run of the game passes through: a text board with the
/// engine's arrow over it, and the task manager polling the pointer.
#[test]
#[ignore = "a measurement rig, not a test: `just bench`"]
fn checker_2000() {
    let Some(dir) = gamedata_checker() else {
        eprintln!("skipping: no Checker 2000 gamedata directory");
        return;
    };
    let mut game = titles::checker::open(&dir).expect("game opens");
    let saves = std::env::temp_dir().join(format!("motionvm-throughput-{}", std::process::id()));
    std::fs::create_dir_all(&saves).expect("a save directory");
    game.set_saves(&saves).expect("saves");
    game.start().expect("START");
    while game.pump().expect("START runs to ANIMPLAY") {}
    for _ in 0..60 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame on the way to the board");
    }
    report("Checker 2000 (32-bit, R78)", &measure!(game));
}

/// The five 16-bit games, each in its own intro.
///
/// The intro rather than a location, because it is what every one of them
/// reaches from `RUN` alone, with no shortcut and no game-specific
/// arrangement — and it is a busy frame: an animation, a text page and a
/// transition apparatus all running.
macro_rules! sixteen_bit {
    ($test:ident, $open:path, $data:path, $name:literal) => {
        #[test]
        #[ignore = "a measurement rig, not a test: `just bench`"]
        fn $test() {
            let Some(dir) = $data() else {
                eprintln!("skipping: no {} gamedata directory", $name);
                return;
            };
            let mut game = $open(&dir).expect("the game opens");
            game.start().expect("RUN starts");
            while game.pump().expect("RUN runs to ANIMPLAY") {}
            report($name, &measure!(game));
        }
    };
}

sixteen_bit!(
    die_enviro_kids_greifen_ein,
    titles::enviro::open,
    gamedata_enviro,
    "Die Enviro-Kids greifen ein (16-bit)"
);
sixteen_bit!(
    hilfe_fuer_amajambere,
    titles::hfa::open,
    gamedata_hfa,
    "Hilfe für Amajambere (16-bit)"
);
sixteen_bit!(
    jeff_jet,
    titles::jeffjet::open,
    gamedata_jeffjet,
    "Jeff Jet (16-bit)"
);
sixteen_bit!(
    victor_loomes,
    titles::vloomes::open,
    gamedata_vloomes,
    "Victor Loomes (16-bit)"
);
sixteen_bit!(
    falsches_spiel_mit_eddie_m,
    titles::eddiem::open,
    gamedata_eddiem,
    "Falsches Spiel mit Eddie M. (16-bit)"
);
