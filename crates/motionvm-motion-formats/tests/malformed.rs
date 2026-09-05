//! Damaged input is reported, not crashed on.
//!
//! Every other test in this crate reads the game's own files, which are
//! well-formed by construction — so the whole suite could pass while the
//! readers panicked on the first byte anyone else's copy had wrong. These are
//! the counter-tests: each one hands a reader input that is broken in one
//! specific way and asserts an `Err` comes back rather than an unwind.
//!
//! They need no game data. Everything here is built in memory, which is the
//! point: it is the one file in the suite that runs everywhere.
//!
//! The failures they are aimed at were real. `1usize << max_bits` with a
//! `max_bits` read straight from a sprite header panics in a debug build and
//! asks for terabytes in a release one; the LE fixup walker indexed a
//! file-supplied cursor raw for thirty lines after checking the first byte of
//! it. Both were reachable from a truncated download.
//!
//! The first half hands broken input to the 32-bit readers, the second half
//! to the 16-bit ones.

use motionvm_motion_formats::font::Font;
use motionvm_motion_formats::m16;
use motionvm_motion_formats::m32::{gfx::Sprite, hmi::Song, le::Image, rsc::Rsc, scr::ScrModule};

/// A sprite header with the fields the reader looks at, and nothing after it.
///
/// `unpacked` at +778 and `max_bits` at +786 are the two the decoder trusts
/// before it has read a single code.
fn sprite_header(unpacked: u32, max_bits: u32) -> Vec<u8> {
    let mut v = vec![0u8; 790];
    v[770..778].copy_from_slice(b"32BITGFX");
    v[778..782].copy_from_slice(&unpacked.to_le_bytes());
    v[786..790].copy_from_slice(&max_bits.to_le_bytes());
    v
}

#[test]
fn a_sprite_asking_for_an_impossible_dictionary_is_refused() {
    // 40 bits would be a terabyte of dictionary; 64 is not a shift at all.
    for max_bits in [0, 8, 13, 40, 64, u32::MAX] {
        let item = sprite_header(64, max_bits);
        assert!(
            Sprite::parse(&item).is_err(),
            "max_bits {max_bits} was accepted"
        );
    }
}

#[test]
fn the_widths_the_original_writes_are_still_accepted() {
    // The guard must not be so tight that it rejects real sprites. 11 and 12
    // are what GFXCRUNCH emits; they get past the width check and fail later,
    // on the empty stream, which is a different error and the right one.
    for max_bits in [11, 12] {
        let item = sprite_header(64, max_bits);
        let e = Sprite::parse(&item).unwrap_err().to_string();
        assert!(
            !e.contains("dictionary ceiling"),
            "max_bits {max_bits} should pass the width check, got: {e}"
        );
    }
}

#[test]
fn a_sprite_declaring_four_gigabytes_does_not_try_to_allocate_them() {
    // Reaching the end of this call at all is the assertion: a reservation of
    // the declared size would abort the process before it could return.
    let item = sprite_header(u32::MAX, 12);
    assert!(Sprite::parse(&item).is_err());
}

#[test]
fn a_sprite_smaller_than_its_own_inner_header_is_refused() {
    // `declared - 6` wrapped to about four billion in release builds.
    for declared in [0, 1, 5] {
        let item = sprite_header(declared, 12);
        assert!(
            Sprite::pixel_count(&item).is_err(),
            "declared size {declared} was accepted"
        );
    }
}

#[test]
fn a_truncated_executable_is_reported_rather_than_indexed_past() {
    // A real LE image with everything after the header cut off. The trap the
    // fixup walker has to avoid is indexing a file-supplied cursor raw once it
    // is inside a record: the bounds were checked on the way in, and a record
    // that describes more file than there is walks straight off the end.
    let mut file = vec![0u8; 0x200];
    file[0..2].copy_from_slice(b"MZ");
    file[0x3c..0x40].copy_from_slice(&0x80u32.to_le_bytes());
    file[0x80..0x82].copy_from_slice(b"LE");
    // Page and object counts that describe far more file than there is.
    file[0x80 + 0x14..0x80 + 0x18].copy_from_slice(&0x1000u32.to_le_bytes());
    file[0x80 + 0x28..0x80 + 0x2c].copy_from_slice(&0x1000u32.to_le_bytes());
    file[0x80 + 0x44..0x80 + 0x48].copy_from_slice(&0x40u32.to_le_bytes());
    assert!(Image::parse(&file).is_err());
}

