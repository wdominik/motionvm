//! An ordinary scene draws the same picture either way.
//!
//! The drawer is incremental: the surface persists from frame to frame and
//! only the descriptors the damage map names are repainted, each screen
//! composited in level order onto the display. That is a machine with state,
//! and a machine with
//! state can drift — a change that fails to mark, a rectangle measured a
//! little short, an erase that puts back a copy older than what is under it.
//! Drift shows as smearing: a caption that will not go away, a figure that
//! leaves a trail, a background that eats its neighbours.
//!
//! There is one check that catches all of it at once. Mark every descriptor and
//! draw again — which is what `FADEIN` does through `0x6b0fe` → `0x6a8f9`
//! (`0x74af1`) — and the picture must not move. Whatever the incremental pass
//! left behind, the full one leaves out.
//!
//! It is deliberately run on a *scene* and not on the mailbox: the mailbox
//! keeps pixels no descriptor owns any more, on purpose, and that is what
//! `bbs_wipe.rs` is about.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_motion_engine::Game;
use motionvm_motion_forth::m32::Vm;
use motionvm_motion_testutil::gamedata_ds2;

/// Plays into the park and lets it settle.
fn park(dir: &std::path::Path) -> Game<Vm> {
    let mut game = Game::open(dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(2, "_NEXTLOC", 2).expect("ask for the park");
    for _ in 0..1200 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame");
    }
    assert_eq!(
        game.get_var(2, "_ACTLOC"),
        Some(2),
        "the park is location 2"
    );
    game
}

/// How the two pictures differ, and where the first difference is.
fn compare(game: &mut Game<Vm>) -> Option<(usize, i32, i32)> {
    let before = game.render();
    let screens: Vec<u32> = game.engine.screens().iter().map(|s| s.handle).collect();
    for handle in screens {
        game.engine.repaint_screen(handle);
    }
    game.engine.draw();
    game.engine.present();
    let after = game.render();

    let mut first = None;
    let mut n = 0;
    for y in 0..i32::from(before.height) {
        for x in 0..i32::from(before.width) {
            if before.get(x, y) != after.get(x, y) {
                n += 1;
                first.get_or_insert((x, y));
            }
        }
    }
    first.map(|(x, y)| (n, x, y))
}

#[test]
fn a_scene_is_the_same_picture_drawn_in_full() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = park(&dir);

    // Several times over, with frames in between, because what drifts does so
    // as things move: the figure walks, a caption comes and goes, the pointer
    // crosses the picture.
    for round in 0..6 {
        for _ in 0..40 {
            game.set_input(200 + round * 40, 340, false, false, 0)
                .expect("input");
            game.step().expect("a frame");
        }
        if let Some((n, x, y)) = compare(&mut game) {
            panic!(
                "round {round}: {n} pixels differ between the incremental picture \
                 and the same scene drawn in full, the first at {x},{y}"
            );
        }
    }
}
