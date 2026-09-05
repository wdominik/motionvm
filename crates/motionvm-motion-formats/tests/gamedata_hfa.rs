//! End-to-end checks against the files Hilfe für Amajambere ships.
//!
//! These need the original files. Point `MOTIONVM_GAMEDATA_HFA` at the
//! directory holding `DATA.-1-`, `DATA.-2-` and `BMZ.EXE`; the tests skip
//! themselves if it is missing, so the crate still builds and tests cleanly
//! without the game. Every number asserted here is a measurement over those
//! files.
//!
//! `gamedata_enviro.rs` is the corpus the 16-bit readers were written against
//! and `gamedata_jeffjet.rs` the one that made them read more than one volume.
//! What this third one is for is the case neither of those has: **two volumes
//! whose items are stored plainly**. Jeff Jet packs everything and ENVIRO has
//! only one volume, so until this game the two properties had never been seen
//! apart, and a reader that had quietly tied them together would have passed
//! both.
//!
//! The game this file drives is Hilfe für Amajambere (MOTION 16-bit).

use motionvm_motion_formats::font::FontRefTable;
use motionvm_motion_formats::m16::{Container, Segment, disasm, font, gfx, mz, psm, scr, text};
use motionvm_motion_testutil::{game_file, gamedata_hfa};

