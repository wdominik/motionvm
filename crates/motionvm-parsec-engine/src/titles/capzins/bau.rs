//! Location 12, the Bau. The module is named after the location as the
//! game itself names it in its location table (`Bau`, `1FC4:00FF`).
//!
//! Re-expressed from the handler at `1323:0121` (image `0x13351`), its
//! hook at `1323:01D3` (`0x13403`), its animation hook at `1323:018B`
//! (`0x133BB`) and its eight rule handlers at `1323:0007`, `0034`,
//! `006E`, `0095`, `00A9`, `00CE`, `00DD` and `00F8` (ten rule rows from
//! `1F73:0340`), against [`Api`] alone.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 200, the one the rules ask `1021:07B0` about.
const OBJECT_200: u16 = 0xC8;
/// Object 201, the one the hook animates.
const OBJECT_201: u16 = 0xC9;
/// Object 203, set on animation 0 at the entry.
const OBJECT_203: u16 = 0xCB;
/// Object 13, shown by script event 11.
const OBJECT_13: u16 = 0x0D;

/// `[1CF8h]`: the location state.
const STATE: u16 = 0x1CF8;
/// `[12AAh]`: the animation object 201 plays in a dialogue; negative
/// (`FFFFh`) when it takes no part.
const PLAYING_ANIM: u16 = 0x12AA;
/// `[12ACh]`: set by rule `006E`, read and cleared by the animation hook.
const PENDING: u16 = 0x12AC;
/// `[1E60h]`: the word rule `00F8` compares with C8h (DS1 `[2895h]`).
const WORD_1E60: u16 = 0x1E60;

/// Everything before the frame loop, `0x13351` to `0x13381`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x13351
    let entry = api.word(ENTRY); // 0x13356
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x1335D
    api.visit_enter(loc, entry); // 0x13361
    api.obj_anim_id(OBJECT_203, 0); // 0x1336E
    api.set_word(PLAYING_ANIM, 0xFFFF); // 0x13376
    api.visit_finish_entry(); // 0x1337C
    api.set_word(ENTRY, 0); // 0x13381
}

/// One turn of the frame loop, `0x13387` to `0x1339B`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x13387
    api.dispatch(); // 0x1338C
    api.frame_pass(); // 0x13391
    api.word(ENTRY) == 0 // 0x13396
}

/// Everything after the frame loop, `0x1339D` to `0x133BA`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE); // 0x1339D
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x133A8
    api.visit_leave(); // 0x133B0
    api.slot_unload(); // 0x133B5
}

/// The hook, `1323:01D3`: object 201 animates through a dialogue once
/// script event 10 has shown it. Codes 0 to 8 through the table at
/// `0x13499`; codes 1 to 4 and above 8 do nothing (`0x13497`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 | 5 | 6 => {
            // 0x1341B: `cmp [12AAh], 0 ; jl` — signed.
            if num::signed(api.word(PLAYING_ANIM)) >= 0 {
                api.obj_hide(OBJECT_201); // 0x13425
            }
            api.set_word(PLAYING_ANIM, 0xFFFF); // 0x1342B
        }
        8 => {
            // 0x13433: `cmp [12AAh], 0 ; jl`, then `arg + 1` against it.
            if num::signed(api.word(PLAYING_ANIM)) >= 0 {
                let next = arg.wrapping_add(1); // 0x1343A
                if next != api.word(PLAYING_ANIM) {
                    api.set_word(PLAYING_ANIM, next); // 0x13446
                    api.obj_anim_id(OBJECT_201, num::signed(next)); // 0x1344D
                }
            }
        }
        7 => match arg {
            // 0x13457: events 10 and 11.
            0x0A => {
                api.set_word(PLAYING_ANIM, 0); // 0x13467
                let rec = api.obj_show(OBJECT_201); // 0x1346E
                api.obj_anim(rec, 0); // 0x13476
                api.play_sample(0x17); // 0x13480
            }
            0x0B => {
                api.play_sample(0x18); // 0x13489
                api.obj_show(OBJECT_13); // 0x13491
            }
            _ => {}
        },
        _ => {}
    }
}

