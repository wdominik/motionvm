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
            // Nought for a curtain that waits for nothing: the step costs
            // the master clock no time, and the frame it falls in is not
            // stretched by it.
            (Some(c), _) => c.ticks_per_band.max(0),
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

    /// What the step about to run costs the master counter, in its own
    /// ticks: the spin's quantization of the frame or band, or, for a frame
    /// of a running slide, one refresh of the display.
    ///
    /// The slide's frames are paced by the vertical retrace (`0x6cdf4`
    /// polls port `0x3da`), not by the tick clock, and the refresh of VESA
    /// mode `0x101` is 60 Hz — which divides the 1020 Hz master exactly, so
    /// nothing is rounded: seventeen master ticks a frame.
    pub(crate) fn step_raw_ticks(&self) -> u64 {
        if self.transitions.slide.is_some() {
            return u64::from(Self::RAW_TICKS_PER_SECOND / crate::Slide::REFRESH_HZ);
        }
        Self::quantized_raw(self.step_ticks())
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
    /// `Engine::step_raw_ticks` rendered at the master rate. `None` when
    /// the count is not positive — `DELAY -1` is the original's "do not wait
    /// at all" — and a frontend then runs at whatever pace it likes. The
    /// quantization also touches the frame: eight ticks cost 41 master
    /// ticks, 40.2 ms, so the game runs at 24.88 fps as the original does,
    /// not a rounder 25.
    pub fn frame_duration(&self) -> Option<std::time::Duration> {
        let raw = self.step_raw_ticks();
        (raw > 0).then(|| {
            std::time::Duration::from_nanos(
                1_000_000_000u64 * raw / u64::from(Self::RAW_TICKS_PER_SECOND),
            )
        })
    }

    /// The master counter as the frames so far have advanced it.
    ///
    /// The original's counter runs on a hardware timer whatever the game
    /// does; here it runs on the frames, each worth what the window waits
    /// for it (`Engine::step_raw_ticks`), which keeps a run reproducible
    /// and keeps the count in step with wall time while the game keeps its
    /// pace.
    pub fn master_ticks(&self) -> u64 {
        self.master_ticks
    }

    /// Moves the master counter on by the step that has just run — called
    /// once at the top of every step.
    pub(crate) fn advance_clock(&mut self) {
        self.master_ticks = self.master_ticks.saturating_add(self.step_raw_ticks());
    }
}

/// The engine's timer objects — the ones `OPENTIMER` hands the scripts, and
/// the ones the engine keeps for itself under a sample or a frame limiter.
///
/// Read from R78's four routines: `0x1e4c0` allocates eight bytes holding a
/// kind and the master count at that moment, and takes the kind as 1, 2 or
/// 3, anything else as 3; `0x1e5d0` sets a reading by moving the start back
/// by the reading scaled — ×20 for kind 1, ×10 for kind 2, ×5 for kind 3;
/// `0x1e630` reads the count since the start, divided by 20 for kind 1, by
/// 10 for kind 2, and ×10/51 for kind 3, every division truncating;
/// `0x1e540` frees the eight bytes. R109 keeps the same four (`0x236fe`,
/// `0x23761` for the read). Kind 3 is the unit the whole engine keeps time in
/// — 200 Hz to the digit at a 1020 Hz master, [`Engine::TICKS_PER_SECOND`] —
/// and the one the samples and `OPENTIMER`'s one caller use. Note the set
/// scales by 5 where the read divides by 5.1: a reading set and read straight
/// back comes back two percent short.
#[derive(Debug, Default)]
pub(crate) struct Timers {
    /// The open timers by handle.
    open: std::collections::BTreeMap<i32, Timer>,
    /// Handles are counted from 1; the original hands out the allocation's
    /// address, which the scripts only ever pass back.
    next: i32,
}

/// One timer object: its kind and the master count it measures from.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Timer {
    kind: TimerKind,
    /// May run ahead of the counter: `SETTIMER` moves the start back by the
    /// reading it sets, which for a reading below zero lands it after now.
    start: i64,
}

/// The three scalings of the master count a timer can answer in. The kernel
/// word and the sample layer both open kind 3; kinds 1 and 2 are read off
/// the routine and reached by nothing shipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TimerKind {
    /// Kind 1: the count over twenty — 51 Hz.
    Twentieth,
    /// Kind 2: the count over ten — 102 Hz.
    Tenth,
    /// Kind 3: the count times ten over fifty-one — 200 Hz, the engine's
    /// unit; also what any other kind number is taken as.
    Unit3,
}

impl TimerKind {
    /// What `OPENTIMER`'s argument means (`0x1e4de`–`0x1e51a`).
    fn of(kind: i32) -> Self {
        match kind {
            1 => Self::Twentieth,
            2 => Self::Tenth,
            _ => Self::Unit3,
        }
    }

