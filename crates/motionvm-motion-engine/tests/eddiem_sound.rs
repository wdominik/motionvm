//! The two kernel words only Falsches Spiel mit Eddie M. reaches: `PLAYSAMPLE`
//! and `GIVEDATE`.
//!
//! Both are read out of `STERN.EXE`, the one build whose game calls them, and
//! both are answered by the engine as the generation's. `GIVEDATE` is the DOS
//! date pushed day, month, year; `PLAYSAMPLE` is two halves: a tune that is
//! playing is cut — no fade, a half-second hold, the driver's Stop — and then
//! the block goes to the digital driver, whole. What is asked here is that the
//! words take and leave what the handlers do, that the cut reaches the sink
//! as a cut and not as the fading stop `ENDTUNE` asks for, and that the sample
//! reaches it after the hold and not before.
//!
//! These need the game's files (`MOTIONVM_GAMEDATA_EDDIEM`) and skip without
//! them.
//!
//! The game this file drives is Falsches Spiel mit Eddie M. (MOTION 16-bit).

mod common;

use common::word16;
use motionvm_motion_engine::{Game, MusicSink, titles};
use motionvm_motion_forth::m16::Vm;
use motionvm_motion_testutil::gamedata_eddiem;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// A sink that writes down what it was asked for.
#[derive(Default, Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl MusicSink for Log {
    fn start(&mut self, _handle: i32, tune: i32, looping: bool, _song: &[u8]) {
        self.0
            .lock()
            .unwrap()
            .push(format!("start {tune} {looping}"));
    }
    fn stop(&mut self, _handle: i32) {
        self.0.lock().unwrap().push("stop".into());
    }
    fn cut(&mut self, _handle: i32) {
        self.0.lock().unwrap().push("cut".into());
    }
    fn start_sample(&mut self, _handle: i32, _wav: &[u8], _volume: u16, _loops: i32) {}
    fn stop_sample(&mut self, _handle: i32) {}
    fn music_volume(&mut self, _volume: u16) {}
    fn sample(&mut self, block: i32, sample: &[u8]) {
        self.0
            .lock()
            .unwrap()
            .push(format!("sample {block} {}B", sample.len()));
    }
}

/// The game open, with a recording sink and a fixed date, `RUN` parked in
/// the intro's first `ANIMPLAY` — enough for the words to run against.
fn open(dir: &Path) -> (Game<Vm>, Log) {
    let mut game = titles::eddiem::open(dir).expect("opens");
    let log = Log::default();
    game.engine.set_music(Box::new(log.clone()));
    game.engine.fix_date(13, 10, 1994);
    game.start().expect("RUN starts");
    while game.pump().expect("RUN runs to ANIMPLAY") {}
    (game, log)
}

#[test]
fn givedate_pushes_day_month_year_with_the_year_on_top() {
    let Some(dir) = gamedata_eddiem() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
        return;
    };
    let (mut game, _) = open(&dir);
    // The order the handler pushes `dl`, `dh` and `cx` of INT 21h/2Ah in
    // (`STERN.EXE` `0cd3:37d9`), which is the order `STNR` pops them:
    // `_KJAHR ! _KMONAT ! _KTAG !`. The date is the shipped files' own.
    assert_eq!(word16(&mut game, "GIVEDATE", &[]), [13, 10, 1994]);
}

#[test]
fn playsample_takes_two_and_plays_the_block_while_no_tune_plays() {
    let Some(dir) = gamedata_eddiem() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
        return;
    };
    let (mut game, log) = open(&dir);
    // `13 0 PLAYSAMPLE`, the shape of every effect call in the rooms: block
    // and mode go, nothing comes back, and with the driver's flag clear the
    // handler skips straight to the second half — the block, whole, to the
    // driver's play entry. Block 13 is 2494 bytes: the ten-byte header and
    // 2484 of PCM.
    assert_eq!(word16(&mut game, "PLAYSAMPLE", &[13, 0]), []);
    assert!(!game.engine.in_transition(), "nothing to wait for");
    assert_eq!(log.0.lock().unwrap().clone(), ["sample 13 2494B"]);
    // A block the game does not ship plays nothing, as a tune it does not
    // ship plays nothing.
    assert_eq!(word16(&mut game, "PLAYSAMPLE", &[999, 0]), []);
    assert_eq!(log.0.lock().unwrap().len(), 1, "no block, no call");
}

#[test]
fn playsample_cuts_a_playing_tune_and_plays_the_block_when_the_wait_is_over() {
    let Some(dir) = gamedata_eddiem() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
        return;
    };
    let (mut game, log) = open(&dir);
    // `-1 24 STARTTUNE`, as the intro plays its module: the 16-bit word pops
    // both and answers nothing.
    assert_eq!(word16(&mut game, "STARTTUNE", &[-1, 24]), []);
    assert_eq!(log.0.lock().unwrap().clone(), ["start 24 true"]);
    // The effect over the playing tune: the tune is cut — the sink is asked
    // for the hard stop, not the fade — the script is held for the half
    // second the handler spins (`cmp $0x64` against the 200 Hz tick), and
    // the sample is not yet asked for: the original loads the block only
    // after the spin.
    assert_eq!(word16(&mut game, "PLAYSAMPLE", &[17, 0]), []);
    assert_eq!(log.0.lock().unwrap().clone(), ["start 24 true", "cut"]);
    assert!(
        game.engine.in_transition(),
        "the half-second hold is queued"
    );
    // The hold runs out over the frames — a hundred ticks, thirteen a frame
    // — and the sample follows where the tune stopped.
    let mut frames = 0;
    while game.engine.in_transition() {
        game.set_input(160, 100, false, false, 0).expect("input");
        game.step().expect("a frame of the hold");
        frames += 1;
        assert!(frames < 30, "the hold never ended");
    }
    assert_eq!(
        log.0.lock().unwrap().clone(),
        ["start 24 true", "cut", "sample 17 31285B"]
    );
    // `ENDTUNE` over a tune, for the contrast: the fading stop, and no
    // sample.
    word16(&mut game, "STARTTUNE", &[-1, 25]);
    assert_eq!(word16(&mut game, "ENDTUNE", &[]), []);
    assert_eq!(
        log.0.lock().unwrap().clone(),
        [
            "start 24 true",
            "cut",
            "sample 17 31285B",
            "start 25 true",
            "stop"
        ]
    );
}
