//! Dunkle Schatten 2's classroom played through its opening scene, for the
//! suites that need the game to be the player's: the gates and the way
//! through them are in `ds2_free_play.rs`, whose tests hold this.
//!
//! The game this module drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_motion_engine::Game;
use motionvm_motion_forth::Address;
use motionvm_motion_forth::cell;
use motionvm_motion_forth::m32::Vm;

/// The mode cell of `_ORDER`, at +0x0c.
fn order_mode(game: &Game<Vm>) -> i32 {
    let base = motionvm_motion_forth::m32::word_address(&game.vm, 2, "_ORDER")
        .expect("_ORDER")
        .next();
    game.vm
        .fetch(Address::new(base.module(), base.offset() + 0x0c))
        .map_or(-1, cell::signed)
}

/// The answer boxes currently on screen: text descriptors at level 99.
fn answers(game: &Game<Vm>) -> Vec<(i32, i32)> {
    game.engine
        .descriptors()
        .iter()
        .filter(|d| d.active && d.is_text() && d.level == 99)
        .map(|d| (d.x, d.y))
        .collect()
}

/// Whether the game is the player's: both gates open.
fn free(game: &Game<Vm>) -> bool {
    game.get_var(2, "_SYS_LEVEL") == Some(0)
        && !game.engine.in_transition()
        && matches!(order_mode(game), 0 | 8 | 9)
}

/// The classroom, played through its opening scene until the game is idle.
///
/// The only input is answering: whenever a menu is up, the topmost line is
/// clicked — the same one `picking_an_answer_moves_the_conversation_on` uses,
/// chosen by the descriptor's own reported position rather than by coordinates
/// written down here. Everything else is empty frames.
///
/// Returns the number of frames it took, so a change in the scene's length
/// shows up as a number rather than as a timeout.
pub(crate) fn play_into_free_play(dir: &std::path::Path) -> (Game<Vm>, usize) {
    let mut game = Game::open(dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(2, "_NEXTLOC", 1)
        .expect("new game: the classroom");

    for frame in 0..12_000 {
        // Not "still in the classroom": phase 13 of its task machine can set
        // `_NEXTLOC` to 6 and move the story on (module 201, `0x0706c`), so
        // where the player ends up standing is the game's business, not this
        // test's. What matters is that somebody is standing somewhere and the
        // controls answer.
        if free(&game) && game.get_var(2, "_ACTLOC").is_some_and(|l| l > 0) {
            return (game, frame);
        }
        let menu = answers(&game);
        // An answer is waiting: click the topmost, which is the last placed.
        // One box is enough — a conversation narrows down to a single line
        // before it ends, and waiting for three would sit there for ever.
        let click_at = if let Some(&(x, y)) = menu.iter().min_by_key(|(_, y)| *y) {
            Some((x, y))
        } else if (12..=18).contains(&order_mode(&game)) {
            // A line is being spoken with nothing to choose. `ICTRL`'s
            // click-to-advance resets the caption's wait on a fresh press, so a
            // click anywhere moves it on.
            Some((0, 0))
        } else {
            None
        };
        match click_at {
            Some((x, y)) => {
                game.set_input(x, y, true, false, 0).expect("the click");
                game.step().expect("the frame with the click");
                game.set_input(x, y, false, false, 0).expect("input");
                game.step().expect("the frame after it");
            }
            None => {
                game.set_input(0, 0, false, false, 0).expect("input");
                game.step().expect("a frame");
            }
        }
    }
    panic!(
        "the classroom never handed over: _SYS_LEVEL {:?}, order mode {}, answers {:?}",
        game.get_var(2, "_SYS_LEVEL"),
        order_mode(&game),
        answers(&game)
    );
}
