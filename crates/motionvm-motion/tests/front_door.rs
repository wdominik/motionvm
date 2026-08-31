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
    let frame = game.render();
    assert_eq!((frame.width, frame.height), game.display_size());
}
