//! Steps the intro's state machine without a window.
//!
//! The intro is not a picture, it is a sequence: the location macro sets a task
//! number and returns, and from then on the engine calls the location's task
//! manager once per frame. That manager advances a phase when it sees a click,
//! fades between pictures and puts text on the screen.
//!
//! Testing that through the window would mean testing the window. All this
//! needs is the game and a clock, so it runs headless — the same reason the
//! renderer could be verified against the original before any window existed.

use motionvm_engine::Game;
use motionvm_testutil::gamedata;

#[test]
fn the_title_macro_arms_the_task_manager() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.startup_only().expect("startup");
    game.enter_location(23).expect("title macro");

    // The macro's own last three lines: task 1, phase 0, and the handler
    // pointing at LTMANAGER in module 223.
    assert_eq!(
        game.task_phase(),
        (1, 0),
        "the title macro arms task 1 at phase 0"
    );
    let handler = game.get_var(2, "_LTHANDLER").expect("_LTHANDLER exists");
    assert_eq!(
        (handler as u32 >> 16, handler as u32 & 0xffff),
        (223, 0x19c8),
        "the handler is LTMANAGER in module 223"
    );
}

#[test]
fn a_click_advances_the_intro() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.startup_only().expect("startup");
    game.enter_location(23).expect("title macro");

    // Idle frames must not advance anything. Phase 0 waits for input and
    // nothing else, so a manager that ran away on its own would show up here.
    for _ in 0..30 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame with no input");
    }
    assert_eq!(
        game.task_phase(),
        (1, 0),
        "without input the intro stays put"
    );

    // One click, then let it settle.
    game.set_input(0, 0, true, false, 0).expect("input");
    game.step().expect("the frame with the click");
    for _ in 0..10 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame after the click");
    }

    let (task, phase) = game.task_phase();
    assert_eq!(task, 1, "still the same task");
    assert!(
        phase > 0,
        "a click moves the intro on, but the phase is still {phase}"
    );
    eprintln!("intro reached task {task}, phase {phase}");
}

#[test]
fn a_deactivated_descriptor_leaves_nothing_behind() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.startup_only().expect("startup");
    game.enter_location(23).expect("title macro");

    // The macro fades the status bar out on the way in; let that finish, or
    // the frame under test is a transition frame rather than a settled one.
    while game.engine.in_transition() {
        game.step().expect("a curtain frame");
    }
    // And one settled frame after it: the drawer runs once a frame out of the
    // game loop and nowhere else, so a run that stops the moment the curtain
    // does has nothing on the screen but what `FADEIN` drew.
    game.step().expect("a settled frame");

    let first = game.render();
    let drawn = first.pixels.iter().filter(|&&p| p != 0).count();
    assert!(drawn > 0, "the title screen draws something to begin with");

    // Exactly what phase 1 of the intro does to the logo, and what the next
    // drawn frame has to reflect: turn a descriptor off and it is gone. The
    // drawer rebuilds from the list rather than painting on top of what was
    // there, so nothing survives — but it takes a *draw* to happen, because
    // switching a descriptor off changes no pixel by itself. That is the
    // original's order too: `SDINACTIVE` erases nothing, 0x6915b does.
    for d in game.engine.descriptors_mut() {
        d.active = false;
    }
    game.engine.draw();
    // And it takes a *present* to show, which is the original's second step:
    // the drawer fills the surface, 0x1457D copies the marked tiles to the
    // visible screen. A frame that only draws changes nothing anyone can see.
    game.engine.present();
    let second = game.render();
    let left = second.pixels.iter().filter(|&&p| p != 0).count();
    assert_eq!(left, 0, "{left} pixels of the previous frame survived");
}

