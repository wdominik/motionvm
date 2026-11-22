//! The conversions between the values a driver holds and the bytes the chip
//! takes — each named for the rule it is, so a call site says what happens to
//! the bits.
//!
//! These are the ones every driver over the chip applies alike: a register
//! offset formed from a slot number, the low byte of a register computation,
//! and a frame count on the clock. What a single driver's arithmetic does
//! beyond them is that driver's own, beside it.

/// An operator, voice or channel number as the register offset it addresses.
/// The OPL3 has eighteen operator slots and nine voices a bank, and MIDI
/// sixteen channels; nothing here counts past 0x16, so the number is the
/// offset.
#[expect(
    clippy::as_conversions,
    reason = "an index below 0x16 as the byte it adds to a register base"
)]
pub fn reg(i: usize) -> u8 {
    i as u8
}

/// The low byte of a register computation — what the chip is handed of it,
/// which is the driver's own truncation.
pub fn byte(v: u32) -> u8 {
    v.to_le_bytes()[0]
}

/// A frame count on the clock, which counts PIT cycles in 64 bits. Lossless
/// on every target this workspace builds for.
#[expect(
    clippy::as_conversions,
    reason = "a widening on every target this builds for; `u64` has no `From<usize>`"
)]
pub fn frames(n: usize) -> u64 {
    n as u64
}
