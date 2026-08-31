//! The rebuilt PSM 2 player against Victor Loomes' tunes.
//!
//! These need the original files; point `MOTIONVM_GAMEDATA_VLOOMES` at the
//! directory with `DATA.-1-` and `MUSADL.DRV`, or they skip themselves.
//!
//! Two things here are the earlier build's, and both would go unnoticed
//! if only the later games were tested. The songs are stored as a **bare
//! `PLX` section** with no `MTCVTS` module around them, so a reader that
//! insisted on the module tag would find no music in a game that ships
//! fourteen tunes. And the **driver is an older build** — 3915 bytes with
//! fourteen entries against the later games' byte-identical 4480 with
//! fifteen — whose four tables sit `0xd0` earlier in the file. Their contents
//! are the same to the byte, which is what makes reading them at the wrong
//! offset a silent wrong answer rather than a loud one.
//!
//! The game this file drives is Victor Loomes (MOTION 16-bit).

use motionvm_motion_audio::m16::{Driver, Sequencer};
use motionvm_motion_formats::m16::{Container, Segment, psm, psm::Plx};
use motionvm_motion_testutil::gamedata_vloomes;

/// The block numbers the songs sit in — every tune the game ships.
const TUNES: std::ops::RangeInclusive<usize> = 0..=13;

#[test]
fn every_shipped_tune_plays_notes() {
    let Some(dir) = gamedata_vloomes() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
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
    }
}

#[test]
fn the_songs_are_bare_sections_with_no_module_around_them() {
    let Some(dir) = gamedata_vloomes() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    let container = Container::open_dir(&dir).expect("the container");
    for tune in TUNES {
        let block = container.item(Segment::Blk, tune).unwrap().unwrap();
        assert!(psm::is_song(block), "block {tune} is a song");
        // No module, so no section table to read and no sample sections to
        // point at — the later games have both.
        assert!(!psm::is_module(block), "block {tune} carries no MTCVTS tag");
        assert!(psm::sections(block).is_err());
        assert!(psm::tags(block).is_none());
        let song = Plx::parse(block).expect("the PLX section");
        assert!(song.speed > 0, "tune {tune} has no speed");
        assert!(song.tempo > 0, "tune {tune} has no tempo");
    }
    // Nothing outside that run is one, so fourteen is the whole set: the rest
    // of the segment is the game's own tables.
    for id in container.present(Segment::Blk) {
        let block = container.item(Segment::Blk, id).unwrap().unwrap();
        assert_eq!(psm::is_song(block), TUNES.contains(&id), "block {id}");
    }
}

#[test]
fn the_older_driver_holds_the_same_tables_at_its_own_offsets() {
    let Some(dir) = gamedata_vloomes() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    let ours =
        std::fs::read(motionvm_motion_testutil::game_file(&dir, "MUSADL.DRV")).expect("MUSADL.DRV");
    // Fourteen entries where the later games have fifteen, and 565 bytes
    // shorter — the build, not the file, is what says where to read.
    assert_eq!(ours.len(), 3915);
    assert_eq!(&ours[..4], b"MUS\0");
    assert_eq!(u16::from_le_bytes([ours[6], ours[7]]), 14);
    let ours = Driver::parse(&ours).expect("the older build parses");

    let Some(other) = motionvm_motion_testutil::gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory to compare with");
        return;
    };
    let theirs = std::fs::read(motionvm_motion_testutil::game_file(&other, "MUSADL.DRV"))
        .expect("MUSADL.DRV");
    assert_eq!(u16::from_le_bytes([theirs[6], theirs[7]]), 15);
    let theirs = Driver::parse(&theirs).expect("the later build parses");

    // Every table is the same to the byte. That is the point: read at the
    // later build's offsets this file would still yield 256 plausible-looking
    // defaults, and nothing would sound wrong until a note came out flat.
    assert_eq!(ours.defaults, theirs.defaults);
    assert_eq!(ours.mod_ops, theirs.mod_ops);
    assert_eq!(ours.car_ops, theirs.car_ops);
    assert_eq!(ours.notes, theirs.notes);
}