/// The animation hook, `1323:018B`: on code 0Bh, with `[12ACh]` set,
/// three samples, item 12 taken, object 200 hidden and the state to 2.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    // 0x133C1: `cmp ax, 0Bh ; jne` — 0x133C6: `cmp [12ACh], 0 ; je`.
    if code == 0x0B && api.word(PENDING) != 0 {
        api.play_sample(0x1F); // 0x133CF
        api.play_sample(8); // 0x133D7
        api.play_sample(0x14); // 0x133DF
        api.inv_take(0x0C); // 0x133E7
        api.obj_hide(OBJECT_200); // 0x133F0
        api.set_byte(STATE, 2); // 0x133F6
        api.set_word(PENDING, 0); // 0x133FB
    }
}

/// The Bau's rule handlers by their offset in segment `1323h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 1323:0007, verb 1 on object 201: scene 1, 2 or 3 by the state;
        // above 2 no scene.
        0x0007 => {
            match api.byte(STATE) {
                0 => {
                    api.scene_play(1); // 0x1324F
                }
                1 => {
                    api.scene_play(2); // 0x13253
                }
                2 => {
                    api.scene_play(3); // 0x13257
                }
                _ => {} // 0x1324D
            }
            0x0A // 0x1325F
        }
        // 1323:0034, verb 1 on object 200: scene 4 while `1021:07B0`
        // answers non-zero for it, else by the state.
        0x0034 => {
            if api.obj_state(OBJECT_200) != 0 {
                api.scene_play(4); // 0x13274
            } else {
                match api.byte(STATE) {
                    0 => {
                        api.scene_play(5); // 0x1328D
                    }
                    1 | 2 => {
                        api.scene_play(6); // 0x13291
                    }
                    _ => {} // 0x1328B
                }
            }
            0x0A // 0x13299
        }
        // 1323:006E, verbs 9 and 5 with mask 20h (rows 5 and 6).
        0x006E => {
            if api.obj_state(OBJECT_200) != 0 {
                api.set_word(PENDING, 1); // 0x132AE
                api.scene_play(7); // 0x132B4
            } else {
                api.scene_play(8); // 0x132B8
            }
            0x0A // 0x132C0
        }
        // 1323:0095, verb 0Ch on object 13.
        0x0095 => {
            api.set_byte(STATE, 2); // 0x132C8
            api.play_sample(0x18); // 0x132CF
            0 // 0x132D5
        }
        // 1323:00A9, verb 7 on object 200.
        0x00A9 => {
            // 0x132DC: `cmp [1CF8h], 1 ; jb`.
            if api.byte(STATE) >= 1 {
                api.scene_play(9); // 0x132E5
                0x0A // 0x132F9
            } else {
                api.play_sample(3); // 0x132EF
                0 // 0x132F5
            }
        }
        // 1323:00CE, verb 8 on object 200.
        0x00CE => {
            api.play_sample(0x14); // 0x13303
            0 // 0x13309
        }
        // 1323:00DD, verb 2 on object 201.
        0x00DD => {
            // 0x13310: `cmp [1CF8h], 1 ; jb`.
            if api.byte(STATE) >= 1 {
                api.scene_play(0x0B); // 0x13319
                0x0A // 0x1331F
            } else {
                0 // 0x13324
            }
        }
        // 1323:00F8, verbs 9 and 5 with mask 20h (rows 7 and 8).
        0x00F8 => {
            // 0x1332B: `cmp [1E60h], C8h ; jne` — then `1021:07B0(C8h)`.
            if api.word(WORD_1E60) == 0xC8 && api.obj_state(OBJECT_200) != 0 {
                api.scene_play(0x0A); // 0x13342
                0x0A // 0x13348
            } else {
                0 // 0x1334D
            }
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
    anim_hook: Some(anim_hook),
    rule,
};
