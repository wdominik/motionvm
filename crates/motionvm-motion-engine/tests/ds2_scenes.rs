//! Four scenes the engine has to be able to reach, and draw something in.
//!
//! Each test drives the engine to a particular standing picture — the title,
//! a page of intro text, a conversation's answer menu, a page of the help
//! viewer — and asserts it arrived. Reaching them is most of the work: the
//! dialogue menu needs startup, a location entry, the task machine and the
//! conversation apparatus to all behave, and the help viewer has to be opened
//! the way the shell opens it. A panic anywhere on those paths fails here.
//!
//! **These are not golden-frame tests, and they need not be.** A
//! checked-in PNG of one of these pictures cannot be distributed: it is a
//! rendering of the game's own artwork — one of the four was the publisher's
//! logo, another a hand-drawn room with its characters and dialogue in it —
//! and the game's data may not enter this repository in any form, including a
//! fixture derived from it. A **digest** of the picture is not such a fixture,
//! reconstructs nothing, and does the one thing a golden frame was wanted for:
//! it notices a change nobody thought to write an assertion for. So each scene
//! below is checked three ways — it was reached, it is the right size and not
//! blank, and it composed the same bytes it composed last time.
//!
//! What a digest cannot do is say *what* moved, and the assertions here stay
//! for that reason: they say what a scene must contain, where the digest only
//! says whether it changed. Anyone with the game can still render these scenes
//! to PNG and diff them across a change, which is what to do once a digest
//! goes off.
//!
//! **Determinism**, which the digests rest on: `RANDOM` is an LCG seeded to a
//! constant in `Vm::new` and never reseeded, there is no `HashMap` on any
//! drawing path, the engine reads no clock, and the resource directory is
//! walked in sorted order — so a scene reached the same way twice composes the
//! same bytes twice. The one thing that is *not* insensitive is the number of
//! `RANDOM` calls made before the frame: the generator is shared and consumed in
//! call order, so a change elsewhere that draws one more random number moves
//! every later one.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

mod common;

use common::settled_in;
use motionvm_motion_engine::Game;
use motionvm_motion_forth::m32::Vm;
use motionvm_motion_testutil::{Digests, digest, gamedata_ds2};
use motionvm_render::Framebuffer;
use std::path::Path;

// ------------------------------------------------------------------ the check

/// This game's table of reference digests.
fn digests() -> Digests {
    common::digests("ds2")
}

/// Everything that can be said about a composed frame: it is the size the
/// engine promises, something was drawn, and it is the same picture as last
/// time.
///
/// "Not blank" is deliberately the weakest useful form — more than one palette
/// index present. It does not know what the picture should look like, but it
/// does catch the failure that matters most here, a scene that reaches its
/// state and then renders a flat field because a descriptor chain, a palette or
/// a blit stopped working. The digest knows even less about what is right, and
/// catches everything else: `name` is the scene's line in the table.
fn drew_something(name: &str, frame: &Framebuffer) {
    assert_eq!(
        (frame.width, frame.height),
        (640, 480),
        "{name}: the frame is {}x{}",
        frame.width,
        frame.height
    );
    assert_eq!(
        frame.pixels.len(),
        usize::from(frame.width) * usize::from(frame.height),
        "{name}: the buffer does not match its own dimensions"
    );
    let first = frame.pixels[0];
    assert!(
        frame.pixels.iter().any(|&p| p != first),
        "{name}: every pixel is index {first} — the scene was reached but nothing was drawn"
    );
    digests().check(name, digest::frame(frame));
}

// ----------------------------------------------------------------- the scenes

/// The curtain out, and then one settled frame.
///
/// The frame is this suite's own and the reason is what it measures: the
/// drawer runs once a frame out of the game loop and nowhere else, so a run
/// that stops the moment the curtain does has nothing on the screen but what
/// `FADEIN` drew.
fn settled(game: &mut Game<Vm>) {
    common::settle(game);
    game.set_input(0, 0, false, false, 0).expect("input");
    game.step().expect("a settled frame");
}

fn click(game: &mut Game<Vm>) {
    game.set_input(0, 0, true, false, 0).expect("click");
    game.step().expect("the frame with the click");
    game.set_input(0, 0, false, false, 0).expect("input");
    game.step().expect("the frame after it");
    settled(game);
}

fn title(dir: &Path) -> Game<Vm> {
    let mut game = settled_in(dir, 23);
    settled(&mut game);
    game
}

/// A game standing in the title, settled, with the pointer on the status bar.
///
/// `ICTRL` only fills `_IMX` while the pointer is in the bar, and only when it
/// runs at all — while a word is part-way through, the frame goes to that word
/// instead. Waiting for `_IMX` to answer is waiting for both.
fn settled_in_the_title(dir: &Path) -> Game<Vm> {
    let mut game = Game::open(dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    for _ in 0..2000 {
        game.set_input(300, 440, false, false, 0).expect("input");
        game.step().expect("a frame");
        if game.get_var(2, "_IMX") == Some(300) && !game.engine.in_transition() {
            return game;
        }
    }
    panic!("the game never settled anywhere the bar could be clicked");
}

/// The answer menu is four text descriptors at level 99.
fn menu_lines(game: &Game<Vm>) -> usize {
    game.engine
        .descriptors()
        .iter()
        .filter(|d| d.active && d.is_text() && d.level == 99)
        .count()
}

// ------------------------------------------------------------------ the tests

/// The title: the first picture the game draws.
#[test]
fn the_title_screen() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = title(&dir);
    drew_something("title", &game.render());
}

