//! End-to-end checks against the files Falsches Spiel mit Eddie M. ships.
//!
//! These need the original files. Point `MOTIONVM_GAMEDATA_EDDIEM` at the
//! directory holding `DATA.-1-`, `DATA.-2-`, `DATA.-3-` and `STERN.EXE`; the
//! tests skip themselves if it is missing, so the crate still builds and tests
//! cleanly without the game. Every number asserted here is a measurement over
//! those files.
//!
//! What this corpus adds to the other four 16-bit ones: **three volumes**, the
//! most any game ships, so the occupancy word is read as the bitmask it is —
//! `4` names the third volume, where a count would have stopped at two; a
//! container whose second and third volumes leave a gap between their table
//! and their first item; and blocks of two kinds no sibling has, a bare `PLX`
//! jingle beside two `MTCVTS` modules, and thirteen `SM8` samples on their
//! own. Its player is the second-oldest build, with `LL.EXE`'s eighty-word
//! core table under `HPPLAY.EXE`'s domain table.
//!
//! The game this file drives is Falsches Spiel mit Eddie M. (MOTION 16-bit).

use motionvm_motion_formats::font::FontRefTable;
use motionvm_motion_formats::m16::{Container, Segment, disasm, font, gfx, mz, psm, scr, text};
use motionvm_motion_testutil::{game_file, gamedata_eddiem};

macro_rules! container_or_skip {
    () => {
        match gamedata_eddiem() {
            Some(dir) => Container::open_dir(&dir).expect("open the three volumes"),
            None => {
                eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
                return;
            }
        }
    };
}

/// The occupied ids of a segment, as a sanity-checked list.
fn ids(c: &Container, seg: Segment) -> Vec<usize> {
    let v = c.present(seg);
    for &id in &v {
        assert!(
            c.item(seg, id).unwrap().is_some(),
            "{} {id} is present but has no item",
            seg.name()
        );
    }
    v
}

#[test]
fn the_header_names_three_volumes_and_seven_packed_segments() {
    let c = container_or_skip!();
    assert_eq!(c.boot().module, 100);
    assert_eq!(
        c.boot().word,
        411,
        "RUN — id 411 here, 401 in the three later games"
    );
    let counts: Vec<usize> = Segment::ALL.iter().map(|&s| c.slot_count(s)).collect();
    // The later framing's geometry with the smallest text segment of any
    // game: 60 slots, where the siblings reserve 100 or 150.
    assert_eq!(counts, [2500, 1000, 700, 25, 10, 10, 60]);
    assert_eq!((c.volumes(), c.spare_offsets()), (3, 1));
    assert_eq!(c.first_item_offset(), 25872);
    assert!(
        Segment::ALL.iter().all(|&s| c.packed(s)),
        "every segment of this game is packed"
    );
    assert_eq!(
        c.trailing_slack(),
        0,
        "every byte of the three volumes after their tables belongs to an item"
    );
    assert!(
        c.occupancy_mismatches().is_empty(),
        "every occupancy word agrees with the offsets"
    );
}

#[test]
fn the_later_volumes_open_on_a_table_and_a_gap() {
    let Some(dir) = gamedata_eddiem() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
        return;
    };
    for volume in ["DATA.-2-", "DATA.-3-"] {
        let raw = std::fs::read(game_file(&dir, volume)).expect("the volume reads");
        let first = u32::from_le_bytes(raw[..4].try_into().unwrap());
        // 4 × (4305 slots + 1 spare) is 17 224; the first item begins 24
        // bytes past the table's end. This is the game that says the table's
        // first entry is where items *may* begin, not where they must.
        assert_eq!(first, 17248, "{volume}");
    }
}

#[test]
fn every_segment_holds_what_the_game_ships() {
    let c = container_or_skip!();
    let n: Vec<usize> = Segment::ALL.iter().map(|&s| ids(&c, s).len()).collect();
    assert_eq!(n, [1772, 119, 62, 25, 2, 1, 51]);
    let bytes: usize = Segment::ALL
        .iter()
        .flat_map(|&s| ids(&c, s).into_iter().map(move |id| (s, id)))
        .map(|(s, id)| c.item(s, id).unwrap().unwrap().len())
        .sum();
    assert_eq!(bytes, 9_352_179, "unpacked, from 2 677 683 bytes stored");
}

