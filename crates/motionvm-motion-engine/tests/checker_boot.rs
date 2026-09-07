//! Checker 2000 boots on the 32-bit machine, into its registration board.
//!
//! The bootstrap is the authoring template's — `4 =>GET`, `START`, `STARTUP`
//! in module 3 — but what `START` runs is this game's own shell: `TASK_START`
//! puts the task machine at task 0, which is location 18, the *info board*
//! (`_IBON`), and its board 5 is where a new player types a name and a
//! postcode. The board's texts are the first thing in the corpus to use the
//! text record's insert slots: `#s<` shows the name buffer with a cursor
//! after it, and every keystroke writes a byte into that buffer and runs
//! `SDINSERT` again. Enter on a postcode of three digits closes the entry —
//! the first three digits pick one of eight regions — and fades the board
//! over to board 2.
//!
//! These tests drive that far without a window and ask for the picture. They
//! need the game's files — `MOTIONVM_GAMEDATA_CHECKER` — and skip without
//! them. That this directory is told apart and opens as this game is asked in
//! `titles_detected.rs`, one row per game; here the game is already open.
//!
//! The game this file drives is Checker 2000 (MOTION 32-bit).

mod common;

use common::settle;
use motionvm_motion_engine::{Game, PointerShape, titles};
use motionvm_motion_forth::m32::Vm;
use motionvm_motion_testutil::{Digests, digest, gamedata_checker};
use std::path::Path;

/// Words the shell's frames walk past without effect, none of them this
/// game's: the status tables the loader keeps for itself, the sound-card
/// `SPEEDMODE` and `ERRORLEVEL` of the DOS engine, the two resets and the
/// crunch hint. And `GET`'s one miss: block 98 is the registration `SAVESAVE`
/// writes with `PUT`, which a fresh copy has not got yet — the original reads
/// its interrupt table there and finds no name either.
const INERT: &[&str] = &[
    "ERRORLEVEL",
    "GET (block 98 missing)",
    "RESETANIM",
    "RESETFONT",
    "SPEEDMODE",
    "XBLKSTAT-",
    "XGFXCRUNCH",
    "XGFXSTAT-",
    "XPALSTAT-",
    "XSCRSTAT-",
    "XTXTSTAT-",
];

/// The words a run walked past that are not inert.
fn walked_past(game: &Game<Vm>) -> Vec<String> {
    game.engine
        .stubbed()
        .keys()
        .filter(|w| !INERT.contains(&w.as_str()))
        .cloned()
        .collect()
}

/// This game's table of reference digests.
fn digests() -> Digests {
    common::digests("checker")
}

/// The game open, `START` run into `ANIMPLAY`, and the registration board
/// settled: sixty idle frames, which is what a player who has not touched a
/// key sees.
fn at_the_registration(dir: &Path) -> Game<Vm> {
    let mut game = titles::checker::open(dir).expect("opens");
    let saves = std::env::temp_dir().join(format!("motionvm-checker-boot-{}", std::process::id()));
    std::fs::create_dir_all(&saves).expect("a save directory");
    game.set_saves(&saves).expect("saves");
    game.start().expect("START starts");
    let mut frames = 0;
    while game.pump().expect("START runs to ANIMPLAY") {
        frames += 1;
        assert!(frames < 10_000, "START never reached ANIMPLAY");
    }
    common::frames(&mut game, 60, "the registration board");
    game
}

/// Presses `key` on one frame and lets the frame after it go by, as a typist
/// would: the shell reads `?KEY` once a frame.
fn press(game: &mut Game<Vm>, key: i32) {
    game.set_input(320, 240, false, false, key).expect("input");
    game.step().expect("the frame with the key");
    common::frames(game, 3, "after the key");
}

