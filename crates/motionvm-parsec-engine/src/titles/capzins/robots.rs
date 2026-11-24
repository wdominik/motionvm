//! Location 11, the Robots. The module is named after the location as
//! the game itself names it in its location table (`Robots`,
//! `1FC4:00F8`).
//!
//! Re-expressed from the handler at `130A:002F` (image `0x130CF`), its
//! hook at `130A:00D6` (`0x13176`) and the one rule handler its three
//! rule rows share, `130A:0002` (`0x130A2`), against [`Api`] alone. The
//! location has no animation hook.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::Api;

/// Objects 200, 201 and 202: the three that animate together.
const OBJECT_200: u16 = 0xC8;
const OBJECT_201: u16 = 0xC9;
const OBJECT_202: u16 = 0xCA;

/// `[1CF4h]` and `[1CF5h]`: two state bytes the rule reads.
const BYTE_1CF4: u16 = 0x1CF4;
const BYTE_1CF5: u16 = 0x1CF5;
/// `[1EE8h]`: which of the three speak in the dialogue that is up (bits
/// 1 to 3 of its speakers).
const SPEAKS: u16 = 0x1EE8;
/// `[1EE6h]`: the animation they are playing.
const PLAYING_ANIM: u16 = 0x1EE6;
/// `[16F4h]`: the word the frame loop compares with 32h and rewrites
/// (DS1 `[1C82h]`, the hundredths).
const HUNDREDTHS: u16 = 0x16F4;

/// Everything before the frame loop, `0x130CF` to `0x13113`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x130CF
    let entry = api.word(ENTRY); // 0x130D4
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x130DB
    api.visit_enter(loc, entry); // 0x130DF
    api.obj_anim_id(OBJECT_200, 0); // 0x130EC
    api.obj_anim_id(OBJECT_201, 0); // 0x130F9
    api.obj_anim_id(OBJECT_202, 0); // 0x13106
    api.visit_finish_entry(); // 0x1310E
    api.set_word(ENTRY, 0); // 0x13113
}

/// One turn of the frame loop, `0x13119` to `0x13156`: between dispatch
/// and frame pass, a random sample whenever `[16F4h]` is above 32h.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x13119
    api.dispatch(); // 0x1311E
    // 0x13123: `cmp [16F4h], 32h ; jbe`.
    if api.word(HUNDREDTHS) > 0x32 {
        let k = api.random(4, 6); // 0x1312E
        api.play_sample(k); // 0x13137
        let v = api.random(0, 0x1E); // 0x13141
        api.set_word(HUNDREDTHS, v); // 0x13149
    }
    api.frame_pass(); // 0x1314C
    api.word(ENTRY) == 0 // 0x13151
}

/// Everything after the frame loop, `0x13158` to `0x13175`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE); // 0x13158
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x13163
    api.visit_leave(); // 0x1316B
    api.slot_unload(); // 0x13170
}

/// The hook, `130A:00D6`: the three objects' animations follow the
/// dialogue, and one script event. Codes 0 to 9 through the table at
/// `0x13223`; codes 1 to 4, 8 and above 9 do nothing (`0x13221`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x1318E: bits 1 to 3 of the speakers.
            api.set_word(SPEAKS, arg & 0x0E); // 0x13193
            api.set_word(PLAYING_ANIM, 0); // 0x13196
        }
        5 | 6 => {
            // 0x1319F: if one of them spoke, all three back to
            // animation 0.
            if api.word(SPEAKS) != 0 {
                api.obj_anim_id(OBJECT_200, 0); // 0x131AB
                api.obj_anim_id(OBJECT_201, 0); // 0x131B8
                api.obj_anim_id(OBJECT_202, 0); // 0x131C5
                api.set_word(SPEAKS, 0); // 0x131CD
            }
        }
        9 => {
            // 0x131D5: animation `arg + 1` for all three, if it is not
            // the one playing.
            if api.word(SPEAKS) != 0 {
                let next = arg.wrapping_add(1); // 0x131DC
                if next != api.word(PLAYING_ANIM) {
                    api.set_word(PLAYING_ANIM, next); // 0x131E8
                    api.obj_anim_id(OBJECT_200, num::signed(next)); // 0x131EF
                    let playing = api.word(PLAYING_ANIM); // 0x131F7
                    api.obj_anim_id(OBJECT_201, num::signed(playing)); // 0x131FE
                    let playing = api.word(PLAYING_ANIM); // 0x13206
                    api.obj_anim_id(OBJECT_202, num::signed(playing)); // 0x1320D
                }
            }
        }
        7 => {
            // 0x13217: event 10.
            if arg == 0x0A {
                api.set_byte(BYTE_1CF5, 1); // 0x1321C
            }
        }
        _ => {}
    }
}

/// The Robots' rule handler by its offset in segment `130Ah`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 130A:0002, verb 2 on object 200, 201 or 202 (rule rows
        // `1F73:0300` to `1F73:031C`): scene 1 while `[1CF4h]` is 1 and
        // `[1CF5h]` is below 2, else one of scenes 2 to 4.
        0x0002 => {
            // 0x130A5: `cmp [1CF4h], 1 ; jne` — 0x130AC: `cmp [1CF5h], 2 ; jae`.
            if api.byte(BYTE_1CF4) == 1 && api.byte(BYTE_1CF5) < 2 {
                api.scene_play(1); // 0x130B3, 0x130C4
            } else {
                let n = api.random(2, 4); // 0x130BB
                api.scene_play(num::signed(n)); // 0x130C4
            }
            0x0A // 0x130CA
        }
        _ => return None,
    })
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
