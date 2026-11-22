//! Location 28, F2. The module is named after the location as the game
//! itself names it in its location table (`F2`).
//!
//! Re-expressed from the handler at `046C:0006` (image `0x46C6`) and its
//! hook at `046C:00C5` (`0x4785`), against [`Api`] alone. The location has
//! no rules of its own.
//!
//! The handler's prologue differs from the standard one: it hands
//! `[1F4Ah]` to the shared entry directly and writes `[1F4Ch]` only after
//! `0EB9:00B8`. Its frame loop has a check of its own between the input
//! and the frame pass: an event of the hook's asks for task 0 to start
//! half a second later.

use super::logic::Logic;
use crate::num;
use crate::titles::parsec1994::Api;

/// Object 200, the one the dialogue animates.
const OBJECT_200: u16 = 0xC8;

/// `[2726h]`: the sub-state the entry plays its scene by.
const SUBSTATE: u16 = 0x2726;
/// `[194Ch]`: non-zero, task 0 is to be started; its value less one is the
/// animation to come back to.
const TASK_PENDING: u16 = 0x194C;
/// `[194Eh]`: the animation object 200 plays.
const PLAYING_ANIM: u16 = 0x194E;
/// `[1950h]`: the animation it is to play at the next page's end.
const NEXT_ANIM: u16 = 0x1950;
/// `[1C82h]`: the hundredths the scene machine counts.
const HUNDREDTHS: u16 = 0x1C82;
/// `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`, `DS:1F56h`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
const HERE: u16 = 0x1F4C;
const OBJECT_STATES: u16 = 0x1F56;

/// Everything before the frame loop, `0x46C6` to `0x471E`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x46C6
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.visit_enter(loc, entry); // 0x46D3
    api.hover_off(); // 0x46DB
    api.obj_wake(OBJECT_200); // 0x46E3
    // 0x46E9: by [2726h] - 1 through the table at 0x477B.
    match u16::from(api.byte(SUBSTATE)).wrapping_sub(1) {
        0 => {
            api.scene_play(0); // 0x46FD
        }
        1 => {
            api.scene_play(1); // 0x4707
        }
        2 => {
            api.scene_play(2); // 0x470B
        }
        4 => {
            api.scene_play(4); // 0x470F
        }
        _ => {} // 3 and above 4: 0x4713
    }
    api.visit_finish_entry(); // 0x4713
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x471B
    api.set_word(ENTRY, 0); // 0x471E
}

/// One turn of the frame loop, `0x4724` to `0x475B`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    // 0x472E: task 0 started once more than 50 hundredths have passed
    // since the event asked for it (`jbe`: unsigned).
    let pending = api.word(TASK_PENDING);
    if pending != 0 && api.word(HUNDREDTHS) > 0x32 {
        api.set_word(NEXT_ANIM, pending.wrapping_sub(1)); // 0x4740
        api.set_word(TASK_PENDING, 0);
        api.task_start(0); // 0x474B
    }
    api.frame_pass(); // 0x4751
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x475D` to `0x477A`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `046C:00C5`: codes 0 to 10 through the table at `0x4848`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x479D: a dialogue comes up.
            api.set_word(NEXT_ANIM, 0);
            api.set_word(PLAYING_ANIM, 0);
        }
        8 => {
            // 0x47A8: animation 1 for an argument of 1, else 0.
            api.set_word(NEXT_ANIM, u16::from(arg == 1));
        }
        9 | 10 => {
            // 0x47B9: a page's end — the next animation, when it changed.
            if api.word(PLAYING_ANIM) != api.word(NEXT_ANIM) {
                let next = api.word(NEXT_ANIM);
                api.set_word(PLAYING_ANIM, next); // 0x47C5
                api.obj_anim_id(OBJECT_200, num::signed(next)); // 0x47CC
            }
        }
        7 => event(api, arg), // 0x47D6
        _ => {}
    }
}

/// Code 7, `0x47D6`: script events 10 to 22 through the table at `0x482E`.
fn event(api: &mut dyn Api, n: u16) {
    match n {
        0x0A => {
            // 0x47E7: task 0 armed, to start in the frame loop.
            api.task_arm(0);
            let next = api.word(NEXT_ANIM).wrapping_add(1); // 0x47EF
            api.set_word(TASK_PENDING, next);
            api.set_word(NEXT_ANIM, 0);
            api.set_word(HUNDREDTHS, 0); // 0x47FC
        }
        0x14 => {
            // 0x4804: to location 27, entry 100.
            api.set_word(LOCATION, 0x1B);
            api.set_word(ENTRY, 0x64);
        }
        0x15 => {
            // 0x4812: to location 27, entry 200.
            api.set_word(LOCATION, 0x1B);
            api.set_word(ENTRY, 0xC8);
        }
        0x16 => {
            // 0x4820: to location 27, entry 102.
            api.set_word(LOCATION, 0x1B);
            api.set_word(ENTRY, 0x66);
        }
        _ => {}
    }
}

/// The location has no rules of its own (its record's `+2Bh` is null).
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
