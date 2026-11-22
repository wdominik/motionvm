//! The conversions the FM driver applies between the values it holds and the
//! bytes the chip takes — each named for the rule it is, so a call site says
//! what happens to the bits, and the one place this crate changes a value's
//! width.
//!
//! The driver is byte arithmetic read out of the original binary: a level is
//! multiplied in a word and the low byte is what reaches the register, a
//! mixed sample is held at the rails, a signed header field is taken as the
//! unsigned word the arithmetic reads. The functions below are those
//! readings; the ones every driver over the chip applies alike are the
//! chip's own, and come from there.

pub(crate) use motionvm_opl::num::{byte, frames, reg};

/// A mixed sample value back into the device's sixteen bits, held at the
/// rails rather than wrapped: a voice scaled by a volume stays within them
/// by construction, and the clamp is the proof.
pub(crate) fn sample(v: i32) -> i16 {
    i16::try_from(v.clamp(i32::from(i16::MIN), i32::from(i16::MAX))).unwrap_or(0)
}

/// A song's signed header field as the unsigned word the driver's arithmetic
/// takes it for: the same bits, sign-extended to 32.
pub(crate) fn unsigned(v: i32) -> u32 {
    u32::from_le_bytes(v.to_le_bytes())
}
