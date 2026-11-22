//! The sample clock a driver's sequencer runs on.
//!
//! No driver of the time counts in samples. Each asks its host for a tick
//! period in PIT cycles and lets the timer interrupt call it, so playing one
//! at an arbitrary output rate means converting between the two — and doing it
//! without drift, which means integers only. A rendered frame is worth
//! `pit_hz / rate` cycles, so multiplying every side through by `rate` keeps
//! the arithmetic whole and the remainder carries from one buffer to the next.
//!
//! What the drivers do *not* share is `pit_hz`: one program writes the
//! constant as 1 193 180 and another as 1 193 182, and each is what its own
//! music was timed against. So it is an argument here and a constant with
//! each driver.

/// How many frames may be rendered before the next tick falls due, given how
/// far the clock has run into a tick that lasts `tick_period`.
///
/// Rounds up: the tick belongs to the frame that reaches the mark, not to the
/// one before it.
pub fn frames_to_tick(clock: u64, tick_period: u64, pit_hz: u32) -> usize {
    // More frames than a `usize` counts is more than a device asks for in
    // one call, so the saturation only ever answers "as many as you want".
    usize::try_from(
        tick_period
            .saturating_sub(clock)
            .div_ceil(u64::from(pit_hz)),
    )
    .unwrap_or(usize::MAX)
}
