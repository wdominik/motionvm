//! The menu's fades, through the game's own branches.
//!
//! The unit tests in `motionvm-engine`'s own `mod tests` pin what a curtain does to
//! the picture. This file pins the case the effect was reported from: the help
//! pages, where the original turns a page with **two `FADEIN`s and no
//! `FADEOUT` at all** (module 4, 0x03bc0/0x03be8 for the top row of tabs,
//! 0x03cb4/0x03cdc for the bottom one). Nothing there ever blanks the screen,
//! so a page has to appear over the one before it.

use motionvm_engine::Game;
use motionvm_testutil::gamedata;

/// A game standing in the title, settled, with the pointer able to reach the
/// status bar.
///
/// `ICTRL` only fills `_IMX`/`_IMY` while the pointer is in the bar, and only
/// when it runs at all — while a word is part-way through, the frame goes to
/// that word instead. Waiting for `_IMX` to answer is waiting for both.
fn settled_in_the_title(dir: &std::path::Path) -> Game {
    let mut game = Game::open(dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    for _ in 0..2000 {
        game.set_input(300, 440, false, false, 0).expect("input");
        game.step().expect("a frame");
        if game.get_var(2, "_IMX") == Some(300) && !game.engine.in_transition() {
            return game;
        }
    }
    panic!("the game never settled anywhere the bar could be clicked");
}

/// One row of the frame, as palette indices.
fn row(frame: &motionvm_render::Framebuffer, y: i32) -> Vec<u8> {
    (0..frame.width as i32)
        .map(|x| frame.get(x, y).unwrap_or(0))
        .collect()
}

/// Turning a help page shows the new page over the old one.
///
/// The branch under test is `ICTRL`'s `_INVMODE 5` at 0x03ac0: a click in the
/// top row of tabs sets `_DOC`, runs `SHOW_DOC` — which only switches
/// descriptors, it fades nothing itself — and then fades **in** twice, once on
/// the bar and once on the picture. There is no `FADEOUT` in that branch, and
/// none in `SHOW_DOC` either, so at no point is anything blanked.
///
/// The viewer is opened through `DO_INVSEL`'s case 1005 rather than by setting
/// `_INVMODE` by hand, because that case is also what activates `_ANL1` and
/// `_ANL2` — the two halves of image 60 that *are* the page. Set the mode
/// directly and the pages have no paper: the text stands on the cleared
/// buffer, and the only reason the picture would change at all is that the
/// text is drawn in something other than the background color. The assertion
/// below would then be resting on a text-color fault rather than on the
/// reveal it names.
#[test]
fn turning_a_help_page_reveals_it_over_the_page_before() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = settled_in_the_title(&dir);
    game.set_var(2, "_MENCZW", 1005)
        .expect("the documents button");
    game.call(4, "DO_INVSEL", &[]).expect("4:DO_INVSEL");
    assert_eq!(game.get_var(2, "_INVMODE"), Some(5), "the viewer is open");

    let before = game.render();
    // The tabs run from x 195 in steps of 40, at bar rows 7..40 — display rows
    // 407..440. This is the third tab.
    let (x, y) = (195 + 80 + 20, 420);
    game.set_input(x, y, true, false, 0).expect("press");
    game.step().expect("the frame the click lands on");

    assert!(game.engine.in_transition(), "the click starts a fade");
    // One at a time, not two at once: `ICTRL` runs at the top of the
    // interpreter, where a curtain does pause it, so the second `FADEIN` is
    // only reached once the first has run. (`DO_INVSEL` is the one that queues
    // several, because a descriptor callback goes through `call_nested`.)
    let started = game.engine.fades().len() - 1;

    // The bar's fade first, then the picture's. While the picture's band is
    // part-way open the middle of the screen has to show the new page and the
    // top of it the one before; a curtain that blanked what it had not yet
    // reached could not do that.
    let (top_before, middle_before) = (row(&before, 4), row(&before, 200));
    let mut seen = false;
    let mut guard = 0;
    while game.engine.in_transition() || game.engine.fades().len() - started < 2 {
        game.set_input(x, y, false, false, 0).expect("release");
        game.step().expect("a curtain frame");
        guard += 1;
        assert!(guard < 500, "the page turn never finished");
        let frame = game.render();
        assert!(
            frame.pixels[..640 * 400].iter().any(|&p| p != 0),
            "the picture went black while turning a page"
        );
        if row(&frame, 200) != middle_before && row(&frame, 4) == top_before {
            seen = true;
        }
    }
    let turn = &game.engine.fades()[started..];
    assert_eq!(turn.len(), 2, "a page turn is two fades");
    assert!(
        turn.iter().all(|f| f.name == "FADEIN"),
        "and both of them open: {turn:?}"
    );
    assert!(seen, "the new page never opened over the old one");
}

