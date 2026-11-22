//! Damaged input is reported, not crashed on.
//!
//! Every other test in this crate reads the game's own files, which are
//! well-formed by construction — so the whole suite could pass while the
//! readers panicked on the first byte anyone else's copy had wrong. These are
//! the counter-tests: each one hands a reader input that is broken in one
//! specific way and asserts an `Err` comes back rather than an unwind.
//!
//! They need no game data. Everything here is built in memory.

use motionvm_parsec_formats::fgroup::open_dat;
use motionvm_parsec_formats::lz::decompress;
use motionvm_parsec_formats::{Dialogue, HotspotTable, P2l, Picture, Rib, Room};

/// A RIB header whose bufSize would allocate more than the decoder allows.
#[test]
fn a_rib_asking_for_an_impossible_buffer_is_refused() {
    let mut v = b"RIB\0".to_vec();
    v.extend_from_slice(&(16 * 1024 * 1024u32).to_le_bytes());
    v.extend_from_slice(&[1, 2, 3]); // payload shorter than bufSize → LZ path
    assert!(Rib::parse(&v).is_err());
}

/// A truncated RIB is an error, not a panic.
#[test]
fn a_truncated_rib_is_refused() {
    assert!(Rib::parse(b"RIB").is_err());
    assert!(Rib::parse(b"RIB\0\x01").is_err());
}

/// Wrong magic is an error with the name of the reader in it.
#[test]
fn a_non_rib_is_not_parsed_as_one() {
    let e = Rib::parse(b"XXXX\x00\x00\x00\x00").unwrap_err().to_string();
    assert!(e.contains("RIB"), "{e}");
}

/// An LZ run that would write before the start of the buffer is refused.
#[test]
fn an_lz_run_that_overruns_is_refused() {
    assert!(decompress(&[0x0F, 0xAA], 4).is_err());
}

/// A picture header whose dimensions do not match the payload is refused.
#[test]
fn a_picture_with_the_wrong_payload_size_is_refused() {
    // 10×10 wants 100 pixels; we give 3.
    let v = [10, 0, 10, 0, 1, 2, 3];
    assert!(Picture::parse(&v).is_err());
}

/// A zero-size picture is refused rather than producing an empty frame that
/// looks like a successful decode.
#[test]
fn a_zero_size_picture_is_refused() {
    assert!(Picture::parse(&[0, 0, 0, 0]).is_err());
}

/// A hotspot table whose count runs past the buffer is refused.
#[test]
fn a_hotspot_count_that_overruns_is_refused() {
    let mut v = vec![8, 0, 0, 0, 0, 0, 50, 0, 0, 0, 0, 0];
    v.resize(30, 0);
    assert!(HotspotTable::parse(&v).is_err());
}

/// An II blob without the tag is refused.
#[test]
fn a_dialogue_without_the_tag_is_refused() {
    assert!(Dialogue::parse(&[0; 16]).is_err());
}

/// An RDV without the magic is refused.
#[test]
fn an_rdv_without_the_magic_is_refused() {
    let v = vec![0u8; 500];
    assert!(Room::parse(&v).is_err());
}

/// `open_dat` on random bytes is an error, not a guessed layout.
#[test]
fn random_bytes_are_not_a_dat() {
    assert!(open_dat(&[1, 2, 3, 4, 5, 6, 7, 8]).is_err());
}

/// A music library cut short in its header, or naming a table that runs
/// past its end, is refused.
#[test]
fn a_damaged_music_library_is_refused() {
    let mut head = motionvm_parsec_formats::p2l::MAGIC.to_vec();
    assert!(P2l::parse(head.clone()).is_err(), "no triples");
    head.resize(0x34, 0);
    // Table C at 0x34, 16 bytes long, in a file that ends at 0x34.
    head[0x28] = 0x34;
    head[0x30] = 16;
    assert!(P2l::parse(head.clone()).is_err(), "table C past the end");
    // A table that is not whole rows.
    head[0x30] = 15;
    head.resize(0x34 + 15, 0);
    assert!(P2l::parse(head).is_err(), "half a row");
}
