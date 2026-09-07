//! Die Enviro-Kids greifen ein, past the intro: `CTRL`'s frames in the
//! locations.
//!
//! `RUN` plays the intro, enters location 1 and parks in the game's
//! `ANIMPLAY`; from there every frame is `CTRL`'s — the order machine, the
//! walk, the inventory bar, the hover caption — over a location's modules
//! and blocks. These tests drive that far and on, and need the game's files
//! (`MOTIONVM_GAMEDATA_ENVIRO`); they skip without them.
//!
//! The game this file drives is Die Enviro-Kids greifen ein (MOTION 16-bit).

mod common;

use motionvm_motion_engine::{Game, titles};
use motionvm_motion_forth::cell;
use motionvm_motion_forth::m16::Vm;
use motionvm_motion_testutil::{Digests, digest, gamedata_enviro};
use std::path::Path;

/// Words `CTRL`'s frames may walk past without effect: the sprite and text
/// status tables, which the 16-bit handlers keep for their own loader
/// (`ENVIRO.EXE` file `0xb0e2`–`0xb2cf`) and which a lazy loader has no
/// use for.
const INERT: &[&str] = &["TXTSTAT", "XGFXSTAT", "XGFXSTAT+"];

/// `RUN` up to the first frame of the game's own loop: the intro clicked
/// through, location 1 entered, `CTRL` installed. Answers the game and the
/// number of frames the intro took.
fn into_the_game(dir: &Path) -> (Game<Vm>, usize) {
    let mut game = titles::enviro::open(dir).expect("opens");
    game.start().expect("RUN starts");
    while game.pump().expect("RUN runs to the intro's ANIMPLAY") {}
    assert!(game.engine.main_loop(), "the intro's ANIMPLAY was entered");
    // The intro ends on its own or on a click; a click every 200 frames
    // gets through it either way. `RUN` then runs on — `UPLOAD_GAME`,
    // `INCLLOC 1`, `400 SCRCTRL` — into the second `ANIMPLAY`.
    let mut left_intro = false;
    let mut entered = 0;
    for frame in 1..=6000 {
        // A key, not a click: `ICTRL` takes either (`MOUSELK MOUSERK OR
        // ?KEY 0 = NOT OR`), and a click still latched when `CTRL` takes
        // over would land on whatever hotspot sits under the pointer.
        let key = if frame % 200 == 0 { 32 } else { 0 };
        game.set_input(160, 100, false, false, key).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
        if !game.engine.main_loop() {
            left_intro = true;
        } else if left_intro {
            entered = frame;
            break;
        }
    }
    assert!(
        entered > 0,
        "6000 frames and the intro never gave way to CTRL"
    );
    (game, entered)
}

/// Past the scrapyard arrival: location 1's first-visit scene hides the
/// walker (`_WALKER SMDESC SDINACTIVE`, macro 301), plays itself out and
/// ends with `… FINISH_LT SETNOBUSY _WALKER SMDESC SDACTIVE 11 NEXTLOC !`
/// (module 101) — the game moves on to location 11 on its own. Tests
/// that hop somewhere else start from here, because hopping earlier
/// carries a state no played game has: an inactive walker, whose
/// `WALKJEFF` callback (`1643 SDWORD`) the frame loop rightly skips
/// (`016a:05d6` gates on the active bit), and nobody would ever walk.
fn settled_in_the_game(dir: &Path) -> Game<Vm> {
    let (mut game, _) = into_the_game(dir);
    let ms = cell::unsigned(game.get_var(601, "_MS").expect("_MS"));
    let walker = cell::unsigned(game.get_var(601, "_WALKER").expect("_WALKER"));
    for frame in 1..=3000 {
        let active = game
            .engine
            .descriptors()
            .iter()
            .find(|d| d.screen == ms && d.handle == walker)
            .is_some_and(|d| d.active);
        if active
            && game.get_var(601, "ACTLOC") == Some(11)
            && game.get_var(601, "NEXTLOC") == Some(-1)
        {
            return game;
        }
        // No clicks: the scenes run themselves out.
        game.set_input(2, 2, false, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("arrival frame {frame} stopped: {e}"));
    }
    panic!("the arrival never settled in location 11");
}

/// This game's table of reference digests.
fn digests() -> Digests {
    common::digests("enviro")
}

