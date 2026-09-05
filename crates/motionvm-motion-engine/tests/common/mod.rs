//! The drivers more than one suite needs to get a game somewhere and read it.
//!
//! Every file in `tests/` is a crate of its own, so anything two of them want
//! has to be written twice or put here. Written twice is how these started:
//! five copies of [`settled_in`], three of [`word16`], nine of [`digests`] —
//! and copies drift. One that grew a frame where its siblings had none would
//! have made the two suites disagree about what "settled" is, and the suite
//! that was right about the engine would have looked like the broken one.
//!
//! What belongs here is a *driver*: the plumbing of getting to a state. What
//! does not is an assertion, a digest name or an address — those are a suite's
//! own evidence, and moving them out of the file that argues with them is how
//! a test stops being readable. So there is no `boot_the_16_bit_game` here:
//! each of the four does something the others do not, and the file that drives
//! one says which.
//!
//! It cannot live in `motionvm-motion-testutil`, which is where the gamedata
//! lookup and the digest tables are: that crate may depend on the neutral
//! layer and on nothing of the family's, because `motionvm-motion-formats`
//! dev-depends on it and a dependency the other way would be a cycle. These
//! drivers take a `Game`, which is the engine's.
//!
//! Not every suite uses every driver — each takes the two or three it needs,
//! and the rest are dead code *in that crate*, which is what the attribute
//! below is for. An `expect` rather than an `allow`: there is no compilation
//! in which they are all live, and the day one suite takes every driver the
//! attribute says so instead of sitting there for nothing.
#![expect(
    dead_code,
    reason = "every suite takes the drivers it needs and leaves the rest"
)]

use motionvm_motion_engine::{Engine, Game};
use motionvm_motion_forth::{Host, Machine, m16, m32};
use motionvm_motion_testutil::Digests;
use std::path::Path;

/// A game's table of reference digests, by its slug.
///
/// `CARGO_MANIFEST_DIR` expands here to the engine crate's own directory,
/// which is the one holding `tests/digests/`, so a suite naming its slug is
/// all this needs.
pub(crate) fn digests(slug: &str) -> Digests {
    Digests::of(env!("CARGO_MANIFEST_DIR"), slug)
}

/// Runs frames until nothing is fading, so a still picture can be measured.
///
/// A state predicate rather than a frame count on purpose: a step is one band
/// while a curtain runs, so a fixed number of them would spend most of itself
/// inside the fade.
///
/// This stops the moment the curtain does. A suite that wants the picture the
/// fade left has to take one more frame itself — the drawer runs once a frame
/// out of the game loop and nowhere else — and the ones that measure pixels
/// say so where they do it.
pub(crate) fn settle<M>(game: &mut Game<M>)
where
    M: Machine,
    Engine: Host<M>,
{
    let mut guard = 0;
    while game.engine.in_transition() {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a curtain frame");
        guard += 1;
        assert!(guard < 500, "a transition never ended");
    }
}

/// `n` frames with the pointer parked in the middle of the view and no click.
///
/// The middle rather than a corner because the 16-bit games put their menu
/// strip along the bottom edge and their inventory along the top: a pointer
/// resting on either would be hovering something, and the frames would not be
/// idle ones.
pub(crate) fn frames<M>(game: &mut Game<M>, n: i32, what: &str)
where
    M: Machine,
    Engine: Host<M>,
{
    for frame in 1..=n {
        game.set_input(160, 100, false, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("{what}, frame {frame}: {e}"));
    }
}

/// How many pixels of the current frame are not the background.
pub(crate) fn lit<M>(game: &mut Game<M>) -> usize
where
    M: Machine,
    Engine: Host<M>,
{
    game.render().pixels.iter().filter(|&&p| p != 0).count()
}

/// Runs one kernel word of the 16-bit engine with its arguments, as a script
/// would, and answers what it left on the stack.
pub(crate) fn word16(game: &mut Game<m16::Vm>, name: &str, args: &[i32]) -> Vec<i32> {
    let mut stack = args.to_vec();
    let Game { engine, vm, .. } = game;
    let done = engine
        .plain_word16(name, &mut stack, &mut vm.mem)
        .unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(done, "{name} is a kernel word of the 16-bit engine");
    stack
}

/// Frames at a point, with the click on exactly one of them.
///
/// One frame, because that is what a player's click is worth: the window
/// clears the flag as soon as it has handed it over, so a click the game does
/// not act on in that step is gone. A test that held the button down would
/// pass while the game was unplayable.
pub(crate) fn hold<M>(game: &mut Game<M>, x: i32, y: i32, frames: i32, click_at: i32)
where
    M: Machine,
    Engine: Host<M>,
{
    for f in 0..frames {
        game.set_input(x, y, f == click_at, false, 0)
            .expect("input");
        game.pump().expect("pump");
        game.step()
            .unwrap_or_else(|e| panic!("at {x},{y} frame {f}: {e}"));
    }
}

/// The 32-bit game standing in a location, reached the way the game reaches
/// it.
///
/// `4:START` loads what it needs, runs `STARTUP`, initializes through
/// `DS_INIT` and hands over to `ICTRL` with `0x42150 CTRL`; `ICTRL` then
/// enters `_STARTLOC` on a frame of its own. So the title is where the game
/// puts itself and nobody has to place it there, and any other location is
/// asked for through `_NEXTLOC` — the same cell the scripts write.
///
/// This replaced a pair of entry points that ran `STARTUP` alone and then
/// `INCLLOC` to completion. That reached a location without the shell `START`
/// builds around it: three descriptors fewer than the game ever has, on a
/// frame the original never ran.
pub(crate) fn settled_in(dir: &Path, location: i32) -> Game<m32::Vm> {
    let mut game = Game::open(dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    if location != game.start_location().unwrap_or(0) {
        game.request_location(location).expect("_NEXTLOC");
    }
    for frame in 1..=3000 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} on the way to {location}: {e}"));
        if game.get_var(2, "_ACTLOC") == Some(location)
            && !game.engine.in_transition()
            && !game.is_running()
        {
            return game;
        }
    }
    panic!("the game never settled in location {location}");
}
