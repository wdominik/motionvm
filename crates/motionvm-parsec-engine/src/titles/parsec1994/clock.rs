//! The record the timer counts into, and what a frame takes out of it.
//!
//! The library's init hooks INT 8 (`1950h:1883h`) and registers a record
//! at `DS:1C74h` with the scheduler (`0x5B45`, `push 1C74h`;
//! `1950h:170Fh`). Every interrupt, the handler at image `0x19F40` takes
//! the PIT period that just elapsed — `cs:[96h]`, in cycles of the
//! 1 193 182 Hz chip — and counts it into that record in three groups:
//! every 4A9h cycles `+0` counts up, a thousand a second (`0x19F5D`,
//! `0x19F64`); every 26D6h cycles `+2`, `+4` and `+6`, 120 a second
//! (`0x19F6F`, `0x19F76`); every 2E9Ah cycles `+8` to `+0Eh`, 100 a second
//! (`0x19F88`, `0x19F8F`); and every tenth of those, counted down in
//! `cs:[0ECh]` (`0x19F9B`), `+10h` to `+16h`, ten a second (`0x19FC5`).
//!
//! The interrupt comes every period the library runs the PIT on: its base
//! period, `952h` cycles — 500 a second, set at `0x5B29`–`0x5B2C` through
//! `1950h:166Dh` (`0x1AB6D`), which programs channel 0 in mode 2
//! (`0x1ABE5`) — or a client's shorter one. On the base period the
//! `26D6h` group counts on every fourth or fifth interrupt, the first at
//! or after each boundary; the rates hold in PIT cycles, whatever the
//! interrupt's period.
//!
//! The frame pass takes `+2` — `[1C76h]` — whole and clears it (`0x0ED6E`)
//! and hands the count round as the frame's delta. The other counters are
//! what the scripts read: `[1C82h]` is `+0Eh`, hundredths, which the
//! Zimmer waits on (`0x9CD`) and the transition effects count.
//!
//! Here an interrupt is one period of the tick group, `26D6h` cycles: each
//! one counts exactly one tick, and the other groups count what falls into
//! those cycles. A frame of the visit loop, one interrupt and one frame
//! pass, takes one tick, and a frame pass with no interrupt since the last
//! takes none. The rates are the original's; where a count lands within
//! its period is not, as the original counts on its own interrupts and
//! this clock on its ticks.
//!
//! The countdowns `cs:[0E6h]`, `[0E8h]` and `[0EAh]` ship as zero, and
//! nothing but the handler and `1950h:14BBh` (`0x1A9BB`) writes them;
//! where they stand at a location's entry depends on every interrupt
//! since the registration. Here they start a full period away when the
//! game is opened. The subtraction is the original's, down to its unsigned
//! borrow when a countdown lands on zero.

use crate::num;

/// Cycles of the PIT per second.
const PIT_HZ: u64 = 1_193_182;
/// The three periods, in PIT cycles (`0x19F66`, `0x19F7F`, `0x19FA2`).
const MS_PERIOD: u32 = 0x4A9;
pub(crate) const TICK_PERIOD: u32 = 0x26D6;
const HUNDREDTH_PERIOD: u32 = 0x2E9A;

/// The record at `DS:1C74h`, advanced by whole 120-Hz ticks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Clock {
    /// `+0`: milliseconds.
    pub ms: u16,
    /// `+2`, `[1C76h]`: the ticks no frame pass has taken yet.
    pub ticks: u16,
    /// `+8`, `+0Ah`, `+0Ch`, `+0Eh`: hundredths. Scripts clear and read
    /// them one at a time, so they are four words and not one.
    pub hundredths: [u16; 4],
    /// `+10h` … `+16h`: tenths.
    pub tenths: [u16; 4],
    /// Cycles left until the next millisecond and the next hundredth.
    ms_left: u32,
    hundredth_left: u32,
    /// `cs:[0ECh]`: how many hundredths until the next tenth, set to 10
    /// by the registration (`0x1AC3A`) and refilled by ten (`0x19FD1`).
    tenth_left: i8,
}

