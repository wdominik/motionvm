//! The mailbox wipes its screen a text row at a time.
//!
//! Module 216 changes terminal screens in three phases (for instance at
//! `0x0a0bc`): `HIDSCR` switches all sixteen row descriptors off, `CLSCR` puts
//! a black bar over one row every three frames until `_LASTL`, and `DOFADE`
//! then takes the bars down again one row at a time over the new screen.
//!
//! That only reads as a slow modem redraw because **hiding a descriptor does
//! not erase it**. `SDINACTIVE` (`0x72033` → `0x6ab6e`) marks the descriptor's
//! rectangle on its own level, which repaints what is above it and nothing
//! below, and the drawer never clears a surface (`0x6915b` resets the damage
//! map at `0x69248` and repaints only what that map names). So the rows stay
//! on the screen after `HIDSCR`, and the bars are what takes them away.
//!
//! The pairing is in the data: module 316 builds the sixteen row descriptors
//! with no `SDAUTOBUF` (`0x00f60`) and the forty-three bars with one
//! (`0x01020`) — and bar sprite 4148 is drawn in index 9, which is the exact
//! index the monitor's screen area carries in background sprite 4009. A bar
//! over the background is invisible; a bar over a row is an eraser.
//!
//! The game data this file drives is Dunkle Schatten 2's (MOTION 32-bit).

use motionvm_engine::Game;
use motionvm_forth::m32::Vm;
use motionvm_testutil::gamedata_ds2;

/// The terminal's first text row and how tall one is — `INITFADE` places row
/// *n* at `n * 8 + 59` (module 216, `0x05db0`).
const TOP: i32 = 59;
const ROW: i32 = 8;
/// The bars run from x 65 and are 509 pixels of colour wide, and the rows start
/// one pixel further in — but the measurement below starts past the cursor.
///
/// `CLSCR` parks the terminal's cursor at x 66 on the row it is about to reach
/// (`66 _CCOUNT @ SETCUR`, module 216, `0x054bc`), and it blinks there. Reading
/// a row from x 66 would therefore read the cursor as often as the text.
const LEFT: i32 = 90;
const RIGHT: i32 = 574;

/// Drives the game into the terminal with the player already logged in.
fn terminal(dir: &std::path::Path) -> Game<Vm> {
    let mut game = Game::open(dir).expect("game opens");
    game.start().expect("4:START");
    while game.pump().expect("startup runs") {}
    game.set_var(11, "_COMPMODE", 2).expect("already logged in");
    game.set_var(2, "_NEXTLOC", 16)
        .expect("ask for the mailbox");
    for _ in 0..1500 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame");
        if game.get_var(2, "_ACTLOC") == Some(16) && game.get_var(216, "_EDVMODE") == Some(3) {
            break;
        }
    }
    assert_eq!(
        game.get_var(216, "_EDVMODE"),
        Some(3),
        "the terminal never came up"
    );
    // Its opening writes itself onto the screen row by row; let that finish, or
    // what is measured below is a screen half drawn rather than a whole one.
    for _ in 0..900 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame");
    }
    game
}

/// How many of the terminal's rows, counting from the top, hold nothing.
///
/// Counted as rows rather than as pixels because the terminal's cursor blinks:
/// it is an animation of two sprites (module 316, `0x00480`) and it sits on the
/// row `CLSCR` is about to reach next — `SETCUR` is called with `_CCOUNT` after
/// the count is raised (module 216, `0x054bc`) — so a pixel total wobbles by
/// the cursor's own eight while the wipe walks past it. Whole rows going dark
/// from the top is what the effect *is*, and it does not wobble.
///
/// Index 9 counts as nothing: that is the colour the monitor's screen area
/// carries in background sprite 4009, and the one the bars paint in.
fn dark_rows(game: &mut Game<Vm>, rows: i32) -> i32 {
    let frame = game.render();
    let lit = |y: i32| (LEFT..RIGHT).any(|x| frame.get(x, y).is_some_and(|p| p != 0 && p != 9));
    (0..rows)
        .take_while(|r| !(0..ROW).any(|dy| lit(TOP + r * ROW + dy)))
        .count() as i32
}

/// How many of the first `rows` rows carry anything at all.
fn written_rows(game: &mut Game<Vm>, rows: i32) -> i32 {
    let frame = game.render();
    (0..rows)
        .filter(|r| {
            (0..ROW).any(|dy| {
                (LEFT..RIGHT).any(|x| {
                    frame
                        .get(x, TOP + r * ROW + dy)
                        .is_some_and(|p| p != 0 && p != 9)
                })
            })
        })
        .count() as i32
}

#[test]
fn clscr_takes_the_screen_away_one_row_at_a_time() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no gamedata directory");
        return;
    };
    let mut game = terminal(&dir);
    let rows = game.get_var(216, "_LASTL").expect("_LASTL");
    assert!(
        rows > 1,
        "the terminal has {rows} rows in use, expected several"
    );

    let before = dark_rows(&mut game, rows);
    assert!(
        before < rows,
        "the terminal has text on it to begin with, but all {rows} rows are dark"
    );
    // How many rows actually carry something. The dark count below only moves
    // when one of *these* goes out, because it counts the dark run from the
    // top and the terminal leaves rows empty between its lines.
    let written = written_rows(&mut game, rows);
    assert!(
        written > 1,
        "the terminal shows {written} written row(s), expected several"
    );

    // Task 8 is the way back to the main screen, and it opens with the same
    // `HIDSCR` / `CLSCR` / `DOFADE` sequence every screen change uses. Set by
    // hand rather than clicked through, because what is under test is those
    // three phases and not the route to them.
    for (name, value) in [("_LOCTASK", 8), ("_LOCTASKPHA", 0), ("_LOCTASKWAI", 0)] {
        game.set_var(2, name, value).expect("force the task");
    }

    let mut seen = Vec::new();
    let mut wiped: Option<i32> = None;
    for frame in 0..300 {
        game.set_input(0, 0, false, false, 0).expect("input");
        game.step().expect("a frame");
        let now = dark_rows(&mut game, rows);
        // Phase 0 is `HIDSCR` and then `15 ->LTWAIT`. Every row descriptor is
        // switched off in it and **not one pixel may go out**: that is the
        // whole claim. `CLSCR` cannot have started inside that window.
        if frame < 15 {
            assert_eq!(
                now,
                before,
                "frame {frame}: HIDSCR cleared {} row(s), and it must clear none",
                now - before
            );
        }
        seen.push(now);
        if now == rows {
            wiped = Some(frame);
            break;
        }
    }
    let wiped = wiped.unwrap_or_else(|| panic!("the screen never went dark: {seen:?}"));

    assert!(
        seen.windows(2).all(|w| w[1] >= w[0]),
        "the wipe gave a row back somewhere: {seen:?}"
    );
    // One row every three frames: `CLSCR` sets `_CWAIT` to 2 after each
    // (module 216, `0x054bc`), and the two frames that follow only count it
    // down. So the whole thing cannot be over in fewer, and certainly not in
    // one step.
    let wanted = rows - before;
    assert!(
        wiped >= wanted * 3,
        "the screen went dark after {wiped} frames, too fast for {wanted} rows at three frames each"
    );
    // And it goes out a written row at a time rather than all at once: the
    // dark run from the top grows once per line the screen was showing.
    let steps = seen.windows(2).filter(|w| w[1] > w[0]).count();
    assert_eq!(
        steps as i32, written,
        "the wipe went in {steps} steps and the terminal had {written} written rows: {seen:?}"
    );
}
