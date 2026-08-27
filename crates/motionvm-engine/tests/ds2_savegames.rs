//! Saving and loading, the way the original does it.
//!
//! A savegame in this game is three files under the id `701 + slot`, written by
//! `ICTRL` in module 4 when the player clicks a slot in `_INVMODE 4`:
//!
//! ```text
//! 4 _AKTLT (701+slot) PUT                  \ 701.blk — four bytes: the location
//! (701+slot) DUP PUTANIM DUP =>PUTAS DROP  \ 701.anm and 701.FRZ
//! ```
//!
//! and read back in `_INVMODE 3` with `INCLLOC` in between, so that the
//! location's modules are present before the saved image goes on top of them.
//!
//! The one thing the original does that must not be reproduced is where the
//! files go: it has no path component at all, so a save lands beside
//! `ENGINE.EXE` among the shipped data. Here that directory is read-only, and
//! [`Game::set_saves`] refuses to point anywhere inside it.
//!
//! The game data this file drives is Dunkle Schatten 2's (MOTION 32-bit).

use motionvm_engine::Game;
use motionvm_forth::Host;
use motionvm_forth::m32::Vm;
use motionvm_testutil::{gamedata_ds2, saves_dir};
use std::path::{Path, PathBuf};

/// Opens the game with a save directory attached.
fn game_with_saves(dir: &Path, name: &str) -> (Game<Vm>, PathBuf) {
    let saves = saves_dir(name);
    let mut game = Game::open(dir).expect("game opens");
    game.set_saves(&saves)
        .expect("the save directory is accepted");
    (game, saves)
}

/// Runs one kernel word with the given arguments and returns what it left.
fn kernel(game: &mut Game<Vm>, word: &str, args: &[i32]) {
    game.vm.data.extend_from_slice(args);
    game.engine
        .word(word, &mut game.vm)
        .unwrap_or_else(|e| panic!("{word}: {e}"));
}

/// `PUT` writes module memory to a file and `GET` reads it straight back.
///
/// This is the pair the save path uses for the location number, and it is the
/// only one of the three artifacts whose bytes are the original's: four bytes,
/// no header. The assertion on the file's size and place is what makes the
/// round trip mean something — a `PUT` that wrote elsewhere would still be
/// readable by a `GET` that looked in the same wrong place.
///
/// It cannot see a `PUT` that writes the right bytes from the wrong address,
/// because the address it reads back from is the same one. `_AKTLT` is one cell
/// wide, so there is no second address to compare against here; the full path
/// through `ICTRL` covers that.
#[test]
fn put_writes_a_block_and_get_reads_it_back() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let (mut game, saves) = game_with_saves(&dir, "put_get");

    // `_AKTLT` is where `ICTRL` parks the location number across a save.
    let at = game
        .address(2, "_AKTLT")
        .expect("module 2 has _AKTLT")
        .next();
    game.vm.mem.store(at, 23).expect("the location number");

    kernel(&mut game, "PUT", &[4, at.0 as i32, 701]);
    assert!(
        game.vm.data.is_empty(),
        "PUT should leave nothing behind: {:?}",
        game.vm.data
    );

    let file = saves.join("701.blk");
    assert_eq!(
        std::fs::metadata(&file).map(|m| m.len()).ok(),
        Some(4),
        "701.blk should be 4 bytes"
    );
    assert_eq!(
        std::fs::read(&file).unwrap(),
        23i32.to_le_bytes(),
        "and hold the location"
    );

    game.vm.mem.store(at, 0).expect("clear it again");
    kernel(&mut game, "GET", &[4, at.0 as i32, 701]);
    assert_eq!(
        game.vm.mem.fetch(at).unwrap(),
        23,
        "GET should read the saved location back"
    );
}