#[test]
fn start_reaches_the_registration_board_and_draws_it() {
    let Some(dir) = gamedata_checker() else {
        eprintln!("skipping: no Checker 2000 gamedata directory");
        return;
    };
    let mut game = at_the_registration(&dir);
    assert!(game.engine.main_loop(), "ANIMPLAY was entered");
    assert!(
        game.engine.controller().is_some(),
        "START handed ICTRL to CTRL"
    );
    assert_eq!(
        game.engine.display_size().width,
        640,
        "TOGFX entered 640×480 without a SETRES"
    );
    assert_eq!(game.engine.display_size().height, 480);
    // The shell: task 0 is the info board, `_IBON` says the board is up, and
    // module 3's `STARTUP` chose board 5, the registration.
    assert_eq!(game.get_var(2, "_TASK"), Some(0));
    assert_eq!(game.get_var(2, "_IBON"), Some(1));
    assert_eq!(game.get_var(2, "_IBNR"), Some(5));
    assert_eq!(
        game.get_var(2, "_ACTLOC"),
        Some(18),
        "location 18 is the board"
    );
    assert_eq!(
        game.get_var(218, "_INITSTAT"),
        Some(1),
        "waiting for the name"
    );
    // Of the board's eight texts two are shown: the name with its cursor and
    // the postcode without, `#s<` and `#s` of table 4. The other six — the
    // highscore's names, dates and scores and the two entry texts in their
    // other states — wait inactive.
    let texts: Vec<(i32, i32)> = game
        .engine
        .descriptors()
        .iter()
        .filter(|d| d.is_text() && d.active)
        .filter_map(|d| Some((d.shows.table()?, d.text?)))
        .collect();
    assert_eq!(texts, [(4, 1), (4, 4)], "table 4's `#s<` and `#s`");
    assert_eq!(
        game.engine
            .descriptors()
            .iter()
            .filter(|d| d.is_text())
            .count(),
        8
    );
    let picture = game.render();
    assert_eq!((picture.width, picture.height), (640, 480));
    digests().check("registration", digest::frame(&picture));
    assert!(
        walked_past(&game).is_empty(),
        "words walked past without effect: {:?}",
        walked_past(&game)
    );
}

/// The pointer on the registration board is the engine's own arrow, and it
/// wears the colors `TOGFX` resolved against the system palette — not the
/// board's.
///
/// The game never says `SHOWMOUSE` and reaches no `XATMOUSE` before its
/// story starts, so what the player points with here is what `TOGFX` left:
/// the arrow with its body the entry nearest white and its outline the
/// entry nearest (0,47,47) in `ENGINE.RSC`'s palette 0 — indices 1 and 52
/// — shown by the handler's own `SHOWMOUSE`. `SETPAL` then puts board 5's
/// palette 105 in force and the two indices stay, so on this board the
/// arrow is a dark red inside a blue-green line. That is what a DOSBox-X
/// recording of the original shows at the corner where its mouse driver
/// parks the pointer, pixel for pixel against this frame: the two colors
/// come out as `#712830` and `#006996` in the recording, which is what the
/// DAC makes of (28,10,12) and (0,26,37).
#[test]
fn togfx_leaves_the_arrow_in_the_system_palettes_indices() {
    let Some(dir) = gamedata_checker() else {
        eprintln!("skipping: no Checker 2000 gamedata directory");
        return;
    };
    let mut game = at_the_registration(&dir);
    assert_eq!(
        game.engine.cursor(),
        Some(PointerShape::Arrow {
            body: 1,
            outline: 52
        })
    );
    assert!(game.engine.pointer_visible(), "TOGFX's own SHOWMOUSE");
    // The mouse driver's corner, where the recording has it.
    game.set_input(0, 0, false, false, 0).expect("input");
    game.step().expect("a frame");
    let frame = game.frame();
    let entry = |i: usize| {
        let p = &frame.palette.raw[i * 3..i * 3 + 3];
        (p[0], p[1], p[2])
    };
    assert_eq!(entry(1), (28, 10, 12), "palette 105's entry 1");
    assert_eq!(entry(52), (0, 26, 37), "palette 105's entry 52");
    let at = |x: usize, y: usize| frame.pixels.pixels[y * 640 + x];
    // Row 0 `K`, row 2 `KWK`, row 8 `KWWWWWWWK`, row 11 `....KKKK`.
    assert_eq!(at(0, 0), 52);
    assert_eq!((at(0, 2), at(1, 2), at(2, 2)), (52, 1, 52));
    assert_eq!(at(4, 8), 1);
    assert_eq!(at(8, 8), 52);
    assert_eq!((at(3, 11), at(4, 11), at(7, 11)), (at(3, 12), 52, 52));
    assert_ne!(at(9, 8), 52, "clear where the bitmap is clear");
}