#[test]
fn every_reader_refuses_an_empty_buffer() {
    // The cheapest malformed input there is. A reader that indexes before it
    // measures fails here and nowhere else.
    assert!(Rsc::from_bytes(Vec::new(), "empty".into()).is_err());
    assert!(Sprite::parse(&[]).is_err());
    assert!(ScrModule::parse(&[]).is_err());
    assert!(Song::parse(&[]).is_err());
    assert!(Image::parse(&[]).is_err());
    // The one deliberate exception, and it is documented on the method: text
    // slot 32 of the shipped game is two bytes, so a short table is empty
    // rather than an error.
    assert!(motionvm_motion_formats::m32::text::parse(&[]).is_ok());
}

#[test]
fn a_container_claiming_more_slots_than_the_file_holds_is_refused() {
    // 0x30 of header, six counts, then an offset table that cannot fit.
    let mut data = vec![0u8; 0x30];
    for i in 0..6 {
        data[i * 4..i * 4 + 4].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    }
    let e = match Rsc::from_bytes(data, "made up".into()) {
        Err(e) => e.to_string(),
        Ok(_) => panic!("an impossible slot total was accepted"),
    };
    assert!(
        e.starts_with("RSC container:"),
        "an RSC error should say so: {e}"
    );
}

#[test]
fn a_font_error_does_not_claim_to_be_an_rsc_error() {
    // One `Corrupt` variant is shared by every reader, so its message has to
    // carry the reader's name. Without that, a font error reads as an RSC
    // error and sends the reader to the wrong file.
    let mut item = vec![0u8; 64];
    item[6..8].copy_from_slice(&2048u16.to_le_bytes());
    item[8..10].copy_from_slice(&99u16.to_le_bytes()); // initial width, not 9
    let e = motionvm_motion_formats::m32::font::parse(&item)
        .unwrap_err()
        .to_string();
    assert!(e.starts_with("font:"), "a font error should say so: {e}");
}

// ---- the 16-bit readers ----------------------------------------------------

/// A `DATA.-n-` header with the given slot counts and nothing behind it.
fn dat_header(counts: [u16; 7]) -> Vec<u8> {
    let mut v = vec![0u8; 0x26];
    v[0..2].copy_from_slice(&100u16.to_le_bytes());
    v[2..4].copy_from_slice(&401u16.to_le_bytes());
    for (i, c) in counts.iter().enumerate() {
        v[4 + 2 * i..6 + 2 * i].copy_from_slice(&c.to_le_bytes());
    }
    v
}

#[test]
fn a_data_container_shorter_than_its_header_is_refused() {
    for len in [0, 1, 0x25] {
        assert!(m16::Container::from_bytes(vec![0; len], "t".into()).is_err());
    }
}

#[test]
fn a_data_container_with_no_slots_is_refused() {
    let v = dat_header([0; 7]);
    assert!(m16::Container::from_bytes(v, "t".into()).is_err());
}

#[test]
fn a_data_container_whose_tables_overrun_the_file_is_refused() {
    // Four thousand slots need 24 000 bytes of tables; the file has a header.
    let v = dat_header([2500, 1000, 700, 25, 10, 10, 100]);
    assert!(m16::Container::from_bytes(v, "t".into()).is_err());
}

#[test]
fn a_data_container_whose_first_item_sits_inside_the_tables_is_refused() {
    let counts = [1u16, 1, 1, 1, 1, 1, 1];
    let mut v = dat_header(counts);
    v.extend_from_slice(&[1u16.to_le_bytes(); 7].concat());
    // Seven offsets of 0 — inside the header — then the padding.
    v.extend_from_slice(&[0u8; 7 * 4 + 12]);
    assert!(m16::Container::from_bytes(v, "t".into()).is_err());
}

#[test]
fn a_sprite_whose_length_disagrees_with_its_size_is_refused() {
    let mut item = vec![0u8; 6 + 10];
    item[0..2].copy_from_slice(&4u16.to_le_bytes());
    item[2..4].copy_from_slice(&4u16.to_le_bytes());
    assert!(
        m16::gfx::Sprite::parse(&item).is_err(),
        "16 pixels declared, 10 present"
    );
    item.resize(6 + 16, 0);
    assert!(m16::gfx::Sprite::parse(&item).is_ok());
    assert!(
        m16::gfx::Sprite::parse(&item[..3]).is_err(),
        "no room for a header"
    );
}

