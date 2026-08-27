//! The rebuilt PSM 2 player against Jeff Jet - Abenteuer InfoHighway's tunes.
//!
//! These need the original files; point `MOTIONVM_GAMEDATA_JEFFJET` at the
//! directory with `DATA.-1-`, `DATA.-2-` and `MUSADL.DRV`, or they skip
//! themselves.
//!
//! The driver file is byte-identical to the other 16-bit game's, so what is
//! being asked here is not whether the player works — `enviro_psm.rs` asks
//! that — but whether it works on songs written outside the window that game's
//! ten happen to fall in. Jeff Jet's nine put their `PLX` section between 369 and 2723
//! bytes in, where the others sit between 767 and 1640, and their speeds and
//! tempos are their own. The section table is read, never assumed, and this is
//! the corpus that says so.

use motionvm_audio::psm::{Driver, Sequencer};
use motionvm_formats::m16::psm::Plx;
use motionvm_formats::m16::{Container, Segment};
use motionvm_testutil::gamedata_jeffjet;

/// The block numbers under a PSM tag — every tune the game ships.
const TUNES: std::ops::RangeInclusive<usize> = 1..=9;

#[test]
fn every_shipped_tune_plays_notes() {
    let Some(dir) = gamedata_jeffjet() else {
        eprintln!("skipping: no Jeff Jet gamedata directory");
        return;
    };
    let driver =
        std::fs::read(motionvm_testutil::game_file(&dir, "MUSADL.DRV")).expect("MUSADL.DRV");
    let driver = Driver::parse(&driver).expect("the driver's tables");
    let container = Container::open_dir(&dir).expect("the container");
    for tune in TUNES {
        // The block is a packed item on volume 1; what `Plx::parse` sees is
        // what the container unfolded.
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
fn the_section_table_is_read_and_not_assumed() {
    let Some(dir) = gamedata_jeffjet() else {
        eprintln!("skipping: no Jeff Jet gamedata directory");
        return;
    };
    let container = Container::open_dir(&dir).expect("the container");
    let mut offsets = Vec::new();
    for tune in TUNES {
        let block = container.item(Segment::Blk, tune).unwrap().unwrap();
        let sections = motionvm_formats::m16::psm::sections(block).expect("the section table");
        let plx = sections[0] as usize;
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
    // 369 to 2723, against 767 to 1640 in the other 16-bit game. A loader that
    // had learned that window from one corpus would miss every tune here.
    assert_eq!((low, high), (369, 2723));
}
