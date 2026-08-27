//! The opening scene, which is where a new game begins.
//!
//! Location 1 is the classroom — module 101 names its people `LD_GABY`,
//! `LD_BANK`, `LD_LEHRER`, `LD_COMPI`. Its task manager (`LTMANAGER`, module
//! 201 at 0x06b6c) opens with a title card and then waits for it to go away
//! again:
//!
//! ```text
//! task 2, phase 0:  11 9 320 120 41 3 SETT1   80 ->LTWAIT   NEXTLTP
//! task 2, phase 1:  ?READYT1 …  FADEOUT  NEXTLTP
//! ```
//!
//! `?READYT1` (module 5, 0x027e0) is `_TI1 @ ACTDESC GDACTIVE NOT` — it asks
//! whether the narration descriptor has switched *itself* off. Nothing in the
//! bytecode does that. The frame walk does, out of the descriptor's own
//! callback, and the callback arrives as the top argument of `NEWSETDESC`:
//! module 3 builds `_TI1` with `160 100 100 1 0 0x51780`, and 0x51780 is module
//! 5 at 0x1780 — listing 0x17b0, the body of `FOLLOWMAN`, whose entire
//! definition is `SDINACTIVE EXIT`.
//!
//! Drop that argument and the game gets no further than its own first sentence.
//!
//! The game data this file drives is Dunkle Schatten 2's (MOTION 32-bit).

use motionvm_engine::Game;
use motionvm_forth::m32::Vm;
use motionvm_testutil::gamedata_ds2;

/// The title card times out on its own and the scene moves on.
///
/// Deliberately not an assertion about *which* phase comes next: the rest of
/// the opening is still being built, and a frame further along may still fail.
/// The one thing this pins is that the scene does not stand still — that a
/// timed text runs down, fires its callback and reports itself finished.
#[test]
fn the_opening_scene_gets_past_its_title_card() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.startup_only().expect("startup");
    game.enter_location(1).expect("the classroom");

    // Note this enters the location from outside rather than through `ICTRL`,
    // so the location's task manager runs but `DOORDER` never does. That is
    // enough for what this test is about — a timed text running down — and it
    // keeps the test on the phases rather than on the order machine, which
    // `the_forced_order_reaches_the_talk_verb` covers instead.
    // Counted in controller frames: the card's `80 ->LTWAIT` counts down once
    // per controller invocation, and a step during a fade is a band and runs no
    // controller at all (see `Engine::step_ticks`). Counting raw steps would
    // spend the budget inside the entry fade.
    let mut seen = Vec::new();
    let mut frames = 0;
    let mut guard = 0;
    while frames < 120 {
        let now = game.task_phase();
        if seen.last() != Some(&now) {
            seen.push(now);
        }
        game.set_input(0, 0, false, false, 0).expect("input");
        let fading = game.engine.in_transition();
        // A later frame may still fail on something further into the scene;
        // that is not what this test is about, so it stops rather than fails.
        if game.step().is_err() {
            break;
        }
        if !fading {
            frames += 1;
        }
        guard += 1;
        assert!(guard < 20_000, "the scene stopped running frames");
    }

    let card = seen.iter().position(|p| *p == (2, 1));
    let card =
        card.unwrap_or_else(|| panic!("the title card phase never came up at all: {seen:?}"));
    assert!(
        card + 1 < seen.len(),
        "task 2 phase 1 never ended — the title card never switched itself off: {seen:?}"
    );
}

