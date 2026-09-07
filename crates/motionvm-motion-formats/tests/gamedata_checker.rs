//! End-to-end checks against the files Checker 2000 ships.
//!
//! These need the original files. Point `MOTIONVM_GAMEDATA_CHECKER` at the
//! directory holding `ENGINE.RSC`, `001.RSC` etc.; the tests skip themselves
//! if it is missing, so the crate still builds and tests cleanly without the
//! game.
//!
//! The second game on the 32-bit engine, and the earlier build of it —
//! `ENGINE.EXE` V0.04.15/R78 against Dunkle Schatten 2's V0.06.06/R109 — so
//! what this file holds is mostly what the sibling suite holds *differently*:
//! a fifth container, `ENGINE.RSC`, standing in for the loose `000.FNT` and
//! `000.PAL`; ids filled in two containers at once; a kernel of 375 words
//! bound at another base; blocks that are WAV files; and the two readings
//! the builds disagree on — `SDINSERT`'s arity and whether a curtain waits.
//!
//! The game this file drives is Checker 2000 (MOTION 32-bit).

use motionvm_motion_formats::m32::rsc::{Bank, ENGINE_CONTAINER};
use motionvm_motion_formats::m32::{Kind, Sprite, Wav, le};
use motionvm_motion_testutil::{game_file, gamedata_checker};

macro_rules! bank_or_skip {
    () => {
        match gamedata_checker() {
            Some(dir) => Bank::open_dir(&dir).expect("open RSC banks"),
            None => {
                eprintln!("skipping: no Checker 2000 gamedata directory");
                return;
            }
        }
    };
}

/// Five containers, `ENGINE.RSC` first: it is the one the engine registers
/// first and so the one its lookup lands on for an id filled twice.
#[test]
fn engine_rsc_is_the_first_of_five_containers() {
    let bank = bank_or_skip!();
    let sources: Vec<String> = bank
        .banks()
        .iter()
        .map(|b| {
            std::path::Path::new(b.source())
                .file_name()
                .map(|n| n.to_string_lossy().to_uppercase())
                .unwrap_or_default()
        })
        .collect();
    assert_eq!(
        sources,
        [ENGINE_CONTAINER, "001.RSC", "002.RSC", "003.RSC", "004.RSC"]
    );
    // What each holds, as the inspection CLI counts it.
    let per_bank: Vec<Vec<(Kind, usize)>> = bank
        .banks()
        .iter()
        .map(|b| {
            Kind::ALL
                .iter()
                .map(|&k| (k, b.present(k).len()))
                .filter(|(_, n)| *n > 0)
                .collect()
        })
        .collect();
    assert_eq!(
        per_bank[0],
        [(Kind::Font, 1), (Kind::Palette, 1)],
        "ENGINE.RSC holds the system font and palette"
    );
    assert_eq!(
        per_bank[1],
        [
            (Kind::Gfx8, 33),
            (Kind::Text, 264),
            (Kind::Block, 80),
            (Kind::Script, 119),
            (Kind::Palette, 65),
        ]
    );
    assert_eq!(per_bank[2], [(Kind::Gfx8, 1321), (Kind::Font, 3)]);
    assert_eq!(per_bank[3], [(Kind::Gfx8, 88), (Kind::Palette, 88)]);
    assert_eq!(per_bank[4], [(Kind::Block, 43)]);
    assert_eq!(bank.present(Kind::Gfx8).len(), 1442);
    assert_eq!(bank.present(Kind::Gfx16).len(), 0, "no hicolor sprites");
}

/// Font 0 and palette 0 — the files Dunkle Schatten 2 ships loose as
/// `000.FNT` and `000.PAL` — come out of `ENGINE.RSC` here, and are what the
/// system font and the startup palette are.
#[test]
fn the_system_font_and_palette_live_in_engine_rsc() {
    let bank = bank_or_skip!();
    let font = bank
        .item(Kind::Font, 0)
        .expect("readable")
        .expect("font 0 present");
    let font = motionvm_motion_formats::m32::font::parse(font).expect("font 0 parses");
    assert!(font.glyphs.len() > 90, "{} glyphs", font.glyphs.len());
    assert_eq!(font.height, 12, "the system font's height");
    let pal = bank
        .item(Kind::Palette, 0)
        .expect("readable")
        .expect("palette 0 present");
    assert_eq!(pal.len(), 768);
    assert!(pal.iter().all(|&v| v <= 0x3f), "six-bit DAC values");
    // `000.FRT` is loose here as there.
    let frt = motionvm_motion_formats::font::FontRefTable::parse(
        &std::fs::read(game_file(&gamedata_checker().unwrap(), "000.FRT")).unwrap(),
    )
    .unwrap();
    assert!(frt.glyph_for(b'A').is_some());
}