/// The fade out must finish before the picture is swapped.
///
/// This is the whole point of stopping the interpreter mid-word. Phase 1 of the
/// intro reads
///
/// ```text
/// FADEOUT   _BG @ ACTDESC SDACTIVE   _BG2 @ ACTDESC SDINACTIVE
/// 91 SETPAL   FADEIN   NEXTLTP
/// ```
///
/// all in one call, and the original does not reach the second line until the
/// curtain is closed. Here the phase counter is the visible proxy: `NEXTLTP` is
/// the last thing in the branch, so as long as the closing curtain runs, the
/// phase cannot have moved.
#[test]
fn the_fade_out_finishes_before_the_picture_changes() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.startup_only().expect("startup");
    game.enter_location(23).expect("title macro");
    settle(&mut game);

    game.set_input(0, 0, true, false, 0).expect("click");
    game.step().expect("the frame with the click");
    assert_eq!(
        game.task_phase(),
        (1, 1),
        "the click moves the intro to phase 1"
    );

    game.set_input(0, 0, false, false, 0).expect("input");
    game.step().expect("the frame that starts the fade");
    assert!(game.engine.in_transition(), "phase 1 fades out first");

    // `NEXTLTP` is the last cell of the branch and sits behind `FADEIN`, so the
    // phase cannot move until both curtains have run.
    //
    // **Two numbers, and both are needed**, because the step count alone
    // cannot tell a correct curtain from one running four times too slow.
    //
    // The handler runs `half/8 + 1` passes — 31 on the 640x480 title screen —
    // and waits `duration / bands` ticks after each (`bands = height/16` at
    // 0x74cd0, the division at 0x74cf6, the wait at 0x74db6). At 480 rows that
    // is `50/30 = 1` tick a band, so `FADEOUT` and `FADEIN` together are **62
    // bands and 62 ticks** — 0.31 s, and the fastest fade in the game, because
    // 50/30 truncates where the 400-row screen's 50/25 does not.
    //
    // A step is a band while a curtain runs, so the step count is 62 — and it
    // is 62 under a model that gives each band a whole *frame* as well. Only
    // the tick count separates them:
    //
    // | model                              | steps | ticks |
    // |------------------------------------|-------|-------|
    // | a band a frame                     |    62 |   496 |
    // | a frame's worth of bands per frame |     8 |    64 |
    // | a band a step (this)               |    62 |    62 |
    let mut steps = 0;
    let mut ticks = 0;
    while game.task_phase() == (1, 1) {
        ticks += game.engine.step_ticks();
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a curtain step");
        steps += 1;
        assert!(steps < 500, "the phase never moved on");
    }
    assert_eq!(steps, 62, "two curtains of 31 bands each");
    assert_eq!(
        ticks, 62,
        "and one tick a band, as 50/30 works out on a 480-row screen"
    );
    eprintln!("phase 1 took {steps} bands over {ticks} ticks");
}

#[test]
fn the_old_picture_fades_out_in_its_own_colors() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    // Palette 96 belongs to the logo, 91 to the title art the intro switches
    // to. Loaded straight from the resources so the test names the two by what
    // they are, not by whatever the engine happens to hold.
    let bank = motionvm_formats::rsc::Bank::open_dir(&dir).expect("resources");
    let load = |id: usize| {
        let item = bank
            .item(motionvm_formats::Kind::Palette, id)
            .expect("read")
            .expect("present");
        motionvm_formats::Palette::from_6bit(item)
    };
    let (logo, title) = (load(96), load(91));
    assert_ne!(
        logo, title,
        "the two palettes have to differ for this to mean anything"
    );

    let mut game = Game::open(&dir).expect("game opens");
    game.startup_only().expect("startup");
    game.enter_location(23).expect("title macro");
    while game.engine.in_transition() {
        game.step().expect("a curtain frame");
    }
    assert_eq!(
        game.engine.palette(),
        &logo,
        "the logo is shown in palette 96"
    );

    game.set_input(0, 0, true, false, 0).expect("click");
    game.step().expect("the frame with the click");
    game.set_input(0, 0, false, false, 0).expect("input");
    game.step().expect("the frame that fades");

    // The interpreter is stopped inside the phase, right after FADEOUT and
    // before the cells that swap the picture. So this is not a copy of the old
    // palette kept for the occasion — `SETPAL 91` has genuinely not run yet,
    // exactly as in the original.
    assert!(game.engine.in_transition());
    assert_eq!(
        game.engine.palette(),
        &logo,
        "the closing curtain shows the old picture, so it keeps palette 96"
    );

    // Once the closing curtain is through, the opening one takes over and it
    // belongs to the new picture.
    while *game.engine.palette() == logo {
        game.step().expect("a curtain frame");
        assert!(
            game.engine.in_transition(),
            "the palette never changed over"
        );
    }
    assert_eq!(
        game.engine.palette(),
        &title,
        "the opening curtain uses palette 91"
    );
}

