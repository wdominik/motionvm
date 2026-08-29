//! Four scenes the engine has to be able to reach, and draw something in.
//!
//! Each test drives the engine to a particular standing picture — the title,
//! a page of intro text, a conversation's answer menu, a page of the help
//! viewer — and asserts it arrived. Reaching them is most of the work: the
//! dialogue menu needs startup, a location entry, the task machine and the
//! conversation apparatus to all behave, and the help viewer has to be opened
//! the way the shell opens it. A panic anywhere on those paths fails here.
//!
//! **These are deliberately not golden-frame tests.** Holding the composed
//! picture against a checked-in PNG of itself is the only kind of check that
//! notices a change nobody thought to write an assertion for — and exactly
//! that baseline cannot be distributed: it is a rendering of the game's own
//! artwork — one of the four was the publisher's logo, another a hand-drawn
//! room with its characters and dialogue in it — and the game's data may not
//! enter this repository in any form, including a fixture derived from it. The
//! check was worth a lot and it is gone; shipping the art to keep it was not an
//! option. What is left is the navigation, plus the weakest honest statement
//! about the picture: it is the right size, and it is not blank.
//!
//! Anyone with the game can still get the strong check back locally by
//! rendering these scenes to PNG and diffing them across a change — nothing
//! here prevents that, it just cannot live in the repository.
//!
//! **Determinism**, which those local comparisons rest on: `RANDOM` is an LCG
//! seeded to a constant in `Vm::new` and never reseeded, there is no `HashMap`
//! on any drawing path, the engine reads no clock, and the resource directory is
//! walked in sorted order — so a scene reached the same way twice composes the
//! same bytes twice. The one thing that is *not* insensitive is the number of
//! `RANDOM` calls made before the frame: the generator is shared and consumed in
//! call order, so a change elsewhere that draws one more random number moves
//! every later one.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_engine::Game;
use motionvm_forth::m32::Vm;
use motionvm_render::Framebuffer;
use motionvm_testutil::gamedata_ds2;
use std::path::Path;

// ------------------------------------------------------------------ the check

/// What can still be said about a composed frame without a baseline to hold it
/// against: it is the size the engine promises, and something was drawn.
///
/// "Not blank" is deliberately the weakest useful form — more than one palette
/// index present. It does not know what the picture should look like, but it
/// does catch the failure that matters most here, a scene that reaches its
/// state and then renders a flat field because a descriptor chain, a palette or
/// a blit stopped working.
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
        frame.width as usize * frame.height as usize,
        "{name}: the buffer does not match its own dimensions"
    );
    let first = frame.pixels[0];
    assert!(
        frame.pixels.iter().any(|&p| p != first),
        "{name}: every pixel is index {first} — the scene was reached but nothing was drawn"
    );
}

// ----------------------------------------------------------------- the scenes

/// Runs frames until nothing is fading, so a still picture can be measured.
///
/// A state predicate rather than a frame count on purpose: a step is one band
/// while a curtain runs, so a fixed number of them would spend most of itself
/// inside the fade. The rest of this suite settles the same way.
fn settle(game: &mut Game<Vm>) {
    let mut guard = 0;
    while game.engine.in_transition() {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a curtain frame");
        guard += 1;
        assert!(guard < 500, "a transition never ended");
    }
    // And one settled frame after it. The drawer runs once a frame out of the
    // game loop and nowhere else, so a run that stops the moment the curtain
    // does has nothing on the screen but what `FADEIN` drew.
    game.set_input(0, 0, false, false, 0).expect("input");
    game.step().expect("a settled frame");
}

fn click(game: &mut Game<Vm>) {
    game.set_input(0, 0, true, false, 0).expect("click");
    game.step().expect("the frame with the click");
    game.set_input(0, 0, false, false, 0).expect("input");
    game.step().expect("the frame after it");
    settle(game);
}

fn title(dir: &Path) -> Game<Vm> {
    let mut game = Game::open(dir).expect("game opens");
    game.startup_only().expect("startup");
    game.enter_location(23).expect("title macro");
    settle(&mut game);
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
    settle(&mut game);

    drew_something("help_viewer", &game.render());
}
