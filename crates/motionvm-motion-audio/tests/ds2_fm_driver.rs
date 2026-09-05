//! The rebuilt FM driver against `fmmidi3.com` itself.
//!
//! Every check here reads the original driver's own bytes out of `HMIMDRV.386`
//! and holds the rebuild against them. They need the game's files; point
//! `MOTIONVM_GAMEDATA_DS2` at the directory with `001.RSC`, or they skip themselves.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_motion_audio::Write;
use motionvm_motion_audio::m32::fm::{Fm, Patch, Tables, VOICES};
use motionvm_motion_audio::m32::{Kind, Message};
use motionvm_motion_formats::m32::{DriverArchive, bnk::Bank as InstrumentBank};
use motionvm_motion_testutil::{game_file, gamedata_ds2};

fn archive(dir: &std::path::Path) -> DriverArchive {
    let bytes = std::fs::read(game_file(dir, "HMIMDRV.386")).expect("HMIMDRV.386");
    DriverArchive::parse(&bytes).expect("the .386 chain walks")
}

fn banks(dir: &std::path::Path) -> (InstrumentBank, InstrumentBank) {
    let melodic = std::fs::read(game_file(dir, "MELODIC.BNK")).expect("MELODIC.BNK");
    let drums = std::fs::read(game_file(dir, "DRUM.BNK")).expect("DRUM.BNK");
    (
        InstrumentBank::parse(&melodic).expect("melodic bank"),
        InstrumentBank::parse(&drums).expect("drum bank"),
    )
}

fn driver(dir: &std::path::Path) -> Fm {
    let arc = archive(dir);
    let d = arc.device(Fm::DEVICE).expect("the OPL3 driver is in there");
    let (melodic, drums) = banks(dir);
    Fm::new(d, &melodic, &drums).expect("the tables are where they are said to be")
}

// ---------------------------------------------------------------- the tables

/// T1: the four tables are the driver's own bytes.
///
/// Read here straight out of `HMIMDRV.386` at the four addresses, without
/// going through `Tables::read`, so a wrong address or a wrong length fails.
/// It does **not** see a wrong *use* of a table — that is T2 and T3.
#[test]
fn the_tables_come_out_of_the_driver_image() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let arc = archive(&dir);
    let d = arc.device(Fm::DEVICE).expect("0xA009");
    let img = &d.image;
    let tables = Tables::read(d).expect("tables");

    let u32_at = |o: usize| u32::from_le_bytes(img[o..o + 4].try_into().unwrap());
    assert_eq!(tables.frequency.len(), 103, "notes 12 to 114");
    for (i, &v) in tables.frequency.iter().enumerate() {
        assert_eq!(v, u32_at(Tables::FREQUENCY_AT + i * 4), "frequency[{i}]");
    }
    assert_eq!(
        &tables.operators[..],
        &img[Tables::OPERATORS_AT..Tables::OPERATORS_AT + 18]
    );
    assert_eq!(
        &tables.velocity[..],
        &img[Tables::VELOCITY_AT..Tables::VELOCITY_AT + 64]
    );
    for (i, &v) in tables.octave_down.iter().enumerate() {
        assert_eq!(
            v,
            u32_at(Tables::OCTAVE_DOWN_AT + i * 4),
            "octave_down[{i}]"
        );
    }

    // And the shape the rest of the module relies on: the standard OPL
    // operator pairs, modulator then carrier, for nine channels.
    assert_eq!(
        tables.operators,
        [
            0, 3, 1, 4, 2, 5, 8, 0x0b, 9, 0x0c, 0x0a, 0x0d, 0x10, 0x13, 0x11, 0x14, 0x12, 0x15
        ]
    );
}