#[test]
fn run_plays_through_the_intro_into_location_1_where_ctrl_takes_the_frames() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    let (mut game, frames) = into_the_game(&dir);
    assert!(frames > 1000, "the intro was over after {frames} frames");
    assert!(
        game.engine.pointer_visible(),
        "RUN's SHOWMOUSE after the intro puts the pointer up"
    );
    // `INCLLOC 1` loaded the location's two resident modules and ran the
    // macro module's `LOCINIT`, which it erases again.
    assert!(game.vm.mem.is_loaded(101) && game.vm.mem.is_loaded(501));
    assert!(
        !game.vm.mem.is_loaded(301),
        "the macro module is erased after LOCINIT"
    );
    assert_eq!(game.get_var(601, "ACTLOC"), Some(1));
    assert_eq!(game.get_var(601, "NEXTLOC"), Some(-1));
    assert!(
        !game.finished(),
        "the game is not over when its loop begins"
    );

    // `CTRL` then runs frame after frame — `MOUSEINFO`, `DOORDER`, the
    // location's own task — with nothing the engine lacks, and draws the
    // scrapyard.
    for frame in 1..=300 {
        game.set_input(160, 100, false, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("CTRL frame {frame} stopped: {e}"));
        assert!(
            game.engine.main_loop(),
            "the game loop ended at frame {frame}"
        );
    }
    assert!(common::lit(&mut game) > 0, "location 1 draws a picture");
    // The scrapyard as it stands after three hundred settled frames. Every
    // step to it is the game's own — `RUN` through the intro, `INCLLOC 1`,
    // then `CTRL` — so the picture is the same on any machine with the game.
    digests().check("location_1", digest::frame(&game.render()));
    let stubbed = game.engine.stubbed();
    assert!(
        stubbed.keys().all(|w| INERT.contains(&w.as_str())),
        "words walked past without effect: {stubbed:?}"
    );
}

#[test]
fn a_requested_location_is_entered_through_nextloc_on_the_next_frame() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    let mut game = settled_in_the_game(&dir);
    game.request_location(2)
        .expect("NEXTLOC is a variable of module 601");
    assert_eq!(game.start_location(), Some(2));
    // `CTRL`'s next frame runs `NEXTLOC @ INCLLOC`: the treehouse's
    // modules go, location 2's come — and the fade it starts holds the
    // frames until it is over.
    for frame in 1..=120 {
        game.set_input(300, 120, false, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
    }
    assert_eq!(game.get_var(601, "ACTLOC"), Some(2));
    assert_eq!(game.get_var(601, "NEXTLOC"), Some(-1));
    assert!(game.vm.mem.is_loaded(102) && game.vm.mem.is_loaded(502));
    assert!(!game.vm.mem.is_loaded(101) && !game.vm.mem.is_loaded(501));
    assert!(common::lit(&mut game) > 0, "location 2 draws a picture");
    // Descriptors are numbered per screen on this machine, and `INCLLOC`'s
    // `?LPD 1 + KILLNDESC` took location 1's off the main screen before
    // location 2's came: the main screen's numbers run from 0 without a
    // gap, and the last permanent one — `_LPD`, the walker — is still there.
    let main = cell::unsigned(game.get_var(601, "_MS").expect("_MS"));
    let lpd = cell::unsigned(game.get_var(601, "_LPD").expect("_LPD"));
    let mut numbers: Vec<u32> = game
        .engine
        .descriptors()
        .iter()
        .filter(|d| d.screen == main)
        .map(|d| d.handle)
        .collect();
    numbers.sort_unstable();
    assert_eq!(
        numbers,
        (0..cell::narrow(numbers.len())).collect::<Vec<_>>()
    );
    assert!(
        numbers.contains(&lpd),
        "the walker, descriptor {lpd}, survives the change"
    );
}