/// Ids filled in more than one container. The engine's lookup takes the
/// lowest slot — `ENGINE.RSC`, then `001.RSC` — and so does the bank; what
/// this pins is which ids the rule decides at all.
#[test]
fn ids_filled_twice_resolve_to_the_first_container() {
    let bank = bank_or_skip!();
    let mut twice: Vec<(Kind, usize, Vec<usize>)> = Vec::new();
    for &kind in Kind::ALL.iter() {
        let mut by_id: std::collections::BTreeMap<usize, Vec<usize>> = Default::default();
        for (slot, id) in bank.present(kind) {
            by_id.entry(id).or_default().push(slot);
        }
        for (id, slots) in by_id {
            if slots.len() > 1 {
                twice.push((kind, id, slots));
            }
        }
    }
    let sprites: Vec<usize> = twice
        .iter()
        .filter(|(k, ..)| *k == Kind::Gfx8)
        .map(|(_, id, _)| *id)
        .collect();
    let palettes: Vec<usize> = twice
        .iter()
        .filter(|(k, ..)| *k == Kind::Palette)
        .map(|(_, id, _)| *id)
        .collect();
    assert_eq!(sprites.len(), 19, "{sprites:?}");
    assert_eq!(palettes, [147], "one palette, in 001.RSC and 003.RSC");
    assert_eq!(twice.len(), 20, "nothing else is filled twice: {twice:?}");
    // Every one of them resolves to the first container that holds it, and
    // for sixteen of the nineteen sprites the two pictures differ — which is
    // why the rule matters; the other three are the same picture twice.
    let mut differ = 0;
    let mut pictures_differ = 0;
    for (kind, id, slots) in &twice {
        let first = bank.banks()[slots[0]].item(*kind, *id).unwrap().unwrap();
        let second = bank.banks()[slots[1]].item(*kind, *id).unwrap().unwrap();
        assert_eq!(
            bank.item(*kind, *id).unwrap().unwrap(),
            first,
            "{kind:?} {id}"
        );
        if first != second {
            differ += 1;
        }
        if *kind == Kind::Gfx8 {
            let a = Sprite::parse(first).unwrap();
            let b = Sprite::parse(second).unwrap();
            if (a.width, a.height, &a.pixels) != (b.width, b.height, &b.pixels) {
                pictures_differ += 1;
            }
        }
    }
    assert_eq!(differ, 16, "items whose two copies differ");
    assert_eq!(pictures_differ, 16, "sprites whose two pictures differ");
}

/// The digital sound is blocks that are WAV files, which `STARTSAMPLE`
/// plays through the HMI digital layer at their own rate; the rest of the
/// blocks are the songs and the game's data tables.
#[test]
fn the_wav_blocks_parse_and_say_their_shape() {
    let bank = bank_or_skip!();
    let mut wavs = Vec::new();
    let mut shapes: std::collections::BTreeMap<(u16, u32, u16), usize> = Default::default();
    let mut others = Vec::new();
    for (slot, id) in bank.present(Kind::Block) {
        let item = bank.item(Kind::Block, id).unwrap().unwrap();
        if Wav::is_wav(item) {
            let wav = Wav::parse(item).unwrap_or_else(|e| panic!("block {id}: {e}"));
            assert!(wav.frames() > 0, "block {id} is empty");
            *shapes
                .entry((wav.channels, wav.rate, wav.bits))
                .or_default() += 1;
            wavs.push((slot, id));
        } else {
            others.push((slot, id));
        }
    }
    // Two in `001.RSC` — block 3 is `TEST.WAV`, the sound setup's test file,
    // block 4 a stereo one — and thirty-nine in `004.RSC`, ids 11 to 49,
    // every one of them 22 050 Hz mono 16-bit: the speech and the effects.
    let in_first: Vec<usize> = wavs
        .iter()
        .filter(|(s, _)| *s == 1)
        .map(|(_, id)| *id)
        .collect();
    let in_fourth: Vec<usize> = wavs
        .iter()
        .filter(|(s, _)| *s == 4)
        .map(|(_, id)| *id)
        .collect();
    assert_eq!(in_first, [3, 4]);
    assert_eq!(in_fourth, (11..=49).collect::<Vec<_>>());
    assert_eq!(wavs.len(), 41);
    assert_eq!(
        shapes,
        [
            ((1, 22_050, 8), 1),
            ((1, 22_050, 16), 39),
            ((2, 11_025, 8), 1)
        ]
        .into_iter()
        .collect()
    );
    // What is not a WAV: the songs (1 and 60 to 64, `HMI-MIDISONG`), one
    // headerless sample (2), `STARTUP`'s 200-byte block 99, and a data block
    // per location in the 100s and 200s.
    let songs: Vec<usize> = others
        .iter()
        .map(|(_, id)| *id)
        .filter(|&id| {
            bank.item(Kind::Block, id)
                .unwrap()
                .unwrap()
                .starts_with(b"HMI-MIDISONG")
        })
        .collect();
    assert_eq!(songs, [1, 60, 61, 62, 63, 64]);
    assert_eq!(others.len(), 123 - 41);
}

