//! Die Enviro-Kids greifen ein boots on the 16-bit machine.
//!
//! `RUN` loads the library, plays the intro through `STARTINTRO`, and enters
//! the intro's `ANIMPLAY`; from there every frame is `ICTRL`'s. These tests
//! drive exactly that far and ask for the picture. They need the game's
//! files — `MOTIONVM_GAMEDATA_ENVIRO` — and skip without them.
//!
//! That this directory is told apart and opens as this game is asked in
//! `titles_detected.rs`, one row per game; here the game is already open.
//!
//! The game this file drives is Die Enviro-Kids greifen ein (MOTION 16-bit).

mod common;

use motionvm_motion_engine::titles;
use motionvm_motion_forth::cell;
use motionvm_motion_testutil::{Digests, digest, digest::Digest, gamedata_enviro};

/// This game's table of reference digests.
fn digests() -> Digests {
    common::digests("enviro")
}

#[test]
fn run_reaches_the_intro_loop_and_the_first_frames_draw_a_picture() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    let mut game = titles::enviro::open(&dir).expect("opens");
    game.start().expect("RUN starts");
    // Up to the intro's ANIMPLAY, where the execution parks.
    let mut frames = 0;
    while game.pump().expect("RUN runs to ANIMPLAY") {
        frames += 1;
        assert!(frames < 10_000, "RUN never reached ANIMPLAY");
    }
    assert!(game.engine.main_loop(), "ANIMPLAY was entered");
    assert!(
        game.engine.controller().is_some(),
        "STARTINTRO installed ICTRL with SCRCTRL"
    );
    assert!(
        game.palette().raw.iter().any(|&v| v != 0),
        "STARTINTRO installed palette 22"
    );
    // The intro's screen: 960×544 behind a 320×200 view.
    let screen = game.engine.screens().last().expect("a screen");
    assert_eq!(screen.size, (960, 544));
    assert_eq!(screen.view, (320, 200));
    // Then ICTRL's frames: the intro — the DigiTales logo, the briefing
    // paragraphs about Waldbach and VISALUX, the motifs sliding in — runs
    // for well over a thousand frames without reaching a kernel word the
    // engine lacks, and every frame is a 320×200 picture.
    let mut lit_frames = 0;
    // Every frame of the intro folded into one digest, rather than the last
    // one alone: the intro is an animation, and a still from the end of it
    // would say nothing about the fourteen hundred pictures before it. The
    // frames are rendered here either way, so this costs the fold and nothing
    // else.
    let mut intro = Digest::new();
    for frame in 1..=1500 {
        game.step()
            .unwrap_or_else(|e| panic!("ICTRL stopped at frame {frame}: {e}"));
        assert!(
            game.engine.main_loop(),
            "the intro loop ended at frame {frame}"
        );
        let picture = game.render();
        assert_eq!((picture.width, picture.height), (320, 200));
        intro.number(digest::frame(&picture));
        if picture.pixels.iter().any(|&p| p != 0) {
            lit_frames += 1;
        }
    }
    assert!(
        lit_frames > 1000,
        "only {lit_frames} of 1500 frames drew anything"
    );
    digests().check("intro", intro.value());
    assert!(
        game.engine.stubbed().is_empty(),
        "words walked past without effect: {:?}",
        game.engine.stubbed()
    );
    // The motifs and the text stand on the buffers `SETBUF` allocated and
    // `SDBUF` attached: buffer 1 for the text, 2–6 for the motifs, and more
    // as the intro goes on.
    assert!(
        !game.engine.buffers().is_empty(),
        "the intro allocated buffers"
    );
    assert!(
        game.engine.descriptors().iter().any(|d| d.buffer.is_some()),
        "descriptors draw through buffers"
    );
}

/// The pointer stays off the intro.
///
/// `SHOWMOUSE` and `HIDEMOUSE` move a show counter and are inert until a
/// shape armed the pointer (`14ee:0874`, `14ee:094b`, gate `ds:0x16AA`);
/// the counter starts at zero, so the pointer is invisible until the first
/// `SHOWMOUSE` — which `RUN` calls only after `STARTINTRO` returns. Drawing
/// it anyway put an arrow in the logo scenes, in whatever the intro's
/// palettes made of its colors.
#[test]
fn the_pointer_stays_off_the_intro() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    let mut game = titles::enviro::open(&dir).expect("opens");
    game.start().expect("RUN starts");
    while game.pump().expect("pump") {}
    for frame in 0..400 {
        game.set_input(160, 100, false, false, 0).expect("input");
        game.step().unwrap_or_else(|e| panic!("frame {frame}: {e}"));
        assert!(
            !game.engine.pointer_visible(),
            "frame {frame}: the pointer shows during the intro"
        );
    }
}

