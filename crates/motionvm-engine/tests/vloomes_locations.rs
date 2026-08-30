//! Victor Loomes past its intro, in the room the game starts in.
//!
//! These need the game's files — `MOTIONVM_GAMEDATA_VLOOMES` — and skip
//! without them.
//!
//! The location scheme here is not the later games'. There is no module 601
//! and no `NEXTLOC`: the pending location is `NAO` in module 605, `CTRL` ends
//! every frame on `NAO @ IF NAO @ INCLORT NAO 0! THEN`, and `INCLORT` loads
//! **two** modules for a location rather than three — `N+100` for its script
//! and `N+20` for its macros. Location 3 has neither of its own and shares
//! location 2's pair, which is why modules 23 and 103 do not exist.
//!
//! The game this file drives is Victor Loomes (MOTION 16-bit).

use motionvm_engine::{Game, Playable, titles};
use motionvm_forth::m16::Vm;
use motionvm_testutil::gamedata_vloomes;

/// Runs `RUN` until it has left the intro and entered its first location.
///
/// The intro waits for a click, so this gives it one every so often until
/// `AO` says a location has been entered. `RUN` enters location 1 itself
/// (`1 INCLORT`, the cell before `ANIMPLAY`), so which location that is, is
/// not this harness's choice.
fn into_the_game() -> Option<Game<Vm>> {
    let dir = gamedata_vloomes()?;
    let mut game = titles::vloomes::open(&dir).expect("the game opens");
    game.start().expect("RUN starts");
    for i in 0..4000 {
        let click = i % 150 >= 100 && i % 150 < 102;
        game.set_input(160, 100, click, false, 0).expect("input");
        game.pump().expect("pump");
        game.step().expect("step");
        if game.get_var(605, "AO").unwrap_or(0) > 0 {
            // Let the room settle: the location's own init runs over the
            // frames after `INCLORT`, not inside it.
            for _ in 0..600 {
                game.set_input(160, 100, false, false, 0).expect("input");
                game.pump().expect("pump");
                game.step().expect("step");
            }
            return Some(game);
        }
    }
    panic!("the intro never reached a location");
}

#[test]
fn run_enters_location_1_and_the_room_draws() {
    let Some(mut game) = into_the_game() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    assert_eq!(game.get_var(605, "AO"), Some(1), "RUN's own first location");
    assert_eq!(game.start_location(), Some(1));
    assert!(!game.finished(), "the game is playing, not torn down");

    // The scene, the captions and the inventory bar: a room that drew only
    // its inventory would still light a few hundred pixels, so the bar is
    // asked for by size rather than the total by presence.
    let lit = game.render().pixels.iter().filter(|&&p| p != 0).count();
    assert!(lit > 20_000, "the room drew {lit} pixels");
}

#[test]
fn the_location_is_two_modules_and_the_scene_screen_runs_ctrl() {
    let Some(game) = into_the_game() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    let resident = game.engine.resident();
    // Location 1: script module 101, macro module 21.
    assert!(resident.contains(&101), "the location's script module");
    assert!(resident.contains(&21), "the location's macro module");
    // The library, less the 601 the later games have and plus the 608 they
    // do not: `RUN` fetches 600 and 602 through 609.
    for m in [600, 602, 603, 604, 605, 606, 607, 608, 609] {
        assert!(resident.contains(&m), "library module {m}");
    }
    assert!(!resident.contains(&601), "there is no module 601");
    assert!(!resident.contains(&610), "the intro was dropped");

    // Three screens now, and the one the game is looking at is the scene —
    // 960 by 280 with a 320 by 140 window onto it — running `CTRL`, word 432.
    let screens = game.engine.screens();
    assert_eq!(screens.len(), 3);
    let scene = screens
        .iter()
        .find(|s| s.size == (960, 280))
        .expect("the scene screen");
    assert_eq!(scene.controller, 432, "the scene screen runs CTRL");
    assert!(scene.active);
    // And one screen runs nothing at all, which is the case that makes a
    // single engine-wide controller wrong.
    assert!(screens.iter().any(|s| s.controller < 0));
}