/// `EXIST` answers -1 for a taken slot and 0 for a free one.
///
/// Read at the handler rather than inferred: 0x66b0e calls the runtime's file
/// test and 0x66b18 stores 0xFFFFFFFF on the found side, 0x66b77 a zero on the
/// other. `SHOW_FILES` only ever asks `IF`, so the game itself cannot tell -1
/// from 1 — which is exactly why this asserts the value and not merely that it
/// is non-zero. An `assert_ne!(0, …)` would pass on a `1` just as happily, and
/// the reading would stay wrong in the code with nothing to catch it.
#[test]
fn exist_answers_minus_one_for_a_taken_slot() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let (mut game, saves) = game_with_saves(&dir, "exist");
    std::fs::write(saves.join("701.blk"), 23i32.to_le_bytes()).expect("a slot file");

    kernel(&mut game, "EXIST", &[701]);
    assert_eq!(game.vm.data.pop(), Some(-1), "a taken slot");
    kernel(&mut game, "EXIST", &[702]);
    assert_eq!(game.vm.data.pop(), Some(0), "a free slot");
}

/// `SHOW_FILES` lights the slots that have a file.
///
/// The game's own word, from module 5 at 0x04574: it walks `701 … 705`, writes
/// `id - 700` into `_LOADTABLE` for each one that exists and zero for the rest,
/// then activates the marker descriptor of every taken slot. This is what makes
/// the row of slots in the menu show anything at all.
///
/// It cannot see a slot whose *contents* are wrong — `EXIST` is a file test and
/// nothing here opens the file. That is what the round trip above is for.
#[test]
fn show_files_fills_the_slot_table() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let (mut game, saves) = game_with_saves(&dir, "show_files");
    std::fs::write(saves.join("701.blk"), 23i32.to_le_bytes()).expect("slot 0");
    std::fs::write(saves.join("704.blk"), 4i32.to_le_bytes()).expect("slot 3");

    game.startup_only().expect("STARTUP runs");
    game.call(5, "SHOW_FILES", &[]).expect("5:SHOW_FILES");

    let table = game
        .address(2, "_LOADTABLE")
        .expect("module 2 has _LOADTABLE")
        .next();
    let slots: Vec<i32> = (0..5)
        .map(|i| game.vm.mem.fetch(table.plus_cells(i)).expect("a slot") as i32)
        .collect();
    assert_eq!(slots, [1, 0, 0, 4, 0], "occupied slots carry id - 700");
}

/// The modules the original would have resident, in the order it has them.
///
/// This is the set `=>PUTAS` writes, and it is fixed by the bytecode rather
/// than chosen: `SYSTEM.RSC` is `4 =>GET` and `START`; `4:START` at 0x5420 does
/// `2 5 6 11 13 3 =>GET`, `STARTUP`, `3 =>ERASE`, `12 =>GET`, `DS_INIT`, `12
/// =>ERASE`; and `5:INCLLOC` frees the outgoing location's three modules at
/// 0x1a08–0x1a48 *before* it takes the incoming one's at 0x1b08–0x1b48. Modules
/// 3 and 12 are gone by the time anyone can save, and their slot is what the
/// location's three modules move into.
///
/// The freeing-before-taking is the part worth pinning: it is why slots 7, 8
/// and 9 hold the current location across every move, and why the original's
/// purely positional `=>GETAS` can work at all.
///
/// It cannot see a `=>PUTAS` that ignores this table and writes something else,
/// nor a wrong *order* within one module's own three — the counter-check for
/// that is to make `=>ERASE` do nothing, which pushes the second location's
/// modules into slots 10, 11 and 12 and fails here.
#[test]
fn the_resident_modules_are_the_ones_the_original_would_have() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    assert_eq!(
        game.engine.resident(),
        [4, 2, 5, 6, 11, 13],
        "after the boot sequence, with 3 and 12 already erased again"
    );

    for (location, label) in [(23, "the first location"), (1, "and the next one")] {
        game.set_var(2, "_NEXTLOC", location)
            .expect("ask for the location");
        for _ in 0..600 {
            game.set_input(0, 0, false, false, 0).expect("input");
            game.step().expect("a frame");
            if game.get_var(2, "_ACTLOC") == Some(location) {
                break;
            }
        }
        assert_eq!(
            game.get_var(2, "_ACTLOC"),
            Some(location),
            "{label} was never entered"
        );
        assert_eq!(
            game.engine.resident(),
            [
                4,
                2,
                5,
                6,
                11,
                13,
                100 + location as u32,
                200 + location as u32,
                300 + location as u32
            ],
            "{label}: nine modules, the location's three in the slots 3 and 12 left behind"
        );
    }
}