/// Where the original puts the intro's text, measured off a screenshot of it.
///
/// The window was 1280 wide, exactly twice the game's 640, so these are game
/// pixels: for each of the seven lines, the first row carrying ink and the
/// leftmost and rightmost inked column. They are the 1996 engine's numbers, not
/// this one's, which is the entire point of testing against them.
const ORIGINAL_LINES: [(u32, u32, u32); 7] = [
    (128, 116, 524),
    (147, 125, 515),
    (166, 124, 516),
    (204, 102, 538),
    (223, 120, 519),
    (242, 121, 518),
    (261, 143, 496),
];

/// Runs frames until nothing is fading, so a still picture can be measured.
fn settle(game: &mut Game) {
    let mut guard = 0;
    while game.engine.in_transition() {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a curtain frame");
        guard += 1;
        assert!(guard < 500, "a transition never ended");
    }
}

fn click(game: &mut Game) {
    game.set_input(0, 0, true, false, 0).expect("click");
    game.step().expect("the frame with the click");
    game.set_input(0, 0, false, false, 0).expect("input");
    game.step().expect("the frame after it");
    settle(game);
}

/// The lines of the frame as (first inked row, leftmost column, rightmost).
fn text_lines(frame: &motionvm_render::Framebuffer) -> Vec<(u32, u32, u32)> {
    let (w, h) = (frame.width as u32, frame.height as u32);
    let mut lines = Vec::new();
    let mut open: Option<(u32, u32, u32)> = None;
    for y in 0..h {
        let inked: Vec<u32> = (0..w)
            .filter(|&x| frame.pixels[(y * w + x) as usize] != 0)
            .collect();
        match (inked.first(), inked.last(), open) {
            (Some(&a), Some(&b), None) => open = Some((y, a, b)),
            (Some(&a), Some(&b), Some((t, l, r))) => open = Some((t, l.min(a), r.max(b))),
            (None, _, Some(line)) => {
                lines.push(line);
                open = None;
            }
            _ => {}
        }
    }
    lines.extend(open);
    lines
}

/// The intro shows two texts, in this order.
///
/// `LTMANAGER` asks for 109 and then 110, and text numbers are one-based, so
/// what comes out is the table's entries 108 and 109: the production credit
/// first, the backstory second. Getting that wrong put a line of dialogue from
/// the next scene into the intro, which is how it was noticed.
#[test]
fn the_intro_shows_its_two_texts_in_order() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.startup_only().expect("startup");
    game.enter_location(23).expect("title macro");
    settle(&mut game);
    click(&mut game); // logo away, title art in
    click(&mut game); // title art away, first text in

    let credit = text_lines(&game.render());
    assert_eq!(
        credit.len(),
        4,
        "the production credit is four lines, got {credit:?}"
    );
    // Four lines of 19 around SDVCEN 200: the block is 4 * 19 - 1 = 75 tall.
    assert_eq!(credit[0].0, 166);
    assert_eq!(credit[3].0, 223);

    click(&mut game); // credit away, backstory in
    let story = text_lines(&game.render());
    assert_eq!(
        story.len(),
        7,
        "the backstory is seven lines, got {story:?}"
    );
    for (i, (got, want)) in story.iter().zip(&ORIGINAL_LINES).enumerate() {
        assert_eq!(
            got,
            want,
            "line {} sits differently from the original",
            i + 1
        );
    }
}

