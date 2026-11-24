//! One frame of the parsec1994 generation.
//!
//! The original's frame pass (`0EB9h:01DEh`) takes the tick accumulator
//! `[1C76h]` whole and hands the count round (`0EB9h:163Eh`): to the tasks
//! (`0CFDh:11ECh`, `0x101F4`), the walkers (`0A9Fh:0892h`, `0x101FB`) and
//! the animator (`0883h:0305h`, `0x10202`), in that order, and then the
//! machine's timed wait counts down (`0x10210`); then the room redraws
//! its dirty tiles and the tasks paint their boxes (`0CFDh:16D5h`). A
//! frame of the visit loop here is one tick — the title's
//! `frame_duration` says how long one is — and its frame pass takes the
//! one tick the interrupt before it counted. The leaving's frame pass,
//! right behind the loop's last, takes none; the one after a blocking
//! effect takes every tick the effect waited out.
//!
//! The timer itself is an interrupt, and it fires between frames as far as
//! the program is concerned: a handler's own check of a counter (the
//! Zimmer's `[1C82h] > 64h`, `0x9CD`) and the frame pass behind it read the
//! same interrupt count. So the tick is counted first, before anything in
//! the frame looks at the clock.

use super::Adventure;
use crate::num;

impl Adventure {
    /// One frame as the visit loop runs it: the interrupt, the
    /// dispatcher, then the frame pass — for a test that steps the
    /// machine without a location's handler, which the game's own tick
    /// puts between the last two.
    #[cfg(test)]
    pub(crate) fn tick(&mut self) {
        self.interrupt();
        if self.effects.busy() {
            self.effects_tick();
            return;
        }
        self.dispatch();
        self.frame_pass();
    }

    /// `0x101D5`: the hand-round's first act, the status line's gate.
    ///
    /// Nothing is pending while the age `[28C3h]` is 0 (`0x101D5`), and the
    /// line is held while `[289Bh]` has bit 0 or 2 set — a walk for the
    /// machine, or a line a script set (`0x101DC`). Otherwise the age goes
    /// up by one frame (`0x101E7`), and once it was above 3 (`0x101EB`) the
    /// composer at `0x10051` runs (`0x101F0`), which ends by clearing the
    /// age (`0x101B3`).
    ///
    /// This is the composer's only caller, and the line it sets holds
    /// until the next composition: the painter draws the buffer every
    /// frame and composes nothing.
    pub(crate) fn status_gate(&mut self) {
        let age = self.status_refresh_age;
        if age == 0 || self.bits & 5 != 0 {
            return;
        }
        self.status_refresh_age = age.wrapping_add(1);
        if num::signed(age) > 3 {
            self.status_line = self.status_bytes();
            self.status_refresh_age = 0;
        }
    }

    /// `[1C76h]` as the frame leaves it: the exit the door's gate took,
    /// and where it leads. The loop ends the visit on it (`0x9E8`).
    pub(crate) fn take_leave(&mut self) -> Option<(u8, i32)> {
        self.leave.take()
    }

    /// What INT 8 did since the last frame: one tick into the record at
    /// `DS:1C74h`, its cycles toward the BIOS tick, and the pointer's timer
    /// slot counted down by the hundredths that tick crossed (`1950h:1402h(0, 179Eh:0A90h, 2)`,
    /// registered at `0x05EEE`).
    pub(crate) fn interrupt(&mut self) {
        let hundredths = self.clock.interrupt();
        self.bios.interrupt();
        let mouse = self.mouse;
        let mut keys_moved = None;
        if let Some(c) = &mut self.cursor {
            c.read_key_params(&self.ds);
            c.timer(num::low_word(hundredths), mouse);
            if c.take_keys_moved() {
                keys_moved = Some((i32::from(c.pos.0), i32::from(c.pos.1)));
            }
        }
        // Where the keys took the pointer is where the program has it: the
        // hover and the hotspots follow, as they follow the host's mouse.
        if let Some(at) = keys_moved {
            self.mouse = at;
            self.menu_hover();
        }
    }

    /// `0EB9h:01DEh`: the ticks not yet taken handed round, then the
    /// boxes.
    ///
    /// The hand-round (`0EB9h:163Eh`) ends with the machine's timed wait:
    /// while `[289Bh]` bit 5 is set, `[28BFh]` counts the ticks down and
    /// stops at zero (`0x10210`, `sub [28BFh], si`; `0x10214`, `jg`).
    pub(crate) fn frame_pass(&mut self) {
        let delta = i32::from(self.clock.take_ticks());
        self.status_gate();
        self.tick_tasks(delta);
        // Every chain's records, the player among them, step before any
        // of them animates.
        self.mover_pass(delta);
        self.animator_tick(delta);
        if self.bits & 0x20 != 0 {
            let left = i32::from(num::signed(self.wait_ticks)) - delta;
            self.wait_ticks = if left > 0 { num::low16(left) } else { 0 };
        }
        self.panel_frame();
        self.frame_tasks();
        // `10B8:0009` (0x0ED8C): the picture presented — what the screen
        // holds when an effect starts drawing over it.
        self.presented = self.render_live();
    }
}