#[test]
fn the_occupancy_word_is_a_bitmask_and_the_third_volume_is_artwork() {
    let c = container_or_skip!();
    // 1, 2 and 4: the word names a volume by bit, which only a game on three
    // volumes can show — on two, a count and a mask read the same.
    let mut on_volume = [0usize; 3];
    for id in ids(&c, Segment::Gfx) {
        match c.occupancy(Segment::Gfx, id).unwrap() {
            1 => on_volume[0] += 1,
            2 => on_volume[1] += 1,
            4 => on_volume[2] += 1,
            other => panic!("sprite {id} carries occupancy word {other}"),
        }
    }
    assert_eq!(on_volume, [3, 995, 774]);
    // Volume 3 is sprites and nothing else; volume 2 holds the rest of the
    // artwork with every palette, both fonts, the font reference table, the
    // thirteen samples and the two songs; volume 1 is what the game runs and
    // says — every script and text — with the per-location tables and the
    // animation catalogs, three sprites and the jingle.
    for seg in [Segment::Pal, Segment::Fnt, Segment::Frt] {
        for id in ids(&c, seg) {
            assert_eq!(c.occupancy(seg, id).unwrap(), 2, "{} {id}", seg.name());
        }
    }
    for seg in [Segment::Scr, Segment::Txt] {
        for id in ids(&c, seg) {
            assert_eq!(c.occupancy(seg, id).unwrap(), 1, "{} {id}", seg.name());
        }
    }
    let on_two: Vec<usize> = ids(&c, Segment::Blk)
        .into_iter()
        .filter(|&id| c.occupancy(Segment::Blk, id).unwrap() == 2)
        .collect();
    assert_eq!(
        on_two,
        [1, 9, 10, 12, 13, 14, 15, 17, 18, 20, 21, 22, 23, 24, 25],
        "the samples and the two modules are on volume 2"
    );
    assert!(
        ids(&c, Segment::Blk)
            .into_iter()
            .all(|id| c.occupancy(Segment::Blk, id).unwrap() != 4),
        "no block is on volume 3"
    );
}

#[test]
fn every_sprite_unpacks_to_exactly_its_header_plus_its_pixels() {
    let c = container_or_skip!();
    let mut sizes = std::collections::BTreeSet::new();
    let (mut widest, mut tallest) = (0, 0);
    for id in ids(&c, Segment::Gfx) {
        let item = c.item(Segment::Gfx, id).unwrap().unwrap();
        let s = gfx::Sprite::parse(item).unwrap_or_else(|e| panic!("sprite {id}: {e}"));
        assert_eq!(s.pixels.len(), usize::from(s.width) * usize::from(s.height));
        assert!(
            s.width > 0 && s.height > 0,
            "sprite {id} has a zero dimension"
        );
        assert_eq!(
            item.len(),
            usize::from(s.width) * usize::from(s.height) + 6,
            "sprite {id} unpacks to header and pixels and nothing else"
        );
        sizes.insert((s.width, s.height));
        widest = widest.max(s.width);
        tallest = tallest.max(s.height);
    }
    assert_eq!(sizes.len(), 443, "distinct sizes");
    // A full-width backdrop strip and a sprite as tall as the world viewport:
    // this game's screens are 320×165 over the 35-row strip, and its rooms
    // are 960 wide.
    assert_eq!((widest, tallest), (320, 155));
}

#[test]
fn every_palette_is_768_six_bit_bytes() {
    let c = container_or_skip!();
    let ids = ids(&c, Segment::Pal);
    assert_eq!(
        ids,
        (0..25).collect::<Vec<_>>(),
        "every one of the 25 slots"
    );
    for id in ids {
        let item = c.item(Segment::Pal, id).unwrap().unwrap();
        assert_eq!(item.len(), 768, "palette {id}");
        assert!(
            item.iter().all(|&b| b <= 63),
            "palette {id} has a value above 63"
        );
    }
}

