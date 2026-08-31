//! End-to-end checks against the files Victor Loomes ships.
//!
//! These need the original files. Point `MOTIONVM_GAMEDATA_VLOOMES` at the
//! directory holding `DATA.-1-` and `LL.EXE`; the tests skip themselves if it
//! is missing, so the crate still builds and tests cleanly without the game.
//! Every number asserted here is a measurement over those files.
//!
//! The three other 16-bit games are all the later framing of the container.
//! What this one is for is the earlier one: **no packing field, occupancy
//! sixteen bytes sooner, packing hardwired per segment, and a packed header
//! one word longer**. A reader that had taken the later framing for the
//! format itself would pass every other suite and fail here — which is what
//! it did until the arm this file drives was written. It is also the only
//! game whose kernel binds its domain table anywhere but 105, so it is the
//! test that the base is read from the build rather than assumed.
//!
//! The game this file drives is Victor Loomes (MOTION 16-bit).

use motionvm_motion_formats::m16::{Container, Framing, GfxInf, Segment, gfx, mz, psm, scr, text};
use motionvm_motion_testutil::{game_file, gamedata_vloomes};

macro_rules! container_or_skip {
    () => {
        match gamedata_vloomes() {
            Some(dir) => Container::open_dir(&dir).expect("open DATA.-1-"),
            None => {
                eprintln!("skipping: no Victor Loomes gamedata directory");
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
            "{} {id} is listed present and hands out nothing",
            seg.name()
        );
    }
    v
}

#[test]
fn the_header_is_the_earlier_framing_and_says_so_by_adding_up() {
    let c = container_or_skip!();
    assert_eq!(c.framing(), Framing::Earlier);
    assert_eq!((c.boot().module, c.boot().word), (100, 449));
    let counts: Vec<usize> = Segment::ALL.iter().map(|&s| c.slot_count(s)).collect();
    assert_eq!(counts, [1200, 500, 700, 21, 10, 10, 30]);
    assert_eq!(counts.iter().sum::<usize>(), 2471);
    // One volume, whatever the word at 0x12 says — and it says two.
    assert_eq!((c.volumes(), c.spare_offsets()), (1, 0));
    // 22 + 2*2471 + 4*(2471 + 0), the identity the framing is told by.
    assert_eq!(c.first_item_offset(), 14848);
    assert_eq!(c.trailing_slack(), 0);
}

#[test]
fn packing_is_the_generations_and_not_the_headers() {
    let c = container_or_skip!();
    // Nothing in the file says which segments are packed; the earlier framing
    // has no field for it. These four hold for both games that use it.
    for seg in [Segment::Gfx, Segment::Fnt, Segment::Frt] {
        assert!(c.packed(seg), "{} is packed", seg.name());
    }
    for seg in [Segment::Blk, Segment::Scr, Segment::Pal, Segment::Txt] {
        assert!(!c.packed(seg), "{} is stored plainly", seg.name());
    }
}

#[test]
fn every_slot_that_claims_bytes_has_them() {
    let c = container_or_skip!();
    let per: Vec<usize> = Segment::ALL.iter().map(|&s| ids(&c, s).len()).collect();
    assert_eq!(per, [721, 226, 36, 21, 2, 1, 25]);
    assert_eq!(per.iter().sum::<usize>(), 1032);
    // The occupancy word is a plain 0/1 here, not a volume bitmask, and it
    // agrees with the offsets in every one of the 2471 slots.
    assert!(c.occupancy_mismatches().is_empty());
}

#[test]
fn every_sprite_unpacks_to_exactly_its_header_plus_its_pixels() {
    let c = container_or_skip!();
    let mut widest = 0;
    let mut tallest = 0;
    for id in ids(&c, Segment::Gfx) {
        let item = c.item(Segment::Gfx, id).unwrap().unwrap();
        let s = gfx::Sprite::parse(item).unwrap_or_else(|e| panic!("sprite {id}: {e}"));
        assert_eq!(
            item.len(),
            gfx::HEADER_LEN + s.width as usize * s.height as usize,
            "sprite {id}"
        );
        widest = widest.max(s.width);
        tallest = tallest.max(s.height);
    }
    assert_eq!((widest, tallest), (216, 195));
}

#[test]
fn the_side_table_of_sizes_agrees_with_the_sprites_it_describes() {
    let Some(dir) = gamedata_vloomes() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    let c = Container::open_dir(&dir).expect("open DATA.-1-");
    let inf = GfxInf::open_dir(&dir)
        .expect("read GFX.INF")
        .expect("Victor Loomes ships one");
    // One entry per slot, and the file is nothing but entries.
    assert_eq!(inf.len(), c.slot_count(Segment::Gfx));
    assert_eq!(inf.present(), ids(&c, Segment::Gfx));
    for id in inf.present() {
        let item = c.item(Segment::Gfx, id).unwrap().unwrap();
        let s = gfx::Sprite::parse(item).unwrap();
        assert_eq!(inf.size(id), Some((s.width, s.height)), "slot {id}");
    }
}

#[test]
fn every_palette_is_768_six_bit_bytes() {
    let c = container_or_skip!();
    let present = ids(&c, Segment::Pal);
    assert_eq!(present, (0..21).collect::<Vec<_>>());
    for id in present {
        let item = c.item(Segment::Pal, id).unwrap().unwrap();
        assert_eq!(item.len(), 768, "palette {id}");
        assert!(item.iter().all(|&b| b <= 63), "palette {id} is six-bit");
    }
}

#[test]
fn every_text_table_parses_and_reads_as_german() {
    let c = container_or_skip!();
    let present = ids(&c, Segment::Txt);
    assert_eq!(present.len(), 25);
    let mut strings = 0;
    for id in &present {
        let item = c.item(Segment::Txt, *id).unwrap().unwrap();
        let t = text::parse(item).unwrap_or_else(|e| panic!("text {id}: {e}"));
        strings += t.strings.len();
    }
    assert_eq!(strings, 1336);
    // The credits table names the studio the corpus documents.
    let credits = text::parse(c.item(Segment::Txt, 8).unwrap().unwrap()).unwrap();
    assert!(credits.strings[1].contains("Promotion Software"));
}

#[test]
fn every_module_parses_and_the_locations_come_in_pairs() {
    let c = container_or_skip!();
    let present = ids(&c, Segment::Scr);
    assert_eq!(
        present,
        [
            21, 22, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 100, 101, 102, 104, 105, 106, 107, 108,
            109, 110, 111, 112, 113, 599, 600, 602, 603, 604, 605, 606, 607, 608, 609, 610
        ]
    );
    let (mut code, mut vars, mut consts) = (0, 0, 0);
    for id in &present {
        let m = scr::ScrModule::parse(c.item(Segment::Scr, *id).unwrap().unwrap())
            .unwrap_or_else(|e| panic!("module {id}: {e}"));
        assert_eq!(m.module as usize, *id, "module number is its slot");
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
    assert_eq!((code, vars, consts), (695, 472, 132));

    // A location is two modules, N+100 and N+20, and every macro module
    // defines the one word id 549 — the id-reuse discipline the later games
    // follow with sixteen and twenty of them.
    for n in 1..=13 {
        if n == 3 {
            // Location 3 shares location 2's pair; INCLORT special-cases it.
            continue;
        }
        assert!(
            present.contains(&(n + 100)),
            "script module for location {n}"
        );
        assert!(present.contains(&(n + 20)), "macro module for location {n}");
    }
    for n in [21, 22, 24, 33] {
        let m = scr::ScrModule::parse(c.item(Segment::Scr, n).unwrap().unwrap()).unwrap();
        assert_eq!(m.entries.len(), 1);
        assert_eq!(m.entries[0].id, 549);
    }
}

#[test]
fn the_boot_module_names_the_word_the_header_points_at() {
    let c = container_or_skip!();
    let m = scr::ScrModule::parse(c.item(Segment::Scr, 100).unwrap().unwrap()).unwrap();
    assert_eq!((m.first_id, m.last_id), (400, 449));
    let names: Vec<&str> = m.entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"RUN"), "the boot word");
    assert!(names.contains(&"CTRL"), "the controller");
    assert!(names.contains(&"INCLORT"), "the location switch");
    let run = m.entries.iter().find(|e| e.id == c.boot().word).unwrap();
    assert_eq!(run.name, "RUN");
}

