//! The pointer library's keyboard: twelve keys that move the pointer and
//! press its buttons instead of reaching the program.
//!
//! *Addresses are Dunkle Schatten's `XP01.EXE`: linear offsets into its
//! unpacked image, `cs:` words of the library's segment `179Eh`, `[nnnh]`
//! words of `DGROUP`. Captain Zins's build keeps the same routines at its
//! own addresses, which its [`KeyMap`] names.*
//!
//! **The hook.** When the library starts (`179Eh:0A00h`), it calls each
//! module in its table at `179Dh:0000h` with `AX = 0` (`0x18431`–`0x18447`);
//! the second, `179Eh:13B3h`, takes INT 9 (`0x18DC4`–`0x18DE8`) and puts
//! the keyboard's step in the poll's first slot, `[3FEh] = 14E6h`
//! (`0x18E03`). The library's shutdown calls them again with `AX = FFFFh`
//! (`0x18A4E`) and gives the interrupt back. Nothing in between takes it
//! away: for the whole run the hook is in. What it does depends on one
//! byte, `cs:13B2h`, which the image holds at 1 and which only
//! `179Eh:14B4h` changes — called by the developer's room list alone
//! (`0x0F069`, `0x0F1A6`), which this runtime does not have.
//!
//! **The keys.** The handler (`179Eh:142Ah`) reads the scan code from port
//! `60h` (`0x18E14`) and looks its low seven bits up in a table of 60h
//! bytes at `cs:1352h` (`0x18E25`): a slot from 1 to 12, or 0 for a key it
//! leaves alone. A slot's level is the make code's 1 or the break code's 0
//! (`0x18E2D`, `0x18E2F`) in the twelve bytes at `cs:1346h`. A key with a
//! slot is acknowledged at the PIC and **not passed on** (`0x18E34`–
//! `0x18E3A`): the BIOS never buffers it, and the program never reads it as
//! a keystroke. Dunkle Schatten's build swallows an `E0h` prefix the same
//! way (`0x18E19`); Captain Zins's passes it on, which changes nothing,
//! since the key after it is the one with the slot.
//!
//! The table is filled by `179Eh:1483h(list)`: cleared, then the twelve
//! scan codes of `list` given the slots 1 to 12 in order (`0x18E80`–
//! `0x18E8D`). Dunkle Schatten calls it once, right after the loader's
//! pointer setup (`0x5F10`–`0x5F14`, `push ds ; push 228h`), with
//! `47 48 49 4B 4D 4F 50 51 39 1C 4A 4E` (`DS:0228h`); Captain Zins with
//! the same twelve (`0x1662`, `DS:01BEh`). So:
//!
//! | slots | keys | what they do |
//! |---|---|---|
//! | 1–8 | Home, Up, Page Up, Left, Right, End, Down, Page Down | the eight directions |
//! | 9, 10 | Space, Enter | the first button, the second |
//! | 11, 12 | keypad minus, keypad plus | every direction at its slowest, at its fastest |
//!
//! **The step** (`179Eh:14E6h`, `0x18EC6`), first thing in every poll of
//! the mouse (`179Eh:0AF9h`, `0x184D9`). With `cs:13B2h` clear it clears
//! the key buttons `[3CEh]` and does nothing else (`0x18ED7`). With no
//! units elapsed, `[31Ah] = 0`, it does nothing at all (`0x18ED6`). Else
//! each direction's speed — eight words at `[1868h]` — moves: with slot 11
//! down every speed is the slowest `[3C8h]`, with slot 12 down the fastest
//! `[3CAh]` (`0x18EE0`–`0x18F01`); with neither, a held direction adds
//! `[3CCh]` up to `[3CAh]` and a released one drops back to `[3C8h]`
//! (`0x18F05`–`0x18F32`). A held direction then moves by its speed times
//! the units, at most ten, over 256 (`0x18F34`–`0x18F48`, `0x18FA2`–
//! `0x18FB6`), into the same two words the mouse driver's mickeys go into,
//! `[3E2h]` and `[3E4h]` (`0x18F51`–`0x18F8D`). Last, slots 9 and 10 become
//! bits 0 and 1 of `[3CEh]` (`0x18F91`–`0x18F9E`), which the button logic
//! takes as the buttons' level alongside the mouse's (`0x187BE`).
//!
//! **The conversion** (`0x18500`–`0x185FA`), in the same poll and the same
//! for keys and mouse: per axis, y first, an acceleration factor out of the
//! carried value's size times `[3D8h]` plus half the last factor, at most
//! 1000h; from 100h up it scales the carried value, rounded. The value over
//! the setting's divisor, `[3DAh]` for x and `[3DCh]` for y, is the move in
//! pixels; the rest is carried only when the move is zero. The position
//! takes the move inside the limits.