impl Clock {
    /// A fresh record: every counter at zero, every countdown a period out.
    #[must_use]
    pub(crate) fn new() -> Self {
        Clock {
            ms: 0,
            ticks: 0,
            hundredths: [0; 4],
            tenths: [0; 4],
            ms_left: MS_PERIOD,
            hundredth_left: HUNDREDTH_PERIOD,
            tenth_left: 10,
        }
    }

    /// How many nanoseconds a tick lasts, for the frame clock: `26D6h`
    /// cycles of the PIT.
    pub(crate) fn tick_nanos() -> u64 {
        u64::from(TICK_PERIOD) * 1_000_000_000 / PIT_HZ
    }

    /// Advances by `ticks` frames of `26D6h` cycles each, one period at a
    /// time the way the handler runs.
    #[cfg(test)]
    pub(crate) fn advance(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.interrupt();
        }
    }

    /// One interrupt of `26D6h` cycles. Answers how many hundredths it
    /// counted — the increments the callback scheduler at `0x1A1EE` counts
    /// its slots down by. That scheduler keeps its own countdown,
    /// `cs:[110h]`, with the same period over the same `cs:[96h]`
    /// (`0x1A1F0`–`0x1A202`); it is taken to be in step with this one's
    /// `cs:[0EAh]`: both ship as zero, and both start here a full period
    /// away.
    pub(crate) fn interrupt(&mut self) -> u32 {
        let fired = borrow(&mut self.ms_left, MS_PERIOD, TICK_PERIOD);
        self.ms = self.ms.wrapping_add(num::low_word(fired));
        self.ticks = self.ticks.wrapping_add(1);
        let fired = borrow(&mut self.hundredth_left, HUNDREDTH_PERIOD, TICK_PERIOD);
        for _ in 0..fired {
            for h in &mut self.hundredths {
                *h = h.wrapping_add(1);
            }
            self.tenth_left -= 1;
            if self.tenth_left <= 0 {
                for t in &mut self.tenths {
                    *t = t.wrapping_add(1);
                }
                self.tenth_left += 10;
            }
        }
        fired
    }

    /// `0x0ED6E`: `[1C76h]` taken whole and cleared.
    pub(crate) fn take_ticks(&mut self) -> u16 {
        std::mem::take(&mut self.ticks)
    }

    /// The hundredths a script reads at `[1C82h]`, the record's `+0Eh`.
    pub(crate) fn hundredths_1c82(&self) -> u16 {
        self.hundredths[3]
    }

    /// `[1C82h]` to be written in place, as an effect's prologue does.
    pub(crate) fn hundredths_1c82_mut(&mut self) -> &mut u16 {
        &mut self.hundredths[3]
    }

    /// Writes `[1C82h]`, as the location logic does.
    pub(crate) fn set_1c82(&mut self, v: u16) {
        self.hundredths[3] = v;
    }
}

/// Ticks in a day: the BIOS's own INT 8 clears the count at `1800B0h`
/// and flags midnight, so `0:046Ch` runs from 0 to `1800AFh`.
pub(crate) const BIOS_TICKS_PER_DAY: u32 = 0x1800B0;

