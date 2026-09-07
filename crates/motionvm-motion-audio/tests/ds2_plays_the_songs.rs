//! The sequencer against the songs it has to play.
//!
//! These need the original files; point `MOTIONVM_GAMEDATA_DS2` at the directory with
//! `001.RSC`, or they skip themselves.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_motion_audio::m32::fm::Fm;
use motionvm_motion_audio::m32::{Kind, Message, Sequencer};
use motionvm_motion_formats::m32::{
    DriverArchive, Kind as Res,
    bnk::Bank as InstrumentBank,
    hmi::{Event, Song},
    rsc::Bank,
};
use motionvm_motion_testutil::{Digests, digest::Digest, game_file, gamedata_ds2};

/// This game's table of reference digests.
fn digests() -> Digests {
    Digests::of(env!("CARGO_MANIFEST_DIR"), "ds2")
}

/// Runs `ticks` ticks and returns every message with the tick it fell on.
fn play(song: Song, ticks: u32) -> Vec<(u32, Message)> {
    let mut seq = Sequencer::new(song, Fm::DEVICE, Sequencer::FULL_VOLUME);
    let mut out = Vec::new();
    let mut buf = Vec::new();
    for t in 0..ticks {
        buf.clear();
        seq.tick(&mut buf);
        out.extend(buf.iter().map(|m| (t, *m)));
    }
    out
}

/// What comes out is what the decoder read — same notes, same order, same time.
///
/// The decoder's own oracle is `TEST.MID`; this is the second half of the
/// chain, that the clock does not drop, reorder or double anything on the way
/// from an event list to a stream of messages.
///
/// There is no shift between the two: the first delta is read by the track
/// initializer before play starts (`0x9872B`) and the service routine tests it
/// before decrementing (`0x8FD53`), so an event the decoder puts at tick `n`
/// is dispatched on tick `n`. (The original's own `track+0x4F` counts from 1
/// because it increments first, but that is the counter's labeling, not the
/// event's time.)
#[test]
fn the_notes_come_out_as_the_decoder_read_them() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let data = std::fs::read(game_file(&dir, "TEST.HMI")).expect("TEST.HMI");
    let song = Song::parse(&data).expect("parses");

    // The one place the sequencer is allowed to change a note on its way out:
    // channel 9's velocity is scaled by that channel's cached controller 7
    // (`0x98A89`). So the decoder's velocity is the expectation everywhere
    // else, and on channel 9 the expectation is the scaled one — which needs
    // the same controller cache the sequencer keeps.
    let mut want: Vec<(u32, u8, u8, u8)> = Vec::new();
    let mut scaled = 0;
    // Only the tracks the OPL3 gets: the sequencer loads no other.
    for track in song
        .tracks
        .iter()
        .filter(|t| motionvm_motion_audio::m32::sequencer::plays_on(&t.devices, Fm::DEVICE))
    {
        let mut volume = 0x7fu32;
        for e in &track.events {
            match e.event {
                Event::Control {
                    controller: 7,
                    value,
                } => {
                    volume = u32::from(value);
                }
                Event::NoteOn { note, velocity, .. } => {
                    let velocity = if track.channel == 9 {
                        scaled += 1;
                        u8::try_from(u32::from(velocity) * volume / 127).unwrap()
                    } else {
                        velocity
                    };
                    want.push((e.tick, u8::try_from(track.channel).unwrap(), note, velocity));
                }
                _ => {}
            }
        }
    }
    assert!(scaled > 0, "TEST.HMI has drums, or this proves nothing");
    want.sort_unstable();

    let last = want.last().expect("TEST.HMI has notes").0;
    let mut got: Vec<(u32, u8, u8, u8)> = play(song, last + 8)
        .into_iter()
        .filter_map(|(t, m)| match m.kind {
            Kind::NoteOn { note, velocity } => Some((t, m.channel, note, velocity)),
            _ => None,
        })
        .collect();
    got.sort_unstable();

    assert_eq!(got.len(), want.len(), "note count");
    if let Some((i, (a, b))) = got.iter().zip(&want).enumerate().find(|(_, (a, b))| a != b) {
        panic!("note {i} of {}: sequencer {a:?}, decoder {b:?}", want.len());
    }
}

