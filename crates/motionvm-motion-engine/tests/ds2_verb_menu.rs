//! The verb menu a right click opens.
//!
//! Everything here is pinned to the classroom's own data, read out of the
//! running game before a line of the menu was written:
//!
//! ```text
//! _ORDER (2:0x28ac)  +0x10 = 60   the scene's menu descriptors
//!                    +0x18 = 286 287 …  verb table, three fields per entry
//! area 4             (153,171)-(250,250), no item, flags 6
//! ```
//!
//! Flags 6 are bits 1 and 2, so two icons, and the verb table's entries 1 and
//! 2 give them the sprites 288 and 292. The anchor is the rectangle's center,
//! (201, 210), and the strip is centered on it: `201 − 0x18·2 + 2 = 155`, then
//! one step of 0x30 — so 155 and 203.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_motion_engine::Game;
use motionvm_motion_forth::Address;
use motionvm_motion_forth::cell;
use motionvm_motion_forth::m32::Vm;
use motionvm_motion_testutil::gamedata_ds2;

/// The `_ORDER` block, and a field of it.
fn order(game: &Game<Vm>) -> Address {
    motionvm_motion_forth::m32::word_address(&game.vm, 2, "_ORDER")
        .expect("_ORDER")
        .next()
}

fn field(base: Address, off: u32) -> Address {
    Address::new(base.module(), base.offset() + off)
}

/// Runs the classroom until its opening settles and the order machine is idle.
///
/// A fixed number of steps will not do, and for two reasons that pull the same
/// way. A step is one band while a curtain runs (see `Engine::step_ticks`), so
/// a budget spends itself inside the entry fade; and what these tests actually
/// need is not a count but a *state* — mode 0, the machine ready for a click.
/// Waiting for the state says so, and stops the number drifting whenever the
/// pacing changes.
fn until_idle(game: &mut Game<Vm>, order: Address) {
    let mut guard = 0;
    loop {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame in the classroom");
        let idle = !game.engine.in_transition()
            && game.get_var(2, "_ACTLOC") == Some(1)
            && game.vm.fetch(field(order, 0x0c)).unwrap_or(1) == 0;
        if idle {
            return;
        }
        guard += 1;
        assert!(guard < 20_000, "the classroom never became idle");
    }
}

/// A right click over a hot area puts the verb strip up, and another takes it
/// away again.
///
/// The one thing the test arranges rather than plays into is `_ORDER+0x1CC`:
/// while a location is still running its opening script that gate is set, and
/// the original refuses every click — left as well as right — for exactly as
/// long. Clearing it is the state the game itself reaches a moment later; the
/// alternative would be to wait for a scene that is both idle and has a live
/// area, and the classroom's conversation starts first.
#[test]
fn a_right_click_opens_the_verb_menu() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(2, "_NEXTLOC", 1).expect("the classroom");
    let o = order(&game);
    until_idle(&mut game, o);

    assert_eq!(
        game.vm.fetch(field(o, 0x0c)).expect("mode"),
        0,
        "the machine should be idle"
    );
    game.vm.store(field(o, 0x1cc), 0).expect("open the gate");
    let base = game.vm.fetch(field(o, 0x10)).expect("menu descriptors");

    // The pointer sits inside area 4, and the button is pressed for one frame.
    let (x, y) = (200, 210);
    game.set_input(x, y, false, true, 0)
        .expect("the right click");
    game.step().expect("the frame with the click");

    assert_eq!(
        game.vm.fetch(field(o, 0x0c)).expect("mode"),
        4,
        "a scene menu is mode 4"
    );
    assert_eq!(
        game.vm.fetch(field(o, 0x00)).expect("verb"),
        0,
        "and no verb is chosen yet"
    );

    let strip = |game: &Game<Vm>| -> Vec<(u32, i32, i32)> {
        (0..5)
            .filter_map(|i| {
                game.engine
                    .descriptors()
                    .iter()
                    .find(|d| d.handle == base + i)
            })
            .filter(|d| d.active)
            .map(|d| (d.shows.graphic().unwrap_or(0), d.x, d.y))
            .collect()
    };
    assert_eq!(
        strip(&game),
        [(288, 155, 210), (292, 203, 210)],
        "two icons, from the verb table, centered on the area"
    );

    // A second right click is the cancel: the strip goes, the mode falls back.
    game.set_input(x, y, false, false, 0).expect("input");
    game.step().expect("a frame between the clicks");
    game.vm
        .store(field(o, 0x1cc), 0)
        .expect("the gate stays open");
    game.set_input(x, y, false, true, 0)
        .expect("the second right click");
    game.step().expect("the frame with it");

    assert!(
        strip(&game).is_empty(),
        "the menu should be gone: {:?}",
        strip(&game)
    );
    assert_eq!(
        game.vm.fetch(field(o, 0x0c)).expect("mode"),
        0,
        "and the machine idle again"
    );
}

