//! How long a step lasts.
//!
//! Two clocks, not one. The game asks for a frame rate with `DELAY`, but while
//! a curtain runs the original's frame loop is not running at all — the fade
//! handler spins inside itself, waiting out one band at a time. So a "step" is
//! a frame most of the time and a band during a transition, and the two are far
//! apart: one tick a band on the title screen against eight ticks a frame.
//!
//! The ticks are 200 Hz ticks on both machines, and turning them into
//! wall-clock time goes through each build's own counter and its own
//! arithmetic ([`MasterClock`]), because the arithmetic is visible: on the
//! 32-bit engine a 1-tick wait costs 6 master ticks, not 5.1, where the 16-bit
//! engine's counter has no remainder to carry. See [`Engine::frame_duration`].

use crate::Engine;
use std::time::Duration;

/// The master counter a build keeps time on: what a wait of ticks costs it,
/// and what one of its counts is worth on the wall.
///
/// Both engines wait by spinning on a reading of a counter their interrupt
/// advances, and leave the spin once the reading has reached the tick count
/// asked for. What differs is the counter and the reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MasterClock {
    /// The 32-bit engine's. A counter at the 1020 Hz the arithmetic implies
    /// ([`Engine::RAW_TICKS_PER_SECOND`]), read as unit 3 by a truncating
    /// ×10/51 (`GIVETIMER`, `0x23761`; the spin at `0x74db6`), so a wait of
    /// `n` ticks first ends at `ceil(51n/10)` counts — the remainder that
    /// makes a 1-tick band 6 counts and not 5.1.
    Unit3,
    /// The 16-bit engine's. The sound driver's counter at `ENVIRO.EXE`
    /// `DS:0x7bb8`, advanced by its timer interrupt (`19c3:0a40`) once per
    /// `0x4a9` = 1193 PIT cycles and read as fifths (`110a:042e`, the spin at
    /// `016a:06dd`), so a wait of `n` ticks ends at exactly `5n` counts, and
    /// a count is 1193 cycles of the PIT's 1 193 182 Hz — a millisecond to
    /// within a sixth of a part per thousand, and rendered as such rather
    /// than rounded. The speed class the reading can divide by further
    /// (`DS:0x10ce`) is taken as the 0 the file holds.
    Milliseconds,
}

impl MasterClock {
    /// The PIT's input clock, in Hz.
    const PIT_HZ: u64 = 1_193_182;

    /// What a wait of `ticks` costs the counter: the first count at which
    /// the reading has reached `ticks`. A count below zero is no wait at all.
    pub(crate) fn raw_for_wait(self, ticks: i32) -> u64 {
        let ticks = u64::try_from(ticks).unwrap_or(0);
        match self {
            MasterClock::Unit3 => (ticks * 51).div_ceil(10),
            MasterClock::Milliseconds => ticks * 5,
        }
    }

    /// The counter's rate, as counts per second over a denominator.
    fn rate(self) -> (u64, u64) {
        match self {
            MasterClock::Unit3 => (u64::from(Engine::RAW_TICKS_PER_SECOND), 1),
            MasterClock::Milliseconds => (Self::PIT_HZ, 1193),
        }
    }