/// Runs the game up to a settled location, the way `4:START` does.
fn started_in(dir: &Path, name: &str, location: i32) -> (Game<Vm>, PathBuf) {
    let (mut game, saves) = game_with_saves(dir, name);
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(2, "_NEXTLOC", location)
        .expect("ask for the location");
    for _ in 0..600 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame");
        if game.get_var(2, "_ACTLOC") == Some(location) {
            break;
        }
    }
    assert_eq!(
        game.get_var(2, "_ACTLOC"),
        Some(location),
        "location {location} was never entered"
    );
    // Entered is not settled. While a word is part-way through, `Game::step`
    // hands it the frame instead of running `ICTRL` — and `ICTRL` is the only
    // thing that reads the mouse into `_IMX`, so until it runs no click can
    // land anywhere. Waiting for `_IMX` to answer is waiting for exactly that.
    for _ in 0..2000 {
        game.set_input(300, 440, false, false, 0).expect("input");
        game.step().expect("a frame");
        if game.get_var(2, "_IMX") == Some(300) {
            break;
        }
    }
    assert_eq!(
        game.get_var(2, "_IMX"),
        Some(300),
        "ICTRL never got to read the mouse"
    );
    (game, saves)
}

/// `=>PUTAS` writes the resident modules and `=>GETAS` puts them back.
///
/// This is where a savegame's substance lives. Script variables are module
/// memory, so writing the resident modules writes the inventory, every story
/// flag and every person record without anything having to enumerate them —
/// which is why the original's save path names exactly one value by hand, the
/// location number, and leaves the rest to this.
///
/// The flag used here is `_?STIFT` from module 11, one of the ninety-odd `_?…`
/// the game tracks progress with.
///
/// It cannot see a writer that puts the right modules in the wrong order,
/// because the reader keys records by module number and would find them
/// anyway — the record order is pinned separately, by the assertion below on
/// what the file contains.
#[test]
fn putas_writes_the_resident_modules_and_getas_puts_them_back() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let (mut game, saves) = started_in(&dir, "putas", 23);

    game.set_var(11, "_?STIFT", 7).expect("a story flag");
    // Not "the stack is empty": `START` pushes ten values before `ANIMPLAY`
    // and the handler pops none of them, so a running game always has them
    // underfoot. What matters is that `=>PUTAS` takes its one argument and
    // leaves nothing of its own — the handler has no push helper in its body.
    let before = game.vm.data.len();
    kernel(&mut game, "=>PUTAS", &[701]);
    assert_eq!(
        game.vm.data.len(),
        before,
        "=>PUTAS should take its id and leave nothing"
    );

    game.set_var(11, "_?STIFT", 99).expect("change it again");
    kernel(&mut game, "=>GETAS", &[701]);
    assert_eq!(
        game.get_var(11, "_?STIFT"),
        Some(7),
        "the saved value should be back"
    );

    // And the file holds the nine modules the original would have had, in its
    // order — the six from the boot sequence and the location's three. Read
    // back the same way the loader does, so the two cannot drift apart.
    let bytes = std::fs::read(saves.join("701.FRZ")).expect("701.FRZ");
    let numbers = frz_modules(&bytes);
    assert_eq!(
        numbers,
        [4, 2, 5, 6, 11, 13, 123, 223, 323],
        "the resident set, in slot order"
    );
}

/// The module numbers a `.FRZ` carries, in the order they are written.
///
/// Parsed here rather than through the engine on purpose: a reader that shares
/// code with the writer agrees with it by construction, and this assertion is
/// about the layout being what it says it is.
fn frz_modules(bytes: &[u8]) -> Vec<u32> {
    let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().expect("4 bytes"));
    assert_eq!(&bytes[..8], b"DS2FRZ\0\0", "magic");
    assert_eq!(word(8), 1, "version");
    let count = word(12) as usize;
    let mut at = 16;
    let mut numbers = Vec::with_capacity(count);
    for _ in 0..count {
        numbers.push(word(at));
        let cells = word(at + 4) as usize;
        at += 8 + cells * 4;
    }
    assert_eq!(
        at,
        bytes.len(),
        "the records should account for the whole file"
    );
    numbers
}

