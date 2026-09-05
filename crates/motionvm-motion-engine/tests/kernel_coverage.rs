//! Every kernel word the shipped games reach for is built.
//!
//! A game's modules call kernel words by ordinal, and each ordinal is one of
//! three things here: a primitive the interpreter owns
//! (`motionvm_motion_forth::m32::IMPLEMENTED` and its 16-bit twin), a word the
//! engine implements ([`motionvm_motion_engine::Engine::implements`]), or a
//! word the run stops on with `Unimplemented` the first time a script reaches
//! it. This suite disassembles every module a game ships, counts what it
//! reaches for, and asserts the third set is empty — so the number the
//! inspection CLI's `kernel-usage.txt` cannot give, because that tool
//! deliberately knows no engine, is given here, where both halves are visible.
//!
//! Static, not played: a word in a module the game never runs counts too.
//! That is the stronger claim, and the one worth holding — a word only a
//! debugger's path reaches is still a word a player could meet the day that
//! path is taken. The suites that play the games say which words are *inert*;
//! this one says which are *absent*. For the four 16-bit games the answer is
//! none. Dunkle Schatten 2 names seven, every one of them out of reach, and
//! [`UNBUILT_DS2`] carries each with the reason it stays unbuilt — so a word
//! that goes missing is a red test, and so is one of the seven getting built
//! without its line coming out.
//!
//! Needs the games' files and skips, game by game, without them. The games
//! this file drives are all five: Dunkle Schatten 2 (MOTION 32-bit) and Die
//! Enviro-Kids greifen ein, Jeff Jet, Hilfe für Amajambere and Victor Loomes
//! (MOTION 16-bit).

use motionvm_motion_engine::Engine;
use motionvm_motion_formats::Generation;
use motionvm_motion_formats::m16::{self, Container, Segment, mz};
use motionvm_motion_formats::m32::{self, Kind, rsc::Bank};
use motionvm_motion_forth as forth;
use motionvm_motion_testutil::{
    game_file, gamedata_ds2, gamedata_enviro, gamedata_hfa, gamedata_jeffjet, gamedata_vloomes,
};
use std::path::Path;

/// The kernel words Dunkle Schatten 2's modules name that the engine does
/// not implement, each with why it is left that way. Every one is
/// unreachable in the shipped game, and the reason says from what.
///
/// The addresses are the modules' own, as `motionvm-motion-tools script`
/// lists them.
const UNBUILT_DS2: &[(&str, &str)] = &[
    (
        "?STIME",
        "the sampled-speech pump: `SAMPLE_TIMING` (module 4, `0x00080`) and \
         `SPEECHSEQ->` (module 5, `0x028f0`), which nothing calls — the \
         speech path plays WAV files no copy of the game ships",
    ),
    (
        "->STARTSAMPLE",
        "`->SPEECHSEQ` (module 5, `0x02860`), the start of the same speech \
         path, which nothing calls",
    ),
    (
        "VIEWG8",
        "module 312, the scene macro of location 12, whose location-table \
         entry is uninitialized in the shipped data so the jump goes nowhere \
         in the original too; and module 399, a sprite inspector from the \
         authoring environment shipped by accident and never loaded",
    ),
    (
        "->SCREEN",
        "module 312, location 12's scene macro — unreachable, as above",
    ),
    (
        "GGFXYLEN",
        "module 312, location 12's scene macro — unreachable, as above",
    ),
    (
        "XYCUT",
        "module 330, a scene macro no location-table entry names; and module \
         399, the authoring sprite inspector",
    ),
    (
        "INTERPRET$",
        "the shell's live Forth input line (module 4, `0x051a0`), inside a \
         debug layer gated on `_DEBUGON`: module 2 initializes the cell to 0 \
         and every one of the six stores to it, all in `ICTRL`, sits behind a \
         test that it is already non-zero, so the shipped game cannot open \
         the layer",
    ),
];

/// The kernel words a 32-bit game reaches for that neither the interpreter
/// nor the engine implements, with how often each is reached for.
fn unbuilt_m32(dir: &Path) -> Vec<(String, usize)> {
    let bank = Bank::open_dir(dir).expect("the containers open");
    let img = m32::le::Image::open(game_file(dir, "ENGINE.EXE")).expect("ENGINE.EXE reads");
    let kernel = m32::le::kernel_words(&img);
    let mut dis = m32::disasm::Disassembler::new(&kernel);
    let mut modules = Vec::new();
    for (_, id) in bank.present(Kind::Script) {
        let item = bank
            .item(Kind::Script, id)
            .expect("the index reads")
            .expect("a present slot has bytes");
        let m = m32::ScrModule::parse(item).expect("a shipped module parses");
        dis.learn(&m);
        modules.push(m);
    }
    dis.usage(&modules)
        .into_iter()
        .filter(|(name, _)| {
            !forth::m32::IMPLEMENTED.contains(&name.as_str())
                && !Engine::implements(name, Generation::Motion32)
        })
        .collect()
}