#[test]
fn a_text_table_whose_count_overruns_the_item_is_refused() {
    let mut item = vec![0u8; 6];
    item[0..2].copy_from_slice(&1000u16.to_le_bytes());
    assert!(m16::text::parse(&item).is_err());
    // A real one: one string at offset 0.
    let item = [1u8, 0, 0, 0, b'H', b'i', 0];
    assert_eq!(m16::text::parse(&item).unwrap().get(0), Some("Hi"));
}

#[test]
fn a_font_whose_table_overruns_the_item_is_refused() {
    let mut item = vec![0u8; 8];
    item[0..2].copy_from_slice(&100u16.to_le_bytes());
    item[2..4].copy_from_slice(&12u16.to_le_bytes());
    assert!(Font::from_glyph_table(&item).is_err());
    assert!(m16::font::parse(&item).is_err());
}

/// A module header whose repeated triple can be made to disagree.
fn scr_header(first: u16, last: u16, n: u16, again: (u16, u16, u16)) -> Vec<u8> {
    let mut v = vec![0u8; 0x22];
    for (i, x) in [first, last, n, again.0, again.1, again.2]
        .iter()
        .enumerate()
    {
        v[2 * i..2 * i + 2].copy_from_slice(&x.to_le_bytes());
    }
    v[0x20..0x22].copy_from_slice(&100u16.to_le_bytes());
    v
}

#[test]
fn a_module_whose_repeated_header_disagrees_is_refused() {
    let v = scr_header(400, 401, 2, (400, 401, 3));
    assert!(m16::scr::ScrModule::parse(&v).is_err());
    assert!(
        m16::scr::ScrModule::parse(&v[..10]).is_err(),
        "shorter than a header"
    );
}

#[test]
fn a_module_whose_body_offsets_leave_no_room_for_a_header_is_refused() {
    let mut v = scr_header(400, 400, 1, (400, 400, 1));
    // One word whose body is said to start 2 cells into the dictionary — its
    // sixteen-byte header would have to start before the dictionary does.
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&[0u8; 32]);
    assert!(m16::scr::ScrModule::parse(&v).is_err());
}

#[test]
fn a_module_whose_body_table_overruns_the_item_is_refused() {
    let mut v = scr_header(400, 500, 1000, (400, 500, 1000));
    v.extend_from_slice(&[0u8; 4]);
    assert!(m16::scr::ScrModule::parse(&v).is_err());
}

#[test]
fn a_well_formed_minimal_module_parses() {
    // One word, `##` only: header at dictionary start, body 8 cells in.
    let mut v = scr_header(400, 400, 1, (400, 400, 1));
    v.extend_from_slice(&8u16.to_le_bytes());
    let mut header = vec![3u8];
    header.extend_from_slice(b"RUN\0\0\0\0\0\0\0\0");
    header.extend_from_slice(&400u16.to_le_bytes());
    header.extend_from_slice(&0u16.to_le_bytes());
    v.extend_from_slice(&header);
    v.extend_from_slice(&0x8001u16.to_le_bytes());
    let m = m16::scr::ScrModule::parse(&v).unwrap();
    assert_eq!(m.module, 100);
    assert_eq!(m.entries.len(), 1);
    assert_eq!(m.entries[0].name, "RUN");
    assert_eq!(m.entries[0].id, 400);
    assert_eq!(m.entries[0].body, [0x8001]);
    assert!(!m.entries[0].is_variable());
}

#[test]
fn a_minimal_container_tells_empty_slots_from_occupied_ones() {
    // Three GFX slots: an item, an empty slot, an item. The empty slot shares
    // its offset with the one after it — the rule the whole index rests on —
    // and must read as empty, not as the next item.
    let mut v = dat_header([3, 0, 0, 0, 0, 0, 0]);
    v.extend_from_slice(&[1u16, 0, 1].map(u16::to_le_bytes).concat());
    let first = u32::try_from(v.len() + 3 * 4 + 12).unwrap();
    v.extend_from_slice(&[first, first + 6, first + 6].map(u32::to_le_bytes).concat());
    v.extend_from_slice(&[0u8; 12]);
    v.extend_from_slice(&[1, 0, 1, 0, 0, 0]); // a 1x1 sprite
    v.extend_from_slice(&[2, 0, 1, 0, 0, 0, 7, 8]); // a 2x1 sprite
    let c = m16::Container::from_bytes(v, "t".into()).unwrap();
    assert_eq!(c.present(m16::Segment::Gfx), [0, 2]);
    assert!(c.item(m16::Segment::Gfx, 1).unwrap().is_none());
    assert_eq!(
        c.item(m16::Segment::Gfx, 0).unwrap().map(<[u8]>::len),
        Some(6)
    );
    assert_eq!(
        c.item(m16::Segment::Gfx, 2).unwrap().map(<[u8]>::len),
        Some(8)
    );
    assert!(c.occupancy_mismatches().is_empty());
    assert_eq!(c.trailing_slack(), 0);
    assert_eq!(c.boot().module, 100);
}

