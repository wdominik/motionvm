//! The chain from a song to samples.
//!
//! These need the original files; point `MOTIONVM_GAMEDATA_DS2` at the directory with
//! `001.RSC`, or they skip themselves.
//!
//! The game this file drives is Dunkle Schatten 2 (MOTION 32-bit).

use motionvm_audio::opl::{Fm, Tables, Write};
use motionvm_audio::{Kind, Message, Player};
use motionvm_formats::m32::{
    DriverArchive, Kind as Res, bnk::Bank as InstrumentBank, hmi::Song, rsc::Bank,
};
use motionvm_testutil::{game_file, gamedata_ds2};

fn parts(dir: &std::path::Path) -> (DriverArchive, InstrumentBank, InstrumentBank) {
    let bytes = std::fs::read(game_file(dir, "HMIMDRV.386")).expect("HMIMDRV.386");
    let melodic = std::fs::read(game_file(dir, "MELODIC.BNK")).expect("MELODIC.BNK");
    let drums = std::fs::read(game_file(dir, "DRUM.BNK")).expect("DRUM.BNK");
    (
        DriverArchive::parse(&bytes).expect("the chain walks"),
        InstrumentBank::parse(&melodic).expect("melodic"),
        InstrumentBank::parse(&drums).expect("drums"),
    )
}

fn player(dir: &std::path::Path, rate: u32) -> Player {
    let (arc, melodic, drums) = parts(dir);
    let d = arc.device(Fm::DEVICE).expect("the OPL3 driver");
    Player::new(rate, d, &melodic, &drums).expect("the tables are where they are said to be")
}

fn song(dir: &std::path::Path, id: usize) -> Song {
    let bank = Bank::open_dir(dir).expect("the resource banks open");
    let block = bank
        .item(Res::Block, id)
        .expect("the bank reads")
        .expect("a block");
    Song::parse(block).expect("a song")
}

/// The dominant frequency of a signal, by autocorrelation.
///
/// Fifteen lines instead of a Fourier transform and a crate to go with it: the
/// question here is only "what is the period", and for one sustained tone the
/// first strong correlation peak answers it.
fn pitch(samples: &[i16], rate: u32, low: f64, high: f64) -> f64 {
    let mean = samples.iter().map(|&s| s as f64).sum::<f64>() / samples.len() as f64;
    let x: Vec<f64> = samples.iter().map(|&s| s as f64 - mean).collect();
    let min_lag = (rate as f64 / high).floor().max(2.0) as usize;
    let max_lag = (rate as f64 / low).ceil() as usize;
    let mut best = (min_lag, f64::MIN);
    for lag in min_lag..max_lag.min(x.len() / 2) {
        let c: f64 = x[..x.len() - lag]
            .iter()
            .zip(&x[lag..])
            .map(|(a, b)| a * b)
            .sum();
        if c > best.1 {
            best = (lag, c);
        }
    }
    rate as f64 / best.0 as f64
}

/// The frequency multiplier the chip reads out of the `0x20` register's low
/// nibble. Not a plain number: 0 halves, and four of the sixteen slots repeat.
fn multiple(nibble: u8) -> f64 {
    match nibble & 0x0f {
        0 => 0.5,
        11 | 12 => 12.0,
        13 | 14 => 15.0,
        n => n as f64,
    }
}

// ----------------------------------------------------------------- the seam

/// T1: the register address the chip is handed is the one the driver meant.
///
/// On an OPL3 the second bank *is* the ninth address bit, so the driver's
/// `(bank, reg)` needs no translation — but only as long as `address()` really
/// puts the bank there. Everything downstream silently collapses onto one bank
/// if it does not, and the sound merely goes mono; T4 is what would notice.
#[test]
fn a_write_addresses_the_bank_it_belongs_to() {
    assert_eq!(
        Write {
            bank: 0,
            reg: 0x40,
            value: 0
        }
        .address(),
        0x040
    );
    assert_eq!(
        Write {
            bank: 1,
            reg: 0x40,
            value: 0
        }
        .address(),
        0x140
    );
    assert_eq!(
        Write {
            bank: 1,
            reg: 0xbd,
            value: 0
        }
        .address(),
        0x1bd
    );
}

