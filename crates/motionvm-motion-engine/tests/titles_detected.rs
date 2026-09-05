//! Every game is told apart by the engine binary beside its container, and
//! opens as itself.
//!
//! One row per game, where four boot suites had a copy of the same test each.
//! The question is the same for all of them — does `titles::detect` name this
//! directory, and does `titles::open` hand back a game that says its own name,
//! its own display size and its own pixel shape — and only the answers differ.
//! A copy per game is how one of them loses an assertion without anyone
//! noticing: the enviro copy had no pixel-aspect check at all. A missing row
//! here is visible as a missing row.
//!
//! Each row skips on its own, so a machine that has three of the five games
//! still checks those three, and every assertion names the game it is about.
//!
//! The games this file drives are all five: Im Netzwerk gefangen – Dunkle
//! Schatten 2 (MOTION 32-bit), and Die Enviro-Kids greifen ein, Jeff Jet -
//! Abenteuer InfoHighway, Hilfe für Amajambere and Victor Loomes – Das Spiel
//! (MOTION 16-bit).

use motionvm_motion_engine::{Title, titles};
use motionvm_motion_testutil::{
    gamedata_ds2, gamedata_enviro, gamedata_hfa, gamedata_jeffjet, gamedata_vloomes,
};
use motionvm_playable::{PixelAspect, Size};
use std::path::PathBuf;

/// What one game's directory has to answer.
struct Game {
    /// Where a copy of this game is, or `None` on a machine without one.
    data: fn() -> Option<PathBuf>,
    /// The binary that tells this game from its siblings. All four 16-bit
    /// games ship a `DATA.-1-`; only the binary beside it says which is which.
    binary: &'static str,
    title: Title,
    /// The full title the window shows, spelled out rather than taken from
    /// [`Title::name`]: on the 16-bit side the roster answers with exactly
    /// that method, so reading it back from there would assert nothing.
    name: &'static str,
    /// The mode the game runs in.
    size: Size,
    /// What one of its pixels stood as on the monitor it was written for.
    aspect: PixelAspect,
}

const GAMES: &[Game] = &[
    Game {
        data: gamedata_ds2,
        binary: "ENGINE.EXE",
        title: Title::DunkleSchatten2,
        name: "Im Netzwerk gefangen – Dunkle Schatten 2",
        // 640×480 on a 4:3 monitor: the grid already matches, so the pixels
        // are square.
        size: Size {
            width: 640,
            height: 480,
        },
        aspect: PixelAspect {
            width: 1,
            height: 1,
        },
    },
    Game {
        data: gamedata_enviro,
        binary: "ENVIRO.EXE",
        title: Title::DieEnviroKidsGreifenEin,
        name: "Die Enviro-Kids greifen ein",
        size: Size {
            width: 320,
            height: 200,
        },
        // The 320×200×256 mode `TOGFX` enters filled a 4:3 monitor, so one
        // pixel stood (4/3)/(320/200) = 6/5 as tall as wide: a 5:6 pixel.
        aspect: PixelAspect {
            width: 5,
            height: 6,
        },
    },
    Game {
        data: gamedata_jeffjet,
        binary: "HPPLAY.EXE",
        title: Title::JeffJet,
        name: "Jeff Jet - Abenteuer InfoHighway",
        size: Size {
            width: 320,
            height: 200,
        },
        aspect: PixelAspect {
            width: 5,
            height: 6,
        },
    },
    Game {
        data: gamedata_hfa,
        binary: "BMZ.EXE",
        title: Title::HilfeFuerAmajambere,
        name: "Hilfe für Amajambere",
        size: Size {
            width: 320,
            height: 200,
        },
        aspect: PixelAspect {
            width: 5,
            height: 6,
        },
    },
    Game {
        data: gamedata_vloomes,
        binary: "LL.EXE",
        title: Title::VictorLoomes,
        name: "Victor Loomes – Das Spiel",
        size: Size {
            width: 320,
            height: 200,
        },
        aspect: PixelAspect {
            width: 5,
            height: 6,
        },
    },
];

#[test]
fn every_game_is_told_apart_by_its_engine_binary_and_opens_as_itself() {
    for game in GAMES {
        // The short form in prose and in the skip line, the full one in the
        // assertion: a message that carried the subtitle twice would read
        // worse than one that says it once.
        let name = game.title.short();
        let Some(dir) = (game.data)() else {
            eprintln!("skipping: no {name} gamedata directory");
            continue;
        };
        assert_eq!(
            titles::detect(&dir),
            Some(game.title),
            "{name} is not told apart by {}",
            game.binary
        );
        let opened = titles::open(&dir).unwrap_or_else(|e| panic!("{name} opens: {e}"));
        assert_eq!(opened.name(), game.name, "{name}'s full title");
        assert_eq!(opened.display_size(), game.size, "{name}'s mode");
        assert_eq!(opened.pixel_aspect(), game.aspect, "{name}'s pixel shape");
    }
}

/// The roster is the whole roster: every title the engine knows has a row
/// above.
///
/// Without this the file passes for the wrong reason — a game added to
/// `Title` and forgotten here would simply not be checked, which is the
/// failure a table is supposed to make impossible.
#[test]
fn every_title_the_engine_knows_has_a_row() {
    let listed: Vec<Title> = GAMES.iter().map(|g| g.title).collect();
    assert_eq!(listed, Title::ALL, "a title with no row above");
}
