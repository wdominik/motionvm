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

use motionvm_formats::font::Font;
use motionvm_formats::m16;
use motionvm_formats::m32::{gfx::Sprite, hmi::Song, le::Image, rsc::Rsc, scr::ScrModule};

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
    assert!(motionvm_formats::m32::text::parse(&[]).is_ok());
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
    let e = motionvm_formats::m32::font::parse(&item)
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
    let first = (v.len() + 3 * 4 + 12) as u32;
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
    assert!(m16::mz::binding_of(&[]).is_err(), "no inline words to bind");
}
