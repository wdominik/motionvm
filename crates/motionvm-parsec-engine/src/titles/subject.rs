//! Subjects: what a scene of the parsec1994 generation is *about*.
//!
//! The game keeps a small array of **slots**, and everything a scene
//! does is aimed at one of them: a caption hangs over the slot that is
//! speaking, a task acts on the slot it names. The array is at `DS:1D56h`
//! and a slot is six bytes — a code word and, in place of the two words
//! after it, either a far pointer to a record or a literal point. The
//! stride is 6 (`0x0EA6F`), the kind is added into the code at
//! `[bx+1D56h]` (`0x0EA74`, `0x0EA77`), and the pointer's offset and segment
//! go to `[bx+1D58h]` and `[bx+1D5Ah]` (`0x0EA8A`, `0x0EA86`).
//!
//! Four setters write it, and the kind they stamp into the code's high
//! nibble is the whole difference between them:
//!
//! | routine | image | kind | what it takes |
//! |---|---|---|---|
//! | `0CFDh:1A8Ch` | `0x0EA5C` | `1000h` | a record, far |
//! | `0CFDh:1AD0h` | `0x0EAA0` | `2000h` | a record, far |
//! | `0CFDh:1B14h` | `0x0EAE4` | raw | code and two words, as given |
//! | `0CFDh:1B40h` | `0x0EB10` | `3000h` | nothing but the code |
//!
//! The two record setters both fall back to `3000h` when the pointer they
//! are handed is null (`0x0EA6B`, `0x0EAAF`), so an empty slot and a slot
//! whose record was not found read the same way.
//!
//! The resolver `0x0D290` turns a slot into a screen point, switching on
//! `code & 0F000h` through the table at `0x0D474`. Kinds `1000h` and
//! `2000h` share one arm — both are records — so what the two setters
//! really separate is not *how* a subject resolves but *what kind of
//! thing* the program thinks it put there.
//!
//! There are twelve slots, 0 to 11. The entry function writes 0 through
//! 10; `0CFDh:1810h` writes slot 11 directly (`0x0E826`,
//! `mov word [1D98h], 1001h` — `1D56h + 11·6`), and the task array begins
//! right after it at `DS:1D9Eh`.

use crate::titles::parsec1994::Adventure;

/// The slots, `DS:1D56h` to the task array at `DS:1D9Eh`.
pub(crate) const SLOTS: usize = 12;

/// What a slot holds, once the kind in its code is taken apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Subject {
    /// Kind `3000h`: a code and nothing else. What both record setters
    /// leave behind when the record they were handed was null.
    #[default]
    Nothing,
    /// Kind `0000h` (`0x0D2C0`): the two words in place of the pointer
    /// **are** the point.
    Point(i16, i16),
    /// Kind `1000h`: an object record, named here by the id it carries at
    /// `+13h` rather than by an address, because that is what identifies
    /// it on this side.
    Object(u8),
    /// Kind `2000h`: the player's own record. The entry function puts it
    /// in slot 0 (`0x0EDE7`) with the walker's address, `DS:1693h`.
    Player,
    /// Kind `2000h` too: one of the room's people, a 30h-byte record, by
    /// its place in the room's table. The entry function puts one in a
    /// slot whose id names no object (`0x0EE2E`–`0x0EE3E`), and a
    /// location's own code can (`0CFDh:1AD0h`).
    Mover(usize),
}

impl Subject {
    /// The kind nibble this subject is stored under.
    fn kind(self) -> u16 {
        match self {
            Subject::Point(_, _) => 0x0000,
            Subject::Object(_) => 0x1000,
            Subject::Player | Subject::Mover(_) => 0x2000,
            Subject::Nothing => 0x3000,
        }
    }
}

/// One six-byte slot.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Slot {
    /// The word at `+0`. Its high nibble is the kind, its low twelve bits
    /// the payload — which `0x0D3D6` copies into the task record's `+12h`,
    /// the style word a page's box is placed, aligned and backed by.
    pub code: u16,
    /// What the two words after it say.
    pub subject: Subject,
}

impl Slot {
    /// The payload, `code & 0FFFh` (`0x0D3D6`).
    pub(crate) fn payload(self) -> u16 {
        self.code & 0x0FFF
    }
}

/// The array itself.
#[derive(Debug, Clone, Default)]
pub(crate) struct Subjects {
    slots: [Slot; SLOTS],
}

