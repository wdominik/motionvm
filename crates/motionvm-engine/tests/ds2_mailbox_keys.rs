//! The cursor keys drive the mailbox.
//!
//! Module 216 is the in-game BBS, and its `LTMANAGER` dispatches on four codes
//! at `0x0c21c`: 331 and 333 walk the menu bar with `HOTMEN`, 328 and 336 walk
//! the selection list with `SELUP` and `SELDOWN`, and 13 or 32 takes what is
//! highlighted. Those four numbers are `0x100` over the scan codes of the four
//! cursor keys — `0x4b`, `0x4d`, `0x48`, `0x50` — which is what `?KEY` answers
//! for a key that carries no character (`ENGINE.EXE` `0x23894`, reached from
//! the translator at `0x2379b` that both `?KEY` and `KEY` call).
//!
//! The game's own manual says this is how it is meant to be played (text bank
//! 006, entry 60): *"danach kannst Du Dich mit den Pfeiltasten durch die Menüs
//! bewegen und diese mit RETURN anwählen"*. It could not be, because the
//! frontend answered only characters and dropped every key that has none — the
//! same shape of fault `escape_key.rs` holds down one layer lower.
//!
//! What broke was the frontend, and the unit tests in `motionvm-app`'s `keys`
//! module are what hold the four codes down. This file holds down the other
//! half — that those codes are the ones the terminal moves on — so neither
//! side can be changed on its own without the pair being read again.
//!
//! The setup here uses the game's own switches rather than posing the state.
//! `_COMPMODE` is what `START_MACRO` reads at module 316 `0x00540` to choose
//! which of its three openings runs — 0 is the login the player types through,
//! 2 is the terminal already logged in — and `SETLOCTASK` is the word the
//! script itself uses to move between the terminal's screens. Everything after
//! that is the shipped bytecode reacting to keys.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_engine::Game;
use motionvm_forth::m32::Vm;
use motionvm_testutil::gamedata_ds2;

/// The codes module 216 dispatches on.
const UP: i32 = 328;
const DOWN: i32 = 336;
const LEFT: i32 = 331;
const RIGHT: i32 = 333;
const RETURN: i32 = 13;

/// How long a key is allowed to take effect.
///
/// A key is not acted on by the frame after it arrives. `HOTMEN` slides the
/// cursor animation to the new entry and the dispatch is gated on that
/// animation standing still (`0x0c1fc`: `_ANCURSOR @ SADESC _SMEN @ ?ANIMOVE
/// NOT AND`), so a press is followed by frames until the terminal is ready for
/// the next one.
const SETTLE: usize = 60;

/// One press, then long enough for the terminal to have acted on it.
fn press(game: &mut Game<Vm>, key: i32) {
    game.set_input(0, 0, false, false, key).expect("input");
    game.step().expect("the frame the key arrives on");
    idle(game, SETTLE);
}

fn idle(game: &mut Game<Vm>, frames: usize) {
    for _ in 0..frames {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame");
    }
}

/// Runs until the location's task machine holds still.
///
/// The terminal's screens are task phases, and a phase that is mid-fade or
/// waiting on `->LTWAIT` is not a place to press a key. Nothing announces
/// "done", so what is waited for is the task and phase not moving.
fn settled(game: &mut Game<Vm>, what: &str) {
    let mut held = 0;
    let mut where_ = game.task_phase();
    for _ in 0..3000 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame");
        let now = game.task_phase();
        held = if now == where_ { held + 1 } else { 0 };
        where_ = now;
        if held >= SETTLE {
            return;
        }
    }
    panic!("{what} never came to rest");
}

/// Runs until the terminal is where the test needs it, or says what it was
/// waiting for.
fn until(game: &mut Game<Vm>, want: &str, mut ready: impl FnMut(&Game<Vm>) -> bool) {
    for _ in 0..1500 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame");
        if ready(game) {
            return;
        }
    }
    panic!("the terminal never reached {want}");
}