#[test]
fn the_two_fonts_decode_and_read_lsb_first() {
    let c = container_or_skip!();
    // Two, at 0 and 2 with slot 1 empty, both of 103 glyphs — the count Hilfe
    // für Amajambere's font 7 has, one more than the 102 of the others'.
    assert_eq!(ids(&c, Segment::Fnt), [0, 2]);
    for (id, height) in [(0, 12), (2, 14)] {
        let f = font::parse(c.item(Segment::Fnt, id).unwrap().unwrap())
            .unwrap_or_else(|e| panic!("font {id}: {e}"));
        assert_eq!(f.glyphs.len(), 103, "font {id} glyph count");
        assert_eq!(f.height, height, "font {id} height");
    }
    // Font 0's glyph 0 is an `A`, read least-significant-bit first the way
    // the sibling games' are: a narrow apex widening to the stems.
    let f = font::parse(c.item(Segment::Fnt, 0).unwrap().unwrap()).unwrap();
    let a = &f.glyphs[0];
    let row = |y: u16| (0..a.width).map(|x| a.pixel(x, y)).filter(|&on| on).count();
    assert!(row(1) < row(4), "the apex is narrower than the stems");
}

#[test]
fn the_font_reference_table_maps_a_to_glyph_0() {
    let c = container_or_skip!();
    assert_eq!(ids(&c, Segment::Frt), [0]);
    let item = c.item(Segment::Frt, 0).unwrap().unwrap();
    assert_eq!(item.len(), 516);
    let frt = FontRefTable::parse(item).unwrap();
    assert_eq!(frt.glyph_count, 120);
    assert_eq!(frt.glyph_for(b'A'), Some(0));
    assert_eq!(frt.glyph_for(b'Z'), Some(25));
    assert_eq!(frt.map.iter().filter(|g| g.is_none()).count(), 154);
    // Written for a font of 120 glyphs over fonts of 103: four entries map
    // to glyph 100 or above, and two of those — `0x8C` at 104 and `0xA0` at
    // 103, the same two bytes as in Jeff Jet and Hilfe für Amajambere — point
    // past both fonts. Neither occurs in any of the 2443 shipped strings.
    let high: Vec<(usize, u16)> = frt
        .map
        .iter()
        .enumerate()
        .filter_map(|(b, g)| g.filter(|&g| g >= 100).map(|g| (b, g)))
        .collect();
    assert_eq!(high, [(0x3b, 100), (0x8c, 104), (0x95, 101), (0xa0, 103)]);
    let mut used = [false; 256];
    for id in ids(&c, Segment::Txt) {
        let t = text::parse(c.item(Segment::Txt, id).unwrap().unwrap()).unwrap();
        for s in &t.strings {
            for &b in s.as_bytes() {
                used[usize::from(b)] = true;
            }
        }
    }
    assert!(
        !used[0x8c] && !used[0xa0],
        "a shipped string reaches a glyph the fonts do not have"
    );
}

#[test]
fn every_text_table_parses_and_names_its_hero() {
    let c = container_or_skip!();
    let ids = ids(&c, Segment::Txt);
    assert_eq!(ids.len(), 51);
    assert_eq!(ids.first().copied(), Some(1));
    assert_eq!(ids.last().copied(), Some(59));
    let mut strings = 0;
    let mut names_eddie = false;
    for &id in &ids {
        let t = text::parse(c.item(Segment::Txt, id).unwrap().unwrap())
            .unwrap_or_else(|e| panic!("text table {id}: {e}"));
        strings += t.strings.len();
        if id == 23 {
            // *Eddie*, with an *ie*: the shipped files spell the hero's name
            // the way the title does, and the catalogs that write *Eddy*
            // do not.
            names_eddie = t.strings.iter().any(|s| s.contains("Eddie Mockelby"));
        }
    }
    assert_eq!(strings, 2443);
    assert!(names_eddie, "table 23 introduces Eddie Mockelby by name");
}

