//! Location 13, `Schrank`, as the game names it in its location table
//! (record at image `0x1EDEC`).
//!
//! Re-expressed from the handler at `134A:0269` (image `0x13709`), its
//! animation hook at `134A:0319` (`0x137B9`), its five rule handlers at
//! `134A:0035`, `134A:0052`, `134A:0080`, `134A:01CD` and `134A:024E` (ten
//! rules at `1F73:03E0`) and the near helper at `134A:000B` they share,
//! against [`Api`] alone. The location has no hook (`0000:0000`).
//! Addresses in comments are offsets in segment `134Ah`.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::titles::parsec1994::{Api, Far};

/// Object 200.
const OBJECT_C8: u16 = 0xC8;
/// Object 201.
const OBJECT_C9: u16 = 0xC9;
/// Object 202.
const OBJECT_CA: u16 = 0xCA;
/// Object 203.
const OBJECT_CB: u16 = 0xCB;
/// Object 204.
const OBJECT_CC: u16 = 0xCC;
/// Object 205.
const OBJECT_CD: u16 = 0xCD;

/// `[1CF9h]`: this location's state byte; 1 once rule `0080` has run its
/// arm `0120`.
const STATE: u16 = 0x1CF9;
/// `[1E5Ch]`: the object id rules `0052` and `0080` work on (DS1
/// `[2891h]`).
const RULE_OBJECT: u16 = 0x1E5C;
/// `[1E60h]`: the object id rule `01CD` works on (DS1 `[2895h]`).
const RULE_TARGET: u16 = 0x1E60;

/// The near helper, `134A:000B`, one word argument: the object's record
/// by `obj_find`; with bit 0 of its byte `+2` set, its byte `+9` plus
/// one, else 0. The original does not test the record pointer.
fn shown_frame(api: &mut dyn Api, id: u16) -> u16 {
    let rec = api.obj_find(id); // 0012
    let flags = api.far_byte(Far {
        off: rec.off.wrapping_add(2),
        seg: rec.seg,
    });
    if flags & 1 != 0 {
        // 0028
        let frame = api.far_byte(Far {
            off: rec.off.wrapping_add(9),
            seg: rec.seg,
        });
        u16::from(frame).wrapping_add(1)
    } else {
        0 // 0031
    }
}

/// `obj_show(id)` and that record on animation 0, as at `00EB`, `0100`,
/// `0153`, `0167`, `0207`, `021C`, `02B0` and `02C5`.
fn show_anim0(api: &mut dyn Api, id: u16) {
    let rec = api.obj_show(id);
    api.obj_anim(rec, 0);
}

/// Everything before the frame loop, `0269` to `02E4`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0269
    let entry = api.word(ENTRY); // 026E
    let loc = api.word(LOCATION); // 0272
    api.set_word(HERE, loc); // 0275
    api.visit_enter(loc, entry); // 0279

    // 0281: the helper on object 202, then on 203, and the two summed.
    let a = shown_frame(api, OBJECT_CA);
    let b = shown_frame(api, OBJECT_CB); // 0289
    let sum = a.wrapping_add(b); // 0291
    if sum != 0 {
        if sum == 3 {
            api.obj_show(OBJECT_CC); // 029C
            api.obj_show(OBJECT_CD); // 02A5
        } else {
            show_anim0(api, OBJECT_CC); // 02B0
            show_anim0(api, OBJECT_CD); // 02C5
        }
    }
    api.visit_finish_entry(); // 02DA
    api.set_word(ENTRY, 0); // 02DF
}

/// One turn of the frame loop, `02E5` to `02F9`. Answers whether the loop
/// goes on.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 02E5
    api.dispatch(); // 02EA
    api.frame_pass(); // 02EF
    api.word(ENTRY) == 0 // 02F4
}

/// Everything after the frame loop, `02FB` to `0318`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE); // 02FB
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0306
    api.visit_leave(); // 030E
    api.slot_unload(); // 0313
}

/// Schrank has no hook (`0000:0000` in its record).
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// The animation hook, `134A:0319`: only code 64h does anything.
pub(crate) fn anim_hook(api: &mut dyn Api, rec: Far, code: u16) {
    if code != 0x64 {
        return; // 0322
    }
    api.input_unlock(); // 0324
    let id = api.far_byte(Far {
        off: rec.off.wrapping_add(0x13),
        seg: rec.seg,
    });
    match id {
        0x10 => {
            api.obj_show(OBJECT_C8); // 033C
            api.obj_show_frame(OBJECT_CC, 0); // 0345
            api.play_sample(0x0A); // 0352
        }
        0x0E => {
            api.obj_show(OBJECT_C9); // 035C
            api.obj_show_frame(OBJECT_CD, 0); // 0365 -> 034A
            api.play_sample(0x0A); // 0352
        }
        _ => {} // 033A
    }
}