#[test]
fn clicking_around_the_room_asks_what_is_under_the_pointer() {
    let Some(mut game) = into_the_game() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    // Every click runs `?INSIDE` — the word that asks whether the pointer is
    // in one hot area, which only this game calls and which stopped it dead
    // the first time a player clicked anything. Ten points across the room,
    // each clicked once with frames in between for the game to act on it.
    let spots = [
        (60, 60),
        (160, 60),
        (250, 60),
        (60, 110),
        (200, 110),
        (280, 110),
        (100, 150),
        (40, 175),
        (160, 175),
        (300, 90),
    ];
    for (x, y) in spots {
        hold(&mut game, x, y, 400, 200);
    }
    assert!(!game.finished(), "the game is still playing");
    assert_eq!(game.get_var(605, "AO"), Some(1), "still in the room");
    let lit = game.render().pixels.iter().filter(|&&p| p != 0).count();
    assert!(lit > 20_000, "the room is still drawn: {lit} pixels");
}

#[test]
fn the_panel_comes_down_when_the_pointer_goes_up() {
    let Some(mut game) = into_the_game() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    // The panel is a screen of its own — 320 by 25, the strip along the top —
    // and it is hidden while the game plays. What brings it back is its own
    // controller, `PANCTRL` (module 602, id 1852):
    //
    //   _PANON @ 0 =IF MOUSEY 25 <  IF _PANON 1! 1 50 8 FADEIN  THEN
    //   ELSE           MOUSEY 25 >= IF _PANON 0! 1 50 8 FADEOUT THEN
    //
    // That word runs on a screen the game has switched off, which is the case
    // that says a frame is every screen's controller and not the current
    // one's: `ANIMPLAY` walks its screen slots and runs each one's word
    // (`0104:55ec`), and the activity test earlier in that loop skips only
    // the descriptor work.
    let panel = |g: &Game<Vm>| g.get_var(602, "_PANON");
    let settle = |g: &mut Game<Vm>, y: i32| {
        for _ in 0..400 {
            g.set_input(160, y, false, false, 0).expect("input");
            g.pump().expect("pump");
            g.step().expect("step");
        }
    };

    assert_eq!(panel(&game), Some(0), "the panel starts hidden");
    settle(&mut game, 8);
    assert_eq!(panel(&game), Some(1), "the pointer above 25 brings it down");
    let strip = game
        .engine
        .screens()
        .iter()
        .find(|s| s.size == (320, 25))
        .expect("the panel's screen");
    assert!(strip.active, "and the screen it is drawn on is switched on");
    assert_eq!(strip.controller, 1852, "PANCTRL runs it");

    settle(&mut game, 120);
    assert_eq!(panel(&game), Some(0), "and it goes away again");
}

/// Frames at a point, with the click on exactly one of them.
///
/// One frame, because that is what a player's click is worth: the window
/// clears the flag as soon as it has handed it over, so a click the game does
/// not act on in that step is gone. A test that held the button down would
/// pass while the game was unplayable.
fn hold(game: &mut Game<Vm>, x: i32, y: i32, frames: i32, click_at: i32) {
    for f in 0..frames {
        game.set_input(x, y, f == click_at, false, 0)
            .expect("input");
        game.pump().expect("pump");
        game.step()
            .unwrap_or_else(|e| panic!("at {x},{y} frame {f}: {e}"));
    }
}

#[test]
fn the_menu_saves_through_its_own_box() {
    let Some(mut game) = into_the_game() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    let saves = motionvm_testutil::saves_dir("vloomes-menu");
    game.set_saves(&saves).expect("a save directory");

    // The panel down, then the upper half of the strip on the right, which
    // `CTRL` turns into its own key code 317 (`MOUSEX 268 >= … MOUSEY 11 <`).
    hold(&mut game, 160, 8, 300, -1);
    hold(&mut game, 290, 5, 400, 100);

    // `REQUEST` is up and the machine is standing on the word: the box asks
    // `Spielstand sichern:` over five buttons, one per slot.
    assert!(game.engine.has_request(), "the box is up");

    // Button A. It starts five in from the box's left edge and is
    // `(240 - 10 - 4 * 4) / 5` wide, on the row `h - 19` to `h - 5` — the box
    // itself is at 40,65 and 240 by 70, all of it from `CTRL`'s own call.
    hold(&mut game, 60, 122, 600, 100);
    assert!(!game.engine.has_request(), "the box is answered and gone");

    // `CTRL` takes an answer between 1 and 5 and writes the three files:
    // `DUP 700 + … PUT` for the block, and the bare slot for the other two.
    let mut written: Vec<String> = std::fs::read_dir(&saves)
        .expect("the save directory")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    written.sort();
    assert_eq!(written, ["001.FRZ", "001.anm", "701.blk"]);
    assert!(!game.finished(), "the game plays on");
}