/// A scene change leaves nothing of the last picture.
///
/// The intro swaps its motifs under `FADEOUT`/`FADEIN` pairs and erases
/// nothing itself: the old sprite only goes inactive, and the surface is a
/// 960×544 the new motif covers just a corner of. The erase is `FADEIN`'s —
/// `SCRACT` requests the rebuild (`05f1:094a`, bit 0 of the screen flags)
/// and the compose it runs (`016a:0821`) takes the full branch: surface
/// cleared, everything redrawn (`016a:092c`). So when the title lettering
/// stands — red on black, and nothing else in the frame — every lit pixel
/// belongs to the lettering, and the DigiTales motif of the scene before
/// is gone to the last pixel: anything of it would either be colorful
/// (and the scene would never read as the title here) or sit outside the
/// lettering's own bounding box. Publishing the old picture's remains
/// around the new motif is the bug this pins.
#[test]
fn a_scene_change_leaves_nothing_of_the_last_picture() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    let mut game = titles::enviro::open(&dir).expect("opens");
    game.start().expect("RUN starts");
    while game.pump().expect("pump") {}
    let mut seen = 0;
    for frame in 0..1200 {
        game.set_input(160, 100, false, false, 0).expect("input");
        game.step().unwrap_or_else(|e| panic!("frame {frame}: {e}"));
        let fb = game.render();
        let pal = game.palette().clone();
        let w = usize::from(fb.width);
        let (mut red, mut other) = (0usize, 0usize);
        for &p in &fb.pixels {
            if p == 0 {
                continue;
            }
            let [r, g, b] = pal.rgb8(p);
            if r > 150 && g < 80 && b < 80 {
                red += 1;
            } else if g >= 80 || b >= 80 {
                other += 1;
            }
        }
        // The title is red lettering — bright red with darker red shading —
        // on black and nothing else; the logo scenes around it are colorful.
        if red < 2000 || other > 200 {
            continue;
        }
        seen += 1;
        // The lettering's own box, out of the red pixels; everything lit
        // must lie within it (a two-pixel allowance for its dark rim).
        let (mut x0, mut x1, mut y0, mut y1) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for (i, &p) in fb.pixels.iter().enumerate() {
            let [r, g, b] = pal.rgb8(p);
            if r > 150 && g < 80 && b < 80 {
                let (x, y) = (cell::count(i % w), cell::count(i / w));
                x0 = x0.min(x);
                x1 = x1.max(x);
                y0 = y0.min(y);
                y1 = y1.max(y);
            }
        }
        for (i, &p) in fb.pixels.iter().enumerate() {
            if p == 0 {
                continue;
            }
            let (x, y) = (cell::count(i % w), cell::count(i / w));
            assert!(
                (x0 - 2..=x1 + 2).contains(&x) && (y0 - 2..=y1 + 2).contains(&y),
                "frame {frame}: a pixel at ({x}, {y}) outside the title \
                 lettering ({x0}..{x1}, {y0}..{y1}) survives the scene change"
            );
        }
        // No early exit: the scene's opening wipe reveals the view ring by
        // ring, and what the last picture left behind sits at the edges —
        // the frames that matter are the late, fully open ones.
    }
    assert!(
        seen > 20,
        "1200 frames and the title scene never stood open"
    );
}