/// The BIOS tick, `0:046Ch`, as the library's interrupt keeps it going.
///
/// The library reprograms the PIT to its own period, so the BIOS's INT 8
/// does not run every 65 536 cycles by itself. The handler makes up for
/// it: every interrupt adds the period that just elapsed to a 16-bit word
/// of its own, and when that add carries it leaves through the BIOS's
/// handler instead of returning (`0x19FB6`–`0x19FDA`):
/// the period is added to `cs:[0E4h]` at `0x19FB6`, and on a carry (`0x19FBD`)
/// the handler jumps far through `cs:[8Ah]`, the old INT 8 (`0x19FDA`).
///
/// `cs:[8Ah]` is the INT 8 vector the library found when it hooked the
/// timer: its init asks DOS for it (`int 21h` with `AX = 3508h`) and keeps
/// `BX` and `ES` there before installing its own (`0x1ADCE`–`0x1ADE7`). On
/// a machine nothing else has hooked, that is the BIOS's handler, which
/// counts `0:046Ch`. So the BIOS counts once per 65 536 PIT cycles, as it
/// would unhooked, and here a tick of [`TICK_PERIOD`] cycles feeds the
/// same word.
///
/// Where the count starts is the host's: on the original machine it is
/// the time since midnight, which this engine does not read. A seed
/// stands in for it, and without one it is 0. The word at `cs:[0E4h]`
/// starts at 0 with it, as the countdowns of [`Clock`] start a full
/// period away; on the original, where each of them stands depends on
/// what ran before.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BiosTick {
    /// `0:046Ch` as a 32-bit count, below [`BIOS_TICKS_PER_DAY`].
    count: u32,
    /// `cs:[0E4h]`: the cycles since the BIOS last counted, modulo 65 536.
    cycles: u16,
}

impl BiosTick {
    /// A count that starts at `seed`, taken modulo a day of ticks, with
    /// the carry word at zero.
    #[must_use]
    pub(crate) fn seeded(seed: u64) -> Self {
        let count = seed % u64::from(BIOS_TICKS_PER_DAY);
        BiosTick {
            count: u32::try_from(count).unwrap_or(0),
            cycles: 0,
        }
    }

    /// The count a program reading `0:046Ch` now gets.
    pub(crate) fn count(&self) -> u32 {
        self.count
    }

    /// One interrupt of [`TICK_PERIOD`] cycles: the BIOS counts when the
    /// carry word carries, and starts the day over at midnight.
    pub(crate) fn interrupt(&mut self) {
        let (cycles, carried) = self.cycles.overflowing_add(num::low_word(TICK_PERIOD));
        self.cycles = cycles;
        if carried {
            self.count += 1;
            if self.count >= BIOS_TICKS_PER_DAY {
                self.count = 0;
            }
        }
    }
}