/// A savegame that does not fit the loaded modules is refused, not applied.
///
/// The original checks nothing at all here — `=>GETAS` opens the file without
/// looking at the handle and copies whatever it finds, positionally. That is
/// survivable in a DOS game that can only ever load its own saves from its own
/// build; it is not a property worth reproducing. The interesting half of the
/// assertion is the second one: after the refusal, memory is untouched.
///
/// The damage is put in the *last* record on purpose. A file that is merely
/// truncated never gets past the parse, so it would pass this test however the
/// loading was arranged — which would make the test worthless for the thing it
/// is named after. Renaming the last record's module leaves a file that parses
/// perfectly and only fails when the modules are matched up, which is exactly
/// where a one-pass loader would already have written the other eight.
///
/// It cannot see a savegame whose records are all the right size and name but
/// hold the wrong contents; nothing could.
#[test]
fn a_savegame_that_does_not_fit_is_refused_before_anything_changes() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let (mut game, saves) = started_in(&dir, "refused", 23);
    game.set_var(11, "_?STIFT", 7).expect("a story flag");
    kernel(&mut game, "=>PUTAS", &[701]);
    game.set_var(11, "_?STIFT", 99).expect("change it again");

    // Point the last record at a module that does not exist, leaving every
    // length intact. Module 11 is the second record, so a loader that applied
    // as it went would have restored the flag before it noticed.
    let path = saves.join("701.FRZ");
    let mut bytes = std::fs::read(&path).expect("701.FRZ");
    let word = |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().expect("4 bytes"));
    let mut at = 16;
    for _ in 0..word(&bytes, 12) as usize - 1 {
        at += 8 + word(&bytes, at + 4) as usize * 4;
    }
    bytes[at..at + 4].copy_from_slice(&999u32.to_le_bytes());
    std::fs::write(&path, &bytes).expect("a tampered savegame");

    game.vm.data.push(701);
    let err = game
        .engine
        .word("=>GETAS", &mut game.vm)
        .expect_err("must be refused");
    assert!(
        err.to_string().contains("module 999"),
        "the error should name it: {err}"
    );
    assert_eq!(
        game.get_var(11, "_?STIFT"),
        Some(99),
        "and nothing may have been applied"
    );
}

/// Puts the menu into save or load mode, the way `DO_INVSEL` does.
///
/// Setting `_INVMODE` alone is not enough for loading: the branch at 0x03760
/// only acts on a slot `_LOADTABLE` says is taken, and it is `DO_INVSEL` that
/// fills that table — cases 1003 and 1004 both end in `SHOW_FILES`. Saving
/// happens to work without it, because the save path runs `SHOW_FILES` itself
/// on the way out.
fn open_menu(game: &mut Game<Vm>, mode: i32) {
    game.set_var(2, "_INVMODE", mode).expect("the menu mode");
    game.call(5, "SHOW_FILES", &[]).expect("5:SHOW_FILES");
}
// `_INVMODE` is set here rather than clicked, and it is worth being exact about
// what that skips, because one of the decisions below rests on it.
//
// A player opens the bar by clicking the inventory icon: `_MLK @ _IMX @ 576 >=
// AND` at 0x02dc0, whose branch runs `FREEZESCR` before setting `_INVMODE 2`
// and `_SYS_LEVEL 3`. That whole block sits inside `_SYS_LEVEL @ 1 <` at
// 0x02a40 — the menu is shut while the game is busy, which is faithful, and
// the title sequence these tests stand in is busy from beginning to end: its
// macro raises the count and only `223:LTMANAGER` phase 7 gives it back. So
// the route in is closed *here*, for a reason that belongs to the title and
// not to saving; a player who clicks through the title reaches it normally
// (see `intro_runs.rs`, `the_title_sequence_hands_its_busy_lock_back`).
//
// The consequence for these tests: **no screen is frozen when they save.** They
// exercise the save and load branches of `ICTRL` in full, but not the freeze
// that a real save would have carried into the snapshot.