/// The walker wears the route's scale on both axes.
///
/// The routes carry a per-mille scale ramp (`_XROUTE`, shrink₀/shrink₁),
/// and `SD%SHR` writes the horizontal and the vertical factor both — the
/// 32-bit handler stores the pair (`0x721b8`), the 16-bit one calls
/// `SDH%SHR` and `SDV%SHR` with the one value (`0xb38b`). The walk asserts
/// the interpolated scale every step the same way. Writing only the
/// combined field left a per-axis value the room's script had set
/// standing, and the figure walked the town at the wrong size — town
/// routes shrink to 160–520 per mille where the living room stays near
/// full. Location 8 places the walker at 480, the treehouse at 630; the
/// three fields agreeing is the fix's pin.
#[test]
fn the_walker_wears_the_routes_scale_on_both_axes() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    let (mut game, _) = into_the_game(&dir);
    for _ in 0..600 {
        game.set_input(10, 10, false, false, 0).expect("input");
        game.step().expect("a frame under CTRL");
    }
    let main = cell::unsigned(game.get_var(601, "_MS").expect("_MS"));
    let lpd = cell::unsigned(game.get_var(601, "_LPD").expect("_LPD"));
    for (loc, scale) in [(8, 480), (12, 630)] {
        game.request_location(loc).expect("request");
        for _ in 0..250 {
            game.set_input(10, 10, false, false, 0).expect("input");
            game.step().expect("a frame under CTRL");
        }
        let d = game
            .engine
            .descriptors()
            .iter()
            .find(|d| d.screen == main && d.handle == lpd)
            .expect("the walker");
        for key in [
            motionvm_motion_engine::Field::SD_PCT_SHR,
            motionvm_motion_engine::Field::SDH_PCT_SHR,
            motionvm_motion_engine::Field::SDV_PCT_SHR,
        ] {
            assert_eq!(
                d.fields.get(key),
                Some(scale),
                "location {loc}: {} is not the route's {scale}",
                key.name()
            );
        }
    }
}

/// The mode cell of the `_ORDER` block — the interaction machine's state:
/// 0 idle, 12 to 18 a conversation, 97 to 99 a forced order.
fn order_mode(game: &Game<Vm>) -> i32 {
    let at = game
        .address(601, "_ORDER")
        .expect("_ORDER is a variable of module 601");
    // The variable's cells follow its `_PutAdr`; the mode is the fourth.
    let flat = game.vm.mem.flat(at).expect("loaded") + 2 + 6;
    cell::sign16(game.vm.mem.fetch(flat))
}

/// The first locations the game can reach, each entered the way the scripts
/// enter them, each running a hundred of `CTRL`'s frames with the pointer
/// in the scene.
#[test]
fn the_early_locations_load_and_run_under_ctrl() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    for loc in [3, 11] {
        let mut game = settled_in_the_game(&dir);
        game.request_location(loc).expect("NEXTLOC");
        for frame in 1..=200 {
            game.set_input(300, 120, false, false, 0).expect("input");
            game.step()
                .unwrap_or_else(|e| panic!("location {loc}, frame {frame}: {e}"));
        }
        assert_eq!(game.get_var(601, "ACTLOC"), Some(loc), "location {loc}");
        assert!(
            game.vm.mem.is_loaded(cell::low16(100 + loc))
                && game.vm.mem.is_loaded(cell::low16(500 + loc)),
            "location {loc}'s modules are resident"
        );
        assert!(common::lit(&mut game) > 0, "location {loc} draws a picture");
    }
}