/// One group's borrow loop: take `cycles` off the countdown, and for every
/// period it went under, count once and add the period back
/// (`0x19F6F`–`0x19F86`). `JAE` skips the increment when subtraction lands
/// exactly on zero, and a refill that lands on zero carries and ends the
/// loop. Returns how many times it counted.
fn borrow(left: &mut u32, period: u32, cycles: u32) -> u32 {
    if *left >= cycles {
        *left -= cycles;
        return 0;
    }
    let mut under = cycles - *left;
    let mut fired = 1;
    while under > period {
        under -= period;
        fired += 1;
    }
    *left = period - under;
    fired
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `[1C76h]` counts one per interrupt, and the frame pass takes the
    /// count whole: a pass right behind another takes none, a pass after
    /// three interrupts takes three.
    #[test]
    fn the_frame_pass_takes_the_ticks_the_interrupts_counted() {
        let mut c = Clock::new();
        c.interrupt();
        assert_eq!(c.take_ticks(), 1);
        assert_eq!(c.take_ticks(), 0);
        c.advance(3);
        assert_eq!(c.take_ticks(), 3);
    }

    /// Compare against literal 16-bit SUB/ADD carry behavior from the
    /// original handler, including multiple periods in one interrupt.
    #[test]
    fn countdown_matches_original_unsigned_borrow_and_refill() {
        for period in [MS_PERIOD, TICK_PERIOD, HUNDREDTH_PERIOD] {
            for initial in [0, 1, period - 1, period] {
                for cycles in [0, 1, initial, initial + 1, period * 2, 65_535] {
                    let (mut word, borrowed) =
                        num::low_word(initial).overflowing_sub(num::low_word(cycles));
                    let mut expected = 0;
                    if borrowed {
                        loop {
                            expected += 1;
                            let (next, carry) = word.overflowing_add(num::low_word(period));
                            word = next;
                            if carry {
                                break;
                            }
                        }
                    }
                    let mut left = initial;
                    assert_eq!(
                        borrow(&mut left, period, cycles),
                        expected,
                        "period={period}, initial={initial}, cycles={cycles}"
                    );
                    assert_eq!(left, u32::from(word));
                }
            }
        }
    }

    /// 1,193 frame periods land exactly on millisecond boundary 9,942.
    /// The interrupt increments only after crossing that boundary.
    #[test]
    fn exact_millisecond_boundary_waits_for_the_next_interrupt() {
        let mut clock = Clock::new();
        clock.advance(1193);
        assert_eq!((clock.ms, clock.ms_left), (9941, 0));
        clock.advance(1);
        assert_eq!(clock.ms, 9950);
    }

    /// The carry rule is the instructions of Dunkle Schatten's own timer
    /// handler, at `0x19F6F` in `XP01.EXE`.
    #[test]
    fn the_shipped_timer_handler_has_the_borrow_loop() {
        let expected = [
            0x2E, 0x29, 6, 0xE8, 0, 0x73, 0x12, 0xFF, 0x47, 2, 0xFF, 0x47, 4, 0xFF, 0x47, 6, 0x2E,
            0x81, 6, 0xE8, 0, 0xD6, 0x26, 0x73, 0xEE,
        ];
        let Some(dir) = crate::titles::ds1::gamedata_or_skip() else {
            return;
        };
        let at = 0x19F6F;
        let image = crate::exe::Image::open_any(&dir, "XP01.EXE").unwrap();
        assert_eq!(&image.bytes()[at..at + expected.len()], expected);
    }

    /// A second of ticks counts a hundred hundredths and ten tenths.
    ///
    /// 120 ticks are 120 × `26D6h` = 1 193 040 cycles, which is 142 short
    /// of a second; the hundredth countdown fires the hundredth time at
    /// cycle 1 193 000 and the tenth with it, and the millisecond the
    /// thousandth time at the same cycle.
    #[test]
    fn a_second_of_ticks_is_a_hundred_hundredths_and_ten_tenths() {
        let mut c = Clock::new();
        c.advance(120);
        assert_eq!(c.hundredths, [100; 4]);
        assert_eq!(c.tenths, [10; 4]);
        assert_eq!(c.ms, 1000);
    }

    /// Without a seed the count is 0, and it stays there until 65 536
    /// cycles have gone by: six ticks of `26D6h` are 59 652, the seventh
    /// makes 69 594 and carries.
    #[test]
    fn the_bios_tick_without_a_seed_is_0_and_counts_every_65536_cycles() {
        let mut b = BiosTick::default();
        assert_eq!(b.count(), 0);
        for _ in 0..6 {
            b.interrupt();
        }
        assert_eq!(b.count(), 0);
        b.interrupt();
        assert_eq!(b.count(), 1);
        // 120 ticks are 1 193 040 cycles: 18 carries (1 179 648), not 19.
        let mut b = BiosTick::default();
        for _ in 0..120 {
            b.interrupt();
        }
        assert_eq!(b.count(), 18);
    }

    /// A seed is taken modulo a day, and the day starts over at midnight.
    #[test]
    fn the_bios_tick_starts_at_its_seed_and_wraps_at_midnight() {
        assert_eq!(BiosTick::seeded(0x000C_1234).count(), 0x000C_1234);
        assert_eq!(
            BiosTick::seeded(u64::from(BIOS_TICKS_PER_DAY) + 5).count(),
            5
        );
        let mut b = BiosTick::seeded(u64::from(BIOS_TICKS_PER_DAY) - 1);
        for _ in 0..7 {
            b.interrupt();
        }
        assert_eq!(b.count(), 0);
    }

    /// The Zimmer's wait is a second: `[1C82h] > 64h` first holds on the
    /// 122nd tick.
    #[test]
    fn the_zimmers_wait_of_a_hundred_hundredths_passes_on_tick_122() {
        let mut c = Clock::new();
        let mut ticks = 0;
        while c.hundredths_1c82() <= 0x64 {
            c.advance(1);
            ticks += 1;
        }
        assert_eq!(ticks, 122);
    }
}
