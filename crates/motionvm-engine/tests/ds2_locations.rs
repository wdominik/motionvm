//! Every location the game can enter, entered.
//!
//! Five of the sixteen read through stray pointers, and the engine has to
//! tolerate it. The original's `@` (0x626fc) checks nothing: it works the
//! address out and reads, so a script that dereferences a stray value gets a
//! harmless answer and carries on. Refusing instead is stricter than the
//! engine, and ends the game where the original plays on.
//!
//! What the five scripts actually do:
//!
//! - **5 and 10** test `KRSCHRZIEHE @ 0 =`, but `KRSCHRZIEHER` is the *item
//!   number* 24 from module 11, not the flag `_?KRSCHRZIEHER` the author meant
//!   — every neighboring test in the same chain uses the underscore form. So
//!   the read lands on address 24, which is module 0: the kernel's own module,
//!   built at run time and not something we have.
//! - **6** calls `PSETWALK` on Gaby before her shadow record exists, so it
//!   reads `0 + 44`.
//! - **7 and 8** call `?INVINCL`.
//!
//! Location 12 is left out on purpose: its entry in the location table is
//! uninitialized in the shipped data, so entering it is a jump into nowhere in
//! the original too, and nothing in the game ever asks for it.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_engine::Game;
use motionvm_testutil::gamedata_ds2;

/// All five load and keep running, stray reads and all.
#[test]
fn the_locations_that_read_through_stray_pointers_still_load() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    for want in [5, 6, 7, 8, 10] {
        let mut game = Game::open(&dir).expect("game opens");
        game.start().expect("4:START");
        while game.pump().expect("startup runs") {}
        game.set_var(2, "_NEXTLOC", want)
            .expect("ask for the location");

        // Reached, not still standing there: several of these are mid-game
        // scenes whose task manager hands straight on when it is entered out
        // of context. What is being pinned is that entering them runs.
        let mut reached = false;
        for frame in 0..600 {
            game.set_input(0, 0, false, false, 0).expect("input");
            if let Err(e) = game.step() {
                panic!("location {want} stopped on frame {frame}: {e}");
            }
            reached |= game.get_var(2, "_ACTLOC") == Some(want);
        }
        assert!(reached, "location {want} was never entered");
    }
}