#[test]
fn the_request_box_spaces_its_glyphs_the_way_the_drawer_does() {
    let Some(mut game) = into_the_game() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    game.set_saves(&motionvm_testutil::saves_dir("vloomes-menu-gap"))
        .expect("a save directory");
    hold(&mut game, 160, 8, 300, -1);
    hold(&mut game, 290, 5, 400, 100);
    assert!(game.engine.has_request(), "the box is up");

    // The box is at 40,65 and 240 by 70, and its message — text table 6's
    // *Spielstand sichern:*, nineteen characters — sits five rows down
    // (`0104:7407`).
    //
    // Neither the box drawer nor the run drawer under it carries a spacing of
    // its own: the run drawer measures with the glyph gap at `ds:0x13c4` and
    // advances by it per glyph (`0d06:12d1`, `0d06:11fb`), and that word rests
    // at **1** in this build's data segment. Nineteen glyphs therefore carry
    // eighteen gaps, and the ink is eighteen pixels wider than it would be
    // with none — 98 against 80, which is what running the glyphs together
    // looks like.
    let fb = game.render();
    let at = |x: i32, y: i32| fb.pixels[(y * fb.width as i32 + x) as usize];
    // The box is filled in `SYSBC` before anything is drawn on it, so the
    // ink is whatever differs from that — sampled from a row the message and
    // the buttons both leave alone.
    let paper = at(45, 100);
    let ink: Vec<i32> = (41..279)
        .filter(|&x| (68..86).any(|y| at(x, y) != paper))
        .collect();
    let (&first, &last) = (ink.first().expect("the message drew"), ink.last().unwrap());
    assert_eq!(last - first + 1, 98, "the message's ink, gap for gap");

    // And it is centered the drawer's way — `x − width/2` about the box's
    // middle (`0d06:1142`), not `(w − width)/2` from its left edge.
    assert_eq!((first + last) / 2, 40 + 240 / 2 - 1, "centered on the box");
}

#[test]
fn the_menu_offers_the_slot_that_was_saved() {
    let Some(mut game) = into_the_game() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    let saves = motionvm_testutil::saves_dir("vloomes-menu-load");
    game.set_saves(&saves).expect("a save directory");
    hold(&mut game, 160, 8, 300, -1);
    hold(&mut game, 290, 5, 400, 100);
    hold(&mut game, 60, 122, 600, 100);

    // The lower half of the same strip is key code 318, the load page. It
    // probes the slots with `706 701 DO I =>EXIST LOOP` and offers what it
    // finds — one button, because one slot was written.
    hold(&mut game, 160, 8, 300, -1);
    hold(&mut game, 290, 16, 400, 100);
    assert!(game.engine.has_request(), "the load box is up");
}

#[test]
fn every_location_the_game_has_is_reached_and_drawn() {
    let Some(mut game) = into_the_game() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    // Thirteen, and location 3 shares location 2's pair of modules. Location
    // 13 is the only one that asks for a cycling palette (`SETCYCLE`, in
    // modules 33 and 113 and nowhere else), so it is the only one that would
    // stop on a word the other twelve never reach.
    for loc in 1..=13 {
        game.request_location(loc).expect("ask for a location");
        for _ in 0..900 {
            game.set_input(160, 100, false, false, 0).expect("input");
            game.pump().expect("pump");
            game.step()
                .unwrap_or_else(|e| panic!("location {loc}: {e}"));
        }
        assert_eq!(game.get_var(605, "AO"), Some(loc), "arrived in {loc}");
        let lit = game.render().pixels.iter().filter(|&&p| p != 0).count();
        assert!(lit > 20_000, "location {loc} drew {lit} pixels");
    }
}

#[test]
fn a_request_moves_the_game_to_another_location() {
    let Some(mut game) = into_the_game() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    // `CTRL` polls `NAO` every frame, so a location is asked for by writing
    // it there — the same mechanism the scripts use for their exits.
    game.request_location(4).expect("ask for location 4");
    for _ in 0..900 {
        game.set_input(160, 100, false, false, 0).expect("input");
        game.pump().expect("pump");
        game.step().expect("step");
    }
    assert_eq!(game.get_var(605, "AO"), Some(4), "CTRL acted on NAO");
    assert_eq!(game.get_var(605, "NAO"), Some(0), "and cleared it");
    let resident = game.engine.resident();
    assert!(resident.contains(&104), "location 4's script module");
    assert!(resident.contains(&24), "location 4's macro module");
    assert!(!resident.contains(&101), "location 1's was dropped");
}
