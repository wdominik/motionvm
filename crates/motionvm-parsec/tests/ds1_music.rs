//! The game's music through the front door: every song of `PSMP2L.DAT`
//! through the game's own `MUSADL.DRV`, and the music a window opens.
//!
//! These need the original files; point `MOTIONVM_GAMEDATA_DS1` at the
//! directory with `RESOURCE/`, or they skip themselves.
//!
//! The byte-for-byte check of the register stream is not here — it runs
//! against OPL captures of the original under DOSBox-X, outside this
//! repository: module 0 as the title program plays it agrees for all 4296
//! writes the capture holds, and module 1 as the Zimmer plays it for the
//! first 1705, up to the room's exit fade. What is here is what can be
//! asked of the shipped files alone, and the digest of every song's
//! stream, which holds the streams to what they were.
//!
//! The game this file drives is Dunkle Schatten (PARSEC).

use motionvm_parsec::PARSEC;
use motionvm_parsec_formats::detect::resource_dir;
use motionvm_parsec_formats::{P2l, find_ci};
use motionvm_parsec_testutil::{Digests, digest::Digest, gamedata_ds1};
use motionvm_playable::{Button, Family};
use motionvm_psm::{Driver, Plx, Sequencer};

/// This game's table of reference digests.
fn digests() -> Digests {
    Digests::of(env!("CARGO_MANIFEST_DIR"), "ds1")
}

/// A file of the install's `RESOURCE/`.
fn resource(dir: &std::path::Path, name: &str) -> Vec<u8> {
    let res = resource_dir(dir);
    std::fs::read(find_ci(&res, name).unwrap_or_else(|| panic!("{name}"))).expect(name)
}

/// Every module's song, as the Ad Lib path hands it over, played endlessly
/// for 20 000 ticks: notes keyed, nothing written past the OPL2's bank, the
/// song still playing — and the whole stream the same, write for write, as
/// the digest the table holds.
#[test]
fn every_song_plays_notes_and_digests_as_it_did() {
    let Some(dir) = gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let library = P2l::parse(resource(&dir, "PSMP2L.DAT")).expect("PSMP2L.DAT");
    let driver = motionvm_parsec_formats::p2l::ad_lib_driver(&resource(&dir, "PSMDRV.DAT"))
        .expect("MUSADL.DRV");
    let driver = Driver::parse(&driver).expect("the driver's tables");
    for n in 0..library.modules.len() {
        let song = Plx::from_section(library.song(n).expect("a song")).expect("a PLX");
        let mut seq = Sequencer::new(&driver, song, -1);
        let mut writes = Sequencer::install_writes(&driver);
        for _ in 0..20_000 {
            seq.tick(&mut writes);
        }
        assert!(
            writes.iter().all(|w| w.address() < 0x100),
            "module {n} wrote past the OPL2 bank"
        );
        assert!(
            writes
                .iter()
                .any(|w| (0xb0..=0xb8).contains(&w.reg) && w.value & 0x20 != 0),
            "module {n} never keyed a note"
        );
        assert!(seq.playing(), "module {n} stopped although it loops");
        let mut d = Digest::new();
        for w in &writes {
            d.byte(w.bank).byte(w.reg).byte(w.value);
        }
        digests().check(&format!("module_{n:02}"), d.value());
    }
}

/// A window that opens the music hears the title program's menu: module 0
/// sounds once the menu has faded in.
#[test]
fn the_title_menu_is_heard_through_the_front_door() {
    let Some(dir) = gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let rate = 44_100;
    let mut game = PARSEC.open(&dir).expect("open");
    let mut source = game
        .open_music(rate)
        .expect("the music comes up")
        .expect("a game with music");
    game.start().expect("start");
    // A click skips the intro to the menu.
    game.button(Button::Left, true);
    game.button(Button::Left, false);
    let mut buf = vec![0i16; 2 * 4410];
    let mut peak = 0;
    for _ in 0..600 {
        game.step().expect("a step");
        source.fill(&mut buf);
        peak = peak.max(buf.iter().map(|s| s.unsigned_abs()).max().unwrap_or(0));
    }
    assert!(peak > 500, "the menu is all but silent: peak {peak}");
}

/// A copy without the music library still plays, and says why there is no
/// sound.
#[test]
fn a_copy_without_the_music_library_says_why_sound_is_off() {
    let Some(dir) = gamedata_ds1() else {
        eprintln!("skipping: no Dunkle Schatten gamedata directory");
        return;
    };
    let copy = motionvm_parsec_testutil::saves_dir("ds1-no-music");
    let res = copy.join("RESOURCE");
    std::fs::create_dir_all(&res).expect("a RESOURCE directory");
    for exe in ["XP00.EXE", "XP01.EXE"] {
        let from = find_ci(&dir, exe).expect(exe);
        std::fs::copy(from, copy.join(exe)).expect(exe);
    }
    for entry in std::fs::read_dir(resource_dir(&dir)).expect("RESOURCE/") {
        let path = entry.expect("an entry").path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if !name.eq_ignore_ascii_case("PSMP2L.DAT") {
            std::fs::copy(&path, res.join(name)).expect(name);
        }
    }
    let mut game = PARSEC
        .open(&copy)
        .expect("the game opens without its music");
    let Err(e) = game.open_music(44_100) else {
        panic!("music without its library");
    };
    assert_eq!(e.to_string(), "RESOURCE/PSMP2L.DAT: not found");
}
