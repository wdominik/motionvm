//! Hilfe für Amajambere saves through the same three files.
//!
//! The save scheme is the 16-bit engine's and not the game's: `PUT` writes the
//! location into `.blk`, `PUTANIM` the display into `.anm`, `=>PUTAS` the
//! resident modules into `.FRZ`, and the same five slots 701 to 705 are probed
//! with `706 701 DO I =>EXIST … LOOP`. `enviro_savegames.rs` drives that scheme
//! end to end through the save page; what is asked here is that this game
//! reaches those words and that its slots are found again by a later session.
//!
//! Where the probe sits is this game's own arrangement. Die Enviro-Kids greifen
//! ein and Jeff Jet run it inside `RUN`, so a save in a slot decides whether the
//! start-up page comes up instead of the first room. This game always opens on
//! its menu — `RUN` ends with `2 _INVMODE !` — and the probe is in `SHOW_FILES`
//! (module 650, id 421), which the menu runs when the load page is opened.
//!
//! These need the game's files (`MOTIONVM_GAMEDATA_HFA`) and skip without them.
//!
//! The game this file drives is Hilfe für Amajambere (MOTION 16-bit).

mod common;

use common::{frames, word16};
use motionvm_motion_engine::{Game, titles};
use motionvm_motion_forth::m16::Vm;
use motionvm_motion_testutil::{gamedata_hfa, saves_dir};
use std::path::Path;

/// `RUN` through the intro and out of the menu, with a save directory.
fn into_the_game(dir: &Path, saves: &Path) -> Game<Vm> {
    let mut game = titles::hfa::open(dir).expect("opens");
    game.set_saves(saves).expect("a save directory");
    game.start().expect("RUN starts");
    while game.pump().expect("RUN runs to the intro's ANIMPLAY") {}
    let mut left_intro = false;
    let mut in_the_loop = false;
    for frame in 1..=8000 {
        let key = if frame % 200 == 0 { 32 } else { 0 };
        game.set_input(160, 100, false, false, key).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
        if !game.engine.main_loop() {
            left_intro = true;
        } else if left_intro {
            in_the_loop = true;
            break;
        }
    }
    assert!(in_the_loop, "8000 frames and the intro never gave way");
    // The menu `RUN` leaves up, clicked away in the strip below the world
    // viewport — otherwise `CTRL`'s `_INVMODE @ 2 <` guard holds every request.
    for (press, n) in [(false, 2), (true, 2), (false, 2)] {
        for _ in 0..n {
            game.set_input(30, 185, press, false, 0).expect("input");
            game.step().expect("the menu takes the click");
        }
    }
    frames(&mut game, 200, "settling after the menu");
    game
}

#[test]
fn a_slot_is_written_by_the_three_words_and_found_by_the_next_session() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    let saves = saves_dir("hfa-saves");

    let mut game = into_the_game(&dir, &saves);
    game.request_location(5).expect("NEXTLOC");
    frames(&mut game, 400, "settling in the location");
    assert_eq!(game.get_var(601, "ACTLOC"), Some(5));
    assert_eq!(
        word16(&mut game, "=>EXIST", &[701]),
        [0],
        "slot 701 is empty before the save"
    );

    // The save page's words, in its order — the same three the sibling games'
    // pages run, on the same slot numbers.
    game.set_var(601, "_LOADTABLE", 5).expect("_LOADTABLE");
    let table = game.address(601, "_LOADTABLE").expect("_LOADTABLE");
    let table = i32::from(game.vm.mem.flat(table).expect("loaded")) + 2;
    word16(&mut game, "PUT", &[2, table, 701]);
    word16(&mut game, "PUTANIM", &[701]);
    word16(&mut game, "=>PUTAS", &[701]);
    let slots = game.saves().expect("a save directory").to_path_buf();
    for suffix in ["blk", "anm", "FRZ"] {
        assert!(
            slots.join(format!("701.{suffix}")).exists(),
            "701.{suffix} was written"
        );
    }
    assert_eq!(
        word16(&mut game, "=>EXIST", &[701]),
        [-1],
        "=>EXIST finds the slot"
    );
    drop(game);

    // A later session finds it: the same probe `SHOW_FILES` runs over 701 to
    // 705 when the menu opens its load page.
    let mut fresh = into_the_game(&dir, &saves);
    let found: Vec<i32> = (701..=705)
        .map(|slot| word16(&mut fresh, "=>EXIST", &[slot])[0])
        .collect();
    assert_eq!(found, [-1, 0, 0, 0, 0], "only the written slot answers");
    drop(fresh);
    let _ = std::fs::remove_dir_all(&saves);
}
