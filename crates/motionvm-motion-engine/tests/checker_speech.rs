//! Checker 2000 speaks: the game's own speech layer, live on the one game
//! that ships its voice.
//!
//! `STARTUP` asks `?SOUND` once and keeps the answer in `_SPEECH`, and every
//! scene that talks reads that cell when it is entered: with it set, the
//! location macro puts the task manager on a speech path (`_LOCTASK` 1001,
//! 1050, 1100, …), where `->SPEECHSEQ` plays a WAV file out of `WAVS/`
//! through `->STARTSAMPLE`, a cue table drives the talking head through
//! `SAMPLE_TIMING`, and the scene moves on when `?STIME` says the file is
//! over; without it, the caption path (`_LOCTASK` 1, 50, 100, …), where
//! the same lines are put up as text through `SETT1` and timed by their
//! length. So a voiced game shows no subtitles, and a silent one speaks
//! nothing — the two are the same script on two paths, and which one runs
//! is the sound layer's answer.
//!
//! `?SOUND` (R78 `0x6b340`) tests the digital driver's status bit, which
//! here is a sink being attached: these tests attach one that plays nothing
//! and writes down what it was asked for, drive the game through the
//! registration and both *Play it!* menus into the classroom — the first
//! scene that talks — and hold the frames against a DOSBox-X recording of
//! the original (`docs/motion/verification.md`). They
//! need the game's files (`MOTIONVM_GAMEDATA_CHECKER`) and skip without
//! them.
//!
//! The game this file drives is Checker 2000 (MOTION 32-bit).

mod common;

use common::settle;
use motionvm_motion_engine::{Game, MusicSink, titles};
use motionvm_motion_forth::m32::Vm;
use motionvm_motion_testutil::gamedata_checker;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// A sink that writes down what it was asked for.
#[derive(Default, Clone)]
struct Log(Arc<Mutex<Vec<String>>>);