/// The same for a 16-bit game, whose kernel is read out of `exe`.
fn unbuilt_m16(dir: &Path, exe: &str) -> Vec<(String, usize)> {
    let c = Container::open_dir(dir).expect("the container opens");
    let img = mz::Image::open(game_file(dir, exe)).expect("the player reads");
    let words = mz::kernel_words(&img);
    let binding = mz::binding_of(&img, &words).expect("the kernel binds");
    let mut dis = m16::disasm::Disassembler::new(&binding);
    let mut modules = Vec::new();
    for id in c.present(Segment::Scr) {
        let item = c
            .item(Segment::Scr, id)
            .expect("the index reads")
            .expect("a present slot has bytes");
        let m = m16::ScrModule::parse(item).expect("a shipped module parses");
        dis.learn(&m);
        modules.push(m);
    }
    dis.usage(&modules)
        .into_iter()
        .filter(|(name, _)| {
            !forth::m16::IMPLEMENTED.contains(&name.as_str())
                && !Engine::implements(name, Generation::Motion16)
        })
        .collect()
}

/// Fails with the list, one word per line with its count, so the report says
/// what a player could meet rather than only that something is missing.
fn assert_all_built(game: &str, unbuilt: &[(String, usize)]) {
    let lines: Vec<String> = unbuilt
        .iter()
        .map(|(name, n)| format!("  {name:<16} {n:>6}"))
        .collect();
    assert!(
        unbuilt.is_empty(),
        "{game} reaches for {} kernel words nothing implements:\n{}",
        unbuilt.len(),
        lines.join("\n")
    );
}

/// The set is held both ways: a word that goes missing fails, and so does a
/// word that gets built while its line stays — the list is a claim about the
/// engine and has to stay true in both directions.
#[test]
fn every_word_dunkle_schatten_2_reaches_for_is_built_or_named_here() {
    let Some(dir) = gamedata_ds2() else {
        eprintln!("skipping: no Dunkle Schatten 2 gamedata directory");
        return;
    };
    let unbuilt = unbuilt_m32(&dir);
    let found: Vec<&str> = unbuilt.iter().map(|(name, _)| name.as_str()).collect();
    let named: Vec<&str> = UNBUILT_DS2.iter().map(|&(name, _)| name).collect();
    let missing: Vec<_> = unbuilt
        .iter()
        .filter(|(name, _)| !named.contains(&name.as_str()))
        .cloned()
        .collect();
    assert_all_built("Dunkle Schatten 2", &missing);
    let built: Vec<&str> = named
        .iter()
        .copied()
        .filter(|name| !found.contains(name))
        .collect();
    assert!(
        built.is_empty(),
        "these are listed as unbuilt and are built now; take their lines out: {built:?}"
    );
}

/// Every reason names where the word is reached from, which is what makes
/// the list reviewable rather than a list of exemptions.
#[test]
fn every_unbuilt_word_says_where_it_is_reached_from() {
    for (name, why) in UNBUILT_DS2 {
        assert!(why.contains("module"), "{name}: the reason names no module");
    }
}

#[test]
fn every_word_die_enviro_kids_reach_for_is_built() {
    let Some(dir) = gamedata_enviro() else {
        eprintln!("skipping: no Die Enviro-Kids greifen ein gamedata directory");
        return;
    };
    assert_all_built(
        "Die Enviro-Kids greifen ein",
        &unbuilt_m16(&dir, "ENVIRO.EXE"),
    );
}

#[test]
fn every_word_jeff_jet_reaches_for_is_built() {
    let Some(dir) = gamedata_jeffjet() else {
        eprintln!("skipping: no Jeff Jet gamedata directory");
        return;
    };
    assert_all_built("Jeff Jet", &unbuilt_m16(&dir, "HPPLAY.EXE"));
}

#[test]
fn every_word_hilfe_fuer_amajambere_reaches_for_is_built() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    assert_all_built("Hilfe für Amajambere", &unbuilt_m16(&dir, "BMZ.EXE"));
}

#[test]
fn every_word_victor_loomes_reaches_for_is_built() {
    let Some(dir) = gamedata_vloomes() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    assert_all_built("Victor Loomes", &unbuilt_m16(&dir, "LL.EXE"));
}
