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
//! The game data this file drives is Dunkle Schatten 2's (MOTION 32-bit).

use motionvm_engine::Game;
use motionvm_forth::Address;
use motionvm_forth::m32::Vm;
use motionvm_testutil::gamedata_ds2;

/// The `_ORDER` block, and a field of it.
fn order(game: &Game<Vm>) -> Address {
    motionvm_forth::m32::word_address(&game.vm, 2, "_ORDER")
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
        eprintln!("skipping: no gamedata directory");
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
            .map(|d| (d.sprite.unwrap_or(0), d.x, d.y))
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
        eprintln!("skipping: no gamedata directory");
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
    assert_eq!(shown.table, Some(2), "the description lives in block 2");
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
        eprintln!("skipping: no gamedata directory");
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
            .map(|b| b as char)
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
        let count = game.vm.fetch(field(Address(record), 8)).expect("answers") as i32;
        for i in 0..count.min(32) {
            let entry = field(Address(fields), i as u32 * 0x12).0;
            if name_at(&game, field(Address(entry), 8).0) == "DINFO" {
                // Answers sit 0x12 apart, so every other one starts mid-cell.
                let line = game
                    .vm
                    .mem
                    .fetch_unaligned(field(Address(entry), 0))
                    .expect("its line") as i32;
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
        game.vm.fetch(field(o, 0x194)).expect("the node") as i32,
        line,
        "INFO should have entered at the DINFO answer and stepped to its line"
    );
}
