//! The sequencer and the FM driver against Checker 2000's songs.
//!
//! These need the original files; point `MOTIONVM_GAMEDATA_CHECKER` at the
//! directory with `ENGINE.RSC`, or they skip themselves.
//!
//! Checker 2000 runs the 32-bit engine in its R78 build and ships the same
//! three files the music needs as Dunkle Schatten 2 — `HMIMDRV.386`,
//! `MELODIC.BNK`, `DRUM.BNK` — with its songs as HMI blocks in the resource
//! banks. No recording of the original's register stream exists for this
//! game yet, so what this holds the songs to is the stream they write today,
//! and the set of blocks that are songs.
//!
//! The game this file drives is Checker 2000 (MOTION 32-bit).

use motionvm_motion_audio::m32::Sequencer;
use motionvm_motion_audio::m32::fm::Fm;
use motionvm_motion_formats::m32::{
    DriverArchive, Kind as Res,
    bnk::Bank as InstrumentBank,
    hmi::{Event, Song},
    rsc::Bank,
};
use motionvm_motion_testutil::{Digests, digest::Digest, game_file, gamedata_checker};

/// This game's table of reference digests.
fn digests() -> Digests {
    Digests::of(env!("CARGO_MANIFEST_DIR"), "checker")
}

/// Every song Checker 2000 ships writes the registers it wrote before.
///
/// A block is a song when it parses as one and holds a note-on — the same
/// criterion `ds2_plays_the_songs` uses, because the banks hold dialogue and
/// table blocks too and a lenient parse can make an empty song of some. The
/// set of song ids is digested as well, so a block leaving it is a failing
/// line rather than one test fewer.
#[test]
fn the_songs_write_the_registers_they_wrote_before() {
    let Some(dir) = gamedata_checker() else {
        eprintln!("skipping: no Checker 2000 gamedata directory");
        return;
    };
    let bank = Bank::open_dir(&dir).expect("the resource banks open");
    let archive = std::fs::read(game_file(&dir, "HMIMDRV.386")).expect("HMIMDRV.386");
    let archive = DriverArchive::parse(&archive).expect("the .386 chain walks");
    let device = archive.device(Fm::DEVICE).expect("the OPL3 driver");
    let melodic = std::fs::read(game_file(&dir, "MELODIC.BNK")).expect("MELODIC.BNK");
    let drums = std::fs::read(game_file(&dir, "DRUM.BNK")).expect("DRUM.BNK");
    let melodic = InstrumentBank::parse(&melodic).expect("the melodic bank");
    let drums = InstrumentBank::parse(&drums).expect("the drum bank");

    let mut songs = Vec::new();
    for (_, id) in bank.present(Res::Block) {
        let Ok(Some(item)) = bank.item(Res::Block, id) else {
            continue;
        };
        let Ok(song) = Song::parse(item) else {
            continue;
        };
        let sounds = song.tracks.iter().any(|t| {
            t.events
                .iter()
                .any(|e| matches!(e.event, Event::NoteOn { .. }))
        });
        if sounds {
            songs.push((id, song));
        }
    }
    assert!(
        !songs.is_empty(),
        "no song was found, so this proves nothing"
    );
    let mut ids = Digest::new();
    for (id, _) in &songs {
        ids.number(u64::try_from(*id).unwrap());
    }
    digests().check("songs", ids.value());

    for (id, song) in songs {
        let mut seq = Sequencer::new(song, Fm::DEVICE, Sequencer::FULL_VOLUME);
        let mut fm = Fm::new(device, &melodic, &drums).expect("the driver's tables");
        let mut messages = Vec::new();
        let mut writes = Vec::new();
        fm.take_into(&mut writes);
        for _ in 0..4000 {
            messages.clear();
            seq.tick(&mut messages);
            for m in &messages {
                fm.send(*m);
            }
            fm.take_into(&mut writes);
        }
        assert!(
            writes.len() > 100,
            "song {id} wrote only {} registers in 4000 ticks",
            writes.len()
        );
        let mut d = Digest::new();
        for w in &writes {
            d.byte(w.bank).byte(w.reg).byte(w.value);
        }
        digests().check(&format!("song_{id}"), d.value());
    }
}
