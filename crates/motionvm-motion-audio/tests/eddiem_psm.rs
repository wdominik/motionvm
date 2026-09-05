//! The rebuilt PSM 2 player against Falsches Spiel mit Eddie M.'s tunes.
//!
//! These need the original files; point `MOTIONVM_GAMEDATA_EDDIEM` at the
//! directory with the three `DATA.-n-` volumes and `MUSADL.DRV`, or they
//! skip themselves.
//!
//! Three tunes, in the two forms the corpus knows: blocks 24 and 25 are
//! `MTCVTS` modules with their `PLX` section 1688 and 2422 bytes in, and
//! block 19 — the jingle `SET_POINTS` plays for a scored action — is a bare
//! `PLX` section of 366 bytes, the form every one of Victor Loomes' fourteen
//! takes. This is the one game that ships both forms, so it is where the
//! sniff that tells them apart is exercised on a single container.
//!
//! The game this file drives is Falsches Spiel mit Eddie M. (MOTION 16-bit).

use motionvm_motion_audio::Player as _;
use motionvm_motion_audio::m16::{Driver, Player, Sequencer, time_constant};
use motionvm_motion_formats::m16::{Container, Segment, psm, psm::Plx};
use motionvm_motion_testutil::{Digests, digest::Digest, gamedata_eddiem};

/// The block numbers under a PSM tag — every tune the game ships.
const TUNES: [usize; 3] = [19, 24, 25];

/// This game's table of reference digests.
fn digests() -> Digests {
    Digests::of(env!("CARGO_MANIFEST_DIR"), "eddiem")
}

#[test]
fn every_shipped_tune_plays_notes() {
    let Some(dir) = gamedata_eddiem() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
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
        // register writes in the order the chip would have seen them.
        let mut d = Digest::new();
        for w in &writes {
            d.byte(w.bank).byte(w.reg).byte(w.value);
        }
        digests().check(&format!("tune_{tune}"), d.value());
    }
}

#[test]
fn the_jingle_is_a_bare_section_and_the_two_songs_are_modules() {
    let Some(dir) = gamedata_eddiem() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
        return;
    };
    let container = Container::open_dir(&dir).expect("the container");
    let jingle = container.item(Segment::Blk, 19).unwrap().unwrap();
    assert!(!psm::is_module(jingle), "block 19 has no MTCVTS tag");
    assert!(psm::is_song(jingle), "and is a PLX section all the same");
    let song = Plx::parse(jingle).expect("the bare section parses");
    assert_eq!((song.speed, song.tempo), (13, 5730));
    assert_eq!(
        song.channels.iter().filter(|&&c| c != 0).count(),
        8,
        "eight of the nine channels carry a stream"
    );
    for (tune, at) in [(24, 1688), (25, 2422)] {
        let block = container.item(Segment::Blk, tune).unwrap().unwrap();
        let sections = psm::sections(block).expect("the section table");
        assert_eq!(usize::try_from(sections[0]).unwrap(), at, "tune {tune}");
        let song = Plx::parse(block).expect("the PLX section");
        assert!(song.speed > 0 && song.tempo > 0, "tune {tune}");
    }
}

#[test]
fn the_driver_is_the_one_the_later_games_ship() {
    let Some(dir) = gamedata_eddiem() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
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
    // The 1994 game ships the 4480-byte driver of the 1995/96 ones, byte for
    // byte — the PSM 2 stack it carries is theirs, two years early. Only
    // Victor Loomes has the older build.
    assert_eq!(ours, theirs, "the Ad Lib driver differs between the games");
}

#[test]
fn a_sample_plays_once_over_the_music_for_as_long_as_its_period_says() {
    let Some(dir) = gamedata_eddiem() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
        return;
    };
    let driver =
        std::fs::read(motionvm_motion_testutil::game_file(&dir, "MUSADL.DRV")).expect("MUSADL.DRV");
    let container = Container::open_dir(&dir).expect("the container");
    // Block 13, the one the flat's cupboard plays: 2484 bytes at a period of
    // 83 cycles, which the driver's table makes a time constant of 0xba — 70
    // µs a byte, 14.3 kHz, 173.9 ms.
    let block = container.item(Segment::Blk, 13).unwrap().unwrap();
    let sample = psm::Sample::parse(block).expect("an SM8 block");
    assert_eq!((sample.period, sample.pcm.len()), (83, 2484));
    assert_eq!(time_constant(sample.period), 0xba);
    let rate = 44_100_u32;
    let expected = 70 * 2484 * u64::from(rate) / 1_000_000;

    let mut player = Player::new(rate, &driver).expect("the player");
    // A second player that is never handed the sample: the idle OPL under it
    // is not exactly zero — the chip sits a few LSBs below the mid-point —
    // so what the sample adds is read as the difference between the two.
    let mut quiet = Player::new(rate, &driver).expect("the player");
    assert!(!player.playing(), "silent before anything is asked");
    player.sample(sample);
    assert!(player.playing(), "a sample counts as sounding");
    // A frame at a time until the voice has run out: the count is the
    // sample's length in frames, and what is on them beyond the idle chip is
    // the sample, the same on both channels.
    let (mut out, mut idle) = ([0i16; 2], [0i16; 2]);
    let mut frames = 0u64;
    let mut heard = false;
    while player.playing() {
        player.fill(&mut out);
        quiet.fill(&mut idle);
        let added = [out[0] - idle[0], out[1] - idle[1]];
        heard |= added[0] != 0;
        assert_eq!(
            added[0], added[1],
            "the sample is the same on both channels"
        );
        frames += 1;
        assert!(frames < 20_000, "the sample never ended");
    }
    assert!(heard, "the sample was never on a frame");
    assert!(
        frames.abs_diff(expected) <= 1,
        "{frames} frames of sample against {expected} expected"
    );
    // And after it, the two players are the same chip again.
    let (mut out, mut idle) = ([0i16; 512], [0i16; 512]);
    player.fill(&mut out);
    quiet.fill(&mut idle);
    assert_eq!(out, idle, "silence after the sample");
}

/// The time-constant table is not transcribed but computed, so it is held
/// against the shipped drivers' own: the 256 bytes at `0x60` of every
/// `DMA*.DRV` the game carries, which are the same bytes in all four.
#[test]
fn the_time_constant_table_is_read_out_of_the_shipped_drivers() {
    let Some(dir) = gamedata_eddiem() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
        return;
    };
    let mut seen = 0;
    for name in [
        "DMABLAST.DRV",
        "DMASB2P.DRV",
        "DMASB16M.DRV",
        "DMASB16S.DRV",
    ] {
        let Ok(driver) = std::fs::read(motionvm_motion_testutil::game_file(&dir, name)) else {
            continue;
        };
        assert_eq!(&driver[..4], b"DMA\0", "{name} is a digital driver");
        let table = &driver[0x60..0x160];
        for (period, &constant) in (0..=255u16).zip(table) {
            assert_eq!(time_constant(period), constant, "{name}: period {period}");
        }
        seen += 1;
    }
    if seen == 0 {
        eprintln!("skipping: the copy carries no DMA*.DRV");
    }
}