/// Picking the "look at" icon shows the thing's description.
///
/// The classroom's area 4 carries flags 6 — bits 1 and 2 — so the strip's
/// first slot is verb 2 and its second verb 3. The area's own description is
/// the text in its field +0x14, which the data says is **66**, and it is
/// shown in block 2 through the `_IINFO` descriptor the block names at +0x140.
#[test]
fn looking_at_something_shows_its_description() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(2, "_NEXTLOC", 1).expect("the classroom");
    let o = order(&game);
    until_idle(&mut game, o);

    game.vm.store(field(o, 0x1cc), 0).expect("open the gate");
    let (x, y) = (200, 210);
    game.set_input(x, y, false, true, 0)
        .expect("the right click");
    game.step().expect("the frame with it");
    assert_eq!(
        game.vm.fetch(field(o, 0x0c)).expect("mode"),
        4,
        "the menu is open"
    );

    // The first icon is "look at", at the x the strip's arithmetic gives it.
    game.set_input(155, 210, false, false, 0).expect("input");
    game.step().expect("a frame with the pointer on the icon");
    game.vm
        .store(field(o, 0x1cc), 0)
        .expect("the gate stays open");
    game.set_input(155, 210, true, false, 0).expect("the pick");
    game.step().expect("the frame with the pick");
    for _ in 0..4 {
        game.set_input(155, 210, false, false, 0).expect("input");
        game.vm
            .store(field(o, 0x1cc), 0)
            .expect("the gate stays open");
        game.step().expect("a frame after the pick");
    }

    let desc = game.vm.fetch(field(o, 0x140)).expect("the info descriptor");
    let shown = game
        .engine
        .descriptors()
        .iter()
        .find(|d| d.handle == desc)
        .expect("the descriptor exists");
    assert!(
        shown.active,
        "looking at something activates its description"
    );
    assert_eq!(
        shown.shows.table(),
        Some(2),
        "the description lives in block 2"
    );
    assert_eq!(shown.text, Some(66), "and it is the area's own text");
}

