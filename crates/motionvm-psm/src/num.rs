//! The driver's own byte arithmetic, read out of `MUSADL.DRV`: a level is
//! multiplied in a word and the low byte is what reaches the register, a
//! frequency is a word split across two registers, a loop count is a signed
//! `-1` the driver reads as endless. Each function is named for the rule it
//! is, so a call site says what happens to the bits; the rules every driver
//! over the chip applies alike are the chip's, and come from there.

pub(crate) use motionvm_opl::num::{byte, frames, reg};

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

/// The low sixteen bits of a double word: a quotient the driver's `div`
/// leaves in `ax`, a product's middle word.
pub(crate) fn low(v: u32) -> u16 {
    let [a, b, ..] = v.to_le_bytes();
    u16::from_le_bytes([a, b])
}

/// A 16.16 double word as the driver holds it in two words: the fraction
/// first, then the whole part.
pub(crate) fn split(v: u32) -> [u16; 2] {
    let [a, b, c, d] = v.to_le_bytes();
    [u16::from_le_bytes([a, b]), u16::from_le_bytes([c, d])]
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