/// The opening scene plays its conversation, line by line.
///
/// This one goes through the real controller, because most of what it exercises
/// only exists there: `FORCE_ORDER` parks a verb in the `_ORDER` block and sets
/// the mode to 97, and it is `ICTRL`'s own `DOORDER` on the next frame that
/// picks it up (0x7ea22), calls `EXECORDER`, dispatches verb 5 to the `TALK`
/// case at 0x7c69e, and from there into `CALCDIALOG`. Entering a location from
/// outside never runs `DOORDER` at all, so none of that would be touched.
///
/// Starting it is a matter of writing `_NEXTLOC` once the loop is running,
/// which is exactly what picking "new game" does.
///
/// The line asserted below is the one visible in the original: the bank manager
/// handing over the computer. It is entry 2 of text table 50 and it is the
/// conversation's first node, so it also pins that the entry node comes out of
/// the record's +4 and not from somewhere else.
///
/// The run ends where the reading ends — at the first node that offers the
/// player a choice, 0x7b9fd.
#[test]
fn the_opening_conversation_speaks_its_lines() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(2, "_NEXTLOC", 1)
        .expect("new game: the classroom");

    let mut said: Vec<String> = Vec::new();
    let mut stopped = None;
    for _ in 0..2200 {
        game.set_input(0, 0, false, false, 0).expect("input");
        if let Err(e) = game.step() {
            stopped = Some(e.to_string());
            break;
        }
        let showing = game.engine.descriptors().to_vec();
        for d in showing
            .iter()
            .filter(|d| d.active && d.kind == motionvm_engine::DescriptorKind::Text)
        {
            if let Some(line) = game.engine.descriptor_text(d)
                && !line.is_empty()
                && said.last() != Some(&line)
            {
                said.push(line);
            }
        }
    }

    // The title card first, then the conversation opens.
    assert!(
        said.first()
            .is_some_and(|l| l.trim_start().starts_with("Kleine Feier")),
        "the scene should open on its title card, got {:?}",
        said.first()
    );
    assert_eq!(
        said.iter()
            .position(|l| l.starts_with("Dank dieses neuen PC's")),
        Some(1),
        "the bank manager's line should follow the card, got {said:?}"
    );
    assert!(
        said.len() >= 5,
        "the opening is a whole conversation, got {said:?}"
    );
    assert_eq!(
        stopped, None,
        "the conversation should run without stopping, got {stopped:?}"
    );
}

/// What the frame walk does with a wait, read off 0x68c9a onwards.
///
/// ```text
/// 0x68c9a  cmpl $0,0x14(%eax)   ; a callback at all?
/// 0x68c9e  jle  0x68d39         ; none: nothing happens, not even the count
/// 0x68ca7  cmpl $0,0x18(%eax)   ; wait run out?
/// 0x68cab  jne  0x68d2a         ; no: 0x68d36 counts it down
///          … run the callback
/// ```
///
/// Three cases, all of them live in the game: **-1** never comes due, which is
/// what `NEWSETDESC` leaves behind (0x70ccf) so a fresh descriptor does not
/// fire before anything is set up; **0** is due every frame, which is how
/// `_IINFO` gets a per-frame handler out of `0x53370 SDWORD 0 SDWAIT`; and a
/// positive value counts down once per frame and fires at zero, which is how a
/// line of narration times out.
///
/// Without a callback none of it happens — so `SDWAIT` alone does nothing, and
/// the two have to be modeled together or not at all.
#[test]
fn a_wait_counts_down_only_behind_a_callback() {
    let mut e = motionvm_engine::Engine::new();
    let handle = 1;
    e.add_descriptor(motionvm_engine::Descriptor {
        handle,
        callback: 0,
        wait: 2,
        ..Default::default()
    });

    assert_eq!(
        e.tick_descriptor(handle),
        None,
        "no callback, nothing to run"
    );
    assert_eq!(
        e.descriptors_mut()[0].wait,
        2,
        "no callback, so not even a count down"
    );

    e.descriptors_mut()[0].callback = 0x5_1780;
    assert_eq!(e.tick_descriptor(handle), None);
    assert_eq!(e.tick_descriptor(handle), None);
    assert_eq!(e.descriptors_mut()[0].wait, 0, "two frames, two steps");
    assert_eq!(e.tick_descriptor(handle), Some(0x5_1780), "due at zero");
    assert_eq!(
        e.tick_descriptor(handle),
        Some(0x5_1780),
        "and again: zero stays due"
    );

    e.descriptors_mut()[0].wait = -1;
    assert_eq!(e.tick_descriptor(handle), None, "-1 never comes due");
    assert_eq!(e.descriptors_mut()[0].wait, -1, "and never counts down");
}