/// T2: nothing playing is silence, and it stays silence.
///
/// The switch-on sequence runs in `Player::new`, so the chip has been written
/// to before a single note; if any of those writes keyed something on, this is
/// where it shows. *Does not see*: a chip that is never driven at all — T3.
#[test]
fn silence_is_silent() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut p = player(&dir, 48_000);
    let mut out = vec![0i16; 48_000 * 2];
    p.fill(&mut out);
    assert!(
        out.iter().all(|&s| s == 0),
        "the chip sounds before anything asked it to"
    );
    assert!(!p.playing());
}

/// T3: a note comes out at its own pitch.
///
/// The expected frequency is worked out from the chip's own arithmetic —
/// `fnum × 49716 / 2^(20 − block)`, times the carrier's frequency multiplier —
/// with the f-number taken from the driver's table at the index the driver
/// uses. *Cross-check*: drop the `- 12` from that indexing and the tone lands an
/// octave out, well past the tolerance here.
#[test]
fn a_note_sounds_at_its_own_pitch() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let (arc, melodic, _) = parts(&dir);
    let tables = Tables::read(arc.device(Fm::DEVICE).unwrap()).unwrap();
    let patches = motionvm_audio::opl::convert_bank(&melodic);

    let rate = 48_000u32;
    for note in [57u8, 69, 81] {
        let mut p = player(&dir, rate);
        // Program 0 is `PIANO1`; the driver reads a channel's program at
        // note-on, and an untouched channel is on program 0 already.
        p.send(Message {
            channel: 0,
            kind: Kind::NoteOn {
                note,
                velocity: 127,
            },
        });
        let mut out = vec![0i16; rate as usize / 2 * 2];
        p.fill(&mut out);

        let packed = tables.frequency[note as usize - 12];
        let (block, fnum) = ((packed >> 10) & 7, packed & 0x3ff);
        let carrier_multiple = multiple(patches[0].am_vib[1]);
        let want = fnum as f64 * 49716.0 / (1u64 << (20 - block)) as f64 * carrier_multiple;

        // The attack is over long before the second half of the buffer.
        let tail = &out[out.len() / 2..];
        let mono: Vec<i16> = tail.as_chunks::<2>().0.iter().map(|c| c[0]).collect();
        let got = pitch(&mono, rate, want * 0.5, want * 2.0);
        let cents = 1200.0 * (got / want).log2();
        assert!(
            cents.abs() < 40.0,
            "note {note}: wanted {want:.1} Hz, measured {got:.1} Hz ({cents:+.0} cents)"
        );
    }
}