/// The bar is what the menu fades; the picture above it is left alone.
///
/// A fade works on the screen `ACTSCR` selected, not on the display — the
/// menu's own fades take `_IS`, which is the 640×80 status bar at display row
/// 400. The options page is the cleanest of them to drive: moving a speed
/// slider is `_IS @ ACTSCR  1 50 8 FADEIN` and nothing else (`ICTRL` 0x03ef8),
/// with no file to write and no picture to rebuild. Whatever it does, nothing
/// above row 400 may move.
#[test]
fn the_menu_fades_the_bar_and_not_the_picture() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = settled_in_the_title(&dir);
    game.set_var(2, "_INVMODE", 7).expect("the options page");

    let before = game.render();
    // The text-speed row: x 517 in steps of 40, bar rows 7..40.
    let (x, y) = (517 + 40 + 10, 420);
    game.set_input(x, y, true, false, 0).expect("press");
    game.step().expect("the frame the click lands on");
    assert!(
        game.engine.in_transition(),
        "moving the slider fades the bar"
    );
    assert_eq!(
        game.get_var(2, "_TSMODE"),
        Some(2),
        "and it is the second setting now"
    );

    let mut guard = 0;
    while game.engine.in_transition() {
        game.set_input(x, y, false, false, 0).expect("release");
        game.step().expect("a curtain frame");
        guard += 1;
        assert!(guard < 500, "the fade never finished");
        let frame = game.render();
        assert_eq!(
            &frame.pixels[..640 * 400],
            &before.pixels[..640 * 400],
            "the picture moved while the bar was fading"
        );
    }
}

/// Every band of a fade reaches the screen.
///
/// The original waits `delay` ticks and then calls the presenter, once per
/// pass (0x74db6–0x74dcf) — so a 480-row curtain is 31 pictures, not one
/// picture per frame. Driving the curtain off the frame clock instead showed
/// only four of them, each edge jumping 64 rows: the wall clock was right and
/// the movement was gone. That is what made the intro look too fast while the
/// status bar, whose ten-ticks-a-band happens to match a frame, looked right.
#[test]
fn a_fade_puts_every_band_on_the_screen() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.startup_only().expect("startup");
    game.enter_location(23).expect("title macro");
    while game.engine.in_transition() {
        game.step().expect("a curtain step");
    }
    // Phase 1 of the title: FADEOUT, swap, FADEIN, all on the 640x480 screen.
    game.set_input(0, 0, true, false, 0).expect("click");
    game.step().expect("the step with the click");
    game.set_input(0, 0, false, false, 0).expect("input");
    game.step().expect("the step that starts the fade");
    assert!(game.engine.in_transition(), "phase 1 fades out first");

    let mut seen: Vec<Vec<u8>> = Vec::new();
    let mut steps = 0;
    while game.engine.in_transition() {
        let frame = game.render().pixels;
        if seen.last() != Some(&frame) {
            seen.push(frame);
        }
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a curtain step");
        steps += 1;
        assert!(steps < 500, "the fade never finished");
    }
    assert_eq!(
        steps, 62,
        "two curtains of 31 bands on the 640x480 title screen"
    );
    // Not all 62 are distinct, and the repeats are accounted for: once the
    // closing band has met in the middle the rest of `FADEOUT` is the same
    // black, and `FADEIN`'s first pass marks a band of height zero that
    // 0x18436 rejects outright (`jle` at 0x18471). Measured: 50.
    //
    // The number that matters is the order of magnitude. A 40 ms frame clock
    // can deliver at most eight pictures over these 62 ticks, whatever else is
    // true — so anything above thirty says the bands are reaching the screen
    // one at a time.
    assert!(
        seen.len() >= 40,
        "only {} of {steps} bands reached the screen",
        seen.len()
    );
}