/// A one-volume `DATA.-n-` over three GFX slots: an item, an empty slot, an
/// item — with the header words the caller wants to set.
///
/// `volumes` is the count at 18 and `packed_gfx` the GFX flag at `0x16`; the
/// two items are handed in whole, header and all, so a test can give a packed
/// one a header of its choosing.
fn dat_one_volume(volumes: u16, packed_gfx: u16, items: [&[u8]; 2]) -> Vec<u8> {
    let mut v = dat_header([3, 0, 0, 0, 0, 0, 0]);
    v[18..20].copy_from_slice(&volumes.to_le_bytes());
    v[0x16..0x18].copy_from_slice(&packed_gfx.to_le_bytes());
    v.extend_from_slice(&[1u16, 0, 1].map(u16::to_le_bytes).concat());
    let first = u32::try_from(v.len() + 3 * 4).unwrap();
    let second = first + u32::try_from(items[0].len()).unwrap();
    v.extend_from_slice(&[first, second, second].map(u32::to_le_bytes).concat());
    v.extend_from_slice(items[0]);
    v.extend_from_slice(items[1]);
    v
}

/// The eight bytes a packed item opens with, and then a GFXCRUNCH stream of
/// three literal 9-bit codes — the shortest real one there is.
fn packed_item(dictionary: u16, initial_width: u16) -> Vec<u8> {
    let stream = [0x00u8, 0x80, 0x80, 0x60];
    let mut v = Vec::new();
    v.extend_from_slice(&3u16.to_le_bytes());
    v.extend_from_slice(&u16::try_from(stream.len()).unwrap().to_le_bytes());
    v.extend_from_slice(&dictionary.to_le_bytes());
    v.extend_from_slice(&initial_width.to_le_bytes());
    v.extend_from_slice(&stream);
    v
}

#[test]
fn a_container_declaring_more_volumes_than_it_was_handed_is_refused() {
    // Half a game reads as a whole one with most of its slots empty, and a
    // 16-bit game can keep every palette and font on the volume that is
    // missing. Saying so is the only safe answer.
    let v = dat_one_volume(2, 0, [&[1, 0, 1, 0, 0, 0], &[2, 0, 1, 0, 0, 0, 7, 8]]);
    assert!(m16::Container::from_bytes(v, "t".into()).is_err());
}

#[test]
fn a_slot_flagged_for_a_volume_that_is_not_there_reads_as_empty() {
    // The occupancy word is a bitmask over volumes. A one-volume container
    // whose slot 2 says "volume 2" is a contradiction, not a reason to index
    // into a volume that does not exist: the slot has no bytes and the
    // container says which slots disagree with it.
    let mut v = dat_one_volume(1, 0, [&[1, 0, 1, 0, 0, 0], &[2, 0, 1, 0, 0, 0, 7, 8]]);
    v[0x26 + 4..0x26 + 6].copy_from_slice(&2u16.to_le_bytes());
    let c = m16::Container::from_bytes(v, "t".into()).unwrap();
    assert_eq!(c.present(m16::Segment::Gfx), [0]);
    assert!(c.item(m16::Segment::Gfx, 2).unwrap().is_none());
    assert_eq!(c.occupancy_mismatches(), [2]);
}

#[test]
fn a_packed_item_whose_header_is_not_gfxcrunch_is_refused() {
    for (dictionary, width) in [(1024, 9), (2048, 12), (0, 0)] {
        let item = packed_item(dictionary, width);
        let v = dat_one_volume(1, 1, [&item, &item]);
        assert!(
            m16::Container::from_bytes(v, "t".into()).is_err(),
            "dictionary {dictionary}, width {width}"
        );
    }
    // A packed flag over an item that carries no header at all is the same
    // answer: the flag says how to read it and the bytes say it cannot be.
    let v = dat_one_volume(1, 1, [&[1, 0, 1, 0, 0, 0], &[2, 0, 1, 0, 0, 0, 7, 8]]);
    assert!(m16::Container::from_bytes(v, "t".into()).is_err());
}