/// The intro's four-picture grid settles with every motif at full size.
///
/// `ICTRL`'s motif phases select once and then write on — phase 16 does
/// `_MOT2 SMDESC`, and the card-flip phase 17 issues its per-frame
/// `SDH%SHR`/`SDSPR`/`SDCX` with **no re-select**. That leans on the
/// frame loop's callback walk (`016a:05d6`–`06da`) skipping inactive
/// descriptors: `_DINFO` carries `1082 SDWORD` from birth but stays
/// `SDINACTIVE`, so on the original it never becomes current between
/// frames. Ticking its callback anyway made every spin write land on
/// `_DINFO`, and the bottom-left motif kept the first write's 100 per
/// mille — a 16-pixel sliver where a 160-wide picture belongs.
#[test]
fn the_intro_grid_settles_all_four_motifs_at_full_size() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    let mut game = titles::enviro::open(&dir).expect("opens");
    game.start().expect("RUN starts");
    while game.pump().expect("runs") {}
    assert!(game.engine.main_loop(), "the intro's ANIMPLAY was entered");
    let mut phase = -1;
    for frame in 1..=4000 {
        game.set_input(160, 100, false, false, 0).expect("input");
        game.step().unwrap_or_else(|e| panic!("frame {frame}: {e}"));
        phase = game.get_var(610, "_PHASE").unwrap_or(-1);
        if phase >= 44 {
            break;
        }
    }
    assert_eq!(phase, 44, "the grid never settled");
    let mot2 = cell::unsigned(game.get_var(610, "_MOT2").expect("_MOT2"));
    let d = game
        .engine
        .descriptors()
        .iter()
        .find(|d| d.handle == mot2 && d.screen == 1)
        .expect("the bottom-left motif")
        .clone();
    assert_eq!(
        d.fields.get(motionvm_motion_engine::Field::SDH_PCT_SHR),
        Some(1000),
        "the flip ends at full width"
    );
    assert_eq!((d.x, d.y), (2, 99), "settled at the grid cell");
    // The settled grid, measured on the original's capture: content from
    // (2, 20), 318 wide (the bottom-right motif is 1050 per mille — 168
    // pixels — and clips at 320), 156 tall.
    let fb = game.render();
    let mut min = (i32::MAX, i32::MAX);
    let mut max = (i32::MIN, i32::MIN);
    let mut bl_columns = [false; 160];
    for y in 0..200i32 {
        for x in 0..320i32 {
            let p = fb.pixels[usize::try_from(y * 320 + x).unwrap()];
            if p != 0 {
                min = (min.0.min(x), min.1.min(y));
                max = (max.0.max(x), max.1.max(y));
                if (2..162).contains(&x) && (99..176).contains(&y) {
                    bl_columns[usize::try_from(x - 2).unwrap()] = true;
                }
            }
        }
    }
    assert_eq!((min.0, min.1), (2, 20), "grid content origin");
    assert_eq!(
        (max.0 - min.0 + 1, max.1 - min.1 + 1),
        (318, 156),
        "grid content size, as the original's settled frames measure"
    );
    let lit = bl_columns.iter().filter(|&&c| c).count();
    assert!(
        lit >= 150,
        "the bottom-left motif fills its cell ({lit}/160 columns lit)"
    );
}

/// A 16-bit screen holds a hundred descriptors, and the hundred-and-first
/// `NEWSETDESC` (`05f1:0ad4`) pops nothing and pushes nothing: its six
/// arguments stay on the stack and no handle comes back.
#[test]
fn the_hundred_and_first_descriptor_leaves_the_stack_as_it_is() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    let mut game = titles::enviro::open(&dir).expect("opens");
    game.start().expect("RUN starts");
    let make = |game: &mut motionvm_motion_engine::Game<motionvm_motion_forth::m16::Vm>| {
        let depth = game.vm.data.len();
        game.vm.data.extend_from_slice(&[10, 20, 15, 0, 15, -1]);
        assert!(game.kernel_word("NEWSETDESC").expect("NEWSETDESC"));
        game.vm.data.len() - depth
    };
    // One to learn which screen is current, then up to the hundred.
    assert_eq!(make(&mut game), 1, "a handle comes back");
    let screen = game
        .engine
        .descriptors()
        .last()
        .expect("the descriptor")
        .screen;
    let on_screen = |game: &motionvm_motion_engine::Game<motionvm_motion_forth::m16::Vm>| {
        game.engine
            .descriptors()
            .iter()
            .filter(|d| d.screen == screen)
            .count()
    };
    while on_screen(&game) < 100 {
        assert_eq!(make(&mut game), 1);
    }
    assert_eq!(on_screen(&game), 100);
    assert_eq!(make(&mut game), 6, "the six arguments stay, no handle");
    assert_eq!(on_screen(&game), 100, "and no descriptor was made");
}
