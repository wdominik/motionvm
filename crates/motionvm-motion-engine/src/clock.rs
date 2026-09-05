//! How long a step lasts.
//!
//! Two clocks, not one. The game asks for a frame rate with `DELAY`, but while
//! a curtain runs the original's frame loop is not running at all — the fade
//! handler spins inside itself, waiting out one band at a time. So a "step" is
//! a frame most of the time and a band during a transition, and the two are far
//! apart: one tick a band on the title screen against eight ticks a frame.
//!
//! The ticks are unit-3 ticks, and turning them into wall-clock time goes
//! through the original's own two-stage arithmetic — a 1020 Hz master counter
//! and a truncating ×10/51 conversion — because the truncation is visible:
//! a 1-tick wait costs 6 master ticks, not 5.1. See [`Engine::frame_duration`].

use crate::Engine;

impl Engine {
    /// The unit-3 tick rate: what `DELAY` divides by.
    ///
    /// Written into the engine twice over. `DELAY` computes `200/n` into the
    /// frame-limiter cell, which only yields a period if the 200 is a rate;
    /// and `GIVETIMER` converts the master counter to unit 3 by ×10/51
    /// (`0x23761`) — exact at a 1020 Hz master, where unit 3 is 200 Hz to the
    /// digit. `25 DELAY` then means the 25 frames a second the game is built
    /// around. Measured against the original running under DOSBox-X — 930
    /// curtain bands, timed by the original's own unit-3 timer — the master
    /// averages ~978 Hz with and without the HMI drivers loaded: the design
    /// rate, minus emulator scheduling loss.
    pub const TICKS_PER_SECOND: u32 = 200;

    /// The master counter's design rate, from which unit 3 is derived.
    ///
    /// ×10/51 is `/5.1`, and `/5.1` lands on a whole number of Hz only at
    /// 1020. A real PIT can merely approximate it (divisor 1170 →
    /// 1019.81 Hz); the arithmetic the engine actually does is 1020/200
    /// exactly, so that is what gets rendered into time here.
    pub const RAW_TICKS_PER_SECOND: u32 = 1020;

    /// How long the next step lasts, in ticks — and a step is not always a
    /// frame.
    ///
    /// While a curtain runs the original's frame loop is not running at all:
    /// the fade handler spins inside itself, waiting `delay` ticks per band and
    /// calling the presenter after each one (0x74db6–0x74dcf). So the clock
    /// that matters during a fade is the band's, not the frame's, and it is
    /// much faster — one tick a band on the 480-row title screen against eight
    /// ticks a frame.
    ///
    /// Running the curtain off the frame clock instead is what made the intro
    /// look wrong. The wall clock came out right, because a frame simply
    /// bought eight bands; but 31 bands then arrived as **four** pictures, each
    /// edge jumping 64 rows, and a wipe that shows four of its steps reads as a
    /// cut. The status bar never showed it: at ten ticks a band, one frame buys
    /// one band, so its six steps arrived as six pictures — which is why the
    /// menu's fades looked right while the intro's did not.
    pub fn step_ticks(&self) -> i32 {
        match (
            self.transitions.curtains.front(),
            self.transitions.wipes.front(),
        ) {
            (Some(c), _) => c.ticks_per_band.max(1),
            (None, Some(w)) => w.ticks_per_ring.max(1),
            (None, None) => self.frame_ticks,
        }
    }

    /// What a wait of `ticks` unit-3 ticks costs in master ticks: the
    /// original's spin quantized.
    ///
    /// The spin at `0x74db6` (and `ANIMPLAY`'s limiter at `0x690e9`) polls
    /// `GIVETIMER`, which truncates `raw × 10 / 51` (`0x23761`), and leaves
    /// once the result reaches `ticks` — which first happens at
    /// `ceil(ticks × 51/10)` master ticks. The remainder is the visible part:
    /// a 1-tick band costs 6 master ticks (5.88 ms), not 5.1; a 10-tick band
    /// costs 51, which is 50 ms exactly. The stretch lands only on the short
    /// delays — the intro's full-screen fades — and skipping it is what made
    /// them run 15 % fast.
    fn quantized_raw(ticks: i32) -> u64 {
        // A count below zero is no wait at all.
        (u64::try_from(ticks).unwrap_or(0) * 51).div_ceil(10)
    }

    /// How long the next frame should last, in wall-clock time.
    ///
    /// A frame is the game's own unit of time — `!LTWAIT` is a single
    /// decrement of `_LOCTASKWAI` per call of the task manager, so a task
    /// asking to wait fifty waits fifty of these. And a game asks for its
    /// pace early — the 32-bit `START` runs `25 DELAY` before entering its
    /// loop, the 16-bit `RUN` runs `15 DELAY` — and startup parks past that
    /// ask before a window ever calls, so the answer is real from the first
    /// frame on for both machines.
    ///
    /// [`Engine::step_ticks`] rendered through `quantized_raw` at the master
    /// rate. `None` when the count is not positive — `DELAY -1` is
    /// the original's "do not wait at all" — and a frontend then runs at
    /// whatever pace it likes. The quantization also touches the frame: eight
    /// ticks cost 41 master ticks, 40.2 ms, so the game runs at 24.88 fps as
    /// the original does, not a rounder 25.
    pub fn frame_duration(&self) -> Option<std::time::Duration> {
        let ticks = self.step_ticks();
        (ticks > 0).then(|| {
            std::time::Duration::from_nanos(
                1_000_000_000u64 * Self::quantized_raw(ticks)
                    / u64::from(Self::RAW_TICKS_PER_SECOND),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// The spin's quantization, pinned to the values the original's
    /// arithmetic produces: `ceil(ticks × 51/10)` master ticks (`0x236FE`,
    /// `0x23761`, spin at `0x74db6`). The shipped screens give band delays of
    /// 1, 2 and 10 (480/400/80 rows), and the frame is 8 ticks.
    #[test]
    fn the_spin_quantizes_as_the_original_divides() {
        assert_eq!(Engine::quantized_raw(1), 6, "480-row band");
        assert_eq!(Engine::quantized_raw(2), 11, "400-row band");
        assert_eq!(Engine::quantized_raw(10), 51, "80-row band");
        assert_eq!(Engine::quantized_raw(8), 41, "the 25-fps frame");
    }

    /// The whole-fade durations, through the public conversion: passes ×
    /// per-band duration.
    #[test]
    fn the_fades_last_what_the_arithmetic_says() {
        let band = |ticks| {
            Duration::from_nanos(
                1_000_000_000 * Engine::quantized_raw(ticks)
                    / u64::from(Engine::RAW_TICKS_PER_SECOND),
            )
        };
        // 31 passes at delay 1, 26 at delay 2, 6 at delay 10.
        assert_eq!((band(1) * 31).as_millis(), 182);
        assert_eq!((band(2) * 26).as_millis(), 280);
        assert_eq!((band(10) * 6).as_millis(), 300);
    }
}