#[test]
fn a_packed_item_is_handed_out_unpacked() {
    let item = packed_item(2048, 9);
    let v = dat_one_volume(1, 1, [&item, &item]);
    let c = m16::Container::from_bytes(v, "t".into()).unwrap();
    assert_eq!(c.item(m16::Segment::Gfx, 0).unwrap(), Some(&[1, 2, 3][..]));
    assert_eq!(c.item(m16::Segment::Gfx, 2).unwrap(), Some(&[1, 2, 3][..]));
    assert!(c.packed(m16::Segment::Gfx));
    assert!(!c.packed(m16::Segment::Txt));
}

#[test]
fn an_executable_that_is_not_mz_or_is_cut_short_is_refused() {
    assert!(m16::mz::Image::parse(vec![]).is_err());
    assert!(m16::mz::Image::parse(vec![0; 64]).is_err(), "no signature");
    let mut v = vec![0u8; 64];
    v[..2].copy_from_slice(b"MZ");
    v[8..10].copy_from_slice(&1000u16.to_le_bytes()); // 16 000 header bytes in 64
    assert!(m16::mz::Image::parse(v).is_err());
}

#[test]
fn an_executable_with_no_kernel_tables_binds_nothing() {
    let mut v = vec![0u8; 256];
    v[..2].copy_from_slice(b"MZ");
    v[8..10].copy_from_slice(&2u16.to_le_bytes()); // a 32-byte header
    let img = m16::mz::Image::parse(v).unwrap();
    assert_eq!(img.header_len(), 32);
    assert!(m16::mz::kernel_words(&img).is_empty());
    assert!(
        m16::mz::binding_of(&img, &[]).is_err(),
        "no inline words to bind"
    );
}

/// An earlier-framing container: a 22-byte header, one boolean occupancy word
/// per slot, then `total + spare` offsets and the items.
///
/// The framing is told from the file rather than from a field, by two
/// identities that have to hold at once — the first offset is where the
/// tables end, and the last slot's offset is the file's length — so a fixture
/// is only in the earlier framing if both are built to hold.
fn dat_earlier(counts: [u16; 7], spare: u16, items: &[&[u8]]) -> Vec<u8> {
    let total: usize = counts.iter().map(|&c| usize::from(c)).sum();
    assert_eq!(total, items.len(), "one item per slot, empty ones included");
    let mut v = vec![0u8; 22];
    v[0..2].copy_from_slice(&100u16.to_le_bytes());
    v[2..4].copy_from_slice(&449u16.to_le_bytes());
    for (i, c) in counts.iter().enumerate() {
        v[4 + 2 * i..6 + 2 * i].copy_from_slice(&c.to_le_bytes());
    }
    // Victor Loomes and Compaq both hold 2 here and ship one volume: in this
    // framing the word is not a count of volumes, and the reader does not
    // read it as one.
    v[18..20].copy_from_slice(&2u16.to_le_bytes());
    v[20..22].copy_from_slice(&spare.to_le_bytes());
    for item in items {
        v.extend_from_slice(&u16::from(!item.is_empty()).to_le_bytes());
    }
    let ends = 22 + total * 2 + (total + usize::from(spare)) * 4;
    let mut at = ends;
    let mut offsets = Vec::new();
    for item in items {
        offsets.push(u32::try_from(at).unwrap());
        at += item.len();
    }
    // The last slot's offset is the file's length, and the spares repeat it.
    let last = offsets.len() - 1;
    offsets[last] = u32::try_from(at).unwrap();
    for _ in 0..spare {
        offsets.push(u32::try_from(at).unwrap());
    }
    for o in &offsets {
        v.extend_from_slice(&o.to_le_bytes());
    }
    for item in items {
        v.extend_from_slice(item);
    }
    v
}

/// A packed item in the earlier framing: the eight-byte header with the unpacked
/// length repeated ahead of it, then a GFXCRUNCH stream.
fn earlier_packed_item(repeat: u16) -> Vec<u8> {
    let stream = [0x00u8, 0x80, 0x80, 0x60];
    let mut v = Vec::new();
    v.extend_from_slice(&repeat.to_le_bytes());
    v.extend_from_slice(&3u16.to_le_bytes());
    v.extend_from_slice(&u16::try_from(stream.len()).unwrap().to_le_bytes());
    v.extend_from_slice(&2048u16.to_le_bytes());
    v.extend_from_slice(&9u16.to_le_bytes());
    v.extend_from_slice(&stream);
    v
}