/// T2: note to f-number, including the `- 12`.
///
/// The first entry belongs to note 12, an octave up keeps the f-number and
/// raises the block by one. Dropping the `- 12` — the mistake the driver's own
/// `0x35E4` alias invites — shifts everything an octave and would still play.
#[test]
fn a_note_lands_on_its_own_f_number() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let arc = archive(&dir);
    let tables = Tables::read(arc.device(Fm::DEVICE).unwrap()).unwrap();

    assert_eq!(tables.frequency[0], 0x157, "note 12 is the first entry");
    for note in 12..24usize {
        let low = tables.frequency[note - 12];
        for octave in 1..8 {
            let high = tables.frequency[note - 12 + octave * 12];
            assert_eq!(
                high & 0x3ff,
                low & 0x3ff,
                "note {note} + {octave} octaves keeps the f-number"
            );
            assert_eq!(
                (high >> 10) & 7,
                ((low >> 10) & 7) + u32::try_from(octave).unwrap(),
                "and raises the block by {octave}"
            );
        }
    }
    // Every entry is a packed (block, f-number) that fits the chip, and the
    // last one is the highest note it can express: a semitone more — a factor
    // of 2^(1/12) — would need an f-number past ten bits, and the block is
    // already at its maximum.
    for (i, &v) in tables.frequency.iter().enumerate() {
        assert_eq!(
            v >> 13,
            0,
            "entry {i} is {v:#x}, which has bits above the block"
        );
    }
    let last = tables.frequency[102];
    assert_eq!((last >> 10) & 7, 7, "the last note is in the top block");
    assert!(
        (last & 0x3ff) * 1059 / 1000 > 0x3ff,
        "and there is no room for another"
    );

    // The halved table is the same notes one block up, with the one slip HMI
    // left in it. Indexed backwards, so entry 11 is the octave's first note.
    for k in 0..12usize {
        let halved = tables.octave_down[11 - k];
        let want = (tables.frequency[k] & 0x3ff).div_ceil(2);
        if k == 6 {
            assert_eq!(
                halved, 248,
                "the seventh entry is 248 where halving gives 243"
            );
        } else {
            assert!(
                halved.abs_diff(want) <= 1,
                "octave_down[{}] = {halved}, halving note {} gives {want}",
                11 - k,
                k + 12
            );
        }
    }
}

/// T3: the velocity law.
///
/// Full velocity on a patch that is already at full level attenuates by
/// nothing; no velocity at all attenuates by everything. The middle is checked
/// against the formula rather than against a number pulled out of the air, and
/// the curve itself has to fall.
#[test]
fn velocity_becomes_a_total_level() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let arc = archive(&dir);
    let tables = Tables::read(arc.device(Fm::DEVICE).unwrap()).unwrap();

    let level = |velocity: u8, patch: u32| -> u32 {
        let c = u32::from(tables.velocity[usize::from(velocity >> 1)]);
        (0x2000 - (0x40 - patch) * ((0x40 - c) * 2)) >> 7
    };
    assert_eq!(
        level(127, 0),
        0,
        "full velocity, loudest patch: no attenuation"
    );
    assert_eq!(level(0, 0), 63, "no velocity: the deepest the six bits go");
    // Tune 25's first note on channel 5: velocity 100, controller 7 at 90, so
    // the law sees ((90 << 7) / 127 * 100) >> 7 = 70 and answers 0x0B — which
    // is the byte the original wrote to register 0x44 in the recording.
    assert_eq!(
        level(70, 0),
        0x0b,
        "the value the original wrote for tune 25"
    );

    for i in 1..64 {
        assert!(
            tables.velocity[i] <= tables.velocity[i - 1],
            "the curve rises at {i}: {:?}",
            &tables.velocity[i - 1..=i]
        );
    }
}

/// T4: a known record folds into known register bytes.
///
/// `PIANO1` is worked out by hand below. A second instrument whose two halves
/// differ catches the mistake `PIANO1` cannot: modulator and carrier swapped.
#[test]
fn an_instrument_folds_into_register_bytes() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let (melodic, _) = banks(&dir);
    let patches = motionvm_motion_audio::m32::fm::convert_bank(&melodic);

    // Straight from the thirty bytes of record 0, by the rule at 0x0CD6.
    let r = &melodic.raw[0];
    let want = Patch {
        am_vib: [
            (r[11] << 7) | (r[12] << 6) | (r[7] << 5) | (r[13] << 4) | r[3],
            (r[24] << 7) | (r[25] << 6) | (r[20] << 5) | (r[26] << 4) | r[16],
        ],
        level: [(r[2] << 6) | r[10], (r[15] << 6) | r[23]],
        attack_decay: [(r[5] << 4) | r[8], (r[18] << 4) | r[21]],
        sustain_release: [(r[6] << 4) | r[9], (r[19] << 4) | r[22]],
        wave: [r[28], r[29]],
        feedback_connection: (r[4] << 1) | r[14],
    };
    assert_eq!(melodic.names[0].name, "PIANO1");
    assert_eq!(patches[0], want);

    // An instrument whose halves differ, so a swap cannot pass unnoticed.
    let uneven = (0..patches.len())
        .find(|&i| {
            let p = patches[i];
            p.am_vib[0] != p.am_vib[1]
                && p.attack_decay[0] != p.attack_decay[1]
                && p.sustain_release[0] != p.sustain_release[1]
        })
        .expect("some instrument has two different operators");
    let p = patches[uneven];
    let r = &melodic.raw[uneven];
    assert_eq!(
        p.am_vib[0],
        (r[11] << 7) | (r[12] << 6) | (r[7] << 5) | (r[13] << 4) | r[3]
    );
    assert_eq!(p.attack_decay[1], (r[18] << 4) | r[21]);

    // And the three the conversion never reaches, because it bounds itself by
    // a count that is one short and then subtracts two more.
    assert_eq!(melodic.used, 127);
    assert_eq!(patches.len(), 128);
    let last = patches[127];
    assert_eq!(
        last.level,
        [melodic.raw[127][2], melodic.raw[127][15]],
        "raw, not folded"
    );
    assert_eq!(melodic.names[127].name, "GUNSHOT");
}

