//! Jeff Jet - Abenteuer InfoHighway boots on the 16-bit machine.
//!
//! The same `RUN` as Die Enviro-Kids greifen ein's, out of the same module 100
//! and the same word id 401: load the library, play the intro, enter its
//! `ANIMPLAY`. These tests drive that far and ask for the picture. They need
//! the game's files — `MOTIONVM_GAMEDATA_JEFFJET` — and skip without them.
//!
//! What they are really for is the two things this game does that the other
//! cannot: it comes off two volumes, and every item of it is packed. If either
//! were read wrongly the game would not fail, it would run without artwork or
//! without colors — so these tests ask for pixels, and for a palette.
//!
//! That this directory is told apart and opens as this game is asked in
//! `titles_detected.rs`, one row per game; here the game is already open.
//!
//! The game this file drives is Jeff Jet (MOTION 16-bit).

mod common;

use motionvm_motion_engine::titles;
use motionvm_motion_testutil::{Digests, digest, digest::Digest, gamedata_jeffjet};

/// This game's table of reference digests.
fn digests() -> Digests {
    common::digests("jeffjet")
}

#[test]
fn run_reaches_the_intro_loop_and_the_first_frames_draw_a_picture() {
    let Some(dir) = gamedata_jeffjet() else {
        eprintln!("skipping: no Jeff Jet gamedata directory");
        return;
    };
    let mut game = titles::jeffjet::open(&dir).expect("opens");
    game.start().expect("RUN starts");
    let mut frames = 0;
    while game.pump().expect("RUN runs to ANIMPLAY") {
        frames += 1;
        assert!(frames < 10_000, "RUN never reached ANIMPLAY");
    }
    assert!(game.engine.main_loop(), "ANIMPLAY was entered");
    assert!(
        game.engine.controller().is_some(),
        "the intro installed its controller with SCRCTRL"
    );
    // Every palette of this game is on volume 2. A container that read only
    // the first would leave this all zeros and the game would run black.
    assert!(
        game.palette().raw.iter().any(|&v| v != 0),
        "a palette was installed"
    );
    let mut lit_frames = 0;
    // Every frame of the intro folded into one digest, rather than the last
    // one alone: the intro is an animation, and a still from the end of it
    // would say nothing about the fourteen hundred pictures before it. The
    // frames are rendered here either way, so this costs the fold and nothing
    // else.
    let mut intro = Digest::new();
    for frame in 1..=1500 {
        game.step()
            .unwrap_or_else(|e| panic!("the intro stopped at frame {frame}: {e}"));
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
    // The kernel table is scanned out of HPPLAY.EXE, whose words sit at
    // different ordinals from ENVIRO.EXE's from 124 up. A word walked past
    // without effect is what a mis-bound table looks like from here.
    assert!(
        game.engine.stubbed().is_empty(),
        "words walked past without effect: {:?}",
        game.engine.stubbed()
    );
}