#[test]
fn every_module_parses_and_its_ids_are_as_declared() {
    let c = container_or_skip!();
    let numbers = ids(&c, Segment::Scr);
    // The authoring template of the three later games, for fifteen
    // locations, and a shared library with no gap: 600 to 615, every one
    // present where the siblings leave 608 and 613 unused.
    let expected: Vec<usize> = (100..=115)
        .chain(301..=315)
        .chain(501..=515)
        .chain(600..=615)
        .collect();
    assert_eq!(numbers, expected);
    let (mut code, mut vars, mut consts) = (0, 0, 0);
    for &n in &numbers {
        let m = scr::ScrModule::parse(c.item(Segment::Scr, n).unwrap().unwrap())
            .unwrap_or_else(|e| panic!("module {n}: {e}"));
        assert_eq!(usize::from(m.module), n, "module number equals the slot");
        assert_eq!(
            m.entries.first().map(|e| e.id),
            Some(m.first_id),
            "module {n}"
        );
        assert_eq!(
            m.entries.last().map(|e| e.id),
            Some(m.last_id),
            "module {n}"
        );
        for e in &m.entries {
            if e.is_variable() {
                vars += 1;
            } else if e.is_constant() {
                consts += 1;
            } else {
                code += 1;
            }
        }
    }
    assert_eq!((code, vars, consts), (493, 784, 266));
    // The boot module carries the menu beside `CTRL` and `RUN` — the words
    // the siblings keep in a module 650 of their own.
    let boot = scr::ScrModule::parse(c.item(Segment::Scr, 100).unwrap().unwrap()).unwrap();
    let named: Vec<&str> = boot.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        named,
        [
            "_AKTTEIL",
            "_DOC_TABLE",
            "_DOSAVE",
            "KILL_GEWINN",
            "GEWINNCODE",
            "SHOW_FILES",
            "REMOVE_FILE",
            "SHOW_DOC",
            "REMOVE_DOC",
            "KILL_MENU",
            "CTRL",
            "RUN"
        ]
    );
    let library = scr::ScrModule::parse(c.item(Segment::Scr, 601).unwrap().unwrap()).unwrap();
    assert!(
        library.entry_named("STARTLOC").is_some() && library.entry_named("NEXTLOC").is_some(),
        "module 601 carries the location variables the frontend moves through"
    );
}

#[test]
fn three_blocks_are_music_thirteen_are_samples_and_nothing_else_carries_a_tag() {
    let c = container_or_skip!();
    let samples = [1, 9, 10, 12, 13, 14, 15, 17, 18, 20, 21, 22, 23];
    for id in ids(&c, Segment::Blk) {
        let item = c.item(Segment::Blk, id).unwrap().unwrap();
        match id {
            // The jingle `SET_POINTS` plays: a bare `PLX` section, the form
            // every one of Victor Loomes' tunes takes and no other later
            // game's does.
            19 => {
                assert!(psm::is_song(item) && !psm::is_module(item), "block 19");
                assert_eq!(item.len(), 366);
            }
            // The two `MTCVTS` modules — the intro's and the rooms' music —
            // with their `MDH` chunk at 56 and their own sample sections.
            24 | 25 => {
                let tags = psm::tags(item).unwrap_or_else(|| panic!("block {id} is no module"));
                assert_eq!(tags.mdh, Some(56), "block {id}");
                assert!(tags.sm8.is_some(), "block {id} has no sample section");
            }
            _ if samples.contains(&id) => {
                assert!(psm::is_sample(item), "block {id}");
                let s = psm::Sample::parse(item).unwrap_or_else(|e| panic!("block {id}: {e}"));
                assert_eq!(s.version, 0x0100, "block {id}");
                assert_eq!(
                    s.pcm.len() + 10,
                    item.len(),
                    "block {id}: the PCM is the rest"
                );
                assert!(
                    (56..=179).contains(&s.period),
                    "block {id}: period {}",
                    s.period
                );
            }
            _ => {
                assert!(
                    !psm::is_song(item) && !psm::is_sample(item),
                    "block {id} carries a sound tag"
                );
            }
        }
    }
    // The period in PIT cycles: 179 is 6.7 kHz, 56 is 21.3 kHz.
    let rates: Vec<u32> = samples
        .iter()
        .map(|&id| {
            psm::Sample::parse(c.item(Segment::Blk, id).unwrap().unwrap())
                .unwrap()
                .rate()
        })
        .collect();
    assert_eq!(rates.iter().min(), Some(&6665));
    assert_eq!(rates.iter().max(), Some(&21306));
}

#[test]
fn the_per_location_tables_have_the_sizes_the_scripts_reserve() {
    let c = container_or_skip!();
    // The same four families at the same sizes as Die Enviro-Kids greifen ein
    // and Jeff Jet, the item table at 200+N as theirs — and all fifteen
    // present, with no gap like Hilfe für Amajambere's.
    for n in 1..=15 {
        for (base, len) in [(200, 1120), (400, 452), (600, 300), (800, 182)] {
            let item = c.item(Segment::Blk, base + n).unwrap();
            assert_eq!(item.map(<[u8]>::len), Some(len), "block {}", base + n);
        }
    }
    // What is left is the animation catalogs: 101–108 and 125–159, 43 blocks.
    let catalogs: Vec<usize> = ids(&c, Segment::Blk)
        .into_iter()
        .filter(|&id| (100..200).contains(&id))
        .collect();
    assert_eq!(catalogs.len(), 43);
    assert_eq!(catalogs.first().copied(), Some(101));
    assert_eq!(catalogs.last().copied(), Some(159));
}

