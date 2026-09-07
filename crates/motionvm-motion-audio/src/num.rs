//! The conversions the drivers apply between the values they hold and the
//! bytes the chip takes — each named for the rule it is, so a call site says
//! what happens to the bits, and the one place this crate changes a value's
//! width.
//!
//! The drivers are byte arithmetic read out of the original binaries: a level
//! is multiplied in a word and the low byte is what reaches the register, a
//! frequency is a word split across two registers, a loop count is a signed
//! `-1` the driver reads as endless. The functions below are those readings.

/// An operator, voice or channel number as the register offset it addresses.
/// The OPL3 has eighteen operator slots and nine voices a bank, and MIDI
/// sixteen channels; nothing here counts past 0x16, so the number is the
/// offset.
#[expect(
    clippy::as_conversions,
    reason = "an index below 0x16 as the byte it adds to a register base"
)]
pub(crate) fn reg(i: usize) -> u8 {
    i as u8
}

/// The low byte of a register computation — what the chip is handed of it,
/// which is the driver's own truncation.
pub(crate) fn byte(v: u32) -> u8 {
    v.to_le_bytes()[0]
}

/// A mixed sample value back into the device's sixteen bits, held at the
/// rails rather than wrapped: a voice scaled by a volume stays within them
/// by construction, and the clamp is the proof.
pub(crate) fn sample(v: i32) -> i16 {
    i16::try_from(v.clamp(i32::from(i16::MIN), i32::from(i16::MAX))).unwrap_or(0)
}

/// The low byte of a driver word: the `A0` half of a frequency, the level
/// bits of a volume.
pub(crate) fn lo(w: u16) -> u8 {
    w.to_le_bytes()[0]
}

/// The high byte of a driver word: the `B0` half of a frequency, the flag
/// bits above a level.
pub(crate) fn hi(w: u16) -> u8 {
    w.to_le_bytes()[1]
}

/// The low sixteen bits of a driver word held in 32 — the fade level out of
/// its fixed-point position.
pub(crate) fn word(v: i32) -> u16 {
    let [a, b, ..] = v.to_le_bytes();
    u16::from_le_bytes([a, b])
}

/// A driver word read signed, the way a `js` after a `cmp` reads it.
pub(crate) fn signed(w: u16) -> i16 {
    i16::from_le_bytes(w.to_le_bytes())
}

/// A loop count as the driver reads it: unsigned, so `-1` is `0xFFFF` and
/// endless.
pub(crate) fn passes(loops: i16) -> u16 {
    u16::from_le_bytes(loops.to_le_bytes())
}

/// A song's signed header field as the unsigned word the driver's arithmetic
/// takes it for: the same bits, sign-extended to 32.
pub(crate) fn unsigned(v: i32) -> u32 {
    u32::from_le_bytes(v.to_le_bytes())
}

/// A frame count on the clock, which counts PIT cycles in 64 bits. Lossless
/// on every target this workspace builds for.
#[expect(
    clippy::as_conversions,
    reason = "a widening on every target this builds for; `u64` has no `From<usize>`"
)]
pub(crate) fn frames(n: usize) -> u64 {
    n as u64
}
