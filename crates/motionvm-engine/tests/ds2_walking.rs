//! The figure walks.
//!
//! Everything the walk needs was already in place — the person record, the
//! command queue, the walk-cycle lists, the `SDWORD` callback that pumps
//! `DOWALK` every second frame — except the two pieces that do the work:
//! `CROUTE`, which lays out the steps, and the stepper behind the gate at
//! `person[0x1c8]`, which spends them. Without them the queue drained in four
//! calls and every figure reported itself arrived without having moved. In the
//! park that leaves Karsten on his spawn corner at x = −80, off the left edge,
//! which is why the protagonist appeared to be missing altogether.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_engine::Game;
use motionvm_testutil::gamedata_ds2;

/// Karsten walks into the park and stops exactly where he was sent.
///
/// The park's own data says where that is: `_ROUTE` opens with a count of 9
/// and a first record `(−80, 311)-(198, 323)`, which is the diagonal he spawns
/// on, and his queue reads `1002 150 −1 1`. The −1 is not a coordinate but an
/// instruction: take the y off the route's line (0x76448), which at x = 150 is
/// 320. `_XROUTE` gives that route a step of 12 and a scale of 900.
///
/// So the whole chain is pinned by the assertions below — the planner's step
/// size, the line the steps follow, the walk-cycle list in `person[0x18]`
/// (mode 1, because `person[0x1d0]` bit 0 is set), and the arrival.
#[test]
fn the_protagonist_walks_into_the_park() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(2, "_NEXTLOC", 2).expect("the park");

    let mut seen: Vec<(i32, i32, u32)> = Vec::new();
    for _ in 0..300 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame in the park");
        // The figure is whichever descriptor the park hung on `_WALKKARSTE`.
        let Some(handle) = game.get_var(202, "_WALKKARSTE").filter(|&h| h != 0) else {
            continue;
        };
        let Some(d) = game
            .engine
            .descriptors()
            .iter()
            .find(|d| d.handle == handle as u32)
        else {
            continue;
        };
        if let Some(sprite) = d.shows.graphic() {
            let now = (d.x, d.y, sprite);
            if seen.last() != Some(&now) {
                seen.push(now);
            }
        }
    }

    assert!(!seen.is_empty(), "the figure never appeared at all");
    let start = seen.first().copied().expect("a first sighting");
    assert_eq!(
        (start.0, start.1),
        (-80, 311),
        "he starts on the route's corner"
    );

    // He moves, and only ever forwards along the route.
    let xs: Vec<i32> = seen.iter().map(|(x, _, _)| *x).collect();
    assert!(
        xs.windows(2).all(|w| w[1] >= w[0]),
        "the walk should not go backwards: {xs:?}"
    );
    assert!(
        xs.iter().any(|&x| x > -80),
        "the figure never moved: {xs:?}"
    );

    // The steps are the planner's: twelve across, and the y that the route's
    // own line gives for that x. Both are read out of the location's blocks.
    let moving: Vec<(i32, i32, u32)> = seen
        .iter()
        .copied()
        .filter(|&(x, _, _)| x > -80 && x < 150)
        .collect();
    assert!(moving.len() >= 4, "too few steps to judge: {moving:?}");
    // The across step is the route's own, and the down step is whatever it
    // takes to sit back on the line — measured from the point *before* this
    // one, because that is the position the planner still has in hand when it
    // works the step out (0x769f9 reads `curX`, which is the last point).
    for w in moving.windows(2) {
        assert_eq!(w[1].0 - w[0].0, 12, "a step is twelve across: {moving:?}");
        let on_line = 311 + (w[0].0 + 80) * 12 / 278;
        assert_eq!(
            w[1].1, on_line,
            "the step should sit on the route's line: {moving:?}"
        );
    }

    // While he moves he is drawn out of the walk-cycle list for heading 1 —
    // `_KRECHTS`, sprites 102 to 111 — and never the standing frame.
    let cycle: Vec<u32> = moving.iter().map(|(_, _, s)| *s).collect();
    assert!(
        cycle.iter().all(|s| (102..=111).contains(s)),
        "the walk should animate through 102..111: {cycle:?}"
    );
    assert!(
        cycle.windows(2).any(|w| w[0] != w[1]),
        "the cycle never advanced: {cycle:?}"
    );

    // And he arrives where the queue sent him, on the line, not past it.
    let end = seen
        .iter()
        .rev()
        .find(|(x, _, _)| *x == 150)
        .expect("he never got there");
    assert_eq!((end.0, end.1), (150, 320), "the walk ends on the target");
}
