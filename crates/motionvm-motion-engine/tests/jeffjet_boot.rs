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
//! The game this file drives is Jeff Jet (MOTION 16-bit).

use motionvm_motion_engine::{Title, titles};
use motionvm_motion_testutil::gamedata_jeffjet;

#[test]
fn the_directory_is_told_apart_by_its_engine_binary() {
    let Some(dir) = gamedata_jeffjet() else {
        eprintln!("skipping: no Jeff Jet gamedata directory");
        return;
    };
    // All four 16-bit games ship a DATA.-1-; only this one ships HPPLAY.EXE.
    assert_eq!(titles::detect(&dir), Some(Title::JeffJet));
    let game = titles::open(&dir).expect("opens");
    assert_eq!(game.name(), "Jeff Jet - Abenteuer InfoHighway");
    assert_eq!(game.display_size(), (320, 200));
    assert_eq!(
        game.pixel_aspect(),
        motionvm_playable::PixelAspect {
            width: 5,
            height: 6
        }
    );
}

#[test]
fn a_directory_holding_a_container_but_no_engine_binary_is_not_a_game() {
    let Some(dir) = gamedata_jeffjet() else {
        eprintln!("skipping: no Jeff Jet gamedata directory");
        return;
    };
    let tmp = std::env::temp_dir().join("motionvm-jeffjet-detect");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).expect("a scratch directory");
    std::fs::copy(dir.join("DATA.-1-"), tmp.join("DATA.-1-")).expect("the container copies");
    // A container alone cannot say which 16-bit game this is — two of them
    // ship one by that name — so a directory holding one and neither binary
    // is neither game, and says so instead of naming one and failing on a
    // file that was never going to be there.
    assert_eq!(titles::detect(&tmp), None);
    assert!(titles::open(&tmp).is_err());
    let _ = std::fs::remove_dir_all(&tmp);
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
    for frame in 1..=1500 {
        game.step()
            .unwrap_or_else(|e| panic!("the intro stopped at frame {frame}: {e}"));
        let picture = game.render();
        assert_eq!((picture.width, picture.height), (320, 200));
        if picture.pixels.iter().any(|&p| p != 0) {
            lit_frames += 1;
        }
    }
    assert!(
        lit_frames > 1000,
        "only {lit_frames} of 1500 frames drew anything"
    );
    // The kernel table is scanned out of HPPLAY.EXE, whose words sit at
    // different ordinals from ENVIRO.EXE's from 124 up. A word walked past
    // without effect is what a mis-bound table looks like from here.
    assert!(
        game.engine.stubbed().is_empty(),
        "words walked past without effect: {:?}",
        game.engine.stubbed()
    );
}