/// `INFO` steps the conversation to the answer called "DINFO".
///
/// Verbs 6 and 7 do not show anything themselves: they ask their own script
/// word for a **name** — the thing `_PutStringAdr` pushes — and look for the
/// answer that carries it, which becomes the node. When the location has
/// nothing special to say, that name is the literal `"DINFO"` (or `"DGIVE"`),
/// and the shipped conversations carry an answer of exactly that name for the
/// purpose.
///
/// The order is forced the way the game forces one, through mode 97, because
/// no menu can offer these two: no flag constant in the game sets bit 5 or 6.
///
/// One limit worth stating: breaking only the *first* pass leaves this passing,
/// because the classroom's own `DO_INFO` answers with the literal `"DINFO"`
/// anyway and the second pass then finds the same answer. Only breaking both
/// moves the node — to answer 0, which the third pass settles for.
#[test]
fn asking_about_something_enters_the_conversation_at_its_info_answer() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(2, "_NEXTLOC", 1).expect("the classroom");

    let o = order(&game);
    let name_at = |game: &Game<Vm>, base: u32| -> String {
        (0..16u32)
            .map(|i| {
                game.vm
                    .mem
                    .fetch_byte(Address::new(base >> 16, (base & 0xffff) + i))
                    .unwrap_or(0)
            })
            .take_while(|b| *b != 0)
            .map(char::from)
            .collect()
    };

    // Wait for a conversation that offers one, and note the line it names.
    let mut wanted = None;
    for _ in 0..2600 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame in the classroom");
        let record = game.vm.fetch(field(o, 0x17c)).expect("record");
        let fields = game.vm.fetch(field(o, 0x180)).expect("fields");
        if record == 0 || fields == 0 {
            continue;
        }
        let count = cell::signed(game.vm.fetch(field(Address(record), 8)).expect("answers"));
        for i in 0..count.min(32) {
            let entry = field(Address(fields), cell::unsigned(i) * 0x12).0;
            if name_at(&game, field(Address(entry), 8).0) == "DINFO" {
                // Answers sit 0x12 apart, so every other one starts mid-cell.
                let line = cell::signed(
                    game.vm
                        .mem
                        .fetch_unaligned(field(Address(entry), 0))
                        .expect("its line"),
                );
                wanted = Some((i, line));
            }
        }
        if wanted.is_some() {
            break;
        }
    }
    let (index, line) = wanted.expect("no conversation offered a DINFO answer");

    // Force the order, exactly as `FORCE_ORDER` does: verb, target, mode 97.
    game.vm.store(field(o, 0x00), 6).expect("verb INFO");
    game.vm.store(field(o, 0x04), 0).expect("target");
    game.vm.store(field(o, 0x0c), 97).expect("a forced order");
    game.vm.store(field(o, 0x134), 0).expect("gate");
    game.vm.store(field(o, 0x1cc), 0).expect("gate");
    game.set_input(0, 0, false, false, 0).expect("input");
    game.step().expect("the frame that runs the verb");

    // The node found was answer `index`; stepping on from it reaches the line
    // that answer names.
    assert!(index >= 0, "the answer was found");
    assert_eq!(
        cell::signed(game.vm.fetch(field(o, 0x194)).expect("the node")),
        line,
        "INFO should have entered at the DINFO answer and stepped to its line"
    );
}