/// A page of the intro text: the font, the layout and the backing at once.
///
/// Two clicks in: the logo goes, the title art comes and goes, and the first
/// text page is what is left standing.
#[test]
fn the_intro_text_page() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = title(&dir);
    click(&mut game);
    click(&mut game);
    assert!(
        menu_lines(&game) > 0 || game.engine.descriptors().iter().any(|d| d.active),
        "the intro page draws something"
    );
    drew_something("intro_text", &game.render());
}

/// The conversation's answer menu, over the scene it is held in.
///
/// This is the densest picture the game makes on an ordinary path: a location
/// behind, a figure in it, and four lines of proportional text on top. Getting
/// here at all exercises startup, the location loader, the task machine and the
/// dialogue apparatus in one run.
#[test]
fn the_dialogue_answer_menu() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(2, "_NEXTLOC", 1)
        .expect("new game: the classroom");

    let mut arrived = false;
    for _ in 0..2500 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame in the classroom");
        if menu_lines(&game) >= 3 && !game.engine.in_transition() {
            arrived = true;
            break;
        }
    }
    assert!(arrived, "the conversation never put its answers on screen");
    drew_something("dialogue_menu", &game.render());
}

/// The help viewer: a page of documentation over the shell.
///
/// Opened through `DO_INVSEL`'s case 1005 rather than by setting `_INVMODE` by
/// hand, because that case is also what activates `_ANL1` and `_ANL2` — the two
/// halves of image 60 that *are* the page. Set the mode directly and the pages
/// have no paper. This is the one scene where the pointer is not at the origin:
/// it sits on the status bar at (300, 440), which is where it had to be for the
/// bar to answer at all, and `render` draws the cursor there.
#[test]
fn the_help_viewer() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = settled_in_the_title(&dir);
    game.set_var(2, "_MENCZW", 1005)
        .expect("the documents button");
    game.call(4, "DO_INVSEL", &[]).expect("4:DO_INVSEL");
    assert_eq!(game.get_var(2, "_INVMODE"), Some(5), "the viewer is open");
    settled(&mut game);

    drew_something("help_viewer", &game.render());
}

/// The opening, frame by frame, on the game's own path.
///
/// The four scenes above are still pictures, each reached by the shortest
/// route to it. This is the other half: `START` from the top, a click every so
/// often, and every frame of what comes out folded into one digest. A still
/// says the composition is right; a fold says the *sequence* is — that no fade
/// runs a band shorter, no descriptor appears a frame late, nothing that
/// stands still moves.
///
/// Nothing is asserted about the pictures themselves. What they contain is
/// what the four tests above are for; what this adds is that four hundred
/// frames of it are the four hundred frames they were.
#[test]
fn the_opening_plays_the_same_way_twice() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = Game::open(&dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}

    // A click every 120 frames, which is what carries the intro forward: its
    // phases wait for input and nothing else. The interval is arbitrary and
    // has to stay fixed — it is part of what the digest is over.
    let mut opening = digest::Digest::new();
    for frame in 1..=400 {
        let click = frame % 120 == 0;
        game.set_input(0, 0, click, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("the opening stopped at frame {frame}: {e}"));
        opening.number(digest::frame(&game.render()));
    }
    digests().check("opening", opening.value());
}

/// A seed reaches the picture: two runs of the same scene are two runs.
///
/// The whole point of seeding from outside, and the thing the digests above
/// cannot say, because they exist by *not* being seeded. Nothing in either
/// intro draws a random number — the title's task manager, module 223, calls
/// `RANDOM` not once — so the scene has to be a location, and the classroom
/// is where a new game begins. Module 201, its task manager, calls `RANDOM`
/// twenty-two times, and the picture parts company with itself somewhere
/// between the three hundredth frame and the four hundredth.
///
/// Both directions are asserted together on purpose. Different seeds giving
/// different runs is what a player gets; the same seed giving the same run is
/// what every other test in this file rests on, and a change that broke either
/// would look like a fix for the other.
#[test]
fn a_seed_changes_the_run_and_the_same_seed_does_not() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let classroom = |seed: u64| {
        let mut game = Game::open(&dir).expect("game opens");
        motionvm_motion_forth::Machine::seed(&mut game.vm, seed);
        game.start().expect("4:START");
        while game.pump().expect("startup runs") {}
        game.set_var(2, "_NEXTLOC", 1).expect("the classroom");
        let mut d = digest::Digest::new();
        for frame in 1..=400 {
            game.set_input(0, 0, false, false, 0).expect("input");
            game.step()
                .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
            d.number(digest::frame(&game.render()));
        }
        d.value()
    };
    assert_eq!(classroom(4711), classroom(4711), "one seed, one run");
    assert_ne!(
        classroom(4711),
        classroom(4712),
        "two seeds drew the same four hundred frames — the seed is not reaching \
         RANDOM, or nothing on this path draws one"
    );
}