#[test]
fn every_song_is_a_bare_plx_section() {
    let c = container_or_skip!();
    let songs: Vec<usize> = ids(&c, Segment::Blk)
        .into_iter()
        .filter(|&id| psm::is_song(c.item(Segment::Blk, id).unwrap().unwrap()))
        .collect();
    assert_eq!(songs, (0..14).collect::<Vec<_>>());
    for id in &songs {
        let item = c.item(Segment::Blk, *id).unwrap().unwrap();
        // No module around it, so no section table and no sample sections.
        assert!(!psm::is_module(item), "block {id} is not wrapped");
        assert!(psm::tags(item).is_none());
        let plx = psm::Plx::parse(item).unwrap_or_else(|e| panic!("block {id}: {e}"));
        assert!(plx.speed > 0 && plx.tempo > 0, "block {id} has a clock");
        let used: Vec<u16> = plx.channels.iter().copied().filter(|&o| o != 0).collect();
        assert!(used.windows(2).all(|w| w[0] < w[1]), "block {id} ascends");
    }
}

#[test]
fn the_kernel_binds_its_own_base_and_not_the_later_builds() {
    let Some(dir) = gamedata_vloomes() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    let img = mz::Image::open(game_file(&dir, "LL.EXE")).expect("LL.EXE opens");
    // The load image begins where no other build's does.
    assert_eq!(img.header_len(), 0x1e00);
    let words = mz::kernel_words(&img);
    let domain = words.iter().filter(|w| w.table == 0).count();
    let core = words.iter().filter(|w| w.table == 1).count();
    assert_eq!((domain, core), (124, 80));

    // Twenty-one placeholders sit between the tables, against the later
    // builds' twenty-two, which is what moves the domain base from 105 to 102.
    let ds = mz::kernel_words(&img)
        .first()
        .map(|w| {
            let e = w.entry as usize;
            u16::from_le_bytes([img.bytes()[e + 2], img.bytes()[e + 3]])
        })
        .unwrap();
    assert_eq!(mz::placeholder_count(&img, ds), Some(21));

    let b = mz::binding_of(&img, &words).expect("the inline words are all named");
    assert_eq!(b.len(), 204);
    assert_eq!(b.name(1), Some("##"));
    assert_eq!(b.name(102), Some("TOGFX"));
    assert_eq!(b.name(225), Some("_POOR"));
    // The word the later builds added after this one, and which this kernel
    // therefore has no ordinal for.
    assert_eq!(b.ordinal("_PutStringAdr"), None);
    assert_eq!(b.inline.put_string_adr, None);
}

