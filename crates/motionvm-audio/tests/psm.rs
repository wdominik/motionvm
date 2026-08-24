//! The rebuilt PSM 2 player against the tunes it has to play.
//!
//! These need the original files; point `MOTIONVM_GAMEDATA_ENVIRO` at the
//! directory with `DATA.-1-`, or they skip themselves.
//!
//! The game data this file drives is Die Enviro-Kids greifen ein's (MOTION
//! 16-bit). The byte-for-byte check of the register stream is not here — it
//! runs against an OPL capture of the original, outside this repository;
//! what is here is what can be asked of the shipped files alone.

use motionvm_audio::psm::{Driver, Player, Sequencer};
use motionvm_formats::m16::psm::Plx;
use motionvm_formats::m16::{Container, Segment};
use motionvm_testutil::gamedata_enviro;

/// The block numbers under a PSM tag — every tune the game ships.
const TUNES: [usize; 10] = [1, 2, 3, 4, 5, 7, 8, 9, 10, 11];

#[test]
fn every_shipped_tune_plays_notes() {
    let Some(dir) = gamedata_enviro() else { return };
    let driver =
        std::fs::read(motionvm_testutil::game_file(&dir, "MUSADL.DRV")).expect("MUSADL.DRV");
    let driver = Driver::parse(&driver).expect("the driver's tables");
    let container = Container::open_dir(&dir).expect("DATA.-1-");
    for tune in TUNES {
        let block = container
            .item(Segment::Blk, tune)
            .expect("the BLK segment")
            .expect("an occupied slot");
        let song = Plx::parse(block).expect("the PLX section");
        let mut seq = Sequencer::new(&driver, song, -1);
        let mut writes = Vec::new();
        let mut keyed = false;
        for _ in 0..20_000 {
            seq.tick(&mut writes);
        }
        for w in &writes {
            let reg = w.address();
            assert!(
                reg < 0x100,
                "tune {tune} wrote past the OPL2 bank: {reg:#x}"
            );
            if (0xb0..=0xb8).contains(&reg) && w.value & 0x20 != 0 {
                keyed = true;
            }
        }
        assert!(keyed, "tune {tune} never keyed a note in 20000 ticks");
        assert!(
            seq.playing(),
            "tune {tune} stopped although its loop count is endless"
        );
    }
}

#[test]
fn the_player_sounds_and_its_stop_dies_away() {
    let Some(dir) = gamedata_enviro() else { return };
    let driver =
        std::fs::read(motionvm_testutil::game_file(&dir, "MUSADL.DRV")).expect("MUSADL.DRV");
    let container = Container::open_dir(&dir).expect("DATA.-1-");
    let block = container
        .item(Segment::Blk, 8)
        .expect("the BLK segment")
        .expect("tune 8 is there");
    let song = Plx::parse(block).expect("the PLX section");

    let mut player = Player::new(48_000, &driver).expect("the driver comes up");
    player.start(song, -1);
    let mut buf = vec![0i16; 48_000 * 2];
    player.fill(&mut buf);
    player.fill(&mut buf);
    let peak = buf.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0);
    assert!(
        peak > 500,
        "two seconds in, tune 8 is all but silent: peak {peak}"
    );
    assert!(player.playing());

    // `ENDTUNE`: the fade begins at once, the stop lands 500 ms in, and the
    // release rates it opens drain what is left within a couple of seconds.
    player.stop();
    for _ in 0..3 {
        player.fill(&mut buf);
    }
    assert!(!player.playing(), "the stop never landed");
    player.fill(&mut buf);
    let peak = buf.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0);
    assert!(
        peak < 50,
        "three seconds after the stop, something still sounds: peak {peak}"
    );
}