/// The highscore's texts carry the `#`-formatter's directives: five `#s` a
/// text, one per insert slot.
#[test]
fn the_highscore_texts_hold_the_format_directives() {
    let bank = bank_or_skip!();
    let item = bank.item(Kind::Text, 4).unwrap().expect("text table 4");
    let t = motionvm_motion_formats::m32::text::parse(item).unwrap();
    assert_eq!(t.get(0), Some("#s<"), "the name entry, cursor and all");
    assert_eq!(
        t.get(25),
        Some("1. #s\n\n2. #s\n\n3. #s\n\n4. #s\n\n5. #s\n")
    );
    assert_eq!(t.get(26), Some("#s \n\n#s \n\n#s \n\n#s \n\n#s "));
    assert_eq!(t.get(27), Some("#i.\n\n#i.\n\n#i.\n\n#i.\n\n#i.\n\n"));
}

/// The earlier build's image and kernel: fewer pages, fewer words, and the
/// same three tables plus the shell's words registered one by one — so the
/// same derivation binds it, at a base of its own.
#[test]
fn engine_executable_relocates_and_yields_the_kernel_word_table() {
    let Some(dir) = gamedata_checker() else {
        eprintln!("skipping: no Checker 2000 gamedata directory");
        return;
    };
    let img = le::Image::open(game_file(&dir, "ENGINE.EXE")).expect("parse LE");
    assert_eq!(img.pages(), 146, "page count");
    assert_eq!(img.fixups_applied(), 15075, "32-bit fixups applied");
    assert_eq!(img.objects().len(), 4, "object count");
    assert!(img.objects()[0].executable());

    let words = le::kernel_words(&img);
    let mut per_group = [0usize; 4];
    for w in &words {
        per_group[w.table] += 1;
    }
    assert_eq!(per_group, [91, 27, 214, 43], "group sizes");
    assert_eq!(words.len(), 375);

    let by_name = |n: &str| {
        words
            .iter()
            .find(|w| w.name == n)
            .unwrap_or_else(|| panic!("kernel word {n} missing"))
    };
    for (name, table, index) in [
        ("##", 0, 0),
        ("DUP", 0, 7),
        ("DROP", 0, 10),
        ("VAR", 0, 17),
        ("CONST", 0, 18),
        ("CREATE", 0, 19),
        (":", 1, 0),
        ("TOGFX", 2, 0),
        ("NEWSCREEN", 2, 20),
        ("TEST", 3, 0),
        ("->RSCPATH", 3, 33),
    ] {
        let w = by_name(name);
        assert_eq!((w.table, w.index), (table, index), "{name} position");
    }

    // The ordinals the compiled modules call by, read out of this build's own
    // init: 91 core words after the first at 104 put the compiling table at
    // 584 and the domain table at 934, where R109's starts at 1039.
    let binding = le::binding_of(&img, &words).expect("the kernel binds");
    assert_eq!(binding.len(), 91 + 27 + 214 + 1 + 43, "with `_FNAME`");
    for (name, ordinal) in [
        ("##", 104),
        ("DUP", 139),
        (":", 584),
        ("TEST", 719),
        ("->RSCPATH", 884),
        ("TOGFX", 934),
        ("NEWSCREEN", 1034),
        ("SDINSERT", 1844),
        ("GIVEDATE", 1924),
    ] {
        assert_eq!(binding.ordinal(name), Some(ordinal), "{name}");
    }
}

/// The one point the two builds' text handling differs on, read off the
/// handler: this build's `SDINSERT` pops two, Dunkle Schatten 2's three.
#[test]
fn sdinsert_takes_no_kind_in_this_build() {
    let Some(dir) = gamedata_checker() else {
        eprintln!("skipping: no Checker 2000 gamedata directory");
        return;
    };
    let img = le::Image::open(game_file(&dir, "ENGINE.EXE")).expect("parse LE");
    let words = le::kernel_words(&img);
    assert!(!le::sdinsert_takes_kind(&img, &words).unwrap());
}

/// The other point: this build's curtains wait for nothing. Neither fade
/// handler divides the duration by the band count or spins on the timer
/// (`0x60360`, `0x60560`); Dunkle Schatten 2's both do.
#[test]
fn the_curtains_do_not_wait_in_this_build() {
    let Some(dir) = gamedata_checker() else {
        eprintln!("skipping: no Checker 2000 gamedata directory");
        return;
    };
    let img = le::Image::open(game_file(&dir, "ENGINE.EXE")).expect("parse LE");
    let words = le::kernel_words(&img);
    assert!(!le::fades_wait(&img, &words).unwrap());
}