macro_rules! container_or_skip {
    () => {
        match gamedata_hfa() {
            Some(dir) => Container::open_dir(&dir).expect("open DATA.-1- and DATA.-2-"),
            None => {
                eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
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
fn the_header_names_two_volumes_and_no_packed_segment() {
    let c = container_or_skip!();
    assert_eq!(c.boot().module, 100);
    assert_eq!(c.boot().word, 401, "RUN");
    let counts: Vec<usize> = Segment::ALL.iter().map(|&s| c.slot_count(s)).collect();
    // Die Enviro-Kids greifen ein and Jeff Jet reserve 100 text tables and
    // Victor Loomes 30; this one reserves 150. The
    // counts are read out of the header, so a different geometry is data and
    // not a case to handle.
    assert_eq!(counts, [2500, 1000, 700, 25, 10, 10, 150]);
    assert_eq!((c.volumes(), c.spare_offsets()), (2, 7));
    assert_eq!(c.first_item_offset(), 26436);
    assert!(
        Segment::ALL.iter().all(|&s| !c.packed(s)),
        "this game is on two volumes and packs none of them"
    );
    assert_eq!(
        c.trailing_slack(),
        0,
        "every byte of both volumes after their tables belongs to an item"
    );
}

#[test]
fn the_second_volume_opens_on_a_table_of_its_own_length() {
    let Some(dir) = gamedata_hfa() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
        return;
    };
    let raw = std::fs::read(game_file(&dir, "DATA.-2-")).expect("DATA.-2- reads");
    let first = u32::from_le_bytes(raw[..4].try_into().unwrap());
    // 4 × (4395 slots + 7 spare): the table describes its own byte length, and
    // the first item begins right after it.
    assert_eq!(first, 17608);
}

#[test]
fn a_slot_flagged_empty_is_empty_and_the_over_claimed_ones_are_counted() {
    let c = container_or_skip!();
    // This game flags occupancy by whole segment ranges rather than by slot:
    // all 2500 GFX slots claim volume 2 and all ten FNT slots do, while only
    // 1045 and 7 of them carry bytes. ENVIRO's container has no such slot and
    // Jeff Jet has two, so this is the game that says the flag word is a hint
    // and the offsets are the truth. Nothing downstream is affected — what has
    // bytes is what `item` answers with.
    assert_eq!(c.occupancy_mismatches().len(), 1534);
    // Block 307 is the one absence that is not over-claiming: flag and offsets
    // agree that location 7's item table was never written. See
    // `hfa_locations.rs` for what the game does when it enters that room.
    assert_eq!(c.occupancy(Segment::Blk, 307).unwrap(), 0);
    assert!(c.item(Segment::Blk, 307).unwrap().is_none());
}

#[test]
fn every_segment_holds_what_the_game_ships() {
    let c = container_or_skip!();
    let n: Vec<usize> = Segment::ALL.iter().map(|&s| ids(&c, s).len()).collect();
    assert_eq!(n, [1045, 153, 76, 24, 7, 1, 95]);
    let bytes: usize = Segment::ALL
        .iter()
        .flat_map(|&s| ids(&c, s).into_iter().map(move |id| (s, id)))
        .map(|(s, id)| c.item(s, id).unwrap().unwrap().len())
        .sum();
    // Stored as they are read: the two files less their headers and tables.
    assert_eq!(bytes, 5_289_565);
}

#[test]
fn the_volumes_are_split_by_kind_and_not_by_half() {
    let c = container_or_skip!();
    // Jeff Jet cuts its sprites across the two volumes; this game cuts by what
    // a thing is. Volume 2 is everything the game draws — every sprite, every
    // palette, all seven fonts, the font reference table — and volume 1 is
    // everything it runs and says. A reader that opened only the first would
    // find every script and nothing to show.
    for (seg, volume) in [
        (Segment::Gfx, 2),
        (Segment::Pal, 2),
        (Segment::Fnt, 2),
        (Segment::Frt, 2),
        (Segment::Blk, 1),
        (Segment::Scr, 1),
        (Segment::Txt, 1),
    ] {
        for id in ids(&c, seg) {
            assert_eq!(
                c.occupancy(seg, id).unwrap(),
                volume,
                "{} {id} is not on volume {volume}",
                seg.name()
            );
        }
    }
}

#[test]
fn every_sprite_is_exactly_its_header_plus_its_pixels() {
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
            "sprite {id} is stored plainly, header and pixels and nothing else"
        );
        sizes.insert((s.width, s.height));
        widest = widest.max(s.width);
        tallest = tallest.max(s.height);
    }
    assert_eq!(sizes.len(), 276, "distinct sizes");
    // As in the sibling games, nothing here is a whole screen: the tallest
    // sprite fits inside the 165-row world viewport and full-screen artwork is
    // a block.
    assert_eq!((widest, tallest), (304, 162));
}

#[test]
fn every_palette_is_768_six_bit_bytes() {
    let c = container_or_skip!();
    let ids = ids(&c, Segment::Pal);
    assert_eq!(ids, (0..24).collect::<Vec<_>>());
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
fn the_seven_fonts_decode_and_read_lsb_first() {
    let c = container_or_skip!();
    // Seven, where ENVIRO ships three and Jeff Jet two — and slot 1 is the one
    // gap. The two 90-glyph fonts are the headline faces.
    assert_eq!(ids(&c, Segment::Fnt), [0, 2, 3, 4, 5, 6, 7]);
    for (id, glyphs, height) in [
        (0, 102, 12),
        (2, 102, 14),
        (3, 90, 18),
        (4, 90, 20),
        (5, 102, 18),
        (6, 102, 20),
        (7, 103, 11),
    ] {
        let f = font::parse(c.item(Segment::Fnt, id).unwrap().unwrap())
            .unwrap_or_else(|e| panic!("font {id}: {e}"));
        assert_eq!(f.glyphs.len(), glyphs, "font {id} glyph count");
        assert_eq!(f.height, height, "font {id} height");
    }
    // Font 0's glyph 0 is an `A`, read least-significant-bit first the way the
    // sibling games' are: a narrow apex widening to the stems, and a solid
    // crossbar.
    let f = font::parse(c.item(Segment::Fnt, 0).unwrap().unwrap()).unwrap();
    let a = &f.glyphs[0];
    let row = |y: u16| (0..a.width).map(|x| a.pixel(x, y)).filter(|&on| on).count();
    assert!(row(1) < row(4), "the apex is narrower than the stems");
    assert!(row(3) >= row(1), "the crossbar is not the narrowest row");
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
    // Written for a font of 120 glyphs, and the largest this game ships holds
    // 103, so the same two entries as Jeff Jet's point past every font: 0x8C at
    // 104 and 0xA0 at 103. Neither byte occurs in any of the 2709 shipped
    // strings, so the renderer's rule for an index it has no glyph for — leave
    // the glyph out — is never reached by this game's own texts.
    let past: Vec<(usize, u16)> = frt
        .map
        .iter()
        .enumerate()
        .filter_map(|(b, g)| g.filter(|&g| g >= 103).map(|g| (b, g)))
        .collect();
    assert_eq!(past, [(0x8c, 104), (0xa0, 103)]);
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
        past.iter().all(|&(b, _)| !used[b]),
        "a shipped string reaches a glyph the fonts do not have"
    );
}

#[test]
fn every_text_table_parses_and_reads_as_german() {
    let c = container_or_skip!();
    let ids = ids(&c, Segment::Txt);
    assert_eq!(ids.len(), 95);
    assert_eq!(ids.first().copied(), Some(9));
    assert_eq!(ids.last().copied(), Some(113));
    let mut strings = 0;
    let mut names_its_setting = false;
    for &id in &ids {
        let t = text::parse(c.item(Segment::Txt, id).unwrap().unwrap())
            .unwrap_or_else(|e| panic!("text table {id}: {e}"));
        strings += t.strings.len();
        if id == 14 {
            names_its_setting = t.strings.iter().any(|s| s.contains("Amajambere"));
        }
    }
    assert_eq!(strings, 2709);
    assert!(
        names_its_setting,
        "table 14 is where the game names the region it is set in"
    );
}

#[test]
fn every_module_parses_and_its_ids_are_as_declared() {
    let c = container_or_skip!();
    let numbers = ids(&c, Segment::Scr);
    // The authoring template the sibling games follow, for twenty locations —
    // more than either of them — and the same shared library, with 608 and 613
    // unused.
    let expected: Vec<usize> = (100..=120)
        .chain(301..=320)
        .chain(501..=520)
        .chain(600..=607)
        .chain(609..=612)
        .chain([614, 650, 651])
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
    assert_eq!((code, vars, consts), (469, 533, 193));
    let boot = scr::ScrModule::parse(c.item(Segment::Scr, 100).unwrap().unwrap()).unwrap();
    let named: Vec<&str> = boot.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        named,
        ["CTRL", "RUN"],
        "the boot module, as in the siblings"
    );
    let library = scr::ScrModule::parse(c.item(Segment::Scr, 601).unwrap().unwrap()).unwrap();
    assert!(
        library.entry_named("STARTLOC").is_some() && library.entry_named("NEXTLOC").is_some(),
        "module 601 carries the location variables the frontend moves through"
    );
}