/// Clicks a savegame slot the way a player does, and lets it play out.
///
/// `ICTRL` only fills `_IMX` and `_IMY` while the pointer is in the status bar
/// — `MOUSEY 400 <` at 0x028a0 decides it, and `_IMY` is `MOUSEY - 400`. The
/// slot is `(_IMX - 239) / 80`, so x = 280 is slot 0 and the id is 701. The
/// press has to be a fresh one: both branches test `_MLK @ _MPRESSED @ NOT AND`.
fn click_slot(game: &mut Game<Vm>, slot: i32) {
    let x = 239 + 80 * slot + 20;
    game.set_input(x, 440, true, false, 0).expect("press");
    game.step().expect("the frame the click lands on");
    for _ in 0..400 {
        game.set_input(x, 440, false, false, 0).expect("release");
        game.step().expect("a frame");
    }
}

/// Saving and loading through the game's own menu, end to end.
///
/// Nothing here calls a kernel word directly: `_INVMODE` goes to 4, a click
/// lands on the first slot, and `ICTRL`'s own branch at 0x03580 does the rest —
/// `PUT`, `PUTANIM`, `=>PUTAS`, `SHOW_FILES`, `1 _DOSAVE !`. Then `_INVMODE`
/// goes to 3 and the branch at 0x03760 loads it back, with `INCLLOC` in the
/// middle. This is the path a player takes, and it is the only one built.
///
/// It cannot see a load that restores the modules but leaves the display state
/// stale, because a story flag lives in module memory; the checks on the
/// descriptors below are what cover that half.
#[test]
fn a_game_saves_and_loads_through_its_own_menu() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let (mut game, saves) = started_in(&dir, "menu", 23);
    game.set_var(11, "_?STIFT", 7).expect("a story flag");
    let descriptors = game.engine.descriptors().len();

    open_menu(&mut game, 4);
    click_slot(&mut game, 0);

    assert_eq!(
        game.get_var(2, "_DOSAVE"),
        Some(1),
        "the game should know it has been saved"
    );
    for suffix in ["blk", "FRZ", "anm"] {
        let file = saves.join(format!("701.{suffix}"));
        assert!(file.exists(), "701.{suffix} should have been written");
    }
    assert_eq!(
        std::fs::read(saves.join("701.blk")).unwrap(),
        23i32.to_le_bytes(),
        "the location number, which is the one value the save path names by hand"
    );
    let table = game.address(2, "_LOADTABLE").expect("_LOADTABLE").next();
    assert_eq!(
        game.vm.mem.fetch(table).unwrap() as i32,
        1,
        "slot 0 now shows as taken"
    );

    // Undo it in the running game, then load the slot back.
    game.set_var(11, "_?STIFT", 99).expect("change it again");
    open_menu(&mut game, 3);
    click_slot(&mut game, 0);

    assert_eq!(
        game.get_var(11, "_?STIFT"),
        Some(7),
        "the saved flag should be back"
    );
    assert_eq!(
        game.get_var(2, "_ACTLOC"),
        Some(23),
        "and the saved location entered"
    );
    assert!(
        game.engine.descriptors().len() >= descriptors,
        "the scene should have been rebuilt, not emptied"
    );
}