/// The verb strip follows the pointer into a scrolled view.
///
/// `GSCRX` has one writer per generation: the 32-bit `SCRX` fills
/// `origin` (always zero in that game), the 16-bit `SCRX` the scroll at
/// scr+0 (`Screen::pos`), which the wide locations move. The strip's
/// clamp (`0d34:2517`: `[sx+28h, sx+w-28h]` with `sx = GSCRX`),
/// `MOUSEINFO` and the 16-bit conversation all ask the native
/// `screen_origin_x`; answering `origin` alone dropped the scroll term
/// and the strip drew shifted — off the view at the shopping center's
/// 304. The fix answers `origin + pos`, the sum the damage map already
/// bases on.
///
/// Location 7 scrolls without a story task in the way: `MYCALCMOVE`
/// (module 107) slides to 304 as soon as the walker crosses x 400.
#[test]
fn the_verb_strip_follows_the_pointer_into_a_scrolled_view() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    let mut game = settled_in_the_game(&dir);
    game.request_location(7).expect("NEXTLOC");
    for frame in 1..=200 {
        game.set_input(300, 120, false, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
    }
    assert_eq!(game.get_var(601, "ACTLOC"), Some(7));
    let main = cell::unsigned(game.get_var(601, "_MS").expect("_MS"));
    let pos_x = |game: &Game<Vm>| -> i32 {
        i32::from(
            game.engine
                .screens()
                .iter()
                .find(|s| s.handle == main)
                .expect("the main screen")
                .pos
                .0,
        )
    };

    // Walk east until module 107 slides the view to 304.
    let mut scroll = pos_x(&game);
    for frame in 1..=3000 {
        let click = frame % 25 == 0;
        game.set_input(310, 130, click, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("walking east, frame {frame}: {e}"));
        scroll = pos_x(&game);
        if scroll == 304 {
            break;
        }
    }
    assert_eq!(scroll, 304, "the walk east scrolls the shopping street");
    // Let the pointer come to rest mid-view before the menu opens.
    for _ in 0..10 {
        game.set_input(160, 80, false, false, 0).expect("input");
        game.step().expect("rest frame");
    }

    // The menu opens on a right-click over something with verbs: probe a
    // grid of view positions until the strip comes up, then hold that
    // pointer position.
    let mut xs: Vec<i32> = Vec::new();
    let mut at = (0, 0);
    'grid: for py in [130, 100, 70, 40] {
        for px in (20..320).step_by(40) {
            for _ in 0..3 {
                game.set_input(px, py, false, false, 0).expect("input");
                game.step().expect("hover frame");
            }
            game.set_input(px, py, false, true, 0).expect("input");
            game.step().expect("right press");
            for _ in 0..4 {
                game.set_input(px, py, false, false, 0).expect("input");
                game.step().expect("menu frame");
            }
            xs = game
                .engine
                .descriptors()
                .iter()
                .filter(|d| d.screen == main && d.active && (101..=105).contains(&d.level))
                .map(|d| d.x)
                .collect();
            if !xs.is_empty() {
                at = (px, py);
                break 'grid;
            }
        }
    }
    xs.sort_unstable();
    assert!(
        !xs.is_empty(),
        "no right-click in the view raised the strip"
    );
    // The clamp `[sx+28h, sx+w-28h]` works in world coordinates with
    // `sx = GSCRX`. Reading the origin without the scroll clamped the
    // anchor to at most world 280 — left of the whole visible window
    // once the street stands at 304 — so the pin is: every icon lies
    // inside the scrolled view, near the click.
    for &x in &xs {
        assert!(
            x >= scroll && x < scroll + 320,
            "icon at world {x} is outside the view at scroll {scroll} (strip {xs:?})"
        );
    }
    let mid = (xs[0] + xs[xs.len() - 1]) / 2;
    assert!(
        (mid - (at.0 + scroll)).abs() <= 40,
        "strip {xs:?} sits near the click at world {} (pointer {at:?})",
        at.0 + scroll
    );
}

/// Location 13 begins with a conversation of its own — the editor and Maik,
/// two talking heads and a menu of answers — which runs through the
/// conversation machine as read from `ENVIRO.EXE`: a line stands (mode 12 or
/// 13, by speaker), the answers come up (14), a click on one speaks it and
/// the talk goes on until it is over (16) and the block is idle again.
#[test]
fn a_conversation_speaks_its_lines_and_takes_an_answer() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    let mut game = settled_in_the_game(&dir);
    game.request_location(13).expect("NEXTLOC");
    let mut seen = Vec::new();
    let mut answered = false;
    for frame in 1..=1500 {
        // The second answer stands about 50 rows down the view; a click on it
        // once the menu is up.
        let click = !answered && seen.last() == Some(&14);
        game.set_input(200, 50, click, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("location 13, frame {frame}: {e}"));
        let mode = order_mode(&game);
        if seen.last() != Some(&mode) {
            seen.push(mode);
        }
        if click {
            answered = true;
            assert!(common::lit(&mut game) > 0, "the answers are drawn");
        }
        if answered && mode == 0 {
            break;
        }
    }
    assert!(seen.contains(&14), "the answer menu came up: {seen:?}");
    assert!(answered, "an answer was clicked: {seen:?}");
    let after: Vec<i32> = seen.iter().copied().skip_while(|&m| m != 14).collect();
    assert!(
        after.iter().any(|&m| m == 12 || m == 13),
        "the chosen answer was spoken: {seen:?}"
    );
    assert_eq!(seen.last(), Some(&0), "the conversation ended: {seen:?}");
    assert!(
        game.engine
            .stubbed()
            .keys()
            .all(|w| w == "SDNORM" || INERT.contains(&w.as_str())),
        "words walked past without effect: {:?}",
        game.engine.stubbed()
    );
}

