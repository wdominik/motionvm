//! Jeff Jet - Abenteuer InfoHighway saves through the same three files.
//!
//! The save scheme is the 16-bit engine's and not the game's: `PUT` writes the
//! location into `.blk`, `PUTANIM` the display into `.anm`, `=>PUTAS` the
//! resident modules into `.FRZ`, and `RUN` probes slots 701 to 705 at start-up
//! to decide whether to put its start-up page up. `enviro_savegames.rs` drives
//! that scheme end to end through the save page; what is asked here is that
//! this game reaches those words and that its slots are found — with a
//! container on two volumes behind them, and every module image packed.
//!
//! These need the game's files (`MOTIONVM_GAMEDATA_JEFFJET`) and skip without
//! them.
//!
//! The game this file drives is Jeff Jet (MOTION 16-bit).

use motionvm_engine::{Game, Playable, titles};
use motionvm_forth::m16::Vm;
use motionvm_testutil::gamedata_jeffjet;
use std::path::{Path, PathBuf};

/// Where `RUN` stands after the intro: in the game loop, or — with a save in a
/// slot — on its start-up page, waiting for a click.
#[derive(Debug, PartialEq, Eq)]
enum After {
    InTheGame,
    OnThePage,
}

/// `RUN` through the intro, with a save directory.
///
/// `RUN` enters its own first location, and then — only when that location is
/// 13, the one it starts in — probes slots 701 to 705 with
/// `706 701 DO I =>EXIST … LOOP`. A slot that answers puts `_?STARTUP` up,
/// opens the start-up page with `3 _INVMODE !` and `SHOW_FILES`, and only then
/// does `ANIMPLAY` take the frames. So the page is up before the game loop
/// runs, and what says which of the two happened is `_INVMODE`.
fn into_the_game(dir: &Path, saves: &Path) -> (Game<Vm>, After) {
    let mut game = titles::jeffjet::open(dir).expect("opens");
    game.set_saves(saves).expect("a save directory");
    game.start().expect("RUN starts");
    while game.pump().expect("RUN runs to the intro's ANIMPLAY") {}
    let mut left_intro = false;
    for frame in 1..=8000 {
        // One key, at frame 300, and no more: the intro takes either, and a
        // second one would click the start-up page away.
        let key = if frame == 300 { 27 } else { 0 };
        game.set_input(160, 100, false, false, key).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
        if !game.engine.main_loop() {
            left_intro = true;
        } else if left_intro {
            let after = match game.get_var(601, "_INVMODE") {
                Some(3) => After::OnThePage,
                _ => After::InTheGame,
            };
            return (game, after);
        }
    }
    panic!("8000 frames and the intro never gave way to the game loop");
}

/// Runs one kernel word with its arguments, as a script would, and answers
/// what it left on the stack.
fn word(game: &mut Game<Vm>, name: &str, args: &[i32]) -> Vec<i32> {
    let mut stack = args.to_vec();
    let Game { engine, vm, .. } = game;
    let done = engine
        .plain_word16(name, &mut stack, &mut vm.mem)
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(done, "{name} is a kernel word of the 16-bit engine");
    stack
}

fn frames(game: &mut Game<Vm>, n: usize) {
    for frame in 1..=n {
        game.set_input(160, 100, false, false, 0).expect("input");
        game.step().unwrap_or_else(|e| panic!("frame {frame}: {e}"));
    }
}

fn temp_saves(tag: &str) -> PathBuf {
    motionvm_testutil::saves_dir(&format!("jeffjet-{tag}"))
}

#[test]
fn a_slot_is_written_by_the_three_words_and_found_by_the_next_start() {
    let Some(dir) = gamedata_jeffjet() else {
        eprintln!("skipping: no Jeff Jet gamedata directory");
        return;
    };
    let saves = temp_saves("saves");

    let (mut game, after) = into_the_game(&dir, &saves);
    assert_eq!(after, After::InTheGame, "no save yet, so no page");
    game.request_location(5).expect("NEXTLOC");
    frames(&mut game, 200);
    assert_eq!(game.get_var(601, "ACTLOC"), Some(5));
    assert_eq!(
        word(&mut game, "=>EXIST", &[701]),
        [0],
        "slot 701 is empty before the save"
    );

    // The save page's words, in its order — the same three the other 16-bit
    // game's page runs, on the same slot numbers.
    game.set_var(601, "_LOADTABLE", 5).expect("_LOADTABLE");
    let table = game.address(601, "_LOADTABLE").expect("_LOADTABLE");
    let table = game.vm.mem.flat(table).expect("loaded") as i32 + 2;
    word(&mut game, "PUT", &[2, table, 701]);
    word(&mut game, "PUTANIM", &[701]);
    word(&mut game, "=>PUTAS", &[701]);
    for suffix in ["blk", "anm", "FRZ"] {
        assert!(
            saves.join(format!("701.{suffix}")).exists(),
            "701.{suffix} was written"
        );
    }
    assert_eq!(
        word(&mut game, "=>EXIST", &[701]),
        [-1],
        "=>EXIST finds the slot"
    );
    drop(game);

    // `RUN`'s probe — `706 701 DO I =>EXIST … LOOP` — is what puts the
    // start-up page up instead of walking straight into the first location.
    let (mut fresh, after) = into_the_game(&dir, &saves);
    assert_eq!(
        after,
        After::OnThePage,
        "a save in a slot puts the start-up page up"
    );
    assert_eq!(fresh.get_var(607, "_?STARTUP"), Some(1));
    assert!(
        fresh.render().pixels.iter().any(|&p| p != 0),
        "the page draws"
    );
    drop(fresh);
    let _ = std::fs::remove_dir_all(&saves);
}
