//! End-to-end checks against the files Die Enviro-Kids greifen ein ships.
//!
//! These need the original files. Point `MOTIONVM_GAMEDATA_ENVIRO` at the
//! directory holding `DATA.-1-` and `ENVIRO.EXE`; the tests skip themselves
//! if it is missing, so the crate still builds and tests cleanly without the
//! game. Every number asserted here is a measurement over that one file —
//! the corpus the 16-bit readers were written against.

use motionvm_formats::font::FontRefTable;
use motionvm_formats::m16::{Container, Segment, disasm, font, gfx, mz, psm, scr, text};
use motionvm_testutil::{game_file, gamedata_enviro};

macro_rules! container_or_skip {
    () => {
        match gamedata_enviro() {
            Some(dir) => Container::open_dir(&dir).expect("open DATA.-1-"),
            None => {
                eprintln!("skipping: no ENVIRO gamedata directory");
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
fn the_header_names_the_boot_word_and_seven_segments() {
    let c = container_or_skip!();
    assert_eq!(c.boot().module, 100);
    assert_eq!(c.boot().word, 401, "RUN");
    let counts: Vec<usize> = Segment::ALL.iter().map(|&s| c.slot_count(s)).collect();
    assert_eq!(counts, [2500, 1000, 700, 25, 10, 10, 100]);
    assert_eq!(c.open_fields(), [1, 3]);
    assert_eq!(c.first_item_offset(), 26120);
    assert_eq!(
        c.trailing_slack(),
        0,
        "every byte after the tables belongs to an item"
    );
    assert!(
        c.occupancy_mismatches().is_empty(),
        "the occupancy table agrees with the offsets slot for slot"
    );
}

#[test]
fn every_segment_holds_what_the_game_ships() {
    let c = container_or_skip!();
    let n: Vec<usize> = Segment::ALL.iter().map(|&s| ids(&c, s).len()).collect();
    assert_eq!(n, [1586, 130, 65, 23, 3, 1, 96]);
}

#[test]
fn every_sprite_is_exactly_its_header_plus_its_pixels() {
    let c = container_or_skip!();
    let mut sizes = std::collections::BTreeSet::new();
    let mut widest = 0;
    for id in ids(&c, Segment::Gfx) {
        let item = c.item(Segment::Gfx, id).unwrap().unwrap();
        let s = gfx::Sprite::parse(item).unwrap_or_else(|e| panic!("sprite {id}: {e}"));
        assert_eq!(s.pixels.len(), s.width as usize * s.height as usize);
        assert!(
            s.width > 0 && s.height > 0,
            "sprite {id} has a zero dimension"
        );
        sizes.insert((s.width, s.height));
        widest = widest.max(s.width);
    }
    assert_eq!(sizes.len(), 262, "distinct sizes");
    assert_eq!(widest, 320, "nothing is wider than the display");
    for id in 0..12 {
        let s = gfx::Sprite::parse(c.item(Segment::Gfx, id).unwrap().unwrap()).unwrap();
        assert_eq!((s.width, s.height), (64, 115), "placeholder {id}");
    }
    let pointer = gfx::Sprite::parse(c.item(Segment::Gfx, 399).unwrap().unwrap()).unwrap();
    assert_eq!(
        (pointer.width, pointer.height),
        (16, 15),
        "the pointer sprite"
    );
}

#[test]
fn every_palette_is_768_six_bit_bytes() {
    let c = container_or_skip!();
    let ids = ids(&c, Segment::Pal);
    assert_eq!(ids, (0..23).collect::<Vec<_>>());
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
fn the_three_fonts_decode_and_read_lsb_first() {
    let c = container_or_skip!();
    assert_eq!(ids(&c, Segment::Fnt), [0, 2, 7]);
    for (id, height) in [(0, 12), (2, 14), (7, 11)] {
        let f = font::parse(c.item(Segment::Fnt, id).unwrap().unwrap())
            .unwrap_or_else(|e| panic!("font {id}: {e}"));
        assert_eq!(f.glyphs.len(), 116, "font {id} glyph count");
        assert_eq!(f.height, height, "font {id} height");
        assert_eq!(
            f.table_end(),
            468,
            "font {id}: bitmaps start right after the table"
        );
    }
    // Glyph 0 of font 0 is an `A` read least-significant-bit first: two set
    // pixels in the middle of row 2 widening to the stems on row 4. Read the
    // other way round the rows are mirrored and nothing spells anything.
    let f = font::parse(c.item(Segment::Fnt, 0).unwrap().unwrap()).unwrap();
    let a = &f.glyphs[0];
    assert_eq!(a.width, 6);
    let row = |y: u16| (0..6).map(|x| a.pixel(x, y)).collect::<Vec<_>>();
    assert_eq!(row(2), [false, false, true, true, false, false]);
    assert_eq!(row(4), [true, true, false, false, true, true]);
    assert_eq!(row(5), [true, true, true, true, true, true]);
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
    assert_eq!(frt.map.iter().filter(|g| g.is_none()).count(), 143);
    assert_eq!(
        frt.map.iter().flatten().max(),
        Some(&115),
        "the last glyph of a 116-glyph font"
    );
}

#[test]
fn every_text_table_parses_and_reads_as_german() {
    let c = container_or_skip!();
    let ids = ids(&c, Segment::Txt);
    assert_eq!(ids.len(), 96);
    assert!(
        ![0, 19, 83, 84].iter().any(|id| ids.contains(id)),
        "the four empty slots"
    );
    let mut total = 0;
    for &id in &ids {
        let t = text::parse(c.item(Segment::Txt, id).unwrap().unwrap())
            .unwrap_or_else(|e| panic!("text {id}: {e}"));
        assert!(!t.strings.is_empty(), "text {id} is empty");
        total += t.strings.len();
    }
    assert_eq!(total, 3508);
    let t20 = text::parse(c.item(Segment::Txt, 20).unwrap().unwrap()).unwrap();
    assert_eq!(t20.get(3), Some("Jetzt bin ich Eva."));
    assert_eq!(t20.get(4), Some("Jetzt bin ich Maik."));
    let t14 = text::parse(c.item(Segment::Txt, 14).unwrap().unwrap()).unwrap();
    assert!(
        t14.get(1)
            .is_some_and(|s| s.starts_with(" Viele Leute in Waldbach"))
    );
}

#[test]
fn every_module_parses_and_its_ids_are_as_declared() {
    let c = container_or_skip!();
    let numbers = ids(&c, Segment::Scr);
    let expected: Vec<usize> = [100]
        .into_iter()
        .chain(101..=115)
        .chain([117])
        .chain(301..=315)
        .chain([317])
        .chain(501..=515)
        .chain([517])
        .chain(600..=607)
        .chain(609..=612)
        .chain(614..=615)
        .chain([650, 651])
        .collect();
    assert_eq!(numbers, expected);
    let (mut code, mut vars, mut consts, mut long) = (0, 0, 0, 0);
    let mut lowest = u16::MAX;
    let mut highest = 0;
    for &n in &numbers {
        let m = scr::ScrModule::parse(c.item(Segment::Scr, n).unwrap().unwrap())
            .unwrap_or_else(|e| panic!("module {n}: {e}"));
        assert_eq!(m.module as usize, n, "module number equals the slot");
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
            if e.declared_len > 11 {
                long += 1;
            }
            lowest = lowest.min(e.id);
            highest = highest.max(e.id);
        }
        match n {
            301..=317 => {
                assert_eq!(m.entries.len(), 1, "a room macro is one word");
                assert_eq!(m.first_id, 549, "every macro has id 549");
            }
            101..=117 => assert_eq!(m.first_id, 560, "scene modules start at 560"),
            501..=517 => assert_eq!(m.first_id, 750, "click modules start at 750"),
            _ => {}
        }
    }
    assert_eq!((code, vars, consts), (672, 792, 310));
    assert_eq!((lowest, highest), (400, 1944));
    assert_eq!(
        long, 151,
        "names longer than their stored eleven characters"
    );
    let boot = scr::ScrModule::parse(c.item(Segment::Scr, 100).unwrap().unwrap()).unwrap();
    assert_eq!(boot.entry(401).map(|e| e.name.as_str()), Some("RUN"));
    assert_eq!(boot.entry(400).map(|e| e.name.as_str()), Some("CTRL"));
    let macro1 = scr::ScrModule::parse(c.item(Segment::Scr, 301).unwrap().unwrap()).unwrap();
    assert_eq!(macro1.entries[0].name, "MUELL");
}

#[test]
fn the_ten_songs_carry_the_psm_tags_and_nothing_else_does() {
    let c = container_or_skip!();
    let songs = [1, 2, 3, 4, 5, 7, 8, 9, 10, 11];
    for id in ids(&c, Segment::Blk) {
        let item = c.item(Segment::Blk, id).unwrap().unwrap();
        let tags = psm::tags(item);
        if songs.contains(&id) {
            let tags = tags.unwrap_or_else(|| panic!("block {id} is not a PSM 2 module"));
            assert_eq!(tags.mdh, Some(56), "block {id}");
            assert!(tags.sm8.is_some(), "block {id} has no sample tag");
        } else {
            assert!(tags.is_none(), "block {id} carries the PSM tag");
        }
    }
}

#[test]
fn the_per_location_tables_have_the_sizes_the_scripts_reserve() {
    let c = container_or_skip!();
    for n in (1..=15).chain([17]) {
        // `A_LDITEM S_LDITEM *` = 35 * 32, and so on — the constants in
        // module 601, with a two-byte count in front of the route and click
        // tables.
        for (base, len) in [(200, 1120), (400, 452), (600, 300), (800, 182)] {
            let item = c.item(Segment::Blk, base + n).unwrap();
            assert_eq!(item.map(<[u8]>::len), Some(len), "block {}", base + n);
        }
    }
}

/// The kernel tables and their binding, from `ENVIRO.EXE`.
fn kernel_or_skip() -> Option<(mz::Image, Vec<motionvm_formats::KernelWord>)> {
    let dir = gamedata_enviro()?;
    let img = mz::Image::open(game_file(&dir, "ENVIRO.EXE")).expect("ENVIRO.EXE parses");
    let words = mz::kernel_words(&img);
    Some((img, words))
}

#[test]
fn the_kernel_tables_are_found_where_they_sit_and_bind_as_the_modules_use_them() {
    let Some((img, words)) = kernel_or_skip() else {
        eprintln!("skipping: no ENVIRO gamedata directory");
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
    assert_eq!(domain.len(), 151);
    assert_eq!(core.len(), 82);
    assert_eq!(domain[0].entry, 0x1f9f6);
    assert_eq!(core[0].entry, 0x2064e);
    assert_eq!(
        (domain[0].name.as_str(), domain[150].name.as_str()),
        ("TOGFX", "?SAMPLE")
    );
    assert_eq!(
        (core[0].name.as_str(), core[81].name.as_str()),
        ("##", "$->")
    );
    assert_eq!(
        core[0].handler, 0x16f62,
        "the handler of ## as the table holds it"
    );

    let b = mz::binding_of(&words).expect("the inline words are all named");
    assert_eq!(b.len(), 233);
    assert_eq!(b.name(1), Some("##"));
    assert_eq!(b.name(80), Some("_PutLit"));
    assert_eq!(b.name(105), Some("TOGFX"));
    assert_eq!(b.name(255), Some("?SAMPLE"));
    assert!(
        (83..=104).all(|o| b.name(o).is_none()),
        "the gap between the tables"
    );
    let inline = b.inline;
    assert_eq!(
        (
            inline.put_lit,
            inline.put_adr,
            inline.put_const,
            inline.put_string,
            inline.put_string_adr
        ),
        (80, 37, 38, 78, 81)
    );
    assert_eq!(
        (
            inline.check_if,
            inline.check_eif,
            inline.ch_else_dup,
            inline.check_else,
            inline.loop_break
        ),
        (41, 42, Some(43), 44, 51)
    );
    assert_eq!(
        (
            inline.until,
            inline.repeat,
            inline.loop_end,
            inline.add_loop,
            inline.u_loop_end
        ),
        (50, 52, 40, 47, 48)
    );
}

#[test]
fn every_module_disassembles_without_an_unknown_ordinal() {
    let Some((_, words)) = kernel_or_skip() else {
        eprintln!("skipping: no ENVIRO gamedata directory");
        return;
    };
    let c = container_or_skip!();
    let b = mz::binding_of(&words).unwrap();
    let mut dis = disasm::Disassembler::new(&b);
    let modules: Vec<scr::ScrModule> = c
        .present(Segment::Scr)
        .into_iter()
        .map(|n| scr::ScrModule::parse(c.item(Segment::Scr, n).unwrap().unwrap()).unwrap())
        .collect();
    for m in &modules {
        dis.learn(m);
    }
    for m in &modules {
        for e in &m.entries {
            for cell in dis.decode(e) {
                assert!(
                    !matches!(cell, disasm::Cell::UnknownKernel { .. }),
                    "module {} {}: {cell:?}",
                    m.module,
                    e.name
                );
            }
        }
    }
    let usage = dis.usage(&modules);
    assert_eq!(usage.len(), 151, "distinct kernel words the game uses");
    let count = |name: &str| usage.iter().find(|(n, _)| n == name).map_or(0, |(_, n)| *n);
    assert_eq!(count("_PutLit"), 5230);
    assert_eq!(count("##"), 673);
    assert_eq!(count("SDSPR"), 358);
    assert_eq!(count("SETBUF"), 129);
    assert_eq!(count("BUFON"), 1);
    assert_eq!(usage.iter().map(|(_, n)| n).sum::<usize>(), 24282);
}

#[test]
fn run_and_ctrl_decode_as_the_boot_sequence_and_its_branches() {
    let Some((_, words)) = kernel_or_skip() else {
        eprintln!("skipping: no ENVIRO gamedata directory");
        return;
    };
    let c = container_or_skip!();
    let b = mz::binding_of(&words).unwrap();
    let mut dis = disasm::Disassembler::new(&b);
    let parse =
        |n: usize| scr::ScrModule::parse(c.item(Segment::Scr, n).unwrap().unwrap()).unwrap();
    for n in [100, 600, 601, 603, 605, 606] {
        dis.learn(&parse(n));
    }
    let boot = parse(100);
    let run = boot.entry_named("RUN").unwrap();
    let text: Vec<String> = dis.decode(run).iter().map(disasm::Cell::render).collect();
    assert!(
        text.join(" ")
            .starts_with("_PutLit 600 =>GET _PutLit 601 =>GET _PutLit 602 =>GET"),
        "{}",
        text[..8].join(" ")
    );
    assert!(text.join(" ").contains("TOGFX 0 SETPAL NEWANIM BUFON"));
    assert!(text.join(" ").contains("_PutLit 400 SCRCTRL"));
    assert!(text.join(" ").ends_with("##"));
    // `CTRL` opens `?KEY DUP _CheckIf 95`, operand at cell 3, landing on 98.
    let ctrl = boot.entry_named("CTRL").unwrap();
    let cells = dis.decode(ctrl);
    assert!(matches!(&cells[2], disasm::Cell::Kernel { name, .. } if name == "_CheckIf"));
    assert_eq!(
        cells[3],
        disasm::Cell::Branch {
            distance: 95,
            target: 98
        }
    );
    // A variable is its word, its cell, and then data — never code.
    let globals = parse(601);
    let link = globals.entry_named("_LOCLINK").unwrap();
    let cells = dis.decode(link);
    assert!(matches!(&cells[0], disasm::Cell::Kernel { name, .. } if name == "_PutAdr"));
    assert!(
        cells[1..]
            .iter()
            .all(|c| matches!(c, disasm::Cell::Data(_)))
    );
    assert_eq!(cells.len(), 18);
}