impl Subjects {
    /// Reads a slot back.
    pub(crate) fn get(&self, n: usize) -> Option<Slot> {
        self.slots.get(n).copied()
    }

    /// `0CFDh:1A8Ch` and `0CFDh:1AD0h`: a record, or `3000h` when there is
    /// none. Which of the two setters is meant is the `subject`'s own kind,
    /// because that is the only thing that differs between them.
    pub(crate) fn set(&mut self, n: usize, payload: u16, subject: Subject) {
        let subject = match subject {
            // Both record setters test the pointer they were handed and
            // stamp `3000h` instead when it is null.
            Subject::Object(_) | Subject::Player | Subject::Mover(_) => subject,
            other => other,
        };
        self.write(n, payload | subject.kind(), subject);
    }

    /// `0CFDh:1B40h`: clear a slot to `3000h` and the payload it is given.
    pub(crate) fn clear(&mut self, n: usize, payload: u16) {
        self.write(n, payload | 0x3000, Subject::Nothing);
    }

    /// `0CFDh:1B14h`: the code and the two words exactly as given, which is
    /// how a literal point gets in.
    pub(crate) fn set_point(&mut self, n: usize, code: u16, x: i16, y: i16) {
        self.write(n, code, Subject::Point(x, y));
    }

    fn write(&mut self, n: usize, code: u16, subject: Subject) {
        if let Some(s) = self.slots.get_mut(n) {
            *s = Slot { code, subject };
        }
    }

    /// Where a slot is on the screen, the way `0CFDh:02C0h` (`0x0D290`)
    /// works it out.
    ///
    /// The record arm reads the same fields the object drawer does — `+4`
    /// and `+6` for the point, `+0Eh` and `+10h` to scale the cel's size
    /// unless the factor is `0` or `100h`, `+3` to move the point by it —
    /// and the three alignments are three different moves, not one:
    /// alignment 0 adds half the width to `x` (`0x0D3A5`), 1 takes half the
    /// height off `y` (`0x0D3AF`), 2 takes the whole height off `y`
    /// (`0x0D3B7`).
    ///
    /// A slot of kind `3000h` resolves to the global point `[1854h]` /
    /// `[1856h]`, which has no writer among the plain forms scanned — so
    /// it answers `None` here rather than a made-up place.
    ///
    /// A record's point then moves by the slot's style (`0x0D3ED`–`0x0D42C`):
    /// a vertical style of 2 puts it the cel's height lower, 3 half of it
    /// lower; a horizontal style of 4 half the width to the left, 8 half of
    /// it to the right.
    pub(crate) fn point(&self, game: &Adventure, n: usize) -> Option<(i32, i32)> {
        let slot = self.get(n)?;
        let (x, y, w, h) = match slot.subject {
            Subject::Nothing => return None,
            Subject::Point(x, y) => return Some((i32::from(x), i32::from(y))),
            Subject::Player => game.player_box()?,
            Subject::Mover(i) => game.mover_box(i)?,
            Subject::Object(id) => game.slot_object_box(id)?,
        };
        let style = slot.payload();
        let y = match style & 3 {
            2 => y + h,
            3 => y + (h >> 1),
            _ => y,
        };
        let x = match style & 0x0C {
            4 => x - (w >> 1),
            8 => x + (w >> 1),
            _ => x,
        };
        Some((x, y))
    }
}

/// The move an alignment byte makes to a resolved point (`0x0D38F`).
///
/// Shared so the two record kinds cannot drift apart: `sar` is an
/// arithmetic shift, and the extents are what the cel measures after
/// scaling.
pub(crate) fn align_point(align: u8, x: i32, y: i32, w: i32, h: i32) -> (i32, i32) {
    match align {
        0 => (x + (w >> 1), y),
        1 => (x, y - (h >> 1)),
        2 => (x, y - h),
        // The switch has three arms and falls through to the point
        // untouched (`0x0D3A3`).
        _ => (x, y),
    }
}

/// The cel extent a scale factor gives, `8.8` fixed point (`0x0D336`).
///
/// The multiply is skipped for `0` and `100h`, which is the same skip the
/// drawer makes.
pub(crate) fn scaled(extent: i32, factor: u16) -> i32 {
    if factor == 0 || factor == 0x100 {
        extent
    } else {
        (extent * i32::from(factor)) >> 8
    }
}