/// T4: panning puts a voice on one side.
///
/// This is also the only test that would notice the bank bit going missing: the
/// driver's whole stereo mechanism is the same voice written to both banks with
/// one of the two copies turned down, so a stream that collapsed onto one bank
/// would come out with both sides equal.
///
/// **It records what our code does, not which speaker that is.** The driver
/// quietens the first bank when controller 10 is at or above 64. Which
/// physical side that is cannot be settled from a register recording: bank 0
/// takes `0xC0` bit 5 and bank 1 bit 4, which on the chip are channel B and
/// channel A — right and left in the ordinary wiring — yet the driver quietens
/// bank 0 as controller 10 moves *right*. Only the wiring can answer it, so
/// this test asserts the separation and not the orientation.
#[test]
fn panning_lands_on_one_side() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let rate = 48_000u32;
    let energy = |pan: u8| -> (f64, f64) {
        let mut p = player(&dir, rate);
        p.send(Message {
            channel: 0,
            kind: Kind::Control {
                controller: 10,
                value: pan,
            },
        });
        p.send(Message {
            channel: 0,
            kind: Kind::NoteOn {
                note: 69,
                velocity: 127,
            },
        });
        let mut out = vec![0i16; rate as usize / 4 * 2];
        p.fill(&mut out);
        let mut l = 0.0;
        let mut r = 0.0;
        for f in out.as_chunks::<2>().0 {
            l += (f[0] as f64).powi(2);
            r += (f[1] as f64).powi(2);
        }
        (l.sqrt(), r.sqrt())
    };

    let (center_l, center_r) = energy(64);
    assert!(
        center_l > 0.0 && center_r > 0.0,
        "a centered note sounds on both sides"
    );

    let (hard_l, hard_r) = energy(0);
    let (other_l, other_r) = energy(127);
    // Hard over is hard over: one side is a great deal quieter than the other,
    // and the two extremes lean opposite ways.
    assert!(
        hard_l / hard_r > 4.0 || hard_r / hard_l > 4.0,
        "pan 0 is not hard over: {hard_l} {hard_r}"
    );
    assert!(
        (hard_l > hard_r) != (other_l > other_r),
        "pan 0 and pan 127 lean the same way: {hard_l}/{hard_r} against {other_l}/{other_r}"
    );
    // And the center really is between them.
    let spread = |a: f64, b: f64| (a - b).abs() / (a + b);
    assert!(
        spread(center_l, center_r) < spread(hard_l, hard_r),
        "the center is no more even than hard over"
    );
}

/// T5: the clock runs at the rate the original really used.
///
/// **Not the rate the song header names.** A song counts its deltas in 120ths
/// of a second, but the sequencer rides on the sound layer's master timer
/// rather than getting one of its own, and 1500.86 / 120 is 12.507 master
/// ticks. The original takes 13 of them, so the music runs at 115.45 Hz — four
/// per cent slower than the header says. That number is measured against the
/// recording; see `Player::master_ticks`.
///
/// *Cross-check*: clock the player at the header's 120 Hz and five seconds carry
/// 600 ticks instead of 577, which is the drift T6 sees as a piece slowly
/// running away from itself.
#[test]
fn the_clock_runs_at_the_rate_the_original_did() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let s = song(&dir, 25);
    assert_eq!(s.tick_hz, 120, "block 25 asks for 120ths");
    assert_eq!(
        Player::master_ticks(120),
        13,
        "and gets thirteen master ticks each"
    );

    for rate in [44_100u32, 48_000, 22_050, 32_000] {
        let seconds = 5u64;
        let mut whole = player(&dir, rate);
        whole.start(song(&dir, 25));
        assert!(
            (whole.tick_hz() - 115.450).abs() < 0.001,
            "{}",
            whole.tick_hz()
        );

        let mut out = vec![0i16; rate as usize * seconds as usize * 2];
        whole.fill(&mut out);
        // 115.45 Hz for five seconds is 577 ticks, and one either way is the
        // most the boundary can cost.
        let want = (whole.tick_hz() * seconds as f64).round() as u64;
        assert!(
            whole.ticks().abs_diff(want) <= 1,
            "at {rate} Hz five seconds carried {} ticks, not {want}",
            whole.ticks()
        );
        assert!(
            out.iter().any(|&s| s != 0),
            "at {rate} Hz nothing was heard"
        );

        // And the same five seconds in ragged little pieces, the way an audio
        // device really asks. The clock has to survive being cut anywhere,
        // including in the middle of the gap between two ticks.
        let mut piecemeal = player(&dir, rate);
        piecemeal.start(song(&dir, 25));
        let mut buf = vec![0i16; 1024];
        let mut left = rate as usize * seconds as usize;
        let mut size = 1;
        while left > 0 {
            let frames = size.min(left).min(buf.len() / 2);
            piecemeal.fill(&mut buf[..frames * 2]);
            left -= frames;
            size = size * 3 % 509 + 1;
        }
        assert_eq!(
            piecemeal.ticks(),
            whole.ticks(),
            "at {rate} Hz the clock depends on how the buffer was cut"
        );
    }
}