    /// How long `raw` counts last on the wall.
    pub(crate) fn duration(self, raw: u64) -> Duration {
        let (num, den) = self.rate();
        let nanos = u128::from(raw) * 1_000_000_000 * u128::from(den) / u128::from(num);
        Duration::from_nanos(u64::try_from(nanos).unwrap_or(u64::MAX))
    }
}

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
    /// averages ~978 Hz with and without the HMI drivers loaded. Where the
    /// four per cent go is not measured: the emulator's interrupt batching is
    /// the likely place, the provider's own rate another.
    pub const TICKS_PER_SECOND: u32 = 200;

    /// The master counter's design rate, from which unit 3 is derived.
    ///
    /// A model the engine's arithmetic implies, not a measured provider.
    /// ×10/51 is `/5.1`, and `/5.1` yields the 200 Hz that `DELAY`'s `200/n`
    /// assumes at exactly 1020 — not because 1020 is the one rate that
    /// divides to a whole number (510 gives 100, 1530 gives 300) but because
    /// 200 is the rate the rest of the engine is written against. A real PIT
    /// can merely approximate it (divisor 1170 → 1019.81 Hz), and which timer
    /// feeds the counter sits behind a loader-patched thunk that is not read.
    /// The arithmetic the engine does is 1020/200 exactly, so that is what
    /// gets rendered into time here; a provider rate read off the binary
    /// would replace this one constant.
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

    /// What a wait of `ticks` costs in master ticks: the build's spin
    /// quantized ([`MasterClock::raw_for_wait`]).
    ///
    /// On the 32-bit engine the spin at `0x74db6` (and `ANIMPLAY`'s limiter
    /// at `0x690e9`) polls `GIVETIMER`, which truncates `raw × 10 / 51`
    /// (`0x23761`), and leaves once the result reaches `ticks` — which first
    /// happens at `ceil(ticks × 51/10)` master ticks. The remainder is the
    /// visible part: a 1-tick band costs 6 master ticks (5.88 ms), not 5.1;
    /// a 10-tick band costs 51, which is 50 ms exactly. The stretch lands
    /// only on the short delays — the intro's full-screen fades — and
    /// skipping it is what made them run 15 % fast. The 16-bit spin reads
    /// a millisecond counter as fifths, and `ticks` costs exactly five
    /// times itself.
    fn quantized_raw(&self, ticks: i32) -> u64 {
        self.profile.clock.raw_for_wait(ticks)
    }

    /// What the step about to run costs the master counter, in its own
    /// ticks: the spin's quantization of the frame or band, or, for a frame
    /// of a running slide, one refresh of the display.
    ///
    /// The 32-bit slide's frames are paced by the vertical retrace
    /// (`0x6cdf4` polls port `0x3da`), not by the tick clock, and the
    /// refresh of VESA mode `0x101` is 60 Hz — which divides the 1020 Hz
    /// master exactly, so nothing is rounded: seventeen master ticks a
    /// frame. Only that machine slides; the 16-bit scroll runs on frames.
    pub(crate) fn step_raw_ticks(&self) -> u64 {
        if self.transitions.slide.is_some() {
            return u64::from(Self::RAW_TICKS_PER_SECOND / crate::Slide::REFRESH_HZ);
        }
        self.quantized_raw(self.step_ticks())
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
    /// `Engine::step_raw_ticks` rendered at the build's master rate. `None`
    /// when the count is not positive — `DELAY -1` is the original's "do not
    /// wait at all" — and a frontend then runs at whatever pace it likes.
    /// The 32-bit quantization also touches the frame: eight ticks cost 41
    /// master ticks, 40.2 ms, so that game runs at 24.88 fps as the original
    /// does, not a rounder 25; the 16-bit frame of 13 ticks is 65 counts of a
    /// millisecond counter, 64.99 ms at the PIT's rate.
    pub fn frame_duration(&self) -> Option<Duration> {
        let raw = self.step_raw_ticks();
        (raw > 0).then(|| self.profile.clock.duration(raw))
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

    /// The spin's quantization, pinned to the values the original's
    /// arithmetic produces: `ceil(ticks × 51/10)` master ticks (`0x236FE`,
    /// `0x23761`, spin at `0x74db6`). The shipped screens give band delays of
    /// 1, 2 and 10 (480/400/80 rows), and the frame is 8 ticks.
    #[test]
    fn the_spin_quantizes_as_the_original_divides() {
        let unit3 = MasterClock::Unit3;
        assert_eq!(unit3.raw_for_wait(1), 6, "480-row band");
        assert_eq!(unit3.raw_for_wait(2), 11, "400-row band");
        assert_eq!(unit3.raw_for_wait(10), 51, "80-row band");
        assert_eq!(unit3.raw_for_wait(8), 41, "the 25-fps frame");
        assert_eq!(unit3.raw_for_wait(-1), 0, "no wait at all");
    }

    /// The 16-bit counter: a wait of `n` ticks is `5n` milliseconds of the
    /// driver's interrupt, which runs at 1 193 182 / 1193 Hz — the `15
    /// DELAY` frame of 13 ticks is 65 counts, 64.99 ms, and `ENDTUNE`'s
    /// hold of 100 ticks is 499.92 ms, not the 500 the 32-bit arithmetic
    /// would give it.
    #[test]
    fn the_millisecond_counter_carries_no_remainder() {
        let ms = MasterClock::Milliseconds;
        assert_eq!(ms.raw_for_wait(13), 65);
        assert_eq!(ms.raw_for_wait(1), 5);
        assert_eq!(ms.raw_for_wait(-1), 0);
        assert_eq!(ms.duration(65).as_micros(), 64_990);
        assert_eq!(ms.duration(500).as_micros(), 499_923);
        assert_eq!(MasterClock::Unit3.duration(1020).as_millis(), 1000);
    }

    /// The whole-fade durations, through the public conversion: passes ×
    /// per-band duration.
    #[test]
    fn the_fades_last_what_the_arithmetic_says() {
        let band = |ticks| MasterClock::Unit3.duration(MasterClock::Unit3.raw_for_wait(ticks));
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

    /// The master counter moves by what a step is worth on its machine, and
    /// a slide's frame by exactly one refresh.
    #[test]
    fn the_clock_advances_by_the_step() {
        let mut e = Engine::new(crate::Profile::motion16());
        e.frame_ticks = 13;
        e.advance_clock();
        assert_eq!(e.master_ticks(), 65, "a 15 DELAY frame is 65 milliseconds");
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