/// A note ends where its own length says, and not a tick sooner or later.
///
/// The format carries no note-offs; the length rides on the note-on and the
/// engine ages it in a queue (`0x8FCE0`). Two details of that queue decide the
/// answer, and both are in the code: the ageing pass runs at the **top** of the
/// service (`0x8FCD6`), so a note started this tick is first aged on the next
/// one; and it fires when the counter it **reads** is already zero, not when
/// the decrement reaches zero. Together that puts the end `duration + 1` ticks
/// after the start. The extra tick is the pre-decrement, not a rounding
/// choice — measured, and it is what a naive `off = on + duration` would miss.
#[test]
fn a_note_ends_where_its_length_says() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let data = std::fs::read(game_file(&dir, "TEST.HMI")).expect("TEST.HMI");
    let song = Song::parse(&data).expect("parses");
    // Every note-on of the first track, by the tick it is dispatched on.
    // The same pitch sounds on several channels at once, so ons and offs are
    // paired per channel and in order — matching on the pitch alone finds an
    // earlier note's ending and says nothing.
    const WINDOW: u32 = 900;
    let mut want: std::collections::BTreeMap<(u8, u8), Vec<u32>> = Default::default();
    for track in &song.tracks {
        let channel = u8::try_from(track.channel).unwrap();
        for e in &track.events {
            if let Event::NoteOn { note, duration, .. } = e.event
                && e.tick + duration + 1 < WINDOW
            {
                want.entry((channel, note))
                    .or_default()
                    .push(e.tick + duration + 1);
            }
        }
    }
    assert!(!want.is_empty());

    let log = play(song, WINDOW);
    let mut got: std::collections::BTreeMap<(u8, u8), Vec<u32>> = Default::default();
    for (t, m) in &log {
        if let Kind::NoteOff { note } = m.kind {
            got.entry((m.channel, note)).or_default().push(*t);
        }
    }

    let mut checked = 0;
    for (key, ends) in &want {
        let mine = got.get(key).map(Vec::as_slice).unwrap_or(&[]);
        assert!(
            mine.len() >= ends.len(),
            "channel {} note {}: {} endings, expected at least {}",
            key.0,
            key.1,
            mine.len(),
            ends.len()
        );
        for (i, want_tick) in ends.iter().enumerate() {
            assert_eq!(
                mine[i], *want_tick,
                "channel {} note {} ending {i}",
                key.0, key.1
            );
            checked += 1;
        }
    }
    assert!(
        checked > 50,
        "only {checked} endings were inside the window"
    );
}

/// The loop takes every track back, not just the one that asked.
///
/// `FE 15` names a branch id, and the original sends **all** tracks of the song
/// to their own `FE 10` marker with that id (`0x90714`). A rebuild that only
/// rewound the track holding the `FE 15` would drift the others apart within
/// one pass — so the test looks at every track, and at the notes that come
/// after the jump.
#[test]
fn the_loop_takes_every_track_back() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let bank = Bank::open_dir(&dir).expect("banks");
    // Song 6 is the shortest looping one, so the second pass is reachable.
    let data = bank.item(Res::Block, 6).expect("read").expect("song 6");
    let song = Song::parse(data).expect("parses");
    let marked = song
        .tracks
        .iter()
        .filter(|t| !t.branch_points.is_empty())
        .count();
    assert!(marked > 0, "song 6 has branch points to come back to");

    let ends: Vec<u32> = song
        .tracks
        .iter()
        .map(|t| t.events.last().map_or(0, |e| e.tick))
        .collect();
    let longest = *ends.iter().max().expect("tracks");

    // Play well past the end of the longest track. Without a loop everything
    // would be silent by then.
    let log = play(song, longest * 2);
    let after: Vec<_> = log
        .iter()
        .filter(|(t, m)| *t > longest + 2 && matches!(m.kind, Kind::NoteOn { .. }))
        .collect();
    assert!(
        after.len() > 20,
        "after the loop point only {} notes played — the song did not come back",
        after.len()
    );
    let channels: std::collections::BTreeSet<u8> = after.iter().map(|(_, m)| m.channel).collect();
    assert!(
        channels.len() > 1,
        "only channel {channels:?} came back, so the branch was not collective"
    );
}