use std::fmt;

/// The twelve scan codes a program can hand the library, the slots in
/// order.
const SLOTS: usize = 12;

/// The eight directions, slots 1 to 8, as `(dx, dy)`: the order the step
/// adds them in (`0x18F51`–`0x18F8D`).
const DIRECTIONS: [(i16, i16); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// Where a build keeps the keyboard's words.
#[derive(Debug, Clone, Copy)]
pub(crate) struct KeyMap {
    /// The twelve scan codes the program hands `179Eh:1483h`, in `DGROUP`.
    pub list: u16,
    /// Image offset of the hook's switch, `cs:13B2h`.
    pub switch: usize,
    /// `[1868h]`: the eight directions' speeds.
    pub speeds: u16,
    /// `[3C8h]`: the slowest speed; the fastest and the increment follow.
    pub limits: u16,
    /// `[3D8h]`: the acceleration's multiplier.
    pub acceleration: u16,
    /// `[3DAh]`: the x divisor; the y divisor follows.
    pub divisors: u16,
}

/// What became of a key the host reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Route {
    /// The key has no slot, or the hook is off: the BIOS takes it.
    Passed,
    /// The key has a slot and is kept from the program.
    Taken,
    /// One of the two button slots, its level: `first` is slot 9.
    Button { first: bool, down: bool },
}

/// The words the step and the conversion read out of `DGROUP` at every
/// poll: the options' mouse row writes the divisors while the game runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Params {
    /// `[3C8h]`, `[3CAh]`, `[3CCh]`: slowest, fastest, increment.
    pub speed: [u16; 3],
    /// `[3D8h]`.
    pub acceleration: u16,
    /// `[3DAh]`, `[3DCh]`: x, y.
    pub divisors: [i16; 2],
}

impl Params {
    /// The words as `ds` holds them now.
    pub(crate) fn read(map: &KeyMap, ds: &[u8]) -> Self {
        let w = |at: u16| super::cursor::word(ds, at);
        Params {
            speed: [w(map.limits), w(map.limits + 2), w(map.limits + 4)],
            acceleration: w(map.acceleration),
            divisors: [
                crate::num::signed(w(map.divisors)),
                crate::num::signed(w(map.divisors + 2)),
            ],
        }
    }
}

/// The keyboard's part of the library's state.
#[derive(Clone)]
pub(crate) struct Keyboard {
    /// `cs:13B2h`.
    pub on: bool,
    /// `cs:1352h`: the slot of each scan code, 0 for none.
    slots: [u8; 0x60],
    /// `cs:1346h`: each slot's level.
    held: [bool; SLOTS],
    /// `[1868h]`.
    speeds: [u16; 8],
    /// `[3E2h]`, `[3E4h]`: what is carried, x and y, in mickeys.
    pub carried: [i16; 2],
    /// `[3DEh]`, `[3E0h]`: the last acceleration factor, x and y.
    pub gain: [u16; 2],
    /// `[3CEh]`: the key buttons, bit 0 the first.
    pub buttons: u8,
    /// The words read at the last poll.
    pub params: Params,
}

impl fmt::Debug for Keyboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Keyboard")
            .field("on", &self.on)
            .field("held", &self.held)
            .field("speeds", &self.speeds)
            .field("carried", &self.carried)
            .field("gain", &self.gain)
            .field("buttons", &self.buttons)
            .finish_non_exhaustive()
    }
}

impl Keyboard {
    /// The keyboard as the program leaves it after its setup call: the
    /// switch out of the image, the slots out of the program's list, the
    /// words out of `ds`.
    ///
    /// `179Eh:1483h` writes a code up to and including 60h (`0x18E84`,
    /// `ja`), and 60h lands one past the table, on the switch itself; the
    /// lists *Dunkle Schatten* and *Captain Zins* hand it stop at 51h.
    pub(crate) fn load(map: &KeyMap, ds: &[u8], image: &[u8]) -> Self {
        let mut on = image.get(map.switch).is_some_and(|&b| b != 0);
        let mut slots = [0; 0x60];
        for (i, slot) in (1u8..).zip(0..SLOTS) {
            let code = ds.get(usize::from(map.list) + slot).copied().unwrap_or(0);
            match usize::from(code) {
                c @ 0..0x60 => slots[c] = i,
                0x60 => on = true,
                _ => {}
            }
        }
        let w = |at: u16| super::cursor::word(ds, at);
        let speeds = std::array::from_fn(|k| w(map.speeds + 2 * crate::num::word(k)));
        Keyboard {
            on,
            slots,
            held: [false; SLOTS],
            speeds,
            carried: [0; 2],
            gain: [0; 2],
            buttons: 0,
            params: Params::read(map, ds),
        }
    }

