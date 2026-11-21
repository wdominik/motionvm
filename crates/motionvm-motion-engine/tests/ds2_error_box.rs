//! A run-time error the original reports in its box: the box, and what the
//! run does around it.
//!
//! The 32-bit engine reports a division by zero in its error box, waits for
//! a key or the left button, and abandons the word that was running; the
//! caller goes on. Here the division is real — `7 0 /` assembled into the
//! free tail of `_GAMEINV` and called while the game is in its frame loop —
//! and everything after it is the engine's.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_motion_engine::Game;
use motionvm_motion_formats::m32::le::TAG_KERNEL;
use motionvm_motion_forth::Address;
use motionvm_motion_forth::m32::Vm;
use motionvm_motion_testutil::gamedata_ds2;

/// The game in its frame loop, a couple of hundred frames in.
fn in_the_frame_loop(dir: &std::path::Path) -> Game<Vm> {
    let mut game = Game::open(dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    for _ in 0..200 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame");
    }
    game
}

/// Assembles `7 0 /` and the end of the word where nothing lives, and
/// answers where.
fn division_by_zero(game: &mut Game<Vm>) -> Address {
    let ordinal = |game: &Game<Vm>, name: &str| {
        (0..0x1000)
            .find(|&o| game.vm.ordinal_name(o) == Some(name))
            .unwrap_or_else(|| panic!("no ordinal for {name}"))
    };
    let inv = game.address(2, "_GAMEINV").expect("_GAMEINV");
    let at = Address::new(inv.module(), inv.offset() + 4 + 300);
    let lit = TAG_KERNEL | ordinal(game, "_PutLit");
    let div = TAG_KERNEL | ordinal(game, "/");
    for (i, cell) in [lit, 7, lit, 0, div, 0].into_iter().enumerate() {
        let n = u32::try_from(i).expect("six cells");
        game.vm
            .store(Address::new(at.module(), at.offset() + 4 * n), cell)
            .expect("a scratch cell");
    }
    at
}

/// The box goes up, the word is abandoned with the 0 the handler leaves on
/// the stack, and the left button closes the box once it is up again.
#[test]
fn a_division_by_zero_puts_up_the_box_and_play_goes_on() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = in_the_frame_loop(&dir);
    let at = division_by_zero(&mut game);
    let depth = game.vm.data.len();
    game.call_at(at)
        .expect("the division is reported, not raised");
    assert!(game.engine.error_box_open(), "no box");
    assert_eq!(game.vm.data.len(), depth + 1, "the 0 `/` leaves");
    assert_eq!(game.vm.data.last(), Some(&0));

    // The box is drawn over the frame: its gray fills the corner at 40,40.
    let frame = game.render();
    let width = usize::from(frame.width);
    assert_eq!(
        frame.pixels[41 * width + 41],
        frame.pixels[40 * width + 40],
        "the panel's gray"
    );

    // Held, the frame stands; the box goes when the button is up.
    for _ in 0..3 {
        game.set_input(0, 0, true, false, 0).expect("input");
        game.step().expect("pressed");
        assert!(game.engine.error_box_open(), "closed before the release");
    }
    game.set_input(0, 0, false, false, 0).expect("input");
    game.step().expect("released");
    assert!(!game.engine.error_box_open(), "still open");
    assert!(!game.finished(), "an error in a called word ended the run");
    for _ in 0..50 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("play goes on");
    }
}

/// A key closes it as well, and the right button does not: the wait reads
/// the left button's field and the keyboard, nothing else.
#[test]
fn a_key_closes_the_box_and_the_right_button_does_not() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = in_the_frame_loop(&dir);
    let at = division_by_zero(&mut game);
    game.call_at(at).expect("reported");
    game.set_input(0, 0, false, true, 0).expect("input");
    game.step().expect("right button");
    game.set_input(0, 0, false, false, 0).expect("input");
    game.step().expect("released");
    assert!(game.engine.error_box_open(), "the right button closed it");
    game.set_input(0, 0, false, false, 13).expect("input");
    game.step().expect("a key");
    assert!(!game.engine.error_box_open(), "a key did not close it");
}
