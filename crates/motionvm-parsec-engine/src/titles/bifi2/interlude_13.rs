//! Location 13, a row of kind 7 without a name (the record's `+37h` is
//! null, image `0x21BBB`): an interlude between location 39 and the
//! Öliens-Halle (location 28). Its one exit is `72h` (the row's `+9`),
//! destination `1Ch`, location 28 (`+19h`).
//!
//! Re-expressed from the handler at `13FAh:0008h` (image `0x13FA8` to its
//! `retf` at `0x1402E`) and its animation hook `13FAh:008Fh` (`0x1402F` to
//! its `retf` at `0x1407E`, the record's `+43h`), against [`Api`] alone.
//! The record's rule table `+3Bh` and hook `+3Fh` are null: the location
//! has no rules and the library's empty hook. The segment holds nothing
//! else: Dino Zoo's rules (location 14, segment `1407h`) begin at
//! `0x1407F`, and no write to `[2D66h]`/`[2D68h]` lies between `0x13FA8`
//! and `0x1407E`, so there is no gate of its own.
//!
//! The location is reached from location 39: that row's animation hook
//! (`172Bh:0118h`, `0x173C8`) sends code `65h` through its table at
//! `cs:01AEh` to `0x17409`, which writes `[200Ch]` = `0Dh` and `[200Ah]` =
//! `7Fh`. No other `mov word [200Ch], 0Dh` is in the image, no exit of any
//! row names location 13, and the Kaffee's onward row at `DS:182Ch` does
//! not name it. Location 39 is reached in the same way from location 6,
//! whose animation hook (`1187h:017Ah`, table at `cs:028Bh`) sends code
//! `6Ch` to `0x11ACB` (`[200Ch]` = `27h`, `[200Ah]` = `7Fh`); location 6 is
//! the destination of the exits `6Eh` and `72h` of location 41.
//!
//! Object `F0h` is hidden and set on animation 0 on entry; the input is
//! taken over and given back nowhere in the segment. The animation's codes
//! drive the rest: `64h` cuts the digital effects (when a digital driver is
//! installed) and plays sample `0Eh`, `66h` plays sample `0Fh` on three of
//! every four calls, and `65h` leaves for the Öliens-Halle by entry `72h`,
//! the entry whose arrival there walks the figure to node `0FA0h`.
//!
//! One word outside the saved block (`DS:2B98h`, `4Eh` bytes) is the
//! location's own: `[2EBEh]`, the count of code `66h`, cleared on every
//! entry (`0x13FF0`); no other direct reference to it is in the image.

use super::logic::{Logic, WORDS};
use crate::titles::parsec1994::{Api, Far};

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x14017`).
const OBJECT_STATES: u16 = 0x2018;

/// Object `F0h`, hidden and set on animation 0 on entry (`push 0F0h`,
/// `0x13FC1`).
const OBJECT_F0: u16 = 0xF0;

/// `[2EBEh]`: how often animation code `66h` came, cleared on entry.
const COUNT_66: u16 = 0x2EBE;

/// The Öliens-Halle, location 28 (`mov word [200Ch], 1Ch`, `0x1405F`),
/// and the entry it is given (`mov word [200Ah], 72h`, `0x14065`).
const OELIENS_HALLE: u16 = 0x1C;
const ENTRY_72: u16 = 0x72;

/// Everything before the frame loop, `0x13FA8` to `0x13FFA`. The location
/// the handler runs for, `[200Eh]`, is written only after the entry is
/// finished (`0x13FE8`).
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x13FA8
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.visit_enter(loc, entry); // 0x13FB5
    let r = api.obj_show_frame(OBJECT_F0, -1); // 0x13FC4
    api.obj_anim(r, 0); // 0x13FCE
    api.input_lock(); // 0x13FD6
    api.hover_off(); // 0x13FDB
    api.visit_finish_entry(); // 0x13FE0
    let loc = api.word(WORDS.location); // 0x13FE5
    api.set_word(WORDS.here, loc); // 0x13FE8
    api.set_word(WORDS.entry, 0); // 0x13FED
    api.set_word(COUNT_66, 0); // 0x13FF0
    api.play_sample(0x0F); // 0x13FF5
}

/// One turn of the frame loop, `0x13FFB` to `0x1400F`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x13FFB
    api.dispatch(); // 0x14000
    api.frame_pass(); // 0x14005
    api.word(WORDS.entry) == 0 // 0x1400A
}

/// Everything after the frame loop, `0x14011` to `0x1402E`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x1401C
    api.visit_leave(); // 0x14024
    api.slot_unload(); // 0x14029
}

/// The record names no hook (`+3Fh` is `0000h:0000h`).
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// The animation hook, `13FAh:008Fh`: the code in `[bp+0Ah]`; codes `64h`,
/// `65h` and `66h` have arms (`0x14032`–`0x14044`).
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        // 0x14046: the effects cut, sample 0Eh.
        0x64 => {
            // `cmp word [2B0h], 0 ; jz`: with a digital driver, its
            // `StopAll` without the wait.
            api.digital_stop(0); // 0x1404F, 1975h:0436h
            api.play_sample(0x0E); // 0x14057
        }
        // 0x1405F: on to the Öliens-Halle by entry 72h.
        0x65 => {
            api.set_word(WORDS.location, OELIENS_HALLE); // 0x1405F
            api.set_word(WORDS.entry, ENTRY_72); // 0x14065
        }
        // 0x1406D: the count read, then stepped; sample 0Fh unless the
        // count read is a multiple of 4.
        0x66 => {
            let n = api.word(COUNT_66); // 0x1406D
            api.set_word(COUNT_66, n.wrapping_add(1)); // 0x14070
            if n & 3 != 0 {
                // 0x14074
                api.play_sample(0x0F); // 0x14079, 0x14057
            }
        }
        _ => {}
    }
}

/// The location has no rule table (the record's `+3Bh` is null).
pub(crate) fn rule(_api: &mut dyn Api, _off: u16, _flags: u16) -> Option<i16> {
    None
}

/// This location's entry points, as [`super::logic`] calls them.
pub(super) const LOGIC: Logic = Logic {
    enter,
    frame,
    leave,
    hook,
    anim_hook: Some(anim_hook),
    rule,
};