/// A fade takes its screen out of the picture, and brings it back.
///
/// `FADEOUT` clears bit 0x80 of the screen's byte 0x13 and `FADEIN` sets it —
/// the very bit `GSCRACT` reads. That coupling is not decoration: `INCLLOC`
/// ends with `GSCRACT NOT IF … FADEIN THEN`, so a location is only ever
/// revealed *because* fading out left its screen inactive.
#[test]
fn a_fade_takes_its_screen_out_of_the_picture() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.startup_only().expect("startup");
    game.begin_location(23).expect("start entering");

    // The title macro fades the status bar out first thing, so one pump is
    // enough to be inside that.
    game.pump().expect("the first frame of the entry");
    let bar = game
        .engine
        .screens()
        .iter()
        .find(|s| s.handle == 1)
        .expect("the status bar");
    assert!(!bar.active, "fading a screen out has to deactivate it");
    assert!(game.engine.in_transition());

    // And it stays out: nothing fades it back in, so it is gone from the
    // composed frame afterwards too.
    while game.pump().expect("a frame of the entry") {}
    let bar = game
        .engine
        .screens()
        .iter()
        .find(|s| s.handle == 1)
        .expect("bar");
    assert!(!bar.active, "and nothing brought it back");
}

/// The intro hands over to the first room by writing `_NEXTLOC`.
///
/// Phase 7 ends with `1 _KINTRO !  2 _NEXTLOC !`, and no word in the game ever
/// reads `_NEXTLOC` — 23 writes, no reads, the same shape as `_LTHANDLER`. The
/// native loop picks it up, so the rebuilt one has to as well, or the game
/// simply stops when the title sequence is over.
#[test]
fn the_intro_hands_over_to_the_next_location() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.startup_only().expect("startup");
    game.enter_location(23).expect("title macro");
    settle(&mut game);
    for _ in 0..4 {
        click(&mut game);
    }

    // The intro is over and has asked for the park. ( lives in
    // another module and is not what this is about.)
    assert_eq!(
        game.get_var(2, "_NEXTLOC"),
        Some(2),
        "and asks for location 2"
    );

    // One more frame takes the request up: the entry fades out first, which is
    // what leaves the screen inactive for the fade in to reveal.
    let _ = game.step();
    assert_eq!(
        game.get_var(2, "_NEXTLOC"),
        Some(0),
        "the request is consumed, not repeated"
    );
    let main = game
        .engine
        .screens()
        .iter()
        .find(|s| s.handle == 2)
        .expect("screen 2");
    assert!(!main.active, "entering the park fades the screen out first");
}

/// The intro really does arrive in the park, with its own task running.
///
/// This is the first scene past the title sequence, and getting there exercises
/// the whole chain: the phase machine, the handover through `_NEXTLOC`, the
/// location entry with its fade, and `DOWALK` — which is not pathfinding but the
/// interpreter of a command queue held in the person record.
#[test]
fn the_game_reaches_the_park() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.startup_only().expect("startup");
    game.enter_location(23).expect("title macro");
    settle(&mut game);
    for _ in 0..4 {
        click(&mut game);
    }

    // Let the handover and the park's entry play out.
    for _ in 0..400 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame in the park");
    }

    assert_eq!(
        game.get_var(2, "_ACTLOC"),
        Some(2),
        "the park is location 2"
    );
    let (task, _) = game.task_phase();
    assert_eq!(task, 101, "and it has started its own task");

    // Its background is on screen: a full-width picture on the main screen.
    let drawn = game
        .engine
        .descriptors()
        .iter()
        .filter(|d| d.active && d.screen == 2 && d.sprite.or(d.block).is_some())
        .count();
    assert!(
        drawn >= 3,
        "the park draws its scenery, got {drawn} descriptors"
    );
}