/// The mailbox on its main menu: six entries, the bar on screen, the first
/// entry highlighted.
fn main_menu(dir: &std::path::Path) -> Game<Vm> {
    let mut game = Game::open(dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(11, "_COMPMODE", 2).expect("already logged in");
    game.set_var(2, "_NEXTLOC", 16)
        .expect("ask for the mailbox");
    until(&mut game, "the terminal", |g| {
        g.get_var(2, "_ACTLOC") == Some(16) && g.get_var(216, "_EDVMODE") == Some(3)
    });
    // The opening sequence has to be over before a task is put in its place,
    // or it puts its own back.
    settled(&mut game, "the terminal's opening");
    // Location task 5 is the main menu — `0x07c9c` builds it with `NEWMEN`,
    // six `ADDMEN` and `0 HOTMEN`, and `SHWMEN` puts the bar and the cursor on
    // screen.
    game.call(5, "SETLOCTASK", &[5]).expect("the main menu");
    settled(&mut game, "the main menu");
    assert_eq!(
        game.get_var(216, "_AMEN"),
        Some(6),
        "six entries on the bar"
    );
    assert_eq!(game.get_var(216, "_SMEN"), Some(1), "the bar is on screen");
    assert_eq!(
        game.get_var(216, "_ASEL"),
        Some(0),
        "the bar opens on entry 0"
    );
    game
}

#[test]
fn left_and_right_walk_the_menu_bar() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = main_menu(&dir);

    press(&mut game, RIGHT);
    assert_eq!(game.get_var(216, "_ASEL"), Some(1), "333 moves one right");
    press(&mut game, RIGHT);
    assert_eq!(game.get_var(216, "_ASEL"), Some(2));
    press(&mut game, LEFT);
    assert_eq!(game.get_var(216, "_ASEL"), Some(1), "331 moves one left");

    // `0x0c2dc`: left off the first entry answers `_AMEN @ 1 -`, the last one.
    press(&mut game, LEFT);
    press(&mut game, LEFT);
    assert_eq!(
        game.get_var(216, "_ASEL"),
        Some(5),
        "and wraps round the end"
    );
}

#[test]
fn up_and_down_walk_the_selection_list() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = main_menu(&dir);

    // Down to the file list and open it: Files, then Jump — `0x0ccbc` sets
    // `_SELFLAG` and shows the bar, which is what `0x0c45c` and `0x0c4bc` are
    // gated on.
    press(&mut game, RETURN);
    until(&mut game, "the file areas", |g| {
        g.get_var(216, "_AMEN") == Some(4)
    });
    settled(&mut game, "the file areas");
    press(&mut game, RETURN);
    until(&mut game, "a file list", |g| {
        g.get_var(216, "_SELMAX") == Some(2)
    });
    settled(&mut game, "the file list");
    press(&mut game, RETURN);
    assert_eq!(
        game.get_var(216, "_SELFLAG"),
        Some(1),
        "the list takes the cursor keys now"
    );
    assert_eq!(game.get_var(216, "_SELACT"), Some(0));

    press(&mut game, DOWN);
    assert_eq!(game.get_var(216, "_SELACT"), Some(1), "336 is SELDOWN");
    press(&mut game, DOWN);
    assert_eq!(game.get_var(216, "_SELACT"), Some(2));
    // `SELDOWN` at `0x05180` compares against `_SELMAX` and starts over.
    press(&mut game, DOWN);
    assert_eq!(
        game.get_var(216, "_SELACT"),
        Some(0),
        "and wraps at the end"
    );
    // `SELUP` at `0x05200` answers `_SELMAX` for a zero.
    press(&mut game, UP);
    assert_eq!(game.get_var(216, "_SELACT"), Some(2), "328 is SELUP");
}

/// The list keys are not the bar keys: with no list open they do nothing, which
/// is the `_SELFLAG @` in front of both `SELUP` and `SELDOWN` (`0x0c45c`).
#[test]
fn the_list_keys_are_ignored_while_no_list_is_open() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = main_menu(&dir);
    assert_eq!(game.get_var(216, "_SELFLAG"), Some(0));

    press(&mut game, DOWN);
    press(&mut game, UP);
    assert_eq!(game.get_var(216, "_SELACT"), Some(0));
    assert_eq!(game.get_var(216, "_ASEL"), Some(0), "and the bar stays put");
}