/// The rule handlers by their offset in segment `134Ah`. Answers what the
/// original answers the rule search with, or `None` for an offset that is
/// not one of them.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 134A:0035 (verb 1 on objects 200 and 201): scene 2 once
        // `[1CF9h]` is 1 or more (`jb`), else not handled.
        0x0035 => {
            if api.byte(STATE) >= 1 {
                api.scene_play(2); // 003F
                0x0A
            } else {
                0 // 0049
            }
        }
        // 134A:0052 (verb 1 on objects 202 and 203): by the helper on
        // `[1E5Ch]`.
        0x0052 => {
            let id = api.word(RULE_OBJECT); // 0055
            match shown_frame(api, id) {
                1 => {
                    api.scene_play(4); // 0069
                    0x0A
                }
                2 => {
                    api.scene_play(0x0A); // 0073 -> 006B
                    0x0A
                }
                _ => 0, // 0077
            }
        }
        // 134A:0080 (verb 4 on objects 202 and 203).
        0x0080 => open_or_close(api),
        // 134A:01CD (verb 5, item 13 on objects 202 and 203).
        0x01CD => {
            let other = api.word(RULE_TARGET) ^ 1; // 01D0
            let state = shown_frame(api, other); // 01D7
            if state <= 1 {
                if state == 1 {
                    let other = api.word(RULE_TARGET) ^ 1; // 01E4
                    api.obj_hide(other); // 01EB
                    api.play_sample(4); // 01F1
                }
                // 01F9
                let id = api.word(RULE_TARGET);
                api.obj_show_frame(id, 1); // 01FF
                show_anim0(api, OBJECT_CC); // 0207
                show_anim0(api, OBJECT_CD); // 021C
                api.play_sample(0x18); // 0231
                api.play_sample(0x0A); // 0239
                api.inv_take(0x0D); // 0241
            }
            1 // 0249
        }
        // 134A:024E (verb 4 on objects 200 and 201): scene 0 while
        // `[1CF9h]` is below 1 (`jae`), else not handled.
        0x024E => {
            if api.byte(STATE) >= 1 {
                0 // 0265
            } else {
                api.scene_play(0); // 0258
                0x0A
            }
        }
        _ => return None,
    })
}

/// Rule `134A:0080`.
fn open_or_close(api: &mut dyn Api) -> i16 {
    if api.byte(STATE) >= 1 {
        return 0; // 008A -> 01C2
    }
    let id = api.word(RULE_OBJECT); // 008D
    match shown_frame(api, id) {
        0 => {
            // 00AC: by the helper on the other of the pair,
            // `[1E5Ch] ^ 1`.
            let other = api.word(RULE_OBJECT) ^ 1;
            match shown_frame(api, other) {
                0 => first_shown(api), // 00DD
                1 => {
                    let other = api.word(RULE_OBJECT) ^ 1; // 00C8
                    api.obj_hide(other); // 00CF
                    api.play_sample(4); // 00D5
                    first_shown(api); // 00DD
                }
                2 => {
                    let id = api.word(RULE_OBJECT);
                    api.obj_show_frame(id, 0); // 0120
                    api.play_sample(5); // 012E
                    api.play_sample(6); // 0136

                    api.input_lock(); // 013E
                    api.play_sample(0x1E); // 0143
                    api.play_sample(0x1E); // 014B
                    show_anim0(api, 0x10); // 0153
                    show_anim0(api, 0x0E); // 0167
                    api.obj_anim_id(OBJECT_CC, 1); // 017B
                    api.obj_anim_id(OBJECT_CD, 1); // 0188
                    api.set_byte(STATE, 1); // 0195
                }
                _ => {} // 00C5
            }
            1 // 01C8
        }
        1 => {
            api.play_sample(4); // 019C
            api.obj_hide(OBJECT_CC); // 01A4
            api.obj_hide(OBJECT_CD); // 01AD
            let id = api.word(RULE_OBJECT);
            api.obj_hide(id); // 01B6
            1 // 01BF -> 011C -> 01C8
        }
        2 => 0, // 00A6 -> 01C2
        _ => 1, // 00A9 -> 01C8
    }
}

/// `00DD`: the rule's object shown on frame 0, objects 204 and 205 shown
/// on animation 0, sample 5.
fn first_shown(api: &mut dyn Api) {
    let id = api.word(RULE_OBJECT);
    api.obj_show_frame(id, 0); // 00DD
    show_anim0(api, OBJECT_CC); // 00EB
    show_anim0(api, OBJECT_CD); // 0100
    api.play_sample(5); // 0115
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
