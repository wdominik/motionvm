//! End-to-end checks against the files Jeff Jet - Abenteuer InfoHighway ships.
//!
//! These need the original files. Point `MOTIONVM_GAMEDATA_JEFFJET` at the
//! directory holding `DATA.-1-`, `DATA.-2-` and `HPPLAY.EXE`; the tests skip
//! themselves if it is missing, so the crate still builds and tests cleanly
//! without the game. Every number asserted here is a measurement over those
//! files.
//!
//! The 16-bit readers were written against Die Enviro-Kids greifen ein, and
//! `gamedata_enviro.rs` is that corpus. This is the second one, and what it is
//! really for is the two things ENVIRO cannot exercise: a game on more than one
//! volume, and a game whose items are packed.
//!
//! The game this file drives is Jeff Jet (MOTION 16-bit).

use motionvm_motion_formats::font::FontRefTable;
use motionvm_motion_formats::m16::{Container, Segment, disasm, font, gfx, mz, psm, scr, text};
use motionvm_motion_testutil::{game_file, gamedata_jeffjet};

macro_rules! container_or_skip {
    () => {
        match gamedata_jeffjet() {
            Some(dir) => Container::open_dir(&dir).expect("open DATA.-1- and DATA.-2-"),
            None => {
                eprintln!("skipping: no Jeff Jet gamedata directory");
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
fn the_header_names_two_volumes_and_seven_packed_segments() {
    let c = container_or_skip!();
    assert_eq!(c.boot().module, 100);
    assert_eq!(c.boot().word, 401, "RUN");
    let counts: Vec<usize> = Segment::ALL.iter().map(|&s| c.slot_count(s)).collect();
    assert_eq!(
        counts,
        [2500, 1000, 700, 25, 10, 10, 100],
        "the same seven counts as Die Enviro-Kids greifen ein"
    );
    assert_eq!((c.volumes(), c.spare_offsets()), (2, 7));
    assert_eq!(c.first_item_offset(), 26136);
    assert!(
        Segment::ALL.iter().all(|&s| c.packed(s)),
        "every segment of this game is packed"
    );
    assert_eq!(
        c.trailing_slack(),
        0,
        "every byte of both volumes after their tables belongs to an item"
    );
}

#[test]
fn the_second_volume_opens_on_a_table_of_its_own_length() {
    let Some(dir) = gamedata_jeffjet() else {
        eprintln!("skipping: no Jeff Jet gamedata directory");
        return;
    };
    let second = std::fs::read(game_file(&dir, "DATA.-2-")).expect("DATA.-2- reads");
    let first = usize::try_from(u32::from_le_bytes(second[..4].try_into().unwrap())).unwrap();
    // A secondary volume carries no header: it is the offset table over the
    // same 4345 slots plus the seven spare entries, and then the items. Its
    // first entry is where its own table ends, which is the arithmetic saying
    // so out loud.
    assert_eq!(first, 4 * (4345 + 7));
    assert_eq!(first, 17408);
}

#[test]
fn two_slots_are_flagged_for_a_volume_that_gives_them_nothing() {
    let c = container_or_skip!();
    // The occupancy word says sprite 1319 is on volume 1 and 1848 on volume 2,
    // and both volumes give them a length of zero. Present is what has bytes,
    // so neither is present, and the disagreement is reported rather than
    // handed out as an empty sprite.
    assert_eq!(c.occupancy_mismatches(), [1319, 1848]);
    assert_eq!(c.occupancy(Segment::Gfx, 1319).unwrap(), 1);
    assert_eq!(c.occupancy(Segment::Gfx, 1848).unwrap(), 2);
    assert!(c.item(Segment::Gfx, 1319).unwrap().is_none());
    assert!(c.item(Segment::Gfx, 1848).unwrap().is_none());
}

#[test]
fn every_segment_holds_what_the_game_ships() {
    let c = container_or_skip!();
    let n: Vec<usize> = Segment::ALL.iter().map(|&s| ids(&c, s).len()).collect();
    assert_eq!(n, [1470, 119, 55, 16, 2, 1, 65]);
    let bytes: usize = Segment::ALL
        .iter()
        .flat_map(|&s| ids(&c, s).into_iter().map(move |id| (s, id)))
        .map(|(s, id)| c.item(s, id).unwrap().unwrap().len())
        .sum();
    assert_eq!(bytes, 8_412_811, "unpacked, from 2 459 890 bytes stored");
}

#[test]
fn the_second_volume_carries_everything_the_game_draws_through() {
    let c = container_or_skip!();
    // Ignoring volume 2 would not make this game look worse, it would make it
    // look like nothing: every palette, both fonts and the font reference
    // table are on it, and so is a third of the artwork.
    for id in ids(&c, Segment::Pal) {
        assert_eq!(c.occupancy(Segment::Pal, id).unwrap(), 2, "palette {id}");
    }
    for id in ids(&c, Segment::Fnt) {
        assert_eq!(c.occupancy(Segment::Fnt, id).unwrap(), 2, "font {id}");
    }
    assert_eq!(c.occupancy(Segment::Frt, 0).unwrap(), 2);
    let on_two = ids(&c, Segment::Gfx)
        .into_iter()
        .filter(|&id| c.occupancy(Segment::Gfx, id).unwrap() == 2)
        .count();
    assert_eq!(on_two, 523, "sprites on the second volume");
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
        sizes.insert((s.width, s.height));
        widest = widest.max(s.width);
        tallest = tallest.max(s.height);
    }
    assert_eq!(sizes.len(), 433, "distinct sizes");
    // Nothing here is a whole screen: the widest sprite is a backdrop strip and
    // the tallest fills the world viewport, which is 155 or 156 rows in this
    // engine. Full-screen artwork is a block, not a sprite.
    assert_eq!((widest, tallest), (288, 156));
}

#[test]
fn every_palette_is_768_six_bit_bytes() {
    let c = container_or_skip!();
    let ids = ids(&c, Segment::Pal);
    assert_eq!(ids, (0..16).collect::<Vec<_>>());
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
    assert_eq!(ids(&c, Segment::Fnt), [0, 2]);
    for (id, height) in [(0, 12), (2, 14)] {
        let f = font::parse(c.item(Segment::Fnt, id).unwrap().unwrap())
            .unwrap_or_else(|e| panic!("font {id}: {e}"));
        assert_eq!(f.glyphs.len(), 102, "font {id} glyph count");
        assert_eq!(f.height, height, "font {id} height");
        assert_eq!(
            f.table_end(),
            412,
            "font {id}: bitmaps start right after the table"
        );
    }
    // Font 0's glyph 0 is an `A`, read least-significant-bit first the way Die
    // Enviro-Kids greifen ein's is: a narrow apex widening to the stems, and a
    // solid crossbar.
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
    // The table was written for a font of 120 glyphs and the game ships two of
    // 102, so its last two entries point past both: 0x8C at 104 and 0xA0 at
    // 103. Neither byte occurs in any of the 5709 shipped strings, so the
    // renderer's rule for an index it has no glyph for — leave the glyph out —
    // is never reached by this game's own texts.
    let past: Vec<(usize, u16)> = frt
        .map
        .iter()
        .enumerate()
        .filter_map(|(b, g)| g.filter(|&g| g >= 102).map(|g| (b, g)))
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
    assert_eq!(ids.len(), 65);
    assert_eq!(ids.first().copied(), Some(9));
    assert_eq!(ids.last().copied(), Some(82));
    let mut strings = 0;
    let mut credits = false;
    for &id in &ids {
        let t = text::parse(c.item(Segment::Txt, id).unwrap().unwrap())
            .unwrap_or_else(|e| panic!("text table {id}: {e}"));
        strings += t.strings.len();
        if id == 14 {
            credits = t
                .strings
                .iter()
                .any(|s| s.contains("Jeff Jet - Abenteuer InfoHighway"));
        }
    }
    assert_eq!(strings, 5709);
    assert!(
        credits,
        "table 14 is where the game spells its own title out"
    );
}

#[test]
fn every_module_parses_and_its_ids_are_as_declared() {
    let c = container_or_skip!();
    let numbers = ids(&c, Segment::Scr);
    // The authoring template Die Enviro-Kids greifen ein follows, for thirteen
    // locations instead of sixteen: a scene, a macro and a click module per
    // location, then the shared library.
    let expected: Vec<usize> = (100..=113)
        .chain(301..=313)
        .chain(501..=513)
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
    assert_eq!((code, vars, consts), (481, 650, 267));
    let boot = scr::ScrModule::parse(c.item(Segment::Scr, 100).unwrap().unwrap()).unwrap();
    let named: Vec<&str> = boot.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(named, ["CTRL", "RUN"], "the boot module, as in ENVIRO");
    let library = scr::ScrModule::parse(c.item(Segment::Scr, 601).unwrap().unwrap()).unwrap();
    assert!(
        library.entry_named("STARTLOC").is_some() && library.entry_named("NEXTLOC").is_some(),
        "module 601 carries the location variables the frontend moves through"
    );
}

#[test]
fn the_nine_songs_carry_the_psm_tags_and_nothing_else_does() {
    let c = container_or_skip!();
    let songs = 1..=9;
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
    for n in 1..=13 {
        // The same four families at the same sizes as Die Enviro-Kids greifen
        // ein, for this game's thirteen locations.
        for (base, len) in [(200, 1120), (400, 452), (600, 300), (800, 182)] {
            let item = c.item(Segment::Blk, base + n).unwrap();
            assert_eq!(item.map(<[u8]>::len), Some(len), "block {}", base + n);
        }
    }
}

/// The kernel tables and their binding, from `HPPLAY.EXE`.
fn kernel_or_skip() -> Option<(mz::Image, Vec<motionvm_motion_formats::KernelWord>)> {
    let dir = gamedata_jeffjet()?;
    let img = mz::Image::open(game_file(&dir, "HPPLAY.EXE")).expect("HPPLAY.EXE parses");
    let words = mz::kernel_words(&img);
    Some((img, words))
}

#[test]
fn the_kernel_of_the_older_build_is_a_subset_and_binds_at_shifted_ordinals() {
    let Some((img, words)) = kernel_or_skip() else {
        eprintln!("skipping: no Jeff Jet gamedata directory");
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
    assert_eq!(domain.len(), 146);
    assert_eq!(core.len(), 82);
    assert_eq!(domain[0].entry, 0x1f42e);
    assert_eq!(core[0].entry, 0x20236);
    assert_eq!(domain[0].name.as_str(), "TOGFX");
    assert_eq!(
        (core[0].name.as_str(), core[81].name.as_str()),
        ("##", "$->")
    );

    let b = mz::binding_of(&img, &words).expect("the inline words are all named");
    assert_eq!(b.len(), 228, "five fewer than the later build");
    assert_eq!(b.name(1), Some("##"));
    assert_eq!(b.name(105), Some("TOGFX"), "the domain table starts here");
    // This build has no SETMOUSEX/Y/LB/RB, and the later one inserted them in
    // the middle of a live ordinal space rather than appending: from ordinal
    // 124 up, every word of this game sits four below its namesake there. A
    // table baked from Die Enviro-Kids greifen ein would mis-name the whole tail, which is
    // why the binding is scanned out of the game's own binary at open time.
    assert_eq!(b.name(239), Some("DOWALK"));
    assert_eq!(b.name(250), Some("PLAYSAMPLE"));
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
    assert_eq!(bodies, 481, "every colon definition in the game");
}
