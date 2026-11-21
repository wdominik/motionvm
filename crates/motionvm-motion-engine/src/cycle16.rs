//! `SETCYCLE`: the 16-bit engine's rotating palette.
//!
//! The word (`LL.EXE` `0104:5319`) takes `( delay last first -- )` and, when
//! `first < last`, arms a rotation over the palette entries `first` through
//! `last` inclusive: it stores the three, sets the rotation amount to one and
//! the tick counter to zero, and floors a delay below one at one. With
//! `first >= last` it disarms the rotation and touches nothing else — the
//! DAC keeps whatever the last turn left in it until the next `SETPAL`.
//!
//! The turn itself is a frame effect. `ANIMPLAY` calls the tick (`0104:536d`)
//! once at the end of every pass of its loop, after the wait and the blit
//! (`0104:5756`), so `delay` counts frames. Each tick: the counter goes up,
//! and once it reaches `delay` the working palette (`ds:63ee`) is copied
//! whole from the master (`ds:1b66`, the 768 bytes `SETPAL` last installed),
//! every entry `i` of the range is written to entry `i + amount`, wrapped
//! back into the range past `last`, the working palette goes to the DAC,
//! the amount goes up by one — back to one once it would exceed
//! `last - first` — and the counter starts over. The amount is never zero,
//! so a cycling range never shows its master order; at the wrap it jumps
//! from `last - first` straight to one.
//!
//! Only Victor Loomes calls the word: `1 127 32 SETCYCLE` as the time
//! machine (location 13) is set up, `1 0 0 SETCYCLE` to stop it.

use crate::Engine;
use motionvm_render::Palette;

/// One armed rotation, with the tick state the handler keeps in its globals.
#[derive(Debug, Clone)]
pub(crate) struct PaletteCycle {
    /// The first entry of the range (`ds:7c10`).
    first: i32,
    /// The last entry, inclusive (`ds:6ff0`).
    last: i32,
    /// Frames between turns (`ds:66ee`), at least one.
    delay: i32,
    /// Frames counted since the last turn (`ds:66f2`).
    counter: i32,
    /// How far the next turn moves every entry (`ds:48f4`).
    amount: i32,
    /// The master palette the turns are taken from — whatever the display
    /// held that the cycle did not put there.
    master: Option<Palette>,
    /// What the last turn installed, so that a palette the script changed
    /// underneath — a `SETPAL` while the range still cycles — is told apart
    /// from the cycle's own work and becomes the new master, the way the
    /// original's `SETPAL` writes `ds:1b66` while the tick keeps reading it.
    installed: Option<Palette>,
}

impl PaletteCycle {
    /// What `SETCYCLE` arms; the caller has already checked `first < last`.
    pub(crate) fn new(first: i32, last: i32, delay: i32) -> Self {
        PaletteCycle {
            first,
            last,
            delay: delay.max(1),
            counter: 0,
            amount: 1,
            master: None,
            installed: None,
        }
    }
}

