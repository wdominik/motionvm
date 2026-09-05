//! The rebuilt PSM 2 player against Hilfe für Amajambere's tunes.
//!
//! These need the original files; point `MOTIONVM_GAMEDATA_HFA` at the
//! directory with `DATA.-1-`, `DATA.-2-` and `MUSADL.DRV`, or they skip
//! themselves.
//!
//! The driver file is byte-identical to Die Enviro-Kids greifen ein's and Jeff
//! Jet's — the three 1995/96 builds ship the same one — so what is
//! being asked here is not whether the player works — `enviro_psm.rs` asks
//! that — but whether it works on a third set of songs written outside the
//! windows the other two fall in. These four put their `PLX` section between
//! 1227 and 2199 bytes in, where Jeff Jet's nine sit between 369 and 2723 and
//! ENVIRO's ten between 767 and 1640. The section table is read, never
//! assumed, and these are the corpora that say so.
//!
//! The game this file drives is Hilfe für Amajambere (MOTION 16-bit).

use motionvm_motion_audio::m16::{Driver, Sequencer};
use motionvm_motion_formats::m16::{Container, Segment, psm::Plx};
use motionvm_motion_testutil::{Digests, digest::Digest, gamedata_hfa};

/// The block numbers under a PSM tag — every tune the game ships.
const TUNES: std::ops::RangeInclusive<usize> = 1..=4;

/// This game's table of reference digests.
fn digests() -> Digests {
    Digests::of(env!("CARGO_MANIFEST_DIR"), "hfa")
}

#[test]
fn every_shipped_tune_plays_notes() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    let driver =
        std::fs::read(motionvm_motion_testutil::game_file(&dir, "MUSADL.DRV")).expect("MUSADL.DRV");
    let driver = Driver::parse(&driver).expect("the driver's tables");
    let container = Container::open_dir(&dir).expect("the container");
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
        // The whole stream, not a property of it: twenty thousand ticks of
        // register writes in the order the chip would have seen them. The
        // assertions above say the tune is music at all; this says it is the
        // same music, write for write, as the last time anyone looked.
        let mut d = Digest::new();
        for w in &writes {
            d.byte(w.bank).byte(w.reg).byte(w.value);
        }
        digests().check(&format!("tune_{tune}"), d.value());
    }
}

#[test]
fn the_section_table_is_read_and_not_assumed() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    let container = Container::open_dir(&dir).expect("the container");
    let mut offsets = Vec::new();
    for tune in TUNES {
        let block = container.item(Segment::Blk, tune).unwrap().unwrap();
        let sections =
            motionvm_motion_formats::m16::psm::sections(block).expect("the section table");
        let plx = usize::try_from(sections[0]).unwrap();
        assert!(
            plx > 0 && plx < block.len(),
            "tune {tune}: the PLX section is outside the block"
        );
        offsets.push(plx);
        let song = Plx::parse(block).expect("the PLX section");
        assert!(song.speed > 0, "tune {tune} has no speed");
        assert!(song.tempo > 0, "tune {tune} has no tempo");
    }
    let (low, high) = (
        *offsets.iter().min().unwrap(),
        *offsets.iter().max().unwrap(),
    );
    // 1227 to 2199, inside Jeff Jet's 369-to-2723 span and outside ENVIRO's
    // 767-to-1640 one.
    assert_eq!((low, high), (1227, 2199));
}

#[test]
fn the_driver_is_the_one_the_other_games_ship() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    let Some(other) = motionvm_motion_testutil::gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory to compare with");
        return;
    };
    let ours =
        std::fs::read(motionvm_motion_testutil::game_file(&dir, "MUSADL.DRV")).expect("MUSADL.DRV");
    let theirs = std::fs::read(motionvm_motion_testutil::game_file(&other, "MUSADL.DRV"))
        .expect("MUSADL.DRV");
    // One opener serves every 16-bit game: the three 1995/96 ones ship the
    // same driver and Victor Loomes an older build the same reader takes. If a game
    // ever shipped a different one, the rebuilt player would be reading another
    // game's tables and this is where that would show.
    assert_eq!(ours, theirs, "the Ad Lib driver differs between the games");
}