#[test]
fn a_name_and_a_postcode_close_the_registration_and_fade_to_board_2() {
    let Some(dir) = gamedata_checker() else {
        eprintln!("skipping: no Checker 2000 gamedata directory");
        return;
    };
    let mut game = at_the_registration(&dir);
    // "AB", Enter: the name buffer is `_NAME0`, and the board moves the
    // cursor text to the postcode field.
    for key in [65, 66, 13] {
        press(&mut game, key);
    }
    assert_eq!(
        game.get_var(218, "_INITSTAT"),
        Some(2),
        "waiting for the postcode"
    );
    assert_eq!(
        game.get_var(218, "_IPPT"),
        Some(0),
        "the cursor is at the field's start"
    );
    let typed = game.render();
    digests().check("registration-name", digest::frame(&typed));
    // "123", Enter: three digits are enough, 123 is region 1, and the board
    // fades out and back in as board 2.
    for key in [49, 50, 51] {
        press(&mut game, key);
    }
    assert_eq!(game.get_var(218, "_IPPT"), Some(3));
    press(&mut game, 13);
    assert_eq!(
        game.get_var(218, "_INITSTAT"),
        Some(3),
        "the entry is closed"
    );
    assert_eq!(game.get_var(2, "_PLZ"), Some(123));
    assert_eq!(game.get_var(2, "_REGIONAL"), Some(1));
    assert_eq!(game.get_var(2, "_IBNR"), Some(2));
    settle(&mut game);
    common::frames(&mut game, 2, "board 2");
    assert_eq!(
        game.engine.fades().len(),
        2,
        "one FADEOUT and one FADEIN: {:?}",
        game.engine.fades()
    );
    let board = game.render();
    digests().check("board-2", digest::frame(&board));
    assert!(
        walked_past(&game).is_empty(),
        "words walked past without effect: {:?}",
        walked_past(&game)
    );
}

/// Clicks at a point: the press on one frame, the release on the next, as a
/// player's click reaches `MOUSELK`.
fn click(game: &mut Game<Vm>, x: i32, y: i32) {
    game.set_input(x, y, true, false, 0).expect("input");
    game.step().expect("the frame with the press");
    game.set_input(x, y, false, false, 0).expect("input");
    game.step().expect("the frame with the release");
}

/// *Check it!* opens the information book, whose page is the one place the
/// engine's `FADEIN` mode 2 is reached: the curtain with `WHITEBOX`'s box
/// painted first — white at 25,122, 452 by 317, the black frame two in —
/// and the page's text laid on the box once the curtain has opened.
///
/// The box is no descriptor, so it has to outlive every redraw of the text
/// over it, which the drawer's paint list is for. And this build's curtains
/// wait for nothing, so the board is up the frame after the fade began.
/// Held against a DOSBox-X recording of the original pixel for pixel, the
/// pointer included.
#[test]
fn check_it_opens_the_information_book_on_its_white_page() {
    let Some(dir) = gamedata_checker() else {
        eprintln!("skipping: no Checker 2000 gamedata directory");
        return;
    };
    let mut game = at_the_registration(&dir);
    for key in [65, 66, 13, 49, 50, 51, 13] {
        press(&mut game, key);
    }
    settle(&mut game);
    common::frames(&mut game, 30, "the main menu");
    assert_eq!(game.get_var(2, "_IBNR"), Some(2));
    // *Check it!* is x 140–280, y 265–320 of board 2 (module 218).
    click(&mut game, 200, 290);
    settle(&mut game);
    assert_eq!(game.get_var(2, "_IBNR"), Some(8), "the book's first page");
    let fades = game.engine.fades();
    assert_eq!(fades.len(), 4, "two board changes: {fades:?}");
    for _ in 0..30 {
        game.set_input(200, 290, false, false, 0).expect("input");
        game.step().expect("a frame on the page");
    }
    let text: Vec<(i32, i32)> = game
        .engine
        .descriptors()
        .iter()
        .filter(|d| d.is_text() && d.active)
        .filter_map(|d| Some((d.shows.table()?, d.text?)))
        .collect();
    let frame = game.frame();
    let (entries, _) = frame.palette.raw.as_chunks::<3>();
    let white = entries
        .iter()
        .position(|c| *c == [63, 63, 63])
        .expect("the page's palette has a white");
    let black = entries
        .iter()
        .position(|c| *c == [0, 0, 0])
        .expect("and a black");
    let at = |x: usize, y: usize| usize::from(frame.pixels.pixels[y * 640 + x]);
    assert_eq!(at(25, 122), white, "the box's corner");
    assert_eq!(at(26, 123), white);
    assert_eq!(at(27, 124), black, "the frame, two in");
    assert_eq!(at(28, 125), white, "white inside it");
    assert_eq!(at(476, 438), white, "the box's far corner");
    assert_eq!(at(474, 436), black, "the frame's far corner");
    assert_ne!(at(477, 439), white, "the board past the box");
    // The page's text stands on the box: table 158's first entry, in
    // black, from 41,133.
    assert_eq!(text, [(158, 1)]);
    assert!(
        (133..150).any(|y| (41..60).any(|x| at(x, y) == black)),
        "the first line's glyphs are drawn in black on the box"
    );
    let picture = game.render();
    digests().check("info-page", digest::frame(&picture));
    assert!(
        walked_past(&game).is_empty(),
        "words walked past without effect: {:?}",
        walked_past(&game)
    );
}