/// A location takes its scenery with it when it goes.
///
/// `INCLLOC` tears the old one down with a single line — `_BG @ KILLNDESC`
/// (module 5, 0x019a8) — and `_BG` still holds the *previous* location's
/// background, the first descriptor its macro made. `KILLNDESC` (0x71047) looks
/// that handle up in its screen's list and kills whatever sits at that index
/// until the list is that short (0x710bb-0x710e5), so everything from the
/// background on goes and everything made before it stays.
///
/// Read as a screen handle instead — which is what stood here — the argument
/// matched no screen and nothing was ever removed. The park's monument and a
/// second Gaby were left standing in the classroom.
///
/// Sprite 86 is the interesting survivor: it belongs to the title screen, which
/// was already gone before the park, and it sits at level 1, under every
/// background. The original leaves it there too — it is earlier in the list
/// than any `_BG` that has been killed since — so this test names it rather
/// than pretending the list should come out empty.
#[test]
fn a_location_takes_its_scenery_with_it() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}

    // Counted in *controller* frames, not in steps: a step is one band while a
    // curtain runs (see `Engine::step_ticks`), and a location change fades, so
    // a fixed number of steps would spend most of them inside the fade instead
    // of on the scene. `Game::step` runs the controller only when no curtain is
    // up, which is exactly the original's rule — the fade handler spins and the
    // frame loop does not turn.
    let run = |game: &mut Game<Vm>, frames: usize| {
        let mut left = frames;
        let mut guard = 0;
        while left > 0 {
            game.set_input(0, 0, false, false, 0).expect("input");
            let fading = game.engine.in_transition();
            if game.step().is_err() {
                break;
            }
            if !fading {
                left -= 1;
            }
            guard += 1;
            assert!(guard < 20_000, "the game stopped running frames");
        }
    };
    // A background is a block, everything else a sprite; both count as scenery.
    let scenery = |game: &Game<Vm>| -> Vec<u32> {
        let mut v: Vec<u32> = game
            .engine
            .descriptors()
            .iter()
            .filter_map(|d| d.sprite.or(d.block))
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    };

    game.set_var(2, "_NEXTLOC", 2).expect("the park");
    run(&mut game, 120);
    let park = scenery(&game);
    for id in [1020, 1021, 1023] {
        assert!(
            park.contains(&id),
            "the park should be drawing {id}, has {park:?}"
        );
    }

    game.set_var(2, "_NEXTLOC", 1).expect("the classroom");
    run(&mut game, 200);
    let school = scenery(&game);
    for id in [1020, 1021, 1023] {
        assert!(
            !school.contains(&id),
            "the park's {id} is still in the classroom: {school:?}"
        );
    }
    assert!(
        school.contains(&1010),
        "the classroom should be drawing its own background"
    );
}