#[test]
fn this_builds_hot_area_test_has_no_hole_case() {
    let Some(dir) = gamedata_vloomes() else {
        eprintln!("skipping: no Victor Loomes gamedata directory");
        return;
    };
    let img = mz::Image::open(game_file(&dir, "LL.EXE")).expect("LL.EXE opens");
    let words = mz::kernel_words(&img);
    // `?XINSIDE` here is 71 instructions and stops after the four corner
    // comparisons. The two later builds run 101 and pass over an entry whose
    // corners are all zero — so in this game a hole in a hot-area table is a
    // rectangle at the origin, and a point at 0,0 is inside it.
    assert!(!mz::skips_empty_areas(&img, &words));

    // Not the generation's: Jeff Jet is the later framing and its build has
    // no hole case either, while Amajambere's, which sits between the two,
    // has one. Reading it off the game rather than the binary would put this
    // in the wrong place for two of the four.
    for (other, expected) in [
        (motionvm_motion_testutil::gamedata_enviro(), true),
        (motionvm_motion_testutil::gamedata_jeffjet(), false),
        (motionvm_motion_testutil::gamedata_hfa(), true),
    ] {
        let Some(other) = other else { continue };
        let exe = ["ENVIRO.EXE", "HPPLAY.EXE", "BMZ.EXE"]
            .into_iter()
            .find_map(|n| motionvm_motion_formats::find_ci(&other, n))
            .expect("an engine binary");
        let img = mz::Image::open(&exe).expect("the binary opens");
        let words = mz::kernel_words(&img);
        assert_eq!(
            mz::skips_empty_areas(&img, &words),
            expected,
            "{}",
            exe.display()
        );
    }
}