/// T5: the switch-on sequence, in order.
///
/// A rebuild that sent these in any other order, or left `0x104` out, or gave
/// `0xBD` its rhythm bits, would still make a noise.
#[test]
fn the_driver_switches_the_chip_on_in_one_fixed_order() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut fm = driver(&dir);
    let writes = fm.take();
    let seen: Vec<(u16, u8)> = writes.iter().map(|w| (w.address(), w.value)).collect();

    assert_eq!(seen[0], (0x105, 0x01), "out of OPL2 compatibility first");
    assert_eq!(seen[1], (0x104, 0x00), "and no four-operator channels");
    for v in 0..VOICES {
        assert_eq!(
            seen[2 + v * 2],
            (0xb0 + u16::try_from(v).unwrap(), 0),
            "voice {v} silenced on bank 0"
        );
        assert_eq!(
            seen[3 + v * 2],
            (0x1b0 + u16::try_from(v).unwrap(), 0),
            "voice {v} silenced on bank 1"
        );
    }
    assert_eq!(
        seen[2 + VOICES * 2],
        (0xbd, 0xc0),
        "deep vibrato and tremolo, no rhythm mode"
    );
    assert_eq!(seen.len(), 3 + VOICES * 2, "and nothing else");
}

/// T6: allocation takes a free voice, then steals — but not round-robin.
///
/// Nine notes fill the nine voices in order. The tenth steals from the first
/// channel that never sent a pitch bend, which for a run of plain note-ons is
/// voice 0. Round-robin would answer with voice 0 as well for *this* case, so
/// the second half of the test bends one channel and checks that its voice is
/// the one left alone.
#[test]
fn voices_are_taken_free_first_then_stolen_from_the_unbent() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut fm = driver(&dir);
    let voice_of = |w: &[Write]| -> Option<u8> {
        // The key-on is the last 0xB0 write with bit 5 set.
        w.iter()
            .rev()
            .find(|w| (0xb0..=0xb8).contains(&w.reg) && w.value & 0x20 != 0)
            .map(|w| w.reg - 0xb0)
    };

    fm.take();
    for ch in 0..9u8 {
        fm.send(Message {
            channel: ch,
            kind: Kind::NoteOn {
                note: 60 + ch,
                velocity: 100,
            },
        });
        let w = fm.take();
        assert_eq!(
            voice_of(&w),
            Some(ch),
            "channel {ch} takes the next free voice"
        );
    }

    // Channel 3 bends; the tenth note must not land on channel 3's voice.
    fm.send(Message {
        channel: 3,
        kind: Kind::PitchBend { lsb: 0, msb: 0x50 },
    });
    fm.take();
    fm.send(Message {
        channel: 10,
        kind: Kind::NoteOn {
            note: 70,
            velocity: 100,
        },
    });
    let w = fm.take();
    let stolen = voice_of(&w).expect("the tenth note sounds too");
    assert_ne!(stolen, 3, "the bent channel keeps its voice");
    assert_eq!(stolen, 0, "and the first unbent one loses it");
}

/// T7: a note-off writes `0xB0` and nothing else.
///
/// The obvious guess — turn the level down — is wrong, and it matters: the
/// patch and the frequency stay behind, which is what lets the next note on
/// that voice reuse them.
#[test]
fn a_note_off_only_clears_the_key_bit() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut fm = driver(&dir);
    fm.send(Message {
        channel: 0,
        kind: Kind::NoteOn {
            note: 60,
            velocity: 100,
        },
    });
    fm.take();
    fm.send(Message {
        channel: 0,
        kind: Kind::NoteOff { note: 60 },
    });
    let w = fm.take();

    assert_eq!(w.len(), 2, "one write a bank: {w:?}");
    for (i, bank) in [0u8, 1].iter().enumerate() {
        assert_eq!(w[i].bank, *bank);
        assert_eq!(w[i].reg, 0xb0, "only the key register");
        assert_eq!(w[i].value & 0x20, 0, "with the key bit gone");
    }
}