/// The kernel tables and their binding, from `STERN.EXE`.
fn kernel_or_skip() -> Option<(mz::Image, Vec<motionvm_motion_formats::KernelWord>)> {
    let dir = gamedata_eddiem()?;
    let img = mz::Image::open(game_file(&dir, "STERN.EXE")).expect("STERN.EXE parses");
    let words = mz::kernel_words(&img);
    Some((img, words))
}

#[test]
fn the_kernel_is_the_oldest_core_table_under_jeff_jets_domain_table() {
    let Some((img, words)) = kernel_or_skip() else {
        eprintln!("skipping: no Falsches Spiel mit Eddie M. gamedata directory");
        return;
    };
    assert_eq!(img.header_len(), 0x3200);
    let table = |n: usize| words.iter().filter(|w| w.table == n).collect::<Vec<_>>();
    assert_eq!(
        words.iter().map(|w| w.table).max(),
        Some(1),
        "exactly two tables"
    );
    let (domain, core) = (table(0), table(1));
    // 146 domain words — `HPPLAY.EXE`'s set, name for name — over the
    // eighty core words `LL.EXE` has: the two the later builds append,
    // `_PutStringAdr` and `$->`, are not here yet.
    assert_eq!(domain.len(), 146);
    assert_eq!(core.len(), 80);
    assert_eq!(domain[0].entry, 0x1f4fa);
    assert_eq!(core[0].entry, 0x202f0);
    assert_eq!(domain[0].name.as_str(), "TOGFX");
    assert_eq!(
        (core[0].name.as_str(), core[79].name.as_str()),
        ("##", "_PutLit")
    );

    let b = mz::binding_of(&img, &words).expect("the inline words are all named");
    assert_eq!(b.len(), 226);
    assert_eq!(b.name(1), Some("##"));
    assert_eq!(b.name(80), Some("_PutLit"));
    assert!(b.name(81).is_none(), "no _PutStringAdr in this build");
    // Eighty core words and 21 placeholders put the domain table at 102, as
    // in `LL.EXE` — three below the three later builds' 105. So `DOWALK` is
    // 236 here, 239 in `HPPLAY.EXE` and 243 in the other two, and
    // `PLAYSAMPLE` closes the table at 247.
    assert_eq!(b.name(102), Some("TOGFX"), "the domain table starts here");
    assert_eq!(b.name(236), Some("DOWALK"));
    assert_eq!(b.name(246), Some("GIVEDATE"));
    assert_eq!(b.name(247), Some("PLAYSAMPLE"));
    assert!(b.name(248).is_none(), "nothing after PLAYSAMPLE");
    assert!(
        b.name(124) != Some("SETMOUSEX"),
        "the SETMOUSE* four are not in this build"
    );
}

#[test]
fn every_module_disassembles_without_an_unknown_ordinal() {
    let c = container_or_skip!();
    let Some((img, words)) = kernel_or_skip() else {
        return;
    };
    let binding = mz::binding_of(&img, &words).unwrap();
    let modules: Vec<scr::ScrModule> = ids(&c, Segment::Scr)
        .into_iter()
        .map(|n| scr::ScrModule::parse(c.item(Segment::Scr, n).unwrap().unwrap()).unwrap())
        .collect();
    let mut d = disasm::Disassembler::new(&binding);
    for m in &modules {
        d.learn(m);
    }
    let mut bodies = 0;
    for m in &modules {
        for e in &m.entries {
            if e.is_variable() || e.is_constant() {
                continue;
            }
            bodies += 1;
            let cells = d.decode(e);
            assert!(
                !cells
                    .iter()
                    .any(|c| matches!(c, disasm::Cell::UnknownKernel { .. })),
                "module {} word {}: an ordinal the table does not name",
                m.module,
                e.name
            );
            assert!(
                matches!(cells.last(), Some(disasm::Cell::Kernel { name, .. }) if name == "##"),
                "module {} word {} does not end on ##",
                m.module,
                e.name
            );
        }
    }
    assert_eq!(bodies, 493, "every colon definition in the game");
}
