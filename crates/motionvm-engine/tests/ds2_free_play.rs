//! Playing a location's opening scene through, and what becomes reachable then.
//!
//! Two branches of `ICTRL` are gated on the game being the player's rather than
//! the script's, and neither had ever been reached by a test: Escape opening the
//! quit page, and the inventory bar's scroll arrows. Both live in the free-play
//! region that begins at module 4 `0x02A40` and runs 323 cells, and reaching it
//! needs **two** conditions at once:
//!
//! - `_SYS_LEVEL` at 0, which `SETNOBUSY` (module 5, `0x004cc`) sets once the
//!   `_BUSY` counter it shares with `SETBUSY` comes back down. A location's task
//!   machine holds that up for as long as it is working — the classroom's task 2
//!   runs thirteen phases (module 201, `0x06b6c`…`0x0704c`).
//! - the order machine idle, i.e. `_ORDER+0x0c` at 0, 8 or 9.
//!
//! Waiting alone will not do it. The classroom's task machine ends by putting a
//! conversation on screen and waiting for an answer — mode 14, eight of them —
//! so the second condition stays shut until somebody picks one. That is the game
//! working as written, and it is why this file plays rather than waits.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_engine::Game;
use motionvm_forth::Address;
use motionvm_forth::m32::Vm;
use motionvm_testutil::gamedata_ds2;

/// The key code `ICTRL` compares against, from the game's own bytecode.
const ESCAPE: i32 = 27;

/// The mode cell of `_ORDER`, at +0x0c.
fn order_mode(game: &Game<Vm>) -> i32 {
    let base = motionvm_forth::m32::word_address(&game.vm, 2, "_ORDER")
        .expect("_ORDER")
        .next();
    game.vm
        .fetch(Address::new(base.module(), base.offset() + 0x0c))
        .map_or(-1, |v| v as i32)
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
fn play_into_free_play(dir: &std::path::Path) -> (Game<Vm>, usize) {
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

/// The scene ends and the game becomes the player's.
///
/// The baseline for everything else here: if this fails, the two tests below
/// are testing nothing.
#[test]
fn the_opening_scene_hands_over_to_the_player() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let (game, frames) = play_into_free_play(&dir);
    eprintln!(
        "handed over after {frames} frames, in location {:?}",
        game.get_var(2, "_ACTLOC")
    );
    assert_eq!(game.get_var(2, "_SYS_LEVEL"), Some(0));
    assert_eq!(
        game.get_var(2, "_BUSY"),
        Some(0),
        "and nothing is still busy"
    );
}

/// Escape opens the quit page.
///
/// `_AKTKEY @ 27 =` at module 4 `0x02c40`, gated on the order machine being
/// idle; the branch freezes the picture, fades the bar, swaps in the
/// confirmation sprites and ends with `6 _INVMODE !` at `0x02d80`.
#[test]
fn escape_opens_the_quit_page() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let (mut game, _) = play_into_free_play(&dir);
    assert_ne!(
        game.get_var(2, "_INVMODE"),
        Some(6),
        "the quit page should not already be open"
    );

    game.set_input(0, 0, false, false, ESCAPE).expect("escape");
    game.step().expect("the frame with the key");

    // The branch does not finish inside that frame, and this is why: it opens
    // with `FREEZESCR` and a `FADEOUT` (`0x02c98`…`0x02cc8`), and a fade
    // suspends the word until its bands have run. `6 _INVMODE !` is at
    // `0x02d80`, on the far side of that. So the press is one frame and the
    // page arriving is several more — checking straight after the key reads the
    // state before the branch has done anything visible.
    let mut frames = 0;
    while game.engine.in_transition() || game.is_running() {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame of the fade");
        frames += 1;
        assert!(frames < 500, "the quit page never finished opening");
    }
    eprintln!("the quit page opened {frames} frames after the key");

    assert_eq!(
        game.get_var(2, "_INVMODE"),
        Some(6),
        "Escape should open the quit page"
    );
}

/// The inventory bar's scroll arrows move the window.
///
/// The arrows are the other branch that needed free play: `ICTRL` at `0x02b00`
/// tests a fresh button press with `_IMX` in `0…63`, and moves the list's head
/// cell by eight — back below x 32 (`0x02b20`), forward above it (`0x02b58`) —
/// then repaints with `CALCINV`.
///
/// `inventory.rs` has a test for this that is ignored, with this test named
/// as the one that covers the behaviour: the branch does run, and this is
/// where what happens is settled.
#[test]
fn the_inventory_arrows_scroll_the_window() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let (mut game, _) = play_into_free_play(&dir);

    // `_ACTINV` is the list: a scroll offset in its head cell, then the item
    // numbers. More than eight items, so there is somewhere to scroll to.
    let list = game.get_var(2, "_ACTINV").expect("_ACTINV") as u32;
    let head = Address::new(list >> 16, list & 0xffff);
    let offset = |g: &Game<Vm>| g.vm.fetch(head).unwrap_or(0) as i32;

    let mut item = 1;
    while count_items(&game) < 12 && item < 200 {
        game.call(5, "ADDITEM", &[item]).expect("ADDITEM");
        item += 1;
    }
    assert!(
        count_items(&game) >= 10,
        "not enough items to scroll: {}",
        count_items(&game)
    );

    // The **back** arrow, at x below 32. Forward is the wrong way to test:
    // twelve items in eight slots means `CALCINV` already has the window at
    // the end of the list, so adding eight more would only be clamped back and
    // a test that saw no movement would have proved nothing.
    //
    // The click is a press for one frame and a release after, which is the
    // shape `_MPRESSED` debounces against.
    let before = offset(&game);
    assert!(before > 0, "the window is already at the start: {before}");
    for step in 0..8 {
        game.set_input(16, 440, step == 1, false, 0).expect("input");
        game.step().expect("a frame on the bar");
    }
    let after = offset(&game);
    assert!(
        after < before,
        "the back arrow did not move the window (offset stayed {before})"
    );
    eprintln!("the back arrow moved the window {before} -> {after}");
}

/// How many items the bar's list holds, up to its terminating zero.
fn count_items(game: &Game<Vm>) -> usize {
    let list = game.get_var(2, "_ACTINV").unwrap_or(0) as u32;
    let cell = |off: u32| {
        game.vm
            .fetch(Address::new(list >> 16, (list & 0xffff).wrapping_add(off)))
            .unwrap_or(0)
    };
    (0..99).take_while(|i| cell(4 + i * 4) != 0).count()
}
