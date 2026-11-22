//! *Dunkle Schatten* opens, and what it read out of its own program.
//!
//! Every figure below comes from the game's `XP01.EXE` or its `RESOURCE/`.
//!
//! They need the game's files — `MOTIONVM_GAMEDATA_DS1` — and skip without
//! them.
//!
//! The game this file drives is Dunkle Schatten (PARSEC).

use motionvm_parsec_engine::{Game, Runtime, Title};
use motionvm_parsec_testutil::gamedata_ds1;
use motionvm_playable::{Button, Key, KeyPress, LocationRequest};

/// Escape, as a platform reports the key.
const ESCAPE: KeyPress = KeyPress {
    key: Key::Escape,
    text: None,
    shift: false,
    ctrl: false,
    alt: false,
};

/// One step, and whether the game goes on after it.
fn tick(game: &mut Game) -> bool {
    game.step().expect("a step");
    !game.finished()
}

/// The left button pressed and let go where the pointer is; the press is
/// latched and lands in the next step.
fn click(game: &mut Game) {
    game.button(Button::Left, true);
    game.button(Button::Left, false);
}

/// Location `n`, entered however the game enters it.
fn go(game: &mut Game, n: i32) {
    game.request_location(LocationRequest::at(n))
        .expect("a location request");
}

/// The install, or a skip on a machine that has not got it.
macro_rules! ds1_or_skip {
    () => {
        match gamedata_ds1() {
            Some(dir) => match Game::open(&dir) {
                Ok(g) => g,
                Err(e) => panic!("Dunkle Schatten is installed but will not open: {e}"),
            },
            None => {
                eprintln!("skipping: no Dunkle Schatten gamedata directory");
                return;
            }
        }
    };
}

/// The install opens as the right title, and play begins where the batch
/// file begins it: in the title program `XP00`, then — on a new game —
/// in the location `XP01` names, once the options it opens first are
/// closed.
///
/// A click while the first logo fades in skips the intro (`0x323C`, the
/// skip latched from any event); the menu fades in over 66 retraces and
/// takes a button on its first line, "Neues Spiel beginnen" (`0x3A6D`).
/// The backstory's six pages each wait for an event (`0x34FD`), and the
/// picture fades out over 66 more. Then `XP01`: `0x4F2A` asks whether
/// what the loader read left `[1F4Ah]` at zero and, when it did, sets
/// location **2** and entry `7Fh` at `0x4F37` — not location 0, which is
/// the row whose handler is the same routine the main loop calls when
/// there is no location at all. Before the main loop calls it, `0x4F87`
/// opens the game options, always; Escape closes them once they have
/// dissolved in, which takes a hundred hundredths (`0x120CE`), and keys
/// typed before that are flushed (`0x12106`).
#[test]
fn a_fresh_game_opens_in_the_location_the_program_names() {
    let mut g = ds1_or_skip!();
    assert_eq!(g.title(), Title::DunkleSchatten);
    g.start().expect("start");
    click(&mut g);
    for _ in 0..100 {
        tick(&mut g);
    }
    assert_eq!(
        g.start_location(),
        Some(0),
        "the title program runs, no location yet"
    );
    g.pointer(160, 164);
    click(&mut g);
    for _ in 0..150 {
        tick(&mut g);
    }
    for _ in 0..6 {
        click(&mut g);
        tick(&mut g);
    }
    for _ in 0..70 {
        tick(&mut g);
    }
    g.key_down(&ESCAPE);
    for _ in 0..130 {
        tick(&mut g);
    }
    assert_eq!(
        g.start_location(),
        Some(0),
        "the options are up, no location yet"
    );
    g.key_down(&ESCAPE);
    tick(&mut g);
    assert_eq!(g.start_location(), Some(2));
}

/// Escape in the title program's menu is its third line, "Spiel sofort
/// beenden" (`0x3A80`): the picture fades out, the program leaves −1
/// (`0x64E`), and the batch file does not run `XP01` — the game is over.
#[test]
fn escape_in_the_title_menu_ends_the_game() {
    let mut g = ds1_or_skip!();
    g.start().expect("start");
    click(&mut g);
    for _ in 0..100 {
        assert!(tick(&mut g), "the menu is up and waits");
    }
    g.key_down(&ESCAPE);
    let mut over = None;
    for t in 0..100 {
        if !tick(&mut g) {
            over = Some(t);
            break;
        }
    }
    let t = over.expect("the game ends");
    assert!(t >= 65, "after the picture's fade, {t}");
}

/// Every location that has a playfield composes a full 320×200 frame.
///
/// The playfield is the top 128 rows and the verb strip the remaining 72.
/// Rows 0 and 20 are no places: a jump to either walks on to the next row
/// that is one, as the program's main loop does, so they are not counted.
#[test]
fn every_location_with_a_playfield_composes_a_frame() {
    let mut g = ds1_or_skip!();
    let mut drawn = 0;
    for n in (0..60).filter(|&n| n != 0 && n != 20) {
        go(&mut g, n);
        if g.start_location() != Some(n) {
            continue;
        }
        let fb = g.frame().pixels;
        assert_eq!(fb.width, 320, "location {n}");
        assert_eq!(fb.height, 200, "location {n}");
        // The strip is opaque, so the bottom row is never all background.
        let bottom = &fb.pixels[199 * 320..];
        if bottom.iter().any(|&p| p != 0) {
            drawn += 1;
        }
    }
    assert_eq!(drawn, 58, "locations that drew a strip");
}

/// A location out of range is ignored rather than fatal, and leaves the
/// player where they were.
#[test]
fn a_location_that_does_not_exist_is_ignored() {
    let mut g = ds1_or_skip!();
    let before = g.start_location();
    go(&mut g, 9_999);
    assert_eq!(g.start_location(), before);
}

/// The game keeps its files in a directory of its own under the one it is
/// given, named for its slug, so two games pointed at one `saves/` keep
/// their slots apart.
#[test]
fn the_saves_go_into_the_games_own_directory() {
    let mut g = ds1_or_skip!();
    let dir = motionvm_parsec_testutil::saves_dir("ds1-slug");
    g.set_saves(&dir)
        .expect("a save directory outside the install");
    let own = std::fs::canonicalize(&dir).unwrap().join("ds1");
    assert_eq!(g.saves(), Some(own.as_path()));
    assert!(own.is_dir(), "the game's own directory is made");
}

/// An arrival point is a 16-bit word in the game; one that does not fit is
/// refused by name rather than cut.
#[test]
fn an_arrival_point_past_the_entry_word_is_refused() {
    let mut g = ds1_or_skip!();
    let e = g
        .request_location(LocationRequest::via(2, 0x1_0000))
        .expect_err("refused");
    assert_eq!(
        e.to_string(),
        "there is no arrival point 65536: a PARSEC game numbers them from \
         -32768 to 32767"
    );
}