/// The player picks an answer and the conversation goes on.
///
/// `CALCDIALOG` puts up to three answers on `o[0x174]`, `+1`, `+2` and the
/// standing "say nothing" line on `+3` (0x7b9fd), stacking them upwards from
/// `GSCRY + 0x168`, and leaves the block in mode 14. `DOORDER` then hit-tests
/// the pointer against each box (0x7e309) and, on a fresh left press inside
/// one, takes all four away and sets the node from the answer: `answer[+0]`
/// for a real one, `record[+0x20]` for the fourth (0x7e40b).
///
/// The click is aimed at the descriptor's own reported position rather than a
/// number written down here, so the test still means something if the layout
/// is ever found to be off.
#[test]
fn picking_an_answer_moves_the_conversation_on() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(2, "_NEXTLOC", 1)
        .expect("new game: the classroom");

    // Run until the answers are up: four text descriptors on one level.
    let menu = |game: &Game<Vm>| -> Vec<(i32, i32)> {
        game.engine
            .descriptors()
            .iter()
            .filter(|d| {
                d.active && d.kind == motionvm_engine::DescriptorKind::Text && d.level == 99
            })
            .map(|d| (d.x, d.y))
            .collect()
    };
    let mut choices = Vec::new();
    for _ in 0..2500 {
        game.set_input(0, 0, false, false, 0).expect("input");
        if game.step().is_err() {
            break;
        }
        if menu(&game).len() >= 3 {
            choices = menu(&game);
            break;
        }
    }
    assert!(
        choices.len() >= 3,
        "the answer menu never came up, got {choices:?}"
    );

    // The four lines, word for word and in the order the original shows them.
    // They are entries 3, 4 and 5 of text table 52 plus the standing "say
    // nothing" line, and they only come out in this order when the answers are
    // read byte-exactly: they sit 0x12 apart, so every other one starts
    // mid-cell and `Memory::fetch`, which addresses cells, hands back its
    // neighbors instead. That is how a spoken line once turned up among the
    // answers.
    let mut shown: Vec<(i32, String)> = game
        .engine
        .descriptors()
        .to_vec()
        .iter()
        .filter(|d| d.active && d.kind == motionvm_engine::DescriptorKind::Text && d.level >= 99)
        .filter_map(|d| game.engine.descriptor_text(d).map(|t| (d.y, t)))
        .filter(|(_, t)| !t.is_empty())
        .collect();
    shown.sort_by_key(|(y, _)| *y);
    let lines: Vec<&str> = shown.iter().map(|(_, t)| t.as_str()).collect();
    assert_eq!(
        lines,
        [
            "Tja, warum nicht. Zeit und Lust reichlich vorhanden.",
            "N\u{f6}, heute nicht. Ich schieb' Kohldampf.",
            "Wie? Einfach so?",
            "Bis sp\u{e4}ter!",
        ],
        "the answers do not match the original"
    );

    // Click the topmost answer, which is the last one placed.
    let (x, y) = *choices.iter().min_by_key(|(_, y)| *y).expect("an answer");
    game.set_input(x, y, true, false, 0).expect("the click");
    game.step().expect("the frame with the click");
    game.set_input(x, y, false, false, 0).expect("input");
    game.step().expect("the frame after it");

    let after = menu(&game);
    assert!(
        after.len() < choices.len(),
        "the menu should be gone after a choice: {choices:?} -> {after:?}"
    );

    // The top answer agrees to the walk home: dial03 plays out, and its
    // closing branch (node 2002) carries action 5 — run `DC_03b` by name —
    // with successor -1. That successor sits 0x22 apart like everything in
    // the branch table; read in cells it came back as -65536 and took the
    // spoken-line path, which is undefined in the original and an overflow
    // here. Played to its end, the classroom hands over to the park.
    for _ in 0..2500 {
        if game.get_var(2, "_ACTLOC") == Some(2) {
            break;
        }
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step()
            .expect("a frame on the way out of the classroom");
    }
    assert_eq!(
        game.get_var(2, "_ACTLOC"),
        Some(2),
        "the top answer leads to the park"
    );
}
/// A `FADEOUT` hides the picture that was already on the screen.
///
/// Its handler (0x74c79) never calls the drawer — unlike `FADEIN`, which draws
/// once at 0x74af9. So what lies under the closing bands has to be exactly the
/// frame before, pixel for pixel.
///
/// It matters because the game switches descriptors immediately before the
/// call. `LTMANAGER` phase 1 is `_BLACK SMDESC SDINACTIVE` and then `FADEOUT`:
/// in the original that has no effect on the picture, because nothing draws in
/// between, so what fades out is still the black title card. Composing afresh
/// during the fade instead revealed the classroom, faded it out, and faded the
/// very same picture back in.
#[test]
fn a_fade_out_hides_the_frame_that_was_showing() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(2, "_NEXTLOC", 1)
        .expect("new game: the classroom");

    let mut before: Option<motionvm_render::Framebuffer> = None;
    let mut checked = 0;
    for _ in 0..400 {
        let fades = game.engine.fades().len();
        game.set_input(0, 0, false, false, 0).expect("input");
        if game.step().is_err() {
            break;
        }
        let frame = game.render();
        let started = game.engine.fades().len() > fades;
        let fading = game.engine.in_transition();

        // The frame a fade out begins on: compare what is still visible with
        // what was there a frame ago.
        if started
            && fading
            && game.engine.fades()[fades].name == "FADEOUT"
            && let Some(last) = &before
        {
            let differ = (0..frame.height as i32)
                .flat_map(|y| (0..frame.width as i32).map(move |x| (x, y)))
                .filter(|(x, y)| frame.get(*x, *y).is_some_and(|p| p != 0))
                .filter(|(x, y)| frame.get(*x, *y) != last.get(*x, *y))
                .count();
            assert_eq!(
                differ, 0,
                "the fade out is hiding a picture that was never shown: \
                     {differ} visible pixels differ from the frame before"
            );
            checked += 1;
        }
        if !fading {
            before = Some(frame);
        }
    }
    assert!(checked > 0, "no fade out was reached at all");
}

