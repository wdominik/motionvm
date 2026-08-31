//! Escape reaches the game, not the window.
//!
//! `ICTRL` opens each frame with `?KEY DUP _AKTKEY !` (module 4, `0x022a0`)
//! and later tests that variable against 27 (`0x02c40`):
//!
//! ```text
//! _AKTKEY @ 27 =  …  _CheckIf 4  2 CALLMENU
//! ```
//!
//! so Escape presses menu button 2 — the quit page — which is how the original
//! is left: through its own confirmation, `QUITANIM` and `ENDGAME`.
//!
//! Two things had to be wrong at once for this never to happen. The frontend
//! exited the event loop on Escape instead of passing it on, and `?KEY` pushed
//! a *flag* rather than the key code, so `_AKTKEY` only ever held 0 or 1 and
//! every comparison against a code was false. The second is why this test looks
//! at the engine rather than at the window.
//!
//! The press itself, all the way to the quit page appearing, is in
//! `free_play.rs` — it needs a location played through its opening scene first,
//! which is that file\'s business. This one holds down the narrower fact that
//! everything else rests on: the code arrives.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_motion_engine::{Driven, Game};
use motionvm_motion_forth::m32::Vm;
use motionvm_motion_testutil::gamedata_ds2;
use motionvm_playable::{Button, Key, KeyPress};

/// The key code `ICTRL` compares against, read out of the game's own bytecode.
const ESCAPE: i32 = 27;

/// The title, settled, with the shell idle and the player in control.
///
/// The classroom will not do, and why is worth writing down. Escape is gated
/// twice: the branch tests `_ORDER+0x0c` for 0, 9 or 8, and it sits inside
/// `ICTRL`'s free-play region, which runs only while `_SYS_LEVEL` is 0. In the
/// classroom the second is satisfied after the location's task machine works
/// through its thirteen phases — but the first never is, because that machine
/// ends by putting a conversation on screen and waiting for an answer. Order
/// mode 14, eight answers, no player to pick one.
///
/// The title has no such conversation. `_IMX` answering is the same signal
/// `fades.rs` waits on, and it is only written while `ICTRL` reaches its bar
/// handling, so it says the free-play region is running.
fn settled_in_the_title(dir: &std::path::Path) -> Game<Vm> {
    let mut game = Game::open(dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    for _ in 0..3000 {
        game.set_input(300, 440, false, false, 0).expect("input");
        game.step().expect("a frame");
        if game.get_var(2, "_IMX") == Some(300) && !game.engine.in_transition() {
            return game;
        }
    }
    panic!("the game never settled anywhere the bar could be reached");
}

#[test]
fn the_key_code_reaches_aktkey_rather_than_a_flag() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = settled_in_the_title(&dir);

    // The whole of the bug, in one assertion: whatever `?KEY` answers is what
    // `_AKTKEY` holds, and it has to be the code.
    game.set_input(0, 0, false, false, ESCAPE).expect("escape");
    game.step().expect("the frame with the key");
    assert_eq!(
        game.get_var(2, "_AKTKEY"),
        Some(ESCAPE),
        "_AKTKEY should hold the key code; a flag makes every comparison false"
    );
}

/// The same fact through the contract: a press goes into the game's buffer as
/// a [`KeyPress`], and the step that follows takes it out, translates it and
/// delivers it — the family's whole key path, up to the front door. The second
/// step shows the keystroke was spent: one press is one frame's key.
#[test]
fn a_press_through_the_contract_reaches_aktkey_translated() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = settled_in_the_title(&dir);

    let escape = KeyPress {
        key: Key::Escape,
        text: None,
        shift: false,
        ctrl: false,
        alt: false,
    };
    game.key(&escape, true);
    Driven::step(&mut game).expect("the frame with the key");
    assert_eq!(
        game.get_var(2, "_AKTKEY"),
        Some(ESCAPE),
        "the buffered press should arrive as the translated code"
    );
    Driven::step(&mut game).expect("a frame with no key");
    assert_eq!(
        game.get_var(2, "_AKTKEY"),
        Some(0),
        "one press is one frame's key; the next frame reads none"
    );
    game.key(&escape, false);
    Driven::step(&mut game).expect("a frame after a release");
    assert_eq!(
        game.get_var(2, "_AKTKEY"),
        Some(0),
        "a release crosses the seam and never reaches the buffer"
    );
}

/// The button path through the contract: `_MLK` carries the live level the
/// original's `MOUSELK` read — held is held — with a press too short to
/// span a frame stretched to the one frame the original's poll would have
/// given it. The family's whole mouse path, up to the front door.
#[test]
fn the_button_level_reaches_mlk_the_way_mouselk_read_it() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = settled_in_the_title(&dir);

    // Held: the level stays up across frames, exactly as the hardware did.
    game.button(Button::Left, true);
    Driven::step(&mut game).expect("the frame with the press");
    assert_eq!(game.get_var(2, "_MLK"), Some(1), "the press arrives");
    Driven::step(&mut game).expect("a frame while held");
    assert_eq!(
        game.get_var(2, "_MLK"),
        Some(1),
        "held is held: the level does not decay into an edge"
    );
    // And the script's own debounce runs on it: module 4 sets `_MPRESSED`
    // while a button is down, which is what keeps a held button from
    // clicking twice — the game's logic, fed the raw level it was written
    // for.
    assert_eq!(
        game.get_var(2, "_MPRESSED"),
        Some(1),
        "the shell's own debounce flag rises while the button is held"
    );
    game.button(Button::Left, false);
    Driven::step(&mut game).expect("the frame after the release");
    assert_eq!(game.get_var(2, "_MLK"), Some(0), "released is released");

    // A press and release both between two frames: stretched to one frame.
    game.button(Button::Left, true);
    game.button(Button::Left, false);
    Driven::step(&mut game).expect("the frame with the short click");
    assert_eq!(
        game.get_var(2, "_MLK"),
        Some(1),
        "a sub-frame click is stretched to the frame the original's poll caught"
    );
    Driven::step(&mut game).expect("the frame after it");
    assert_eq!(game.get_var(2, "_MLK"), Some(0));
}
