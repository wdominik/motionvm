//! The sample clock both stacks run their sequencer on.
//!
//! Neither driver counts time in samples. Both ask the sound host for a tick
//! period in PIT cycles and let the timer interrupt call them, so playing one
//! at an arbitrary output rate means converting between the two — and doing it
//! without drift, which means integers only. A rendered frame is worth
//! `pit_hz / rate` cycles, so multiplying every side through by `rate` keeps
//! the arithmetic whole and the remainder carries from one buffer to the next.
//!
//! What the two do *not* share is `pit_hz`: the 32-bit engine writes the
//! constant as 1 193 180 and `MUSADL.DRV` as 1 193 182, and each is what its
//! own music was timed against. So it is an argument here and a constant
//! there.

/// How many frames may be rendered before the next tick falls due, given how
/// far the clock has run into a tick that lasts `tick_period`.
///
/// Rounds up: the tick belongs to the frame that reaches the mark, not to the
/// one before it.
pub fn frames_to_tick(clock: u64, tick_period: u64, pit_hz: u32) -> usize {
    tick_period
        .saturating_sub(clock)
        .div_ceil(u64::from(pit_hz)) as usize
}
