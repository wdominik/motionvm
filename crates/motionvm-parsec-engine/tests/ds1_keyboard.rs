//! The keys `XP01.EXE`'s pointer library takes for itself, played through
//! the public interface.
//!
//! The library's keyboard hook keeps twelve keys from the program: the
//! eight direction keys move the pointer, Space and Enter are its two
//! buttons, keypad minus and plus its slowest and fastest speed
//! (`titles::parsec1994::keyboard`). These tests play the options `XP01`
//! opens with, as a player would.
//!
//! A held Down in those options, against `XP01.EXE` under DOSBox-X: Down
//! made at BIOS tick 45 and broken at 53 — eight ticks, 439 ms — moves the
//! pointer off the options' last row and into the strip, and between the
//! picture before and the settled one after, the changed pixels lie in
//! `(132, 99)`–`(189, 173)`: the row's highlight going out and the
//! pointer's two places. Held for 53 steps of 8.33 ms, 442 ms, the rebuild
//! comes to rest on the same picture as the original's, pixel for pixel;
//! held for 51 or for 54 steps it does not.
//!
//! Needs `MOTIONVM_GAMEDATA_DS1`; skips without it.
//!
//! The game this file drives is Dunkle Schatten (PARSEC).

use motionvm_parsec_engine::{Game, Runtime, Start};
use motionvm_playable::{Button, Key, KeyPress};

fn press(key: Key) -> KeyPress {
    KeyPress {
        key,
        text: None,
        shift: false,
        ctrl: false,
        alt: false,
    }
}

/// The game at its start options, settled: 300 steps.
fn at_the_options() -> Option<Game> {
    let dir = motionvm_parsec_testutil::gamedata_ds1()?;
    let mut g = Game::open(&dir).expect("the install opens");
    g.skip_title(Start::New);
    g.start().expect("the game starts");
    for _ in 0..300 {
        g.step().expect("a step");
    }
    Some(g)
}

/// The frame as the DAC shows it: each pixel's color, eight bits a
/// channel.
fn picture(g: &mut Game) -> Vec<[u8; 3]> {
    let f = g.frame();
    f.pixels.pixels.iter().map(|&i| f.palette.rgb8(i)).collect()
}

/// The box around the pixels two pictures differ in, `(x0, y0, x1, y1)`
/// with the right and bottom edges one past, the way the measurement of
/// the original states it.
fn changed(a: &[[u8; 3]], b: &[[u8; 3]]) -> Option<(usize, usize, usize, usize)> {
    let mut bbox: Option<(usize, usize, usize, usize)> = None;
    for (i, _) in a.iter().zip(b).enumerate().filter(|(_, (x, y))| x != y) {
        let (x, y) = (i % 320, i / 320);
        bbox = Some(match bbox {
            None => (x, y, x + 1, y + 1),
            Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1)),
        });
    }
    bbox
}

#[test]
fn a_held_arrow_moves_the_pointer_as_far_as_the_original() {
    let Some(mut g) = at_the_options() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let before = picture(&mut g);
    g.key_down(&press(Key::Down));
    for _ in 0..53 {
        g.step().expect("a step");
    }
    g.key_up(&press(Key::Down));
    for _ in 0..347 {
        g.step().expect("a step");
    }
    let after = picture(&mut g);
    assert_eq!(changed(&before, &after), Some((132, 99, 189, 173)));
}

/// Space is the first button and Enter the second: either, on the options'
/// last row, does what a click there does, and neither is a keystroke the
/// options read. The options take both buttons alike (`cs:0C86h`).
#[test]
fn space_and_enter_are_the_pointers_buttons() {
    let play = |input: &dyn Fn(&mut Game, bool)| -> Option<Vec<[u8; 3]>> {
        let mut g = at_the_options()?;
        // The pointer on *Zum Spiel*, the last row.
        g.pointer(160, 120);
        g.step().expect("a step");
        input(&mut g, true);
        g.step().expect("a step");
        input(&mut g, false);
        for _ in 0..200 {
            g.step().expect("a step");
        }
        Some(picture(&mut g))
    };
    let Some(clicked) = play(&|g, down| g.button(Button::Left, down)) else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let spaced = play(&|g, down| {
        if down {
            g.key_down(&press(Key::Space));
        } else {
            g.key_up(&press(Key::Space));
        }
    })
    .expect("the same install");
    let entered = play(&|g, down| {
        if down {
            g.key_down(&press(Key::Enter));
        } else {
            g.key_up(&press(Key::Enter));
        }
    })
    .expect("the same install");
    let idle = play(&|_, _| {}).expect("the same install");
    assert!(clicked != idle, "the click leaves the options");
    assert!(spaced == clicked, "Space is the first button");
    assert!(
        entered == clicked,
        "Enter is the second, which the options take alike"
    );
}

/// The title program, `XP00.EXE`, carries the same hook on the same
/// timer: Down held in its menu for eight BIOS ticks under DOSBox-X takes
/// the arrow from `(160, 100)` down onto *Neues Spiel beginnen*, and the
/// changed RGB pixels between the menu before and the settled picture
/// after lie in `(160, 100)` up to `(170, 165)`, 158 of them. Held for 30
/// retraces, 428 ms, the rebuild comes to rest on the original's picture
/// pixel for pixel; held for 29 or 32 it does not.
#[test]
fn a_held_arrow_moves_the_title_menus_pointer_as_far_as_the_original() {
    let Some(dir) = motionvm_parsec_testutil::gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let mut g = Game::open(&dir).expect("the install opens");
    g.set_saves(&motionvm_parsec_testutil::saves_dir("ds1-keyboard-title"))
        .expect("a save directory");
    g.start().expect("the game starts");
    // The intro over and the menu up and still.
    for _ in 0..700 {
        g.step().expect("a step");
    }
    let before = picture(&mut g);
    g.key_down(&press(Key::Down));
    for _ in 0..30 {
        g.step().expect("a step");
    }
    g.key_up(&press(Key::Down));
    for _ in 0..200 {
        g.step().expect("a step");
    }
    let after = picture(&mut g);
    assert_eq!(changed(&before, &after), Some((160, 100, 170, 165)));
    assert_eq!(
        before.iter().zip(&after).filter(|(a, b)| a != b).count(),
        158
    );
}
