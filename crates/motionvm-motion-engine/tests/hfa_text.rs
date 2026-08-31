//! Hilfe für Amajambere puts the right words on its pages.
//!
//! Every other test in this suite asks whether text was *drawn* — how wide,
//! how tall, what color the outline is. None of them has ever asked **which
//! text**, and that is the gap this file closes: a descriptor can be pointed
//! at the wrong table and still draw a perfectly shaped line of the wrong
//! words.
//!
//! This game is where it mattered. Its diary and menu pages switch tables with
//! `SDBL`, not `SDTB` — `16 SDBL 1 SDTXT` in module 320, and again per page in
//! module 120's `PUTDIARY` — and on the 16-bit machine those two words are one
//! store to the descriptor's `+0x10`. While motionvm kept a separate field per
//! word, `SDBL` never reached the table, the page stayed on the 11 that
//! `NEWTEXT.` had set, and the intro read out that table's hotspot captions:
//! *Schule*, *Brunnen*, *Baum*, *Tafel*, *Antony's Hof*.
//!
//! These need the game's files (`MOTIONVM_GAMEDATA_HFA`) and skip without them.
//!
//! The game this file drives is Hilfe für Amajambere (MOTION 16-bit).

use motionvm_motion_engine::{Descriptor, Game, titles};
use motionvm_motion_forth::m16::Vm;
use motionvm_motion_testutil::gamedata_hfa;
use std::path::Path;

/// `RUN` through the intro, stopping where the menu is up over location 20.
fn into_the_menu(dir: &Path) -> Game<Vm> {
    let mut game = titles::hfa::open(dir).expect("opens");
    game.start().expect("RUN starts");
    while game.pump().expect("RUN runs to the intro's ANIMPLAY") {}
    let mut left_intro = false;
    for frame in 1..=8000 {
        let key = if frame % 200 == 0 { 32 } else { 0 };
        game.set_input(160, 100, false, false, key).expect("input");
        game.step()
            .unwrap_or_else(|e| panic!("frame {frame} stopped: {e}"));
        if !game.engine.main_loop() {
            left_intro = true;
        } else if left_intro {
            return game;
        }
    }
    panic!("8000 frames and the intro never gave way to CTRL");
}

/// Every line of text the active descriptors resolve to, longest first.
fn shown(game: &mut Game<Vm>) -> Vec<String> {
    let active: Vec<Descriptor> = game
        .engine
        .descriptors()
        .iter()
        .filter(|d| d.active && d.is_text())
        .cloned()
        .collect();
    let mut out: Vec<String> = active
        .iter()
        .filter_map(|d| game.engine.descriptor_text(d))
        .filter(|s| !s.trim().is_empty())
        .collect();
    out.sort_by_key(|s| std::cmp::Reverse(s.len()));
    out
}

#[test]
fn the_menu_page_reads_the_table_sdbl_pointed_it_at() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    let mut game = into_the_menu(&dir);
    let lines = shown(&mut game);
    let page = lines
        .first()
        .unwrap_or_else(|| panic!("the menu page carries no text at all"));

    // Table 16 entry 1 — the paragraph the game opens on. Module 320 asks for
    // it with `16 SDBL 1 SDTXT`, so a descriptor still reading table 11 is the
    // whole bug in one assertion.
    assert!(
        page.starts_with("In den Dörfern an den Hängen"),
        "the menu page should open the story, and reads: {page:?}"
    );
    for caption in ["Schule", "Brunnen", "Baum", "Tafel", "Antony's Hof"] {
        assert!(
            !lines.iter().any(|l| l.trim() == caption),
            "{caption:?} is a hotspot caption out of table 11, not page text: {lines:?}"
        );
    }
}

#[test]
fn the_six_intro_pages_read_table_16_in_order() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    // Table 16 holds exactly six entries and the intro has exactly six pages;
    // `PUTDIARY` walks them with `?LTPHASE`. Reading them out of the container
    // rather than spelling them out here keeps the test about the wiring.
    let container = motionvm_motion_formats::m16::Container::open_dir(&dir).expect("the container");
    let table = motionvm_motion_formats::m16::text::parse(
        container
            .item(motionvm_motion_formats::m16::Segment::Txt, 16)
            .expect("the TXT segment")
            .expect("table 16"),
    )
    .expect("table 16 parses");
    assert_eq!(table.strings.len(), 6, "one entry per intro page");

    let mut game = into_the_menu(&dir);
    // Out of the menu — `CTRL` honours nothing while `_INVMODE` is 2 — and
    // then through the pages, collecting what each one says.
    for (press, n) in [(false, 2), (true, 2), (false, 2)] {
        for _ in 0..n {
            game.set_input(30, 185, press, false, 0).expect("input");
            game.step().expect("the menu takes the click");
        }
    }
    let mut seen: Vec<String> = Vec::new();
    for frame in 1..=3000 {
        game.set_input(160, 100, frame % 120 == 0, false, 0)
            .expect("input");
        game.step().unwrap_or_else(|e| panic!("frame {frame}: {e}"));
        for line in shown(&mut game) {
            if !seen.contains(&line) {
                seen.push(line);
            }
        }
    }

    // Every page that came up has to be one of table 16's, and they have to
    // arrive in the table's own order.
    let order: Vec<usize> = seen
        .iter()
        .filter_map(|line| table.strings.iter().position(|s| s == line))
        .collect();
    assert!(
        order.len() >= 2,
        "at least two intro pages should have been read out of table 16, saw {seen:?}"
    );
    assert!(
        order.windows(2).all(|w| w[0] < w[1]),
        "the pages should arrive in the table's order, got {order:?}"
    );
}