/// The pointer is the mouse layer's own drawing, over everything.
///
/// `SHOWMOUSE` (0x2541d, drawer 0x2543e) composes the shape into an 8-aligned
/// scratch block — save-under first, then the masked blit 0x26594 at the
/// `& 7` remainder — and copies that block onto the video surface, so the net
/// position is pointer minus hotspot and nothing the engine draws ever covers
/// it: engine blits only bracket themselves with `HIDEMOUSE`/`SHOWMOUSE`.
/// `START` gives the pointer its shape through `FATMOUSE` before the first
/// scene.
#[test]
fn the_pointer_draws_itself_over_the_frame() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    let (mx, my) = (300, 200);
    for _ in 0..5 {
        game.set_input(mx, my, false, false, 0).expect("input");
        game.step().expect("a frame");
    }
    // Wait for a frame that actually shows a pointer, and say why one has to be
    // waited for. Two things hide it over the title: `323:START_MACRO` opens
    // with `HIDEMOUSE` and only `223:LTMANAGER` phase 7 gives it back with
    // `SHOWMOUSE`, and both fade handlers bracket their band loop with
    // `HIDEMOUSE` (0x74ae2) / `SHOWMOUSE` (0x74b64). So the title sequence is
    // pointerless in the original too, and this test has to click through it
    // like a player — the phases wait for a click and for nothing else.
    let mut guard = 0;
    while game.engine.in_transition() || !game.engine.pointer_visible() {
        let click = !game.engine.in_transition();
        game.set_input(mx, my, click, false, 0).expect("input");
        game.step().expect("a frame");
        guard += 1;
        assert!(guard < 400, "no frame ever showed a pointer");
    }

    let (id, hx, hy) = game
        .engine
        .cursor()
        .expect("FATMOUSE gave the pointer a shape");
    assert!(
        game.engine.pointer_visible(),
        "nothing has hidden the pointer"
    );

    // The very shape the engine resolved, straight out of the bank.
    let bank = motionvm_formats::m32::rsc::Bank::open_dir(&dir).expect("banks open");
    let item = bank
        .item(motionvm_formats::m32::Kind::Gfx8, id as usize)
        .expect("the cursor sprite")
        .expect("present");
    let sprite = motionvm_formats::m32::Sprite::parse(item).expect("parses");

    let frame = game.engine.render();
    let mut opaque = 0;
    for sy in 0..sprite.height as i32 {
        for sx in 0..sprite.width as i32 {
            let p = sprite.pixels[(sy * sprite.width as i32 + sx) as usize];
            if p == motionvm_render::TRANSPARENT {
                continue;
            }
            opaque += 1;
            assert_eq!(
                frame.get(mx - hx + sx, my - hy + sy),
                Some(p),
                "cursor pixel ({sx},{sy}) at the pointer"
            );
        }
    }
    assert!(opaque > 0, "the cursor shape has visible pixels");
}

/// A MOTION 32-bit container that is not this game's is refused by name.
///
/// The file check `Game::open` opens with asks for a `NNN.RSC`, an
/// `ENGINE.EXE` and a `000.FRT`, and every MOTION 32-bit game ships all
/// three — so passing it does not mean the container holds *this* game. Nor
/// do the words the bootstrap names: `START`, `STARTUP` and `INCLLOC` come
/// from the authoring template, and Checker 2000, another MOTION 32-bit game,
/// exports them from the same modules. What it does not export is module 2's
/// `_STARTLOC`, the variable this game's own compiler named and this code
/// reads.
///
/// Checker 2000's files are no more redistributable than this game's, so what
/// stands in for them here is a directory that reaches the same state out of
/// this game's own files: `003.RSC` holds 57 sprites and no script module at
/// all, so a bank built from it alone has no module 2 — which is also what an
/// incomplete copy of this game looks like. Either way the open has to stop
/// and say so, rather than bind the template's words and fail somewhere
/// inside the VM under this game's name.
#[test]
fn a_container_without_this_game_s_script_is_refused_by_name() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let tmp = std::env::temp_dir().join("motionvm-ds2-signature");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("a scratch directory");
    for name in ["003.RSC", "ENGINE.EXE", "000.FRT"] {
        let from = motionvm_formats::find_ci(&dir, name).expect("the game ships it");
        std::fs::copy(from, tmp.join(name)).expect("it copies");
    }

    // The file check passes: this is a MOTION 32-bit directory by every test
    // that looks at file names alone.
    assert!(motionvm_engine::titles::ds2::missing_data(&tmp).is_empty());

    let Err(err) = Game::<Vm>::open(&tmp) else {
        panic!("no module 2, so this is not this game and the open has to say so");
    };
    let msg = err.to_string();
    assert!(
        msg.contains("does not hold Dunkle Schatten 2's script"),
        "says which game it is not: {msg}"
    );
    assert!(
        msg.contains("_STARTLOC"),
        "names the word it looked for: {msg}"
    );
    let _ = std::fs::remove_dir_all(&tmp);
}