/// `FADEIN` hands the drawer the fading screen itself (0x74af9) and the band
/// loop then runs synchronously inside the handler — no other screen gets a
/// frame while a fade is up. So the fade's one draw must leave every other
/// screen's buffer exactly as the last frame drew it, even when descriptors
/// on it have changed since; those changes wait for the next `ANIMPLAY`
/// frame's draw.
#[test]
fn a_fade_in_draws_only_its_own_screen() {
    let mut e = motionvm_engine::Engine::new();
    for handle in [1u32, 2] {
        let mut s = motionvm_render::Screen::new(handle);
        s.size = (4, 4);
        s.buffer = motionvm_render::Framebuffer::new(4, 4);
        s.active = true;
        e.add_screen(s);
    }
    for (id, color) in [(10u32, 1u8), (11, 2)] {
        e.cache_sprite(
            id,
            motionvm_formats::Sprite {
                width: 1,
                height: 1,
                palette: motionvm_formats::Palette::from_6bit(&[]),
                pixels: vec![color],
            },
        );
    }
    e.add_descriptor(motionvm_engine::Descriptor {
        handle: 1,
        screen: 1,
        sprite: Some(10),
        active: true,
        ..Default::default()
    });
    e.add_descriptor(motionvm_engine::Descriptor {
        handle: 2,
        screen: 2,
        sprite: Some(11),
        active: true,
        ..Default::default()
    });
    let two = |e: &motionvm_engine::Engine, x: i32| {
        e.screens()
            .iter()
            .find(|s| s.handle == 2)
            .expect("screen 2")
            .buffer
            .get(x, 0)
    };

    // A frame passes: both screens are drawn.
    e.draw();
    assert_eq!(two(&e, 0), Some(2), "screen 2 has its pixel after a frame");

    // Something moves on screen 2 — and then screen 1 fades in. The fade
    // draws screen 1 alone; screen 2 keeps showing what the last frame drew.
    e.descriptors_mut()[1].x = 2;
    e.draw_screen(1);
    assert_eq!(
        two(&e, 0),
        Some(2),
        "a fade must not carry another screen's change"
    );
    assert_eq!(two(&e, 2), Some(0), "the moved pixel is not there yet");

    // The next frame draws everything, and the move lands.
    e.draw();
    assert_eq!(two(&e, 2), Some(2), "the frame after the fade draws it");
    assert_eq!(two(&e, 0), Some(0), "and the old spot is clear");
}

/// Drives the title sequence to its end through the real controller.
///
/// `4:START` rather than `enter_location`, because the raise and the release
/// this is about straddle both: `INCLLOC` (module 5, 0x017b8) brackets the
/// location macro with `SETBUSY` … `SETNOBUSY`, and the macro's own raise is
/// released a whole sequence later by `LTMANAGER`.
/// Returns the game once the title is over, and the highest `_BUSY` the title
/// itself stood at.
fn play_the_title(dir: &std::path::Path, twice: bool) -> (Game, i32) {
    let mut game = Game::open(dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    if twice {
        // Asks for the title a second time in the same frame: `ICTRL` enters
        // `_STARTLOC` by itself at 0x04960 and then consumes `_NEXTLOC` at
        // 0x04a20, so both entries happen inside one turn of the loop.
        game.set_var(2, "_NEXTLOC", 23)
            .expect("ask for the title again");
    }
    // `4:START` parks at `ANIMPLAY`; it is `ICTRL`'s own housekeeping that
    // enters `_STARTLOC` on the first frame (0x04960), so the title task does
    // not exist until the loop has turned once.
    let mut guard = 0;
    while game.task_phase().0 != 1 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame before the title");
        settle(&mut game);
        guard += 1;
        assert!(guard < 60, "the title task never started");
    }
    // Phases 0, 2, 4 and 6 wait for `_MLK @ _MRK @ OR _AKTKEY @ OR` and for
    // nothing else — module 223 holds no `->LTWAIT` at all, so the title waits
    // for a player indefinitely, in the original too. Clicking only on an even
    // phase is what a player does; clicking blindly would advance two phases
    // on the frames where the fades have already moved it on.
    let mut guard = 0;
    let mut busy = 0;
    while game.task_phase().0 == 1 {
        busy = busy.max(game.get_var(2, "_BUSY").expect("_BUSY exists"));
        if game.task_phase().1 % 2 == 0 {
            click(&mut game);
        } else {
            // An odd phase fades and moves itself on; it only needs frames.
            game.set_input(0, 0, false, false, 0).expect("input");
            game.step().expect("a frame of the title");
            settle(&mut game);
        }
        guard += 1;
        assert!(
            guard < 40,
            "the title never finished: {:?}",
            game.task_phase()
        );
    }
    (game, busy)
}