/// A load leaves the picture running rather than frozen.
///
/// Leaving `frozen` and `active` out of the snapshot is the one decision here
/// that could plausibly have gone the other way. When a player saves, the
/// location's screen *is* frozen — `FREEZESCR` runs at 0x02e40 as the menu
/// opens — so carrying those flags would be the faithful-looking choice and
/// would load a game that stands still. They are left out, and the load path's
/// own `UNFREEZESCR` and `1 _INVMODE !` re-establish the state instead.
///
/// **What this test does and does not show.** It pins that a load leaves no
/// screen frozen and that a frame still draws. It does *not* on its own justify
/// the decision, because these tests cannot reach the menu the way a player
/// does (see `open_menu`) and so never save from a frozen state; a snapshot
/// that carried the flags faithfully would pass this unchanged. The counter-
/// check that does bite is forcing `frozen` on in `Engine::restore`, which
/// leaves screens 1 and 3 frozen and fails here. The justification itself rests
/// on the reading of the load path, not on this.
#[test]
fn a_load_leaves_the_picture_running() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let (mut game, _saves) = started_in(&dir, "running", 23);
    open_menu(&mut game, 4);
    click_slot(&mut game, 0);
    open_menu(&mut game, 3);
    click_slot(&mut game, 0);

    assert_eq!(
        game.get_var(2, "_INVMODE"),
        Some(1),
        "back to ordinary play"
    );
    let frozen: Vec<u32> = game
        .engine
        .screens()
        .iter()
        .filter(|s| s.frozen)
        .map(|s| s.handle)
        .collect();
    assert!(
        frozen.is_empty(),
        "no screen may still be frozen after a load: {frozen:?}"
    );
    let drawn = game.render().pixels.iter().filter(|p| **p != 0).count();
    assert!(
        drawn > 10_000,
        "and a frame should draw something: {drawn} pixels"
    );
}

/// A savegame loaded into a *fresh* game hands out no handle twice.
///
/// This has to use two `Game<Vm>`s, and that is the whole point. Descriptor handles
/// are counted from one and never reused within a run, so the counter in the
/// game doing the loading is always at least as high as the one in the file —
/// **a single-process test cannot see this fault at all.** Only a second game,
/// which has made fewer descriptors than the one that saved, can.
///
/// The original has no such problem: its handles are heap addresses, and the
/// allocator cannot hand out one that a live object already holds. Ours can,
/// which is why the counter is in the file.
///
/// It cannot see a stale handle inside a *restored* descriptor's own fields —
/// the fields carry sprites and texts, not handles.
#[test]
fn a_savegame_loaded_into_a_fresh_game_hands_out_no_handle_twice() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let (mut far, saves) = started_in(&dir, "handles", 23);
    // Make this run's counter run well ahead of a fresh one's. `NEWSETDESC`
    // takes six arguments: x, y, level, block, an unused one and a callback.
    for _ in 0..50 {
        kernel(&mut far, "NEWSETDESC", &[0, 0, 0, 0, 0, 0]);
        far.vm.data.pop();
    }
    open_menu(&mut far, 4);
    click_slot(&mut far, 0);
    let saved_high = far
        .engine
        .descriptors()
        .iter()
        .map(|d| d.handle)
        .max()
        .expect("descriptors");

    // A second game, which has made fewer descriptors, loads that file.
    let mut fresh = Game::open(&dir).expect("game opens");
    fresh.set_saves(&saves).expect("the same directory");
    fresh.start().expect("4:START");
    while fresh.pump().expect("startup runs") {}
    fresh.set_var(2, "_NEXTLOC", 23).expect("the same location");
    for _ in 0..2000 {
        fresh.set_input(300, 440, false, false, 0).expect("input");
        fresh.step().expect("a frame");
        if fresh.get_var(2, "_IMX") == Some(300) && fresh.get_var(2, "_ACTLOC") == Some(23) {
            break;
        }
    }
    open_menu(&mut fresh, 3);
    click_slot(&mut fresh, 0);

    let restored = fresh
        .engine
        .descriptors()
        .iter()
        .map(|d| d.handle)
        .max()
        .expect("descriptors");
    assert!(
        restored >= saved_high,
        "the saved descriptors should have come back: {restored}"
    );

    kernel(&mut fresh, "NEWSETDESC", &[0, 0, 0, 0, 0, 0]);
    let handed_out = fresh.vm.data.pop().expect("a handle") as u32;
    assert!(
        handed_out > restored,
        "a new descriptor got handle {handed_out}, which {restored} already reaches"
    );
    let mut handles: Vec<u32> = fresh
        .engine
        .descriptors()
        .iter()
        .map(|d| d.handle)
        .collect();
    let before = handles.len();
    handles.sort_unstable();
    handles.dedup();
    assert_eq!(
        handles.len(),
        before,
        "no two descriptors may share a handle"
    );
}