impl Log {
    fn lines(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

impl MusicSink for Log {
    fn start(&mut self, _handle: i32, tune: i32, looping: bool, _song: &[u8]) {
        self.0
            .lock()
            .unwrap()
            .push(format!("tune {tune} {looping}"));
    }
    fn stop(&mut self, _handle: i32) {
        self.0.lock().unwrap().push("stop".into());
    }
    fn cut(&mut self, _handle: i32) {
        self.0.lock().unwrap().push("cut".into());
    }
    fn sample(&mut self, block: i32, _sample: &[u8]) {
        self.0
            .lock()
            .unwrap()
            .push(format!("16-bit sample {block}"));
    }
    fn start_sample(&mut self, handle: i32, wav: &[u8], volume: u16, loops: i32) {
        self.0.lock().unwrap().push(format!(
            "sample {handle} {}B {volume:#x} x{loops}",
            wav.len()
        ));
    }
    fn stop_sample(&mut self, handle: i32) {
        self.0.lock().unwrap().push(format!("stop sample {handle}"));
    }
    fn music_volume(&mut self, volume: u16) {
        self.0.lock().unwrap().push(format!("music {volume:#x}"));
    }
}

/// The game open with or without a sound layer, started, and the shell's
/// registration settled.
fn at_the_registration(dir: &Path, sound: Option<Log>) -> Game<Vm> {
    let mut game = titles::checker::open(dir).expect("opens");
    if let Some(log) = sound {
        game.engine.set_music(Box::new(log));
    }
    let saves =
        std::env::temp_dir().join(format!("motionvm-checker-speech-{}", std::process::id()));
    std::fs::create_dir_all(&saves).expect("a save directory");
    game.set_saves(&saves).expect("saves");
    game.start().expect("START starts");
    while game.pump().expect("START runs to ANIMPLAY") {}
    common::frames(&mut game, 60, "the registration board");
    game
}

/// One game frame: a step, and every curtain step after it until the
/// interpreter runs again. A curtain waits for nothing on this build, so
/// its steps cost the clock nothing and the original counts no frame for
/// them; counting them here would count what the original does not.
fn frame(game: &mut Game<Vm>, x: i32, y: i32, left: bool, key: i32) {
    game.set_input(x, y, left, false, key).expect("input");
    game.step().expect("a frame");
    settle(game);
}

fn press(game: &mut Game<Vm>, key: i32) {
    frame(game, 320, 240, false, key);
    for _ in 0..3 {
        frame(game, 320, 240, false, 0);
    }
}

fn click(game: &mut Game<Vm>, x: i32, y: i32) {
    frame(game, x, y, true, 0);
    frame(game, x, y, false, 0);
}

/// Through the registration and both menus to the first story task: the
/// name, the postcode, *Play it!* on the main menu, *Play it!* on the
/// Futurespiel menu. Answers with the story's first task entered.
fn into_the_story(game: &mut Game<Vm>) {
    for key in [65, 66, 13, 49, 50, 51, 13] {
        press(game, key);
    }
    for _ in 0..30 {
        frame(game, 320, 240, false, 0);
    }
    assert_eq!(game.get_var(2, "_IBNR"), Some(2), "the main menu");
    click(game, 118, 91);
    for _ in 0..30 {
        frame(game, 320, 240, false, 0);
    }
    assert_eq!(game.get_var(2, "_IBNR"), Some(6), "the Futurespiel menu");
    click(game, 150, 70);
    assert_eq!(game.get_var(2, "_IBON"), Some(0), "the board is down");
    assert_eq!(game.get_var(2, "_ACTLOC"), Some(6), "task 1's slide show");
}

/// Frames until the game is in location `n`, or a panic past `limit`.
fn frames_until_location(game: &mut Game<Vm>, n: i32, limit: usize) -> usize {
    for f in 1..=limit {
        frame(game, 320, 240, false, 0);
        if game.get_var(2, "_ACTLOC") == Some(n) {
            return f;
        }
    }
    panic!("location {n} was not entered within {limit} frames");
}

/// The classroom, location 7, is the second story task and the first scene
/// that talks: `START_KLASSE` (module 307) reads `_SPEECH` and puts the
/// task manager at 1001, where `LTMANAGER` (module 207) plays `1_2.WAV`
/// with the cue table `_DTABLE1` and the teacher's mouth as the speaker.
///
/// The frames are the original's, read off a recording at 70 frames a
/// second: the classroom comes up as the task step's curtain opens, the
/// speech begins 0.66 s later — six frames — and the scene leaves for the
/// street 1.03 s after the file's last sample, `SPEECHSEQ->` having seen
/// `?STIME` answer −1 and the manager having waited its five frames.
#[test]
fn the_classroom_speaks_its_first_line_and_leaves_when_it_is_over() {
    let Some(dir) = gamedata_checker() else {
        eprintln!("skipping: no Checker 2000 gamedata directory");
        return;
    };
    let log = Log::default();
    let mut game = at_the_registration(&dir, Some(log.clone()));
    assert_eq!(
        game.get_var(2, "_SPEECH"),
        Some(1),
        "?SOUND answered for the sound layer"
    );
    into_the_story(&mut game);

    // Task 1 is the slide show of location 6, with its own tune; task 2 is
    // the classroom.
    frames_until_location(&mut game, 7, 400);
    assert_eq!(game.get_var(2, "_LOCTASK"), Some(1001), "the speech path");
    let before = log.lines().len();
    let mut started_after = None;
    for f in 1..=12 {
        frame(&mut game, 320, 240, false, 0);
        if log.lines().len() > before {
            started_after = Some(f);
            break;
        }
    }
    // `1_2.WAV`: 468 490 bytes, 22 050 Hz 16-bit mono, 10.62 s, at the
    // layer's full volume — streamed, being over 128 KB.
    assert_eq!(
        log.lines()[before..],
        ["sample 3 468490B 0x7fff x0"],
        "the classroom's first line, and nothing else"
    );
    assert_eq!(
        started_after,
        Some(6),
        "six frames after the curtain opened"
    );
    let sample = game.engine.samples().last().cloned().expect("the node");
    assert!(sample.stream, "streamed");
    assert_eq!(sample.duration, 2124, "468 446 data bytes × 100 / 22 050");
    // No subtitle on the speech path.
    assert!(
        !game
            .engine
            .descriptors()
            .iter()
            .any(|d| d.is_text() && d.active),
        "a voiced scene shows no caption"
    );

    // The manager waits at 1002 for `SPEECHSEQ->` to answer 0, which is
    // `?STIME` answering −1: the DAC has run dry, 85 frames on.
    let mut over_after = None;
    for f in 1..=100 {
        frame(&mut game, 320, 240, false, 0);
        if game.get_var(2, "_LOCTASK") != Some(1002) {
            over_after = Some(f);
            break;
        }
    }
    assert_eq!(over_after, Some(85), "the file is 84.98 frames long");
    assert_eq!(game.get_var(2, "_LOCTASK"), Some(1003));
    // Five frames of `_TASKWAIT`, one for `EXITAHANDLER`, and the task step
    // enters the next scene — task 2's step wrote 4 into `_TASK`, so that is
    // location 5, the schoolyard — with `1 MUSVOLUME` at the step.
    let left_after = frames_until_location(&mut game, 5, 12);
    assert_eq!(left_after, 7, "the scene leaves seven frames after the end");
    assert_eq!(
        log.lines()[before + 1..],
        ["music 0x7fff"],
        "the task step restores the music; neither scene has a tune of its own"
    );
    for _ in 0..6 {
        frame(&mut game, 320, 240, false, 0);
    }
    assert_eq!(
        log.lines().last().map(String::as_str),
        Some("sample 4 2246282B 0x7fff x0"),
        "the schoolyard's first line, `4_5.WAV`, six frames in"
    );
}

/// The same scene without a sound layer: `?SOUND` answers 0, the macro
/// takes the caption path, and the teacher's lines are text.
#[test]
fn without_a_sound_layer_the_classroom_shows_its_lines_as_captions() {
    let Some(dir) = gamedata_checker() else {
        eprintln!("skipping: no Checker 2000 gamedata directory");
        return;
    };
    let mut game = at_the_registration(&dir, None);
    assert_eq!(game.get_var(2, "_SPEECH"), Some(0));
    into_the_story(&mut game);
    frames_until_location(&mut game, 7, 400);
    assert_eq!(game.get_var(2, "_LOCTASK"), Some(1), "the caption path");
    // The same six frames in — the task step's `5 _TASKWAIT !` — the line
    // is a caption: `1 SAY_LEHRER`, table 2's first text, on `_TI1`.
    let mut shown_after = None;
    for f in 1..=12 {
        frame(&mut game, 320, 240, false, 0);
        let captions: Vec<(i32, i32)> = game
            .engine
            .descriptors()
            .iter()
            .filter(|d| d.is_text() && d.active)
            .filter_map(|d| Some((d.shows.table()?, d.text?)))
            .collect();
        if !captions.is_empty() {
            assert_eq!(captions, [(2, 1)]);
            shown_after = Some(f);
            break;
        }
    }
    assert_eq!(shown_after, Some(6));
    assert!(game.engine.samples().is_empty(), "and nothing is played");
}