/// With a save in a slot, `RUN` puts up its start-up page after the first
/// location and waits in a loop of its own — `BEGIN … MOUSELK … UNTIL` —
/// for a click on *Laden* or *Neustart*. The loop polls the pointer, so it
/// turns once a frame here, and the click on *Neustart* lets `RUN` go on
/// into the game.
#[test]
fn the_start_up_page_waits_for_a_click_when_a_save_exists() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    let saves = motionvm_motion_testutil::saves_dir("enviro-start-up-page");
    let mut game = titles::enviro::open(&dir).expect("opens");
    game.set_saves(&saves).expect("saves");
    // Written where the engine decided the slots go, which is the game's own
    // name under the directory it was handed.
    let slots = game.saves().expect("a save directory").to_path_buf();
    std::fs::write(slots.join("701.blk"), b"").expect("a save slot");
    game.start().expect("RUN starts");
    while game.pump().expect("RUN runs to the intro's ANIMPLAY") {}
    // Through the intro, then the page: RUN is busy in its loop and no
    // game loop begins.
    let mut page_frames = 0;
    let mut entered = false;
    for frame in 1..=6000 {
        let key = if frame == 300 { 27 } else { 0 };
        let click = page_frames == 100;
        game.set_input(270, 180, click, false, key).expect("input");
        game.step().unwrap_or_else(|e| panic!("frame {frame}: {e}"));
        if game.is_running() && !game.engine.main_loop() {
            page_frames += 1;
        }
        if page_frames > 100 && game.engine.main_loop() {
            entered = true;
            break;
        }
    }
    let _ = std::fs::remove_dir_all(&saves);
    assert!(
        page_frames > 100,
        "RUN waited on its page for {page_frames} frames"
    );
    assert!(entered, "the click on Neustart let RUN into the game loop");
    assert_eq!(game.get_var(601, "ACTLOC"), Some(1));

    // The page's teardown is `KILL_MENU`: the two full-bar overlays go
    // inactive and `1 50 8 FADEIN` on the bar screen — nothing else. The
    // erase therefore has to come from `FADEIN`'s full compose (`SCRACT`
    // sets the rebuild bit, `05f1:094a`; the compose clears the surface,
    // `016a:092c`). With it, the bar comes back as the game keeps it at
    // start-up: empty slots on black, the menu icon on the right. Without
    // it, the page's sprites 1448/1449 stood in the bar for the rest of
    // the game — the bug this pins.
    for _ in 0..30 {
        game.set_input(160, 100, false, false, 0).expect("input");
        game.step().expect("a frame under CTRL");
    }
    let fb = game.render();
    let w = usize::from(fb.width);
    let lit_left = (165..200)
        .flat_map(|y| (0..280).map(move |x| (x, y)))
        .filter(|&(x, y)| fb.pixels[y * w + x] != 0)
        .count();
    let lit_icon = (165..200)
        .flat_map(|y| (288..320).map(move |x| (x, y)))
        .filter(|&(x, y)| fb.pixels[y * w + x] != 0)
        .count();
    assert!(
        lit_left < 280 * 35 / 10,
        "the bar left of the menu icon still shows the page: {lit_left} lit pixels"
    );
    assert!(lit_icon > 50, "the menu icon is back on the bar");
}

