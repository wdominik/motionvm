//! Falsches Spiel mit Eddie M. saves through the same three files.
//!
//! The save scheme is the 16-bit engine's and not the game's: `PUT` writes the
//! location into `.blk`, `PUTANIM` the display into `.anm`, `=>PUTAS` the
//! resident modules into `.FRZ`, and the same five slots 701 to 705 are probed
//! with `706 701 DO I =>EXIST … LOOP`. `enviro_savegames.rs` drives that
//! scheme end to end through the save page; what is asked here is that this
//! game reaches those words and that its slots are found again by a later
//! session.
//!
//! Where the probe sits is this game's own arrangement: in `SHOW_FILES`, a
//! word of the boot module itself — the menu that the siblings keep in a
//! module 650 lives beside `CTRL` and `RUN` here — which the menu runs when
//! its load page opens. `RUN` probes nothing.
//!
//! These need the game's files (`MOTIONVM_GAMEDATA_EDDIEM`) and skip without
//! them.
//!
//! The game this file drives is Falsches Spiel mit Eddie M. (MOTION 16-bit).

mod common;

use common::{frames, word16};
use motionvm_motion_engine::{Game, titles};
use motionvm_motion_forth::m16::Vm;
use motionvm_motion_testutil::{gamedata_eddiem, saves_dir};
use std::path::Path;

/// `RUN` through the intro into location 3, with a save directory.
fn into_the_game(dir: &Path, saves: &Path) -> Game<Vm> {
    let mut game = titles::eddiem::open(dir).expect("opens");
    game.set_saves(saves).expect("a save directory");
    game.start().expect("RUN starts");
    while game.pump().expect("RUN runs to the intro's ANIMPLAY") {}
    let mut left_intro = false;
    for frame in 1..=2000 {
        let key = if frame % 200 == 0 { 32 } else { 0 };
        game.set_input(160, 100, false, false, key).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
        if !game.engine.main_loop() {
            left_intro = true;
        } else if left_intro && game.get_var(601, "ACTLOC") == Some(3) {
            return game;
        }
    }
    panic!("2000 frames and the intro never gave way to CTRL");
}

#[test]
fn a_slot_is_written_by_the_three_words_and_found_by_the_next_session() {
    let Some(dir) = gamedata_eddiem() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
        return;
    };
    let saves = saves_dir("eddiem-saves");

    let mut game = into_the_game(&dir, &saves);
    game.request_location(5).expect("NEXTLOC");
    frames(&mut game, 400, "settling in the location");
    assert_eq!(game.get_var(601, "ACTLOC"), Some(5));
    assert_eq!(
        word16(&mut game, "=>EXIST", &[701]),
        [0],
        "slot 701 is empty before the save"
    );

    // The save page's words, in its order: `CTRL` stores the location into
    // `_LOADTABLE` and runs `2 _LOADTABLE slot PUT`, then `PUTANIM` and
    // `=>PUTAS` on the same slot.
    game.set_var(601, "_LOADTABLE", 5).expect("_LOADTABLE");
    let table = game.address(601, "_LOADTABLE").expect("_LOADTABLE");
    let table = i32::from(game.vm.mem.flat(table).expect("loaded"));
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
