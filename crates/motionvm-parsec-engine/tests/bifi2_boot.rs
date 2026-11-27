//! *Bi-Fi 2 - Action in Hollywood* opens as a title of its own and plays,
//! through the public interface, from its title program into the Kaffee.
//!
//! It needs the game's files — `MOTIONVM_GAMEDATA_BIFI2` — and skips
//! without them.
//!
//! The game this file drives is Bi-Fi 2 (PARSEC).

use motionvm_parsec_engine::{Game, Runtime, Title};
use motionvm_parsec_testutil::gamedata_bifi2;
use motionvm_playable::{Button, Key, KeyPress};

/// Escape, as a platform reports the key.
const ESCAPE: KeyPress = KeyPress {
    key: Key::Escape,
    text: None,
    shift: false,
    ctrl: false,
    alt: false,
};

fn tick(game: &mut Game) {
    game.step().expect("a step");
}

fn click(game: &mut Game) {
    game.button(Button::Left, true);
    game.button(Button::Left, false);
}

/// The install opens as Bi-Fi 2, and play begins in the title program
/// `XP00`: its animation, which nothing skips, two stills, the title
/// picture and the menu, whose first line, "Neues Spiel beginnen", starts
/// at row 9Ah (`0x3472`). Chosen, `XP01` opens its options, and Escape
/// closes them once they have dissolved in; then the main loop walks on
/// from row 0 to the Kaffee, location 1, which `XP01` names as no
/// location of its own (`0x004BF`).
#[test]
fn a_fresh_game_opens_in_the_kaffee() {
    let Some(dir) = gamedata_bifi2() else {
        eprintln!("skipping: no Bi-Fi 2 gamedata directory");
        return;
    };
    let mut g = Game::open(&dir).expect("Bi-Fi 2 opens");
    assert_eq!(g.title(), Title::BiFi2);
    g.set_saves(&motionvm_parsec_testutil::saves_dir("bifi2-boot"))
        .expect("a save directory");
    g.start().expect("start");
    // The animation, the stills, the title and its fade: a click while
    // the stills hold skips the rest of them.
    for n in 0..3000 {
        if n == 600 {
            click(&mut g);
        }
        tick(&mut g);
    }
    assert_eq!(g.start_location(), Some(0), "the title program runs");
    g.pointer(160, 0x9A + 4);
    click(&mut g);
    for _ in 0..400 {
        tick(&mut g);
    }
    g.key_down(&ESCAPE);
    for _ in 0..50 {
        tick(&mut g);
    }
    assert_eq!(g.start_location(), Some(1), "the Kaffee");
}