/// Walking out through a door changes the palette only behind the wipe.
///
/// The LEAVE verb's `CALCLEAVE` (module 606) runs `INCLLOC` right inside
/// the order machine's callback — `_ORDER 2 +@ INCLLOC` — where this
/// engine cannot park the interpreter, so `INCLLOC`'s `FADEOUT` is only
/// queued and the next room macro's `XSETPAL` runs with the old picture
/// still standing. The original cannot be caught out this way: its fade
/// spins inside the word (`FADEOUT` `05f1:2827`), so `SETPAL`
/// (`05f1:01ff`, the DAC programmed on the spot) always lands on a
/// blacked view. Applied immediately here, the supermarket stood
/// recolored in the shopping center's palette for the whole closing
/// wipe; queued with the wipe (`Wipe::palette_after`), the switch waits
/// its turn — which is what this pins, on the played route it was seen
/// on: the supermarket's exit is item 0 of its table (block 215,
/// `LD_0ZUMEINK`, rect 0..7 × 0..85, `EXIT` 7), on the room's left edge.
#[test]
fn a_door_changes_the_palette_only_behind_the_wipe() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    let mut game = settled_in_the_game(&dir);
    game.request_location(15).expect("NEXTLOC");
    for frame in 1..=200 {
        game.set_input(2, 2, false, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("supermarket frame {frame} stopped: {e}"));
    }
    assert_eq!(game.get_var(601, "ACTLOC"), Some(15));

    // Click the exit and walk out, watching every frame on the way: when
    // the palette moves, the view must already be black — the closing
    // wipe runs first, in the room's own colors.
    let mut last_pal = game.palette().raw.to_vec();
    let mut switches = 0;
    for frame in 0..=900 {
        let click = frame == 0;
        game.set_input(3, 42, click, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("walk-out frame {frame} stopped: {e}"));
        let pal = game.palette().raw.to_vec();
        if pal != last_pal {
            switches += 1;
            let fb = game.render();
            let w = usize::from(fb.width);
            let lit = fb.pixels[..160 * w].iter().filter(|&&p| p != 0).count();
            assert!(
                lit * 10 < 160 * w,
                "walk-out frame {frame}: the palette moved over a standing \
                 picture ({} of {} view pixels lit)",
                lit,
                160 * w
            );
        }
        last_pal = pal;
        if game.get_var(601, "ACTLOC") == Some(7)
            && game.get_var(601, "NEXTLOC") == Some(-1)
            && frame > 200
        {
            break;
        }
    }
    assert_eq!(game.get_var(601, "ACTLOC"), Some(7), "the door was taken");
    assert!(switches >= 1, "the transition switched the palette");
}

/// `ENDTUNE`'s stop routine (`1696:02fd`) starts the music's fade-out and
/// then spins until the 200 Hz tick has counted 100 before it returns to
/// the script — half a second in which the frame loop does not turn.
/// `INCLLOC` calls it between the closing wipe and the opening one, so on
/// a door walk there is exactly one step that long, and the view is black
/// while it stands.
#[test]
fn ending_the_tune_holds_the_room_change_for_half_a_second() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    // The flag the stop routine tests is set by a song the driver started,
    // so a game with no music has no wait either; a sink that plays nothing
    // stands in for the driver here.
    struct Silent;
    impl motionvm_motion_engine::MusicSink for Silent {
        fn start(&mut self, _handle: i32, _tune: i32, _looping: bool, _song: &[u8]) {}
        fn stop(&mut self, _handle: i32) {}
        fn cut(&mut self, _handle: i32) {}
        fn start_sample(&mut self, _handle: i32, _wav: &[u8], _volume: u16, _loops: i32) {}
        fn stop_sample(&mut self, _handle: i32) {}
        fn music_volume(&mut self, _volume: u16) {}
        fn sample(&mut self, _block: i32, _sample: &[u8]) {}
    }
    let mut game = settled_in_the_game(&dir);
    game.set_music(Box::new(Silent));
    game.request_location(15).expect("NEXTLOC");
    for frame in 1..=200 {
        game.set_input(2, 2, false, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("supermarket frame {frame} stopped: {e}"));
    }
    assert_eq!(game.get_var(601, "ACTLOC"), Some(15));

    let half_second = std::time::Duration::from_millis(500);
    let mut holds = 0;
    for frame in 0..=900 {
        let click = frame == 0;
        game.set_input(3, 42, click, false, 0).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("walk-out frame {frame} stopped: {e}"));
        if game.frame_duration() == Some(half_second) {
            holds += 1;
            let fb = game.render();
            let w = usize::from(fb.width);
            let lit = fb.pixels[..160 * w].iter().filter(|&&p| p != 0).count();
            assert_eq!(
                lit, 0,
                "walk-out frame {frame}: the hold stands on a lit view"
            );
        }
        if game.get_var(601, "ACTLOC") == Some(7)
            && game.get_var(601, "NEXTLOC") == Some(-1)
            && !game.engine.in_transition()
        {
            break;
        }
    }
    assert_eq!(holds, 1, "one half-second step per room change with music");
    assert_eq!(game.get_var(601, "ACTLOC"), Some(7), "the door was walked");
}