#[test]
fn the_four_songs_carry_the_psm_tags_and_nothing_else_does() {
    let c = container_or_skip!();
    let songs = 1..=4;
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
    let mut without_items = Vec::new();
    for n in 1..=20 {
        // The same four families as the sibling games at the same sizes, but
        // the item table is block 300+N here where theirs is 200+N — the
        // number `INCLLOC` in module 605 computes with
        // `A_LDITEM S_LDITEM * _LDITEM ACTLOC @ 300 + GET`.
        for (base, len) in [(400, 452), (600, 300), (800, 182)] {
            let item = c.item(Segment::Blk, base + n).unwrap();
            assert_eq!(item.map(<[u8]>::len), Some(len), "block {}", base + n);
        }
        match c.item(Segment::Blk, 300 + n).unwrap() {
            Some(item) => assert_eq!(item.len(), 1120, "block {}", 300 + n),
            None => without_items.push(n),
        }
    }
    // 1120 is `A_LDITEM * S_LDITEM`, 35 × 32, as module 601 declares them.
    // Location 7 is the one the game ships without an item table.
    assert_eq!(without_items, [7]);
}

/// The kernel tables and their binding, from `BMZ.EXE`.
fn kernel_or_skip() -> Option<(mz::Image, Vec<motionvm_motion_formats::KernelWord>)> {
    let dir = gamedata_hfa()?;
    let img = mz::Image::open(game_file(&dir, "BMZ.EXE")).expect("BMZ.EXE parses");
    let words = mz::kernel_words(&img);
    Some((img, words))
}

#[test]
fn the_kernel_is_the_later_builds_table_less_its_last_word() {
    let Some((img, words)) = kernel_or_skip() else {
        eprintln!("skipping: no Hilfe für Amajambere gamedata directory");
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
    assert_eq!(domain.len(), 150);
    assert_eq!(core.len(), 82);
    assert_eq!(domain[0].entry, 0x1f7e6);
    assert_eq!(core[0].entry, 0x2067a);
    assert_eq!(domain[0].name.as_str(), "TOGFX");
    assert_eq!(
        (core[0].name.as_str(), core[81].name.as_str()),
        ("##", "$->")
    );

    let b = mz::binding_of(&img, &words).expect("the inline words are all named");
    assert_eq!(b.len(), 232, "one fewer than `ENVIRO.EXE`");
    assert_eq!(b.name(1), Some("##"));
    assert_eq!(b.name(105), Some("TOGFX"), "the domain table starts here");
    // This build has the four `SETMOUSE*` words `HPPLAY.EXE` lacks, so nothing
    // is shifted against the later build: every ordinal from 105 to 254 names
    // what it names in `ENVIRO.EXE`, and only `?SAMPLE` at 255 is missing. It
    // is Jeff Jet's table that is four low from 124 up — `DOWALK` is 239
    // there and 243 here.
    assert_eq!(b.name(124), Some("SETMOUSEX"));
    assert_eq!(b.name(243), Some("DOWALK"));
    assert_eq!(b.name(254), Some("PLAYSAMPLE"));
    assert!(b.name(255).is_none(), "no ?SAMPLE in this build");
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
    assert_eq!(bodies, 469, "every colon definition in the game");
}