    /// The handler's reading of a key (`179Eh:142Ah`): `scan` the make code,
    /// `down` whether it is the make or the break.
    pub(crate) fn key(&mut self, scan: u8, down: bool) -> Route {
        if !self.on {
            return Route::Passed;
        }
        let Some(&slot) = self.slots.get(usize::from(scan & 0x7F)) else {
            return Route::Passed;
        };
        let Some(i) = usize::from(slot).checked_sub(1) else {
            return Route::Passed;
        };
        self.held[i] = down;
        match i {
            8 | 9 => Route::Button {
                first: i == 8,
                down,
            },
            _ => Route::Taken,
        }
    }

    /// `179Eh:14E6h`: the keys' step over `elapsed` units.
    pub(crate) fn step(&mut self, elapsed: u16) {
        if !self.on {
            self.buttons = 0;
            return;
        }
        if elapsed == 0 {
            return;
        }
        let [slowest, fastest, increment] = self.params.speed;
        if self.held[10] {
            self.speeds = [slowest; 8];
        } else if self.held[11] {
            self.speeds = [fastest; 8];
        } else {
            for (speed, &held) in self.speeds.iter_mut().zip(&self.held) {
                *speed = if held {
                    let s = speed.wrapping_add(increment);
                    if s > fastest { fastest } else { s }
                } else {
                    slowest
                };
            }
        }
        let units = u32::from(elapsed.min(10));
        for ((&(dx, dy), &speed), &held) in DIRECTIONS.iter().zip(&self.speeds).zip(&self.held) {
            if !held {
                continue;
            }
            // `mul word [di]`, then the middle two bytes of the product.
            let d = crate::num::signed(crate::num::low_word((units * u32::from(speed)) >> 8));
            self.carried[0] = self.carried[0].wrapping_add(dx.wrapping_mul(d));
            self.carried[1] = self.carried[1].wrapping_add(dy.wrapping_mul(d));
        }
        self.buttons = u8::from(self.held[8]) | (u8::from(self.held[9]) << 1);
    }

    /// `0x18500`–`0x185FA` without the mouse driver: what is carried,
    /// accelerated and divided into a move in pixels, `(x, y)`.
    pub(crate) fn convert(&mut self) -> (i16, i16) {
        // y first (`0x18500`–`0x18538`), then x (`0x1853B`–`0x1856F`).
        for axis in [1, 0] {
            self.accelerate(axis);
        }
        (self.divide(0), self.divide(1))
    }

    /// One axis's factor and scaling.
    fn accelerate(&mut self, axis: usize) {
        let value = self.carried[axis];
        let size = u32::from(value.unsigned_abs()) * u32::from(self.params.acceleration);
        let gain = (size + u32::from(self.gain[axis] >> 1)).min(0x1000);
        self.gain[axis] = crate::num::low_word(gain);
        if gain >= 0x100 {
            // `imul`, then the middle two bytes, rounded on the low byte's
            // top bit (`shl al, 1 ; … ; adc ax, 0`).
            let product = i32::from(value) * i32::from(self.gain[axis]);
            let middle = crate::num::low_word(crate::num::unsigned32(product >> 8));
            let round = u16::from(product & 0x80 != 0);
            self.carried[axis] = crate::num::signed(middle.wrapping_add(round));
        }
    }

    /// One axis divided: the move, and what is carried on.
    fn divide(&mut self, axis: usize) -> i16 {
        let divisor = self.params.divisors[axis];
        let value = self.carried[axis];
        // The divisors are 10, 20 or 40 (`147Ah:05DCh`); `idiv` by zero
        // would end the program.
        let (q, r) = match (value.checked_div(divisor), value.checked_rem(divisor)) {
            (Some(q), Some(r)) => (q, r),
            _ => (0, value),
        };
        self.carried[axis] = if q == 0 { r } else { 0 };
        q
    }