/// The title sequence hands its busy lock back, and `_BUSY` is a nesting count.
///
/// This replaces a note that claimed location 23 leaves `_BUSY` at 2 for good,
/// which would put the whole menu block behind `_SYS_LEVEL @ 1 <` (module 4,
/// 0x02a40) out of a player's reach. Measurement says otherwise.
///
/// `SETBUSY`/`SETNOBUSY` (module 5, 0x003c8 / 0x004cc) are `_BUSY ++` and
/// `_BUSY --` with the interesting work on the edges only: the 0→1 edge sets
/// `_SYS_LEVEL` to 1, the 1→0 edge puts it back to 0. Over one entry into
/// location 23 the raises and releases are
///
/// ```text
/// INCLLOC          SETBUSY     0 -> 1
/// 323:START_MACRO  SETBUSY     1 -> 2
/// INCLLOC tail     SETNOBUSY   2 -> 1
/// 223:LTMANAGER phase 7        1 -> 0   … and straight back up for location 2
/// ```
///
/// so one net raise stands while the title is on screen — which is the point
/// of it — and the sequence gives it back at the end.
#[test]
fn the_title_sequence_hands_its_busy_lock_back() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let (game, busy) = play_the_title(&dir, false);

    assert_eq!(busy, 1, "one net raise stands over the title, not two");
    assert_eq!(
        game.get_var(11, "_KINTRO"),
        Some(1),
        "the title sequence ran to its end"
    );
    assert_eq!(
        game.get_var(2, "_ACTLOC"),
        Some(2),
        "and handed over to the park"
    );
    // The release itself is not observable from outside a frame: `ICTRL`
    // consumes `_NEXTLOC` in the same invocation that ran the handler
    // (0x04a20), so location 2's own `INCLLOC` has raised the count again
    // before the frame ends. What phase 7 having run proves is that
    // `SETNOBUSY` ran with it — the two are three cells apart.
}

/// A `_BUSY` floor of 2 is an artifact of driving the title twice, not the
/// game.
///
/// The counter-check to the test above, and the reason it is worth keeping: a
/// second entry into location 23 orphans the first `323:START_MACRO`'s
/// `SETBUSY`, because the second `1 SETLOCTASK` puts the task back to phase 0
/// and only one phase-7 `SETNOBUSY` will ever run. The floor is then 1 instead
/// of 0 and the menu really would be out of reach — but only in a run driven
/// that way, never in the game.
#[test]
fn entering_the_title_twice_orphans_a_busy_raise() {
    let Some(dir) = gamedata() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let (game, busy) = play_the_title(&dir, true);

    assert_eq!(busy, 2, "entering twice leaves a raise nobody releases");
    assert_eq!(
        game.get_var(11, "_KINTRO"),
        Some(1),
        "the title still runs to its end"
    );
}
