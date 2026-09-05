//! The family's front door answers coherently, with no game data in the room.

use motionvm_motion::MOTION;
use motionvm_motion_engine::titles::Title;
use motionvm_playable::Family;
use motionvm_playable::Playable;

/// A window may carry an opened game across threads; the contract promises
/// it, and this compiles or it stopped being true.
#[test]
fn the_boxed_contract_is_send() {
    fn is_send<T: Send>() {}
    is_send::<Box<dyn Playable>>();
}

/// The cards a window shows are built from the roster the openers serve, so
/// the two counts cannot differ — and a card carries all three of its names.
#[test]
fn the_card_list_is_the_roster() {
    let cards = MOTION.games();
    assert_eq!(cards.len(), Title::ALL.len());
    for (card, title) in cards.iter().zip(Title::ALL) {
        assert_eq!(card.name, title.name());
        assert_eq!(card.short, title.short());
        assert_eq!(card.needs, title.needs());
    }
}

/// A directory with none of the family's files is not claimed — which is what
/// lets a roster ask every family before any of them opens anything.
#[test]
fn an_empty_directory_is_nobodys() {
    let dir = scratch("detect");
    assert!(MOTION.detect(&dir).is_none());
    let _ = std::fs::remove_dir(&dir);
}

/// Opening it anyway answers the refusal a player reads — pinned here word
/// for word, with no game data in the room, because this is the family's
/// most visible sentence: the roster it prints is the roster the openers
/// serve.
#[test]
fn the_refusal_names_every_game_and_what_it_needs() {
    let dir = scratch("open");
    let Err(refusal) = MOTION.open(&dir) else {
        panic!("an empty directory opened");
    };
    let message = refusal.to_string();
    let expected = format!(
        "{} is not a game motionvm can open\n\
         \x20 Dunkle Schatten 2 needs 001.RSC and ENGINE.EXE\n\
         \x20 Die Enviro-Kids greifen ein needs DATA.-1- and ENVIRO.EXE\n\
         \x20 Jeff Jet needs DATA.-1-, DATA.-2- and HPPLAY.EXE\n\
         \x20 Hilfe für Amajambere needs DATA.-1-, DATA.-2- and BMZ.EXE\n\
         \x20 Victor Loomes needs DATA.-1- and LL.EXE\n\
         \x20 Falsches Spiel mit Eddie M. needs DATA.-1-, DATA.-2-, DATA.-3- and STERN.EXE\n\
         \x20 Another MOTION game, or an incomplete copy of one of these; \
         see \"What a game needs\" in the README.",
        dir.display()
    );
    assert_eq!(message, expected);
    let _ = std::fs::remove_dir(&dir);
}

/// An empty scratch directory of this test's own, under the system's
/// temporary directory.
fn scratch(what: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("motionvm-front-door-{what}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// The whole way through the front door, on the real game: open, music at
/// the device rate, start to the parked loop, a few frames, the picture —
/// every call the window makes, in the order it makes them. Needs the game's
/// files and skips without them.
///
/// The game this file drives is Im Netzwerk gefangen – Dunkle Schatten 2
/// (MOTION 32-bit).
#[test]
fn dunkle_schatten_2_plays_through_the_contract() {
    let Some(dir) = motionvm_motion_testutil::gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = MOTION.open(&dir).expect("opens");
    assert_eq!(game.name(), "Im Netzwerk gefangen – Dunkle Schatten 2");
    let source = game.open_music(44_100).expect("music comes up");
    assert!(source.is_some(), "this family always answers a source");
    game.start().expect("startup parks");
    for _ in 0..25 {
        game.step().expect("a frame");
    }
    let size = game.display_size();
    let frame = game.frame();
    assert_eq!(
        (frame.pixels.width, frame.pixels.height),
        (size.width, size.height)
    );
}

/// What a run has to say about itself reaches the window through the
/// contract, and is not printed by the library on the way.
///
/// The engine's departures ledger says what a run walked past is counted and
/// named at the end of it. This is the channel that makes that sentence true,
/// so it is worth a test that the channel carries. Twenty-five frames of
/// Dunkle Schatten 2's title reach five words that are deliberately inert; the
/// stray reads that the same ledger promises need a location that has one, and
/// are pinned in the engine's own suite.
#[test]
fn a_run_says_what_it_walked_past() {
    let Some(dir) = motionvm_motion_testutil::gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let mut game = MOTION.open(&dir).expect("opens");
    // Nothing has run, so there is nothing to report yet — which says the
    // report is of the run and not of the game.
    assert!(game.diagnostics().is_empty(), "an unstarted game is silent");

    game.start().expect("startup parks");
    for _ in 0..25 {
        game.step().expect("a frame");
    }
    let notes = game.diagnostics();
    let inert = notes
        .iter()
        .find(|d| d.subject == "words reached that do nothing")
        .unwrap_or_else(|| panic!("no inert-word line in {notes:?}"));
    // Which words those are is the engine suite's business — `NO_EFFECT` is
    // where they are argued for. What is asserted here is that the count
    // reached the far side of the contract at all.
    assert!(inert.detail.contains('×'), "{}", inert.detail);
    assert!(
        inert
            .to_string()
            .starts_with("words reached that do nothing: ")
    );
    // Asking again answers again: it is the report as it stands, not a queue.
    assert_eq!(game.diagnostics(), notes);
}
