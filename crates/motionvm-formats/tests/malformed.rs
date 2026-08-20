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

use motionvm_formats::{TextTable, gfx::Sprite, hmi::Song, le::Image, rsc::Rsc, scr::ScrModule};

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
    assert!(TextTable::parse(&[]).is_ok());
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
    let e = motionvm_formats::font::Font::parse(&item)
        .unwrap_err()
        .to_string();
    assert!(e.starts_with("font:"), "a font error should say so: {e}");
}