/// The words that make fonts, templates and screens run only at boot.
///
/// A guard, not a behavior. Several things are left out of the snapshot on the
/// grounds that only `3:STARTUP` and `4:START` ever create them, so a loaded
/// game already has them and has them under the same handles: `+FONT` and
/// `DEFTDT` and `NEWSCREEN` in module 3, `DELAY` and `CTRL` and `STEPMULTI` in
/// module 4. If that ever stops being true — a location macro that defines its
/// own font, say — the reasoning collapses quietly and savegames start losing
/// things. This makes it collapse loudly instead, here rather than in a
/// savegame nobody can explain.
///
/// It cannot see a *runtime* path to those words, only a compiled-in one: a
/// word reached through `EXECUTE` on an address built at run time would not
/// show up in the search.
#[test]
fn only_the_boot_modules_create_fonts_templates_and_screens() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    // The names come from the machine's own ordinal table rather than from the
    // listings: a kernel cell is `0x4000_0000 | ordinal`, and the ordinal is not
    // the word's index in the kernel tables — it is `5 * index + 104`.
    let game = Game::open(&dir).expect("game opens");
    let bank = motionvm_formats::m32::rsc::Bank::open_dir(&dir).expect("resources");
    for word in ["+FONT", "DEFTDT", "NEWSCREEN", "DELAY", "CTRL", "STEPMULTI"] {
        let mut found = Vec::new();
        for (_, id) in bank.present(motionvm_formats::m32::Kind::Script) {
            let item = bank
                .item(motionvm_formats::m32::Kind::Script, id)
                .unwrap()
                .expect("present");
            let Ok(parsed) = motionvm_formats::m32::ScrModule::parse(item) else {
                continue;
            };
            let uses = parsed.entries.iter().any(|e| {
                e.body.iter().any(|c| {
                    c >> 16 == motionvm_formats::m32::le::TAG_KERNEL >> 16
                        && game.vm.ordinal_name(c & 0xffff) == Some(word)
                })
            });
            if uses {
                found.push(parsed.module);
            }
        }
        let want: &[u32] = match word {
            "+FONT" | "DEFTDT" | "NEWSCREEN" => &[3],
            _ => &[4],
        };
        assert_eq!(
            found, want,
            "{word} is used outside the module that may use it"
        );
    }
}

/// The save directory may never be the game data, nor anything inside it.
///
/// The shipped files are checked byte for byte after every round, and a rule
/// that only lives in a person's memory is a rule that eventually gets broken.
/// So the one place where a writable path enters the engine refuses the wrong
/// one, and both sides are canonicalized first — a relative path or a `..`
/// would otherwise walk straight around the test.
///
/// It cannot see a save directory reached through a symlink whose target lies
/// in the game data *and* whose canonical form does not, which cannot happen
/// because canonicalizing resolves symlinks. It also does not replace the
/// checksum run: this proves the engine refuses, not that nothing else wrote.
#[test]
fn the_save_directory_may_not_be_inside_the_game_data() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    assert!(
        game.set_saves(&dir).is_err(),
        "the game data itself must be refused"
    );
    assert!(
        game.set_saves(&dir.join("saves")).is_err(),
        "and anything under it"
    );
    // The refusal must not have left a directory behind in there either.
    assert!(
        !dir.join("saves").exists(),
        "nothing may be created inside the game data"
    );
    // Reached the long way round, through a relative path with a `..` in it.
    // The name comes out of `dir` rather than being written in: a directory
    // called something else is the normal case for anyone but this checkout,
    // and hard-coding it would make the test prove nothing there.
    let name = dir.file_name().expect("the game directory has a name");
    let sideways = dir.join("..").join(name).join("deeper");
    assert!(
        game.set_saves(&sideways).is_err(),
        "a path with .. in it must be refused too"
    );
    assert!(!dir.join("deeper").exists(), "and create nothing");

    assert!(
        game.set_saves(&saves_dir("outside")).is_ok(),
        "somewhere else is fine"
    );
}