impl Engine {
    /// The frame's turn of the rotating palette, if one is armed
    /// (`0104:536d`), to be called where `ANIMPLAY` calls it: once a frame,
    /// after the picture is presented.
    pub(crate) fn tick_palette_cycle(&mut self) {
        let Some(c) = self.palette_cycle.as_mut() else {
            return;
        };
        c.counter += 1;
        if c.counter < c.delay {
            return;
        }
        if c.installed.as_ref() != Some(&self.display.palette) {
            c.master = Some(self.display.palette.clone());
        }
        let master = c
            .master
            .clone()
            .unwrap_or_else(|| self.display.palette.clone());
        let mut working = master.clone();
        let (first, last) = (c.first, c.last);
        for from in first..=last {
            let mut to = from + c.amount;
            if to > last {
                to = to - last + first - 1;
            }
            // The original indexes its two 768-byte tables with whatever the
            // script passed; entries outside the palette are left alone here.
            let (Ok(from), Ok(to)) = (usize::try_from(from), usize::try_from(to)) else {
                continue;
            };
            if from >= 256 || to >= 256 {
                continue;
            }
            working.raw[to * 3..to * 3 + 3].copy_from_slice(&master.raw[from * 3..from * 3 + 3]);
        }
        self.display.palette = working.clone();
        c.installed = Some(working);
        c.amount += 1;
        if last - first < c.amount {
            c.amount = 1;
        }
        c.counter = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine_with_ramp() -> Engine {
        let mut e = Engine::new(crate::Profile::motion16());
        let mut raw = [0u8; Palette::BYTES];
        for i in 0..256 {
            raw[i * 3] = u8::try_from(i).unwrap();
        }
        e.display.palette = Palette { raw };
        e
    }

    /// The handler's arithmetic on a small range: every entry moves up by
    /// the amount, wrapping into the range, and the amount grows by one a
    /// turn until it would exceed the range's span, when it starts over at
    /// one.
    #[test]
    fn the_range_turns_by_a_growing_amount_and_wraps() {
        let mut e = engine_with_ramp();
        e.palette_cycle = Some(PaletteCycle::new(10, 13, 1));
        let red = |e: &Engine, i: usize| e.display.palette.raw[i * 3];
        // Turn 1, amount 1: 10→11, 11→12, 12→13, 13→10.
        e.tick_palette_cycle();
        assert_eq!(
            (red(&e, 10), red(&e, 11), red(&e, 12), red(&e, 13)),
            (13, 10, 11, 12)
        );
        // Turn 2, amount 2: from the master, not the last turn.
        e.tick_palette_cycle();
        assert_eq!(
            (red(&e, 10), red(&e, 11), red(&e, 12), red(&e, 13)),
            (12, 13, 10, 11)
        );
        // Turn 3, amount 3.
        e.tick_palette_cycle();
        assert_eq!(
            (red(&e, 10), red(&e, 11), red(&e, 12), red(&e, 13)),
            (11, 12, 13, 10)
        );
        // Amount 4 exceeds the span of 3, so turn 4 is amount 1 again.
        e.tick_palette_cycle();
        assert_eq!(
            (red(&e, 10), red(&e, 11), red(&e, 12), red(&e, 13)),
            (13, 10, 11, 12)
        );
        // Entries outside the range never move.
        assert_eq!((red(&e, 9), red(&e, 14)), (9, 14));
    }

    /// `delay` counts frames between turns, and the first turn comes once
    /// `delay` frames have been counted.
    #[test]
    fn the_delay_counts_frames_between_turns() {
        let mut e = engine_with_ramp();
        e.palette_cycle = Some(PaletteCycle::new(10, 13, 3));
        let before = e.display.palette.clone();
        e.tick_palette_cycle();
        e.tick_palette_cycle();
        assert_eq!(e.display.palette, before, "two frames are not enough");
        e.tick_palette_cycle();
        assert_ne!(e.display.palette, before, "the third frame turns");
    }

    /// A palette the script installs while the range cycles is the new
    /// master, as `SETPAL`'s write to `ds:1b66` is for the original's tick.
    #[test]
    fn a_palette_set_underneath_becomes_the_master() {
        let mut e = engine_with_ramp();
        e.palette_cycle = Some(PaletteCycle::new(10, 13, 1));
        e.tick_palette_cycle();
        let mut raw = [0u8; Palette::BYTES];
        for i in 0..256 {
            raw[i * 3] = u8::try_from(i).unwrap().wrapping_add(100);
        }
        e.display.palette = Palette { raw };
        e.tick_palette_cycle();
        // Amount 2 over the new master: 10→12, 11→13, 12→10, 13→11.
        let red = |e: &Engine, i: usize| e.display.palette.raw[i * 3];
        assert_eq!(
            (red(&e, 10), red(&e, 11), red(&e, 12), red(&e, 13)),
            (112, 113, 110, 111)
        );
    }
}
