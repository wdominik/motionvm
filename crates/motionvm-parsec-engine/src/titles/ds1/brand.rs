//! Location 15, the Brand: the fire. The module is named after the
//! location as the game itself names it in its location table (`Brand`).
//!
//! Re-expressed from the handler at `035C:0009` (image `0x35C9`) and its
//! hook at `035C:00DD` (`0x369D`), against [`Api`] alone. The location
//! has no rules table and no animation hook.
//!
//! The handler calls `1586:06B1` (image `0x15F11`), the story routine
//! [`story::routine_06b1`] shared with the JZ and the Schule.

use super::logic::Logic;
use super::story;
use crate::num;
use crate::titles::parsec1994::Api;

/// Objects 200, 201, 203, 205, 207 and 209, woken on the first visit.
const WOKEN: [u16; 6] = [0xC8, 0xC9, 0xCB, 0xCD, 0xCF, 0xD1];
/// Objects 211 to 213, shown on the first visit.
const SHOWN: [u16; 3] = [0xD3, 0xD4, 0xD5];
/// Objects 200 and 207, the two the hook's animations play on.
const OBJECT_200: u16 = 0xC8;
const OBJECT_207: u16 = 0xCF;

/// `[2719h]`: the location state; below 4, the fire is new.
const STATE: u16 = 0x2719;
/// `[2722h]`: a scene sequence's counter, reset on the first visit.
const SEQUENCE: u16 = 0x2722;
/// `[2706h]`.
const BYTE_2706: u16 = 0x2706;
/// `[192Ah]` whether objects 200 and 207 follow the pages, `[1926h]`
/// their next animation, `[1928h]` the one they play.
const FOLLOW: u16 = 0x192A;
const NEXT: u16 = 0x1926;
const PLAYING: u16 = 0x1928;
/// `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`, `DS:1F56h`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
const HERE: u16 = 0x1F4C;
const OBJECT_STATES: u16 = 0x1F56;

/// Everything before the frame loop, `0x35C9` to `0x3663`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x35C9
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x35D5
    api.visit_enter(loc, entry); // 0x35D9
    // 0x35E1: `cmp [2719h], 4 ; jae` — below 4, the first visit.
    if api.byte(STATE) < 4 {
        for id in WOKEN {
            api.obj_wake(id); // 0x35E8
        }
        for id in SHOWN {
            api.obj_show(id); // 0x361E
        }
        api.set_byte(SEQUENCE, 0); // 0x3639
        api.set_byte(STATE, 4);
        story::routine_06b1(api); // 0x3643
        api.set_byte(BYTE_2706, 1); // 0x3648
        api.walk_to_node(0x0BB8); // 0x364D
        api.scene_play(0); // 0x3656
    }
    api.visit_finish_entry(); // 0x365E
    api.set_word(ENTRY, 0); // 0x3663
}

/// One turn of the frame loop, `0x3669` to `0x367D`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x367F` to `0x369C`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `035C:00DD`: codes 0 to 9 through the table at `0x3722`;
/// codes 1 to 4 and 7 do nothing.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x36B2: a dialogue comes up; speaker bits 1 and 2.
            api.set_word(FOLLOW, arg & 6);
            api.set_word(NEXT, 0);
            api.set_word(PLAYING, 0);
        }
        5 | 6 => {
            // 0x36C4: back to animation 0.
            if api.word(FOLLOW) != 0 {
                api.set_word(FOLLOW, 0);
                api.obj_wake(OBJECT_200); // 0x36D1
                api.obj_wake(OBJECT_207);
            }
        }
        8 => api.set_word(NEXT, arg.wrapping_add(1)), // 0x36E5
        9 => {
            // 0x36ED: a page's end — the next animation on both.
            if api.word(FOLLOW) != 0 && api.word(PLAYING) != api.word(NEXT) {
                let next = api.word(NEXT); // 0x36FD
                api.set_word(PLAYING, next);
                api.obj_anim_id(OBJECT_200, num::signed(next));
                api.obj_anim_id(OBJECT_207, num::signed(api.word(PLAYING))); // 0x370F
            }
        }
        _ => {}
    }
}

/// The Brand has no rules table.
pub(crate) fn rule(_api: &mut dyn Api, _off: u16, _flags: u16) -> Option<i16> {
    None
}

/// This location's entry points, as [`super::logic`] calls them.
pub(super) const LOGIC: Logic = Logic {
    enter,
    frame,
    leave,
    hook,
    anim_hook: None,
    rule,
};
