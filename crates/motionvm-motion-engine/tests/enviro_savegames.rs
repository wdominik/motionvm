//! Die Enviro-Kids greifen ein saves and loads through its own three files.
//!
//! The game's save page (`DOINVSAVE`, module 650) runs `PUT`, `PUTANIM` and
//! `=>PUTAS` on a slot, and `CTRL`'s load path runs `GET`, `INCLLOC`,
//! `GETANIM`, `=>GETAS` in that order — the same words, the same order and
//! the same three artifacts as Dunkle Schatten 2's, written here in the
//! 16-bit layout. These tests run those words the way the scripts do and
//! need the game's files (`MOTIONVM_GAMEDATA_ENVIRO`); they skip without them.
//!
//! The game this file drives is Die Enviro-Kids greifen ein (MOTION 16-bit).

mod common;

use common::{frames, word16};
use motionvm_motion_engine::{Game, titles};
use motionvm_motion_forth::cell;
use motionvm_motion_forth::m16::Vm;
use motionvm_motion_testutil::{gamedata_enviro, saves_dir};
use std::path::Path;

/// Where `RUN` stands after the intro: in the game loop, or — with a save in
/// a slot — on its start-up page, waiting for a click.
enum After {
    InTheGame,
    OnThePage,
}

/// `RUN` through the intro, with a save directory.
fn into_the_game(dir: &Path, saves: &Path) -> (Game<Vm>, After) {
    let mut game = titles::enviro::open(dir).expect("opens");
    game.set_saves(saves).expect("a save directory");
    game.start().expect("RUN starts");
    while game.pump().expect("RUN runs to the intro's ANIMPLAY") {}
    let mut left_intro = false;
    let mut page_frames = 0;
    for frame in 1..=6000 {
        let key = if frame == 300 { 27 } else { 0 };
        game.set_input(160, 100, false, false, key).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
        if !game.engine.main_loop() {
            left_intro = true;
            if game.is_running() {
                page_frames += 1;
                if page_frames > 100 {
                    return (game, After::OnThePage);
                }
            }
        } else if left_intro {
            return (game, After::InTheGame);
        }
    }
    panic!("6000 frames and the intro never gave way to CTRL or the page");
}

/// One click, then the frames after it.
fn click(game: &mut Game<Vm>, x: i32, y: i32, then: usize) {
    game.set_input(x, y, true, false, 0).expect("input");
    game.step().expect("the click's frame");
    for frame in 1..=then {
        game.set_input(x, y, false, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} after the click: {e}"));
    }
}

#[test]
fn a_game_saved_in_one_location_comes_back_there_with_its_state() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    let saves = saves_dir("enviro-saves");

    // Play into location 2 and change something a savegame must carry: the
    // story day in module 607, and an item in the inventory (module 601).
    let (mut game, after) = into_the_game(&dir, &saves);
    assert!(matches!(after, After::InTheGame), "no save yet, so no page");
    game.request_location(2).expect("NEXTLOC");
    frames(&mut game, 120, "settling in the location");
    assert_eq!(game.get_var(601, "ACTLOC"), Some(2));
    game.set_var(607, "_TAG", 3)
        .expect("_TAG is a variable of module 607");
    game.call(602, "ADDITEM", &[5])
        .expect("ADDITEM puts an item in the bar");
    assert_eq!(
        word16(&mut game, "=>EXIST", &[701]),
        [0],
        "slot 701 is empty before the save"
    );

    // The save page's words, in its order: the location into `.blk`, the
    // display into `.anm`, the resident modules into `.FRZ`.
    game.set_var(601, "_LOADTABLE", 2).expect("_LOADTABLE");
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

    // A fresh game finds the slot and puts its page up over location 1:
    // *Laden*, then the slot row — a click on the first slot's star is what
    // runs `CTRL`'s load path: `GET`, `INCLLOC`, `GETANIM`, `=>GETAS`, the
    // palette, `UPLOAD_GAME`, the fades.
    let (mut fresh, after) = into_the_game(&dir, &saves);
    assert!(
        matches!(after, After::OnThePage),
        "a save in a slot puts the page up"
    );
    assert_eq!(fresh.get_var(601, "ACTLOC"), Some(1));
    assert_eq!(fresh.get_var(607, "_TAG"), Some(1));
    click(&mut fresh, 30, 180, 60);
    assert!(
        fresh.engine.main_loop(),
        "Laden lets RUN into the game loop with the slot row up"
    );
    click(&mut fresh, 118, 180, 300);
    assert_eq!(
        fresh.get_var(601, "ACTLOC"),
        Some(2),
        "the load lands in the saved location"
    );
    assert_eq!(
        fresh.get_var(607, "_TAG"),
        Some(3),
        "the module image brought the day back"
    );
    let list = fresh.get_var(601, "_ACTINV").expect("_ACTINV");
    assert_eq!(
        word16(&mut fresh, "?INVINCL", &[5, list]),
        [1],
        "the item is back in the inventory"
    );
    assert!(
        fresh.render().pixels.iter().any(|&p| p != 0),
        "the loaded game draws a picture"
    );
    // And the game goes on: CTRL runs, descriptors are numbered per screen
    // without a gap, as before the save.
    frames(&mut fresh, 200, "playing on after the load");
    let main = cell::unsigned(fresh.get_var(601, "_MS").expect("_MS"));
    let mut numbers: Vec<u32> = fresh
        .engine
        .descriptors()
        .iter()
        .filter(|d| d.screen == main)
        .map(|d| d.handle)
        .collect();
    numbers.sort_unstable();
    assert_eq!(
        numbers,
        (0..cell::narrow(numbers.len())).collect::<Vec<_>>()
    );
    let _ = std::fs::remove_dir_all(&saves);
}

/// A Dunkle Schatten 2 save in the slot — the same file names — is not this
/// game's and says so instead of loading.
#[test]
fn a_slot_holding_the_other_games_files_is_refused_by_name() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    let saves = saves_dir("enviro-foreign");
    // Into the directory the engine picks under the one it is given, which is
    // where it would find one of this game's own.
    let slots = saves.join("enviro");
    std::fs::create_dir_all(&slots).expect("the game's own save directory");
    std::fs::write(slots.join("701.anm"), b"DS2ANM\0\0junk").expect("a foreign .anm");
    let (mut game, _) = into_the_game(&dir, &saves);
    let mut stack = vec![701];
    let Game { engine, vm, .. } = &mut game;
    let err = engine
        .plain_word16("GETANIM", &mut stack, &mut vm.mem)
        .expect_err("a 32-bit .anm is not a 16-bit one");
    let text = err.to_string();
    assert!(text.contains("not a savegame file"), "{text}");
    let _ = std::fs::remove_dir_all(&saves);
}
