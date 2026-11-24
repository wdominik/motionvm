//! Victor Loomes saves and loads through its own panel, and comes back.
//!
//! The scheme is the 16-bit engine's three files on the five slots 701 to 705,
//! but this game reaches them from `CTRL` itself — there is no module 650 and
//! no save page of its own. Both paths are `CTRL`'s own key codes, raised by a
//! click in the strip at the panel's right end (`0104:74a0`-area geometry:
//! `MOUSEX 268 >=`, `MOUSEY 11 <` for 317 and `>=` for 318):
//!
//! * **317, save** — `REQUEST` over text table 6, then
//!   `DUP 700 + 2 AO ROT PUT`, `DUP PUTANIM`, `DUP =>PUTAS`. The location goes
//!   into `(700+n).blk` and the display and module images into `n.anm` and
//!   `n.FRZ`, so a slot is three files under two different numbers.
//! * **318, load** — `706 701 DO I =>EXIST … LOOP` fills `_LOADTABLE` with
//!   only the slots that answered, `REQUEST` offers those, and the answer runs
//!   `DUP 700 + 2 NAO ROT GET`, `NAO @ INCLORT`, `DUP GETANIM`, `DUP =>GETAS`.
//!   The location comes back through `NAO` rather than through `ACTLOC`, which
//!   this game has no module for.
//!
//! These need the game's files (`MOTIONVM_GAMEDATA_VLOOMES`) and skip without
//! them.
//!
//! The game this file drives is Victor Loomes (MOTION 16-bit).

mod common;

use motionvm_motion_engine::{Game, titles};
use motionvm_motion_forth::m16::Vm;
use motionvm_motion_testutil::gamedata_vloomes;
use std::path::{Path, PathBuf};

/// `RUN` through the intro and into the first location, with a save directory.
fn into_the_game(dir: &Path, saves: &Path) -> Game<Vm> {
    let mut game = titles::vloomes::open(dir).expect("the game opens");
    game.set_saves(saves).expect("a save directory");
    game.start().expect("RUN starts");
    for i in 0..4000 {
        let click = i % 150 >= 100 && i % 150 < 102;
        game.set_input(160, 100, click, false, 0).expect("input");
        game.pump().expect("pump");
        game.step().expect("step");
        if game.get_var(605, "AO").unwrap_or(0) > 0 {
            for _ in 0..600 {
                game.set_input(160, 100, false, false, 0).expect("input");
                game.pump().expect("pump");
                game.step().expect("step");
            }
            return game;
        }
    }
    panic!("the intro never reached a location");
}

/// The panel down, then the half of the right-hand strip that raises `key`.
///
/// 317 is the upper half and 318 the lower — `MOUSEY 11 <` decides.
fn open_page(game: &mut Game<Vm>, key: i32) {
    common::hold(game, 160, 8, 300, -1);
    common::hold(game, 290, if key == 317 { 5 } else { 16 }, 400, 100);
}

/// The first button of a `REQUEST` box that is 240 wide at 40,65 and 70 tall.
fn click_first_slot(game: &mut Game<Vm>) {
    common::hold(game, 60, 122, 600, 100);
}

fn temp_saves(tag: &str) -> PathBuf {
    motionvm_motion_testutil::saves_dir(&format!("vloomes-{tag}"))
}

/// The resident script modules, sorted — what `=>PUTAS` writes and `=>GETAS`
/// brings back.
fn resident(game: &Game<Vm>) -> Vec<u32> {
    let mut r = game.engine.resident();
    r.sort_unstable();
    r
}

#[test]
fn a_game_saved_in_one_location_comes_back_there() {
    let Some(dir) = gamedata_vloomes() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    let saves = temp_saves("roundtrip");
    let _ = std::fs::remove_dir_all(&saves);

    // Away from the location `RUN` enters, so that coming back to it proves
    // something. Location 5 has its own pair of modules, 105 and 25.
    let mut game = into_the_game(&dir, &saves);
    assert_eq!(game.get_var(605, "AO"), Some(1), "RUN's own first location");
    game.request_location(5).expect("NAO");
    for _ in 0..600 {
        game.set_input(160, 100, false, false, 0).expect("input");
        game.pump().expect("pump");
        game.step().expect("step");
    }
    assert_eq!(game.get_var(605, "AO"), Some(5), "the game moved");
    let saved_resident = resident(&game);
    assert!(saved_resident.contains(&105), "location 5's script module");
    assert!(saved_resident.contains(&25), "location 5's macro module");

    open_page(&mut game, 317);
    assert!(game.engine.has_request(), "the save box is up");
    click_first_slot(&mut game);
    assert!(!game.engine.has_request(), "the box is answered and gone");

    // Three files, under the two numbers the words use: the block carries the
    // slot, the other two the answer.
    let mut written: Vec<String> = std::fs::read_dir(game.saves().expect("a save directory"))
        .expect("the save directory")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    written.sort();
    assert_eq!(written, ["001.FRZ", "001.anm", "701.blk"]);
    drop(game);

    // A fresh session starts where `RUN` puts it, not where the save is.
    let mut fresh = into_the_game(&dir, &saves);
    assert_eq!(fresh.get_var(605, "AO"), Some(1), "a new game starts at 1");
    assert!(!resident(&fresh).contains(&105), "and without 5's modules");

    // The load page offers what `=>EXIST` answered for — one slot, so one
    // button, and it fills the box's whole width.
    open_page(&mut fresh, 318);
    assert!(fresh.engine.has_request(), "the load box is up");
    click_first_slot(&mut fresh);
    assert!(!fresh.engine.has_request(), "the box is answered and gone");

    assert_eq!(
        fresh.get_var(605, "AO"),
        Some(5),
        "`GET` into `NAO` and `INCLORT` land in the saved location"
    );
    assert_eq!(
        resident(&fresh),
        saved_resident,
        "`=>GETAS` brings the same modules back"
    );
    let lit = fresh.render().pixels.iter().filter(|&&p| p != 0).count();
    assert!(lit > 20_000, "the loaded game draws its room: {lit} pixels");

    // And it plays on: the frame runs, and the game is not torn down.
    common::hold(&mut fresh, 160, 100, 300, -1);
    assert!(!fresh.finished(), "the game plays on after the load");
    assert_eq!(fresh.get_var(605, "AO"), Some(5), "and stays where it is");

    let _ = std::fs::remove_dir_all(&saves);
}

