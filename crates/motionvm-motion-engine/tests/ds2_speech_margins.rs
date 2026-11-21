//! Every line a conversation puts up stands inside the dialogue machine's
//! margins.
//!
//! The handler sets a line through the words themselves — `SDTB`, `SDCOL`,
//! `SDTDT`, `SDTXT` (`0x7b81e`–`0x7b868`), which measure the new text on
//! the spot — and only then clamps it, twenty pixels in from each side of
//! the view (`0x7b8d0` on). Clamped with the last line's size instead, a
//! long line overhangs the left edge; the classroom's second line did.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_motion_engine::Game;
use motionvm_motion_forth::m32::Vm;
use motionvm_motion_testutil::gamedata_ds2;

/// What a kernel getter answers for descriptor `handle`.
fn get(game: &mut Game<Vm>, handle: u32, word: &str) -> i32 {
    game.vm.data.push(i32::try_from(handle).expect("a handle"));
    assert!(
        game.kernel_word("ACTDESC").expect("ACTDESC"),
        "the kernel has ACTDESC"
    );
    assert!(
        game.kernel_word(word).expect("the getter"),
        "the kernel has {word}"
    );
    game.vm.data.pop().expect("an answer")
}

#[test]
fn every_classroom_line_stands_inside_the_margins() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(2, "_NEXTLOC", 1).expect("the classroom");

    let mut lines = 0;
    let mut last = None;
    for _ in 0..2500 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame in the classroom");
        let speech: Vec<(u32, Option<i32>)> = game
            .engine
            .descriptors()
            .iter()
            .filter(|d| d.active && d.is_text() && d.level == 99)
            .map(|d| (d.handle, d.text))
            .collect();
        for (handle, text) in speech {
            if last == Some((handle, text)) {
                continue;
            }
            last = Some((handle, text));
            let (left, right) = (
                get(&mut game, handle, "GDX"),
                get(&mut game, handle, "GDOX"),
            );
            assert!(
                left >= 20 && right <= 620,
                "line {text:?} on descriptor {handle} stands at {left}..{right}"
            );
            lines += 1;
        }
    }
    assert!(lines >= 3, "only {lines} lines went up");
}