    /// A reading turned into master ticks, as `SETTIMER` scales it
    /// (`0x1e5e6`–`0x1e613`).
    fn to_raw(self, reading: i64) -> i64 {
        match self {
            Self::Twentieth => reading.saturating_mul(20),
            Self::Tenth => reading.saturating_mul(10),
            Self::Unit3 => reading.saturating_mul(5),
        }
    }

    /// Master ticks turned into a reading, as `GIVETIMER` divides them
    /// (`0x1e651`–`0x1e6a3`), truncating toward zero as `idiv` does.
    fn to_reading(self, raw: i64) -> i64 {
        match self {
            Self::Twentieth => raw / 20,
            Self::Tenth => raw / 10,
            Self::Unit3 => raw.saturating_mul(10) / 51,
        }
    }
}

impl Engine {
    /// The master counter as a signed count, for the arithmetic below.
    fn master(&self) -> i64 {
        i64::try_from(self.master_ticks).unwrap_or(i64::MAX)
    }

    /// The routine behind `OPENTIMER` (`0x1e4c0`): a timer of `kind`,
    /// reading zero now. The word passes kind 3 as a constant; the sample
    /// layer opens its timers through the same routine.
    pub(crate) fn open_timer(&mut self, kind: i32) -> i32 {
        self.timers.next = self.timers.next.saturating_add(1);
        let handle = self.timers.next;
        let start = self.master();
        self.timers.open.insert(
            handle,
            Timer {
                kind: TimerKind::of(kind),
                start,
            },
        );
        handle
    }

    /// `SETTIMER ( handle reading -- )`: the timer reads `reading` from now.
    /// A handle nothing opened is left alone; the original would write
    /// through whatever the cell held.
    pub(crate) fn set_timer(&mut self, handle: i32, reading: i32) {
        let now = self.master();
        if let Some(t) = self.timers.open.get_mut(&handle) {
            t.start = now.saturating_sub(t.kind.to_raw(i64::from(reading)));
        }
    }

    /// `GIVETIMER ( handle -- reading )`, or `None` for a handle nothing
    /// opened.
    pub(crate) fn give_timer(&self, handle: i32) -> Option<i32> {
        let t = self.timers.open.get(&handle)?;
        let raw = self.master().saturating_sub(t.start);
        Some(i32::try_from(t.kind.to_reading(raw)).unwrap_or(i32::MAX))
    }

    /// `CLOSETIMER ( handle -- )`: the timer is gone.
    pub(crate) fn close_timer(&mut self, handle: i32) {
        self.timers.open.remove(&handle);
    }

    /// The engine's own use of a kind-3 timer: the count since `start`, in
    /// the 200 Hz unit, as `?STIME` reads a sample's.
    pub(crate) fn unit3_since(&self, start: u64) -> i32 {
        let raw = self
            .master()
            .saturating_sub(i64::try_from(start).unwrap_or(i64::MAX));
        i32::try_from(TimerKind::Unit3.to_reading(raw)).unwrap_or(i32::MAX)
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

    /// The three kinds, set and read: a set of 100 on kind 3 moves the
    /// start back 500 master ticks, which reads back as 98 — the two per
    /// cent the ×5 set loses against the ×10/51 read — and a second's worth
    /// of master ticks reads 200 on kind 3, 102 on kind 2, 51 on kind 1.
    #[test]
    fn the_timers_scale_as_the_four_routines_do() {
        let mut e = Engine::new(crate::Profile::motion32());
        let unit3 = e.open_timer(3);
        let tenth = e.open_timer(2);
        let twentieth = e.open_timer(1);
        let other = e.open_timer(99);
        assert_eq!(e.give_timer(unit3), Some(0));
        e.master_ticks += u64::from(Engine::RAW_TICKS_PER_SECOND);
        assert_eq!(e.give_timer(unit3), Some(200));
        assert_eq!(e.give_timer(other), Some(200), "any other kind is kind 3");
        assert_eq!(e.give_timer(tenth), Some(102));
        assert_eq!(e.give_timer(twentieth), Some(51));
        e.set_timer(unit3, 100);
        assert_eq!(e.give_timer(unit3), Some(98));
        e.set_timer(unit3, 0);
        assert_eq!(e.give_timer(unit3), Some(0));
        e.close_timer(unit3);
        assert_eq!(e.give_timer(unit3), None);
    }

    /// The master counter moves by what a step is worth, and a slide's frame
    /// by exactly one refresh.
    #[test]
    fn the_clock_advances_by_the_step() {
        let mut e = Engine::new(crate::Profile::motion32());
        e.advance_clock();
        assert_eq!(e.master_ticks(), 41, "a 25-fps frame is 41 master ticks");
        assert_eq!(
            Engine::RAW_TICKS_PER_SECOND % crate::Slide::REFRESH_HZ,
            0,
            "the refresh divides the master"
        );
    }
}