#[test]
fn every_slot_the_panel_offers_is_one_that_was_written() {
    let Some(dir) = gamedata_vloomes() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    let saves = temp_saves("probe");
    let _ = std::fs::remove_dir_all(&saves);

    // With nothing saved, `_LTPOINT` stays 0 and `CTRL` puts up the other box
    // — one button, text 21 — rather than the slot list. Both are a `REQUEST`,
    // so the box being up is not the assertion; what is asked is that no slot
    // answers `=>EXIST` before anything is written.
    let mut game = into_the_game(&dir, &saves);
    let existing = |g: &mut Game<Vm>| -> Vec<i32> {
        (701..=705)
            .map(|slot| {
                let mut stack = vec![slot];
                let Game { engine, vm, .. } = g;
                engine
                    .plain_word16("=>EXIST", &mut stack, &mut vm.mem)
                    .expect("=>EXIST is a kernel word");
                stack[0]
            })
            .collect()
    };
    assert_eq!(existing(&mut game), [0; 5], "nothing is saved yet");

    open_page(&mut game, 317);
    click_first_slot(&mut game);
    assert_eq!(
        existing(&mut game),
        [-1, 0, 0, 0, 0],
        "only the slot the box answered with is there"
    );
    drop(game);

    // And a later session finds the same one.
    let mut fresh = into_the_game(&dir, &saves);
    assert_eq!(existing(&mut fresh), [-1, 0, 0, 0, 0]);
    let _ = std::fs::remove_dir_all(&saves);
}

/// A save that stops at its last file leaves the first save — all three of
/// its files, under both of their numbers — to the next session.
///
/// This game's save is the one that does not keep to a single number:
/// `701.blk` from `PUT`, `001.anm` and `001.FRZ` from the other two. The
/// second save here gets through `PUT` and `PUTANIM` and is stopped at
/// `=>PUTAS` by a directory where its temporary would go. A fresh session must
/// find the first save byte for byte, and load it into location 5.
#[test]
fn a_save_stopped_at_its_last_file_leaves_the_first_save() {
    let Some(dir) = gamedata_vloomes() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    let saves = temp_saves("stopped");
    let files = ["701.blk", "001.anm", "001.FRZ"];

    let mut game = into_the_game(&dir, &saves);
    game.request_location(5).expect("NAO");
    common::hold(&mut game, 160, 100, 600, -1);
    assert_eq!(game.get_var(605, "AO"), Some(5), "the game moved");
    open_page(&mut game, 317);
    click_first_slot(&mut game);
    let slots = game.saves().expect("a save directory").to_path_buf();
    let read = || -> Vec<Vec<u8>> {
        files
            .iter()
            .map(|f| std::fs::read(slots.join(f)).expect("the save"))
            .collect()
    };
    let first = read();
    drop(game);

    // The second session saves from location 1 and is stopped at the last
    // of the three files.
    let mut second = into_the_game(&dir, &saves);
    let block = slots.join("001.FRZ.tmp");
    std::fs::create_dir(&block).expect("a directory in the temporary's way");
    open_page(&mut second, 317);
    let mut result = Ok(());
    for f in 0..600 {
        second
            .set_input(60, 122, f == 100, false, 0)
            .expect("input");
        result = second.pump().and_then(|_| second.step());
        if result.is_err() {
            break;
        }
    }
    let err = result.expect_err("the save is stopped");
    assert!(
        err.to_string().contains("001.FRZ"),
        "at the last file: {err}"
    );
    assert_ne!(
        std::fs::read(slots.join("701.blk")).expect("the second .blk"),
        first[0],
        "the second save had replaced the block"
    );
    drop(second);
    std::fs::remove_dir(&block).expect("unblocked");

    let mut fresh = into_the_game(&dir, &saves);
    assert!(read() == first, "the first save, byte for byte");
    open_page(&mut fresh, 318);
    click_first_slot(&mut fresh);
    assert_eq!(fresh.get_var(605, "AO"), Some(5), "and it loads");
    let _ = std::fs::remove_dir_all(&saves);
}