/// Which event kinds the shipped songs actually contain.
///
/// This is the evidence behind the allocation note at the top of `player.rs`.
/// [`Event`] has three variants that own a `Vec` — `SysEx`, `BranchPoint` and
/// `Branch` — and the sequencer clones the event it is about to dispatch. For
/// every other variant that clone is a handful of bytes and costs nothing; for
/// those three it is a heap allocation, and one on the audio thread.
///
/// So the claim "it does not allocate once a song is running" rests on which of
/// them occur, and how often. This counts them rather than assuming.
#[test]
fn the_shipped_songs_hold_no_event_that_allocates_per_note() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let bank = Bank::open_dir(&dir).expect("the resource banks open");

    let (mut sysex, mut branch, mut markers, mut total) = (0, 0, 0, 0);
    for (_, id) in bank.present(Res::Block) {
        let Ok(Some(item)) = bank.item(Res::Block, id) else {
            continue;
        };
        let Ok(song) = Song::parse(item) else {
            continue;
        };
        for track in &song.tracks {
            for e in &track.events {
                total += 1;
                match e.event {
                    Event::SysEx(_) => sysex += 1,
                    Event::Branch { .. } => branch += 1,
                    Event::BranchPoint { .. } => markers += 1,
                    _ => {}
                }
            }
        }
    }

    assert!(total > 0, "no songs were read, so this proves nothing");
    // Markers are allowed: one is dispatched per loop, roughly twice a minute,
    // which is what the note in `player.rs` already says allocates.
    assert_eq!(
        (sysex, branch),
        (0, 0),
        "a shipped song holds an event whose dispatch allocates: \
         {sysex} SysEx and {branch} Branch in {total} events"
    );
    eprintln!("{total} events across the shipped songs, {markers} loop markers");
}

/// Every shipped song, played through the sequencer and the FM driver, held
/// against the register stream it produced last time.
///
/// The whole 32-bit music stack in one check: the `HMI` decoder, the clock
/// that dispatches its events, the voice allocator, the instrument banks, and
/// every table read out of `HMIMDRV.386`. What comes out is the byte sequence
/// the chip would have seen, in order — which is exactly what a capture of the
/// original is compared against, and the one form of it that may be kept here.
///
/// A fixed tick count rather than a whole song, because the songs loop
/// endlessly and there is no natural end; 4000 ticks is well past the point
/// every track has entered.
#[test]
fn the_songs_write_the_registers_they_wrote_before() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
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

    // Which blocks are songs. The segment holds the game's dialogue and text
    // blocks too, and `Song::parse` is lenient enough to make a shape out of
    // some of them — an empty one, with no note in it. So the criterion is
    // the music itself: a block that holds a note-on is a song, and the set of
    // them is digested as well, so that a block quietly leaving the set is a
    // failing line rather than one test fewer.
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
        // The switch-on sequence `Fm::new` runs is part of the stream: it is
        // what the chip sees first, and getting it wrong is silent.
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

/// The opening tune (block 25) carries eight tracks, and two of them — the
/// bass on channel 2 and the second guitar on channel 4 — name no device
/// the OPL3 answers to (`[0xA000, 0xA004, 0xA00A]`, without `0xA002`). The
/// original's song open gives them to no device, so they are silent there,
/// and the sequencer loads six tracks.
#[test]
fn the_opening_tune_gives_two_tracks_to_other_devices() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let bank = Bank::open_dir(&dir).expect("the resource banks open");
    let block = bank
        .item(Res::Block, 25)
        .expect("the bank reads")
        .expect("block 25");
    let song = Song::parse(block).expect("the opening tune parses");
    let silent: Vec<u16> = song
        .tracks
        .iter()
        .filter(|t| !motionvm_motion_audio::m32::sequencer::plays_on(&t.devices, Fm::DEVICE))
        .map(|t| t.channel)
        .collect();
    assert_eq!(silent, [2, 4], "the tracks the OPL3 does not get");
    assert!(
        song.tracks.iter().all(|t| t.devices.contains(&0xa000)),
        "every track names the Ad Lib family"
    );
}