    /// `179Eh:12C7h` from `0x18CDE`: the step run with nothing elapsed, and
    /// what is carried and both factors cleared (`0x18CE6`–`0x18CF1`).
    pub(crate) fn reset(&mut self) {
        self.step(0);
        self.carried = [0; 2];
        self.gain = [0; 2];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keyboard() -> Keyboard {
        let mut slots = [0; 0x60];
        for (i, code) in (1u8..).zip([
            0x47u8, 0x48, 0x49, 0x4B, 0x4D, 0x4F, 0x50, 0x51, 0x39, 0x1C, 0x4A, 0x4E,
        ]) {
            slots[usize::from(code)] = i;
        }
        Keyboard {
            on: true,
            slots,
            held: [false; SLOTS],
            speeds: [0; 8],
            carried: [0; 2],
            gain: [0; 2],
            buttons: 0,
            params: Params {
                speed: [0x200, 0x2000, 0x100],
                acceleration: 0x20,
                divisors: [20, 20],
            },
        }
    }

    /// *Dunkle Schatten*'s and *Captain Zins*'s maps point at the twelve keys and the words the
    /// programs ship: the list their setup call hands over, the switch on,
    /// and the speeds, multiplier and divisors as the images hold them.
    #[test]
    fn each_titles_map_finds_its_keys_and_words() {
        let titles = [
            (
                motionvm_parsec_testutil::gamedata_ds1(),
                crate::titles::ds1::build::POINTER.keys,
            ),
            (
                motionvm_parsec_testutil::gamedata_capzins(),
                crate::titles::capzins::build::POINTER.keys,
            ),
            (
                motionvm_parsec_testutil::gamedata_bifi2(),
                crate::titles::bifi2::build::POINTER.keys,
            ),
        ];
        for (dir, map) in titles {
            let Some(dir) = dir else {
                eprintln!("skipping a title: no gamedata directory");
                continue;
            };
            let map = map.expect("the title's keys");
            let image = crate::exe::Image::open_any(&dir, "XP01.EXE").expect("XP01");
            let ds = image.bytes().get(image.at(0)..).expect("DGROUP");
            let mut k = Keyboard::load(&map, ds, image.bytes());
            assert!(k.on, "{}: the switch the image ships", dir.display());
            assert_eq!(
                k.params,
                Params {
                    speed: [512, 8192, 256],
                    acceleration: 32,
                    divisors: [20, 20],
                },
                "{}",
                dir.display()
            );
            for (code, route) in [
                (0x47, Route::Taken),
                (0x51, Route::Taken),
                (0x4A, Route::Taken),
                (0x4E, Route::Taken),
                (
                    0x39,
                    Route::Button {
                        first: true,
                        down: true,
                    },
                ),
                (
                    0x1C,
                    Route::Button {
                        first: false,
                        down: true,
                    },
                ),
                (0x52, Route::Passed),
                (0x01, Route::Passed),
            ] {
                assert_eq!(k.key(code, true), route, "{}: {code:#04x}", dir.display());
            }
        }
    }

    /// A key with a slot is taken, its break as well as its make; one
    /// without is passed; the two button slots answer as buttons.
    #[test]
    fn a_key_with_a_slot_is_taken_and_one_without_passed() {
        let mut k = keyboard();
        assert_eq!(k.key(0x50, true), Route::Taken);
        assert_eq!(k.key(0x50 | 0x80, false), Route::Taken);
        assert_eq!(k.key(0x01, true), Route::Passed, "Escape");
        assert_eq!(k.key(0x0F, true), Route::Passed, "Tab");
        assert_eq!(
            k.key(0x39, true),
            Route::Button {
                first: true,
                down: true
            }
        );
        assert_eq!(
            k.key(0x1C, false),
            Route::Button {
                first: false,
                down: false
            }
        );
        k.on = false;
        assert_eq!(
            k.key(0x50, true),
            Route::Passed,
            "the switch off passes all"
        );
    }

    /// Nothing elapsed, nothing moves; the switch off clears the buttons.
    #[test]
    fn the_step_needs_units_and_the_switch() {
        let mut k = keyboard();
        k.key(0x50, true);
        k.key(0x39, true);
        k.step(0);
        assert_eq!((k.carried, k.buttons), ([0, 0], 0));
        k.step(1);
        assert_eq!(k.buttons, 1);
        assert_eq!(
            k.carried,
            [0, 1],
            "a held key adds 100h to its speed of 0, over 256"
        );
        k.on = false;
        k.step(1);
        assert_eq!(k.buttons, 0);
    }
}