#[test]
fn a_truncated_earlier_framing_container_is_refused_as_one() {
    // BLK is the one segment the earlier framing does not pack, so the only
    // thing wrong with these is their length.
    let whole = dat_earlier([0, 2, 0, 0, 0, 0, 0], 1, &[b"abcd", b""]);
    assert!(
        m16::Container::from_volumes(vec![whole.clone()], "earlier".into()).is_ok(),
        "the fixture itself is a readable earlier-framing container"
    );

    // Cutting it breaks the second identity — the last slot's offset is no
    // longer the file's length — while the first still holds, because the
    // tables sit ahead of the cut. Without a word for that case the reader
    // fell through to the later framing and then blamed the word at 0x12, which
    // in this framing is not a volume count: both games that use it say two
    // and ship one file.
    for cut in [whole.len() - 1, whole.len() - 4] {
        let err = m16::Container::from_volumes(vec![whole[..cut].to_vec()], "earlier".into())
            .expect_err("a cut container is refused")
            .to_string();
        assert!(
            err.contains("last slot's offset is not the file's length"),
            "cut to {cut} was refused as something else: {err}"
        );
        assert!(
            !err.contains("volume(s)"),
            "cut to {cut} blamed a volume: {err}"
        );
    }

    // Cut back past the tables there is nothing left to recognise, and the
    // reader says so as it does for any other short file.
    let err = m16::Container::from_volumes(vec![whole[..30].to_vec()], "earlier".into())
        .expect_err("a container cut into its tables is refused")
        .to_string();
    assert!(err.contains("only 30 bytes"), "{err}");
}

#[test]
fn an_earlier_framing_packed_item_whose_repeated_length_disagrees_is_refused() {
    // GFX is packed in this framing, and its header repeats the unpacked
    // length ahead of the eight-byte one. The repeat is what says the item is
    // in the earlier framing at all — it agrees in all 723 packed items of
    // Victor Loomes and all 318 of Compaq — so a disagreement is the one
    // thing that distinguishes an earlier-framing header from eight bytes of
    // something else.
    let good = dat_earlier([2, 0, 0, 0, 0, 0, 0], 1, &[&earlier_packed_item(3), b""]);
    assert!(
        m16::Container::from_volumes(vec![good], "earlier".into()).is_ok(),
        "a header whose two lengths agree is read"
    );

    let bad = dat_earlier([2, 0, 0, 0, 0, 0, 0], 1, &[&earlier_packed_item(4), b""]);
    let err = m16::Container::from_volumes(vec![bad], "earlier".into())
        .expect_err("a header whose two lengths disagree is refused")
        .to_string();
    assert!(
        err.contains("do not open") && err.contains("GFXCRUNCH"),
        "{err}"
    );
}

#[test]
fn a_short_gfx_inf_answers_for_the_slots_it_does_describe() {
    // Four bytes an entry and no header, so the file's length is the only
    // thing that says how many slots there are. A trailing partial entry is
    // therefore not an error to report but a slot to leave unanswered — there
    // is nothing else it could mean, and refusing the file would throw away
    // the entries that are whole.
    let mut bytes = Vec::new();
    for (w, h) in [(64u16, 48u16), (0xffff, 0xffff), (16, 8)] {
        bytes.extend_from_slice(&w.to_le_bytes());
        bytes.extend_from_slice(&h.to_le_bytes());
    }
    let whole = m16::GfxInf::parse(&bytes);
    assert_eq!(whole.len(), 3);
    assert_eq!(whole.size(0), Some((64, 48)));
    assert_eq!(
        whole.size(1),
        None,
        "0xFFFF in both halves is an empty slot"
    );
    assert_eq!(whole.size(2), Some((16, 8)));
    assert_eq!(whole.present(), [0, 2]);
    assert_eq!(whole.size(3), None, "past the end is not a panic");

    for cut in [1, 2, 3] {
        let short = m16::GfxInf::parse(&bytes[..bytes.len() - cut]);
        assert_eq!(short.len(), 2, "the partial entry is not counted");
        assert_eq!(
            short.size(0),
            Some((64, 48)),
            "the whole entries still answer"
        );
        assert_eq!(short.size(2), None, "and the partial one does not");
    }

    let empty = m16::GfxInf::parse(&[]);
    assert!(empty.is_empty());
    assert_eq!(empty.size(0), None);
}