/// A right click on the inventory bar during a conversation offers the two
/// verbs that make sense there, and a left click on one of them runs it.
///
/// The classroom's opening scene ends in an answer menu, mode 14, and the bar
/// is live under it: `ICTRL` still splits the pointer into the bar's own
/// coordinates and `DOORDER` still sees the buttons. A fresh right press over a
/// slot with something in it (0x7e52e) puts a strip of two icons over the bar
/// — verbs 6 and 7, `INFO` and `GIVE`, their rest sprites out of the verb
/// table — and the block goes to mode 17. Another right click takes the strip
/// down and goes back to the answers; a left click on the first icon picks
/// `INFO`, mode 18, which runs as a forced order as soon as the figure is free
/// and hands back to the conversation.
///
/// Nothing is carried when the scene ends, so the player is handed an item
/// through the game's own `ADDITEM` first.
#[test]
fn a_right_click_on_the_bar_during_a_conversation_offers_info_and_give() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(2, "_NEXTLOC", 1).expect("the classroom");
    let o = order(&game);
    let mode = |game: &Game<Vm>| game.vm.fetch(field(o, 0x0c)).expect("mode");
    let at = |base: u32, off: u32| Address::new(base >> 16, (base & 0xffff) + off);

    // Into the opening scene's first answer menu, on empty frames.
    let mut guard = 0;
    while mode(&game) != 14 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame of the opening scene");
        guard += 1;
        assert!(guard < 20_000, "the classroom never put an answer menu up");
    }

    // Something to click on: the first item whose record carries a sprite,
    // added the way the game adds one.
    game.call(2, "_FITEM", &[]).expect("_FITEM");
    let items = cell::unsigned(game.vm.data.pop().expect("_FITEM pushes its address"));
    let item = (1..200)
        .find(|&n| {
            game.vm
                .fetch(at(items, cell::unsigned(n) * 20 + 8))
                .is_ok_and(|s| cell::signed(s) > 0)
        })
        .expect("an item with a sprite");
    game.call(5, "ADDITEM", &[item]).expect("ADDITEM");
    let list = cell::unsigned(game.get_var(2, "_ACTINV").expect("_ACTINV"));
    let scroll = game.vm.fetch(at(list, 0)).expect("the scroll offset");
    let first = game
        .vm
        .fetch(at(list, 4 + scroll * 4))
        .expect("the first slot");
    assert_ne!(first, 0, "the bar's first slot holds something");

    // The strip: the five bar-menu descriptors, and the verb table's rest
    // sprites — three cells an entry, from verb 1.
    let base = game.vm.fetch(field(o, 0x14)).expect("bar menu descriptors");
    let rest = |game: &Game<Vm>, verb: u32| {
        game.vm
            .fetch(field(o, 0x18 + 12 * (verb - 1)))
            .expect("rest")
    };
    let strip = |game: &Game<Vm>| -> Vec<(u32, i32, i32)> {
        (0..5)
            .filter_map(|i| {
                game.engine
                    .descriptors()
                    .iter()
                    .find(|d| d.handle == base + i)
            })
            .filter(|d| d.active)
            .map(|d| (d.shows.graphic().unwrap_or(0), d.x, d.y))
            .collect()
    };

    // A right click on the first slot: x 64 to 127 of the bar, which starts
    // at y 400.
    let (x, y) = (96, 440);
    game.set_input(x, y, false, true, 0)
        .expect("the right click");
    game.step().expect("the frame with the click");
    assert_eq!(mode(&game), 17, "the item menu is mode 17");
    assert_eq!(
        game.vm.fetch(field(o, 0x04)).expect("target"),
        first,
        "on the item under the pointer"
    );
    // Two icons, 0x30 apart, centered on the slot's middle at x 96 and 0x30
    // down: 96 - 0x18 * 2 + 2 = 50, then 98.
    assert_eq!(
        strip(&game),
        [(rest(&game, 6), 50, 0x30), (rest(&game, 7), 98, 0x30)],
        "INFO and GIVE over the slot"
    );

    // A second right click is the way back to the answers.
    game.set_input(x, y, false, false, 0).expect("input");
    game.step().expect("a frame between the clicks");
    game.set_input(x, y, false, true, 0)
        .expect("the second right click");
    game.step().expect("the frame with it");
    assert_eq!(mode(&game), 14, "back to the answers");
    assert!(strip(&game).is_empty(), "and the strip is gone");

    // Up again, and the first icon picked: INFO on the item.
    game.set_input(x, y, false, false, 0).expect("input");
    game.step().expect("a frame between the clicks");
    game.set_input(x, y, false, true, 0)
        .expect("the third right click");
    game.step().expect("the frame with it");
    assert_eq!(mode(&game), 17);
    game.set_input(60, 456, false, false, 0)
        .expect("the pointer on the first icon");
    game.step().expect("a frame between the clicks");
    game.set_input(60, 456, true, false, 0)
        .expect("the left click");
    game.step().expect("the frame with it");
    assert_eq!(mode(&game), 18, "the pick waits for the figure");
    assert_eq!(game.vm.fetch(field(o, 0x00)).expect("verb"), 6, "INFO");
    assert_eq!(game.vm.fetch(field(o, 0x04)).expect("target"), first);
    assert!(strip(&game).is_empty());

    // The order runs and the conversation carries on from the answer INFO
    // names — a different node, and a line or the answers within a few
    // hundred frames, never a frame the engine refuses. Mode 98 itself is not
    // sampled: the forced order runs and re-enters the conversation inside
    // one frame.
    let node_before = game.vm.fetch(field(o, 0x194)).expect("node");
    game.set_input(60, 456, false, false, 0).expect("input");
    for frame in 0..600 {
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} after the pick: {e}"));
        if (12..=14).contains(&mode(&game)) {
            break;
        }
    }
    assert!(
        (12..=14).contains(&mode(&game)),
        "the conversation went on after INFO: mode {}",
        mode(&game)
    );
    assert_ne!(
        game.vm.fetch(field(o, 0x194)).expect("node"),
        node_before,
        "INFO moved the conversation to the answer it names"
    );
}
