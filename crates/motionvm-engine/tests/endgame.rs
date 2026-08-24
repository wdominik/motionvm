//! Quitting reaches `ENDGAME`.
//!
//! `4:START` does not return while the game runs: it enters `ANIMPLAY`, the
//! game's own frame loop, and the cells after that call are `3 =>GET ENDGAME
//! 3 =>ERASE` (module 4). So `ENDGAME` runs exactly once, when the loop is
//! over, and the only thing that ends the loop is `QUITANIM` clearing the flag
//! at `0xdb4a4`.
//!
//! Here that call is parked rather than kept on a C stack, which makes
//! `ANIMPLAY` a door that has to be walked back through: park the position,
//! run the loop, resume the cells after the call. A parked position that is
//! written and never read turns the door one-way and `ENDGAME` never happens
//! at all — which is exactly what this test would catch.
//!
//! The game data this file drives is Dunkle Schatten 2's (MOTION 32-bit).

use motionvm_engine::Game;
use motionvm_forth::Host;
use motionvm_testutil::gamedata_ds2;

#[test]
fn quitanim_lets_start_run_on_into_endgame() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}

    // Into a location, so the machine is where it is during play rather than
    // part-way through startup.
    game.set_var(2, "_NEXTLOC", 1).expect("the classroom");
    for _ in 0..600 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame");
        if game.get_var(2, "_ACTLOC") == Some(1) && !game.engine.in_transition() {
            break;
        }
    }

    // `ENDGAME` unloads module 3 on its way out (`3 =>ERASE`), so the module
    // list is what says whether it ran. It is not resident during play.
    assert!(
        !game.engine.resident().contains(&3),
        "module 3 should not be loaded while the game is running"
    );

    // The game's own quit word, reached from the shell's exit button. A kernel
    // word, so it goes through the engine rather than a module lookup.
    game.engine
        .word("QUITANIM", &mut game.vm)
        .expect("QUITANIM");
    assert!(
        !game.engine.main_loop(),
        "QUITANIM clears the main-loop flag"
    );
    // Not the end by itself: the flag says the loop should stop, and `START`
    // has still to be put back and run. Asserting this is what stops the loop
    // below from passing without doing anything — `is_running` is already false
    // here, so a test that waited for *that* would be over before it began.
    assert!(
        !game.finished(),
        "clearing the flag is not by itself the end"
    );

    // Frames from here on must first put `START` back and let it finish.
    let mut frames = 0;
    while !game.finished() {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame after the quit");
        frames += 1;
        assert!(frames < 2000, "the parked START never finished");
    }
    assert!(
        !game.engine.main_loop(),
        "and nothing restarted the loop behind it"
    );
    eprintln!("ENDGAME finished {frames} frame(s) after the quit");
}
