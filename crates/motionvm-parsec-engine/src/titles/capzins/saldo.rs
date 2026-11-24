//! Location 10, the Saldo. The module is named after the location as the
//! game itself names it in its location table (`Saldo`, `1FC4:00F2`).
//!
//! Re-expressed from the handler at `12AC:0152` (image `0x12C12`), its
//! hook at `12AC:040C` (`0x12ECC`), its animation hook at `12AC:02B9`
//! (`0x12D79`) and its eight rule handlers at `12AC:000C`, `0042`,
//! `0084`, `00DC`, `00F6`, `0116`, `0126` and `0136` (eleven rule rows
//! from `1F73:0250`), against [`Api`] alone.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 128 (80h), the one bit 1 of a dialogue's speakers animates.
const OBJECT_128: u16 = 0x80;
/// Objects 11 and 203, the pair bit 2 of the speakers animates.
const OBJECT_11: u16 = 0x0B;
const OBJECT_203: u16 = 0xCB;
/// Objects 201 and 202.
const OBJECT_201: u16 = 0xC9;
const OBJECT_202: u16 = 0xCA;
/// Objects 100 and 101.
const OBJECT_100: u16 = 0x64;
const OBJECT_101: u16 = 0x65;

/// `[1CF3h]`: the location state.
const STATE: u16 = 0x1CF3;
/// `[1EDCh]`: whether object 128 speaks in the dialogue that is up.
const A_SPEAKS: u16 = 0x1EDC;
/// `[1EE0h]`: the animation it is to play next.
const A_NEXT: u16 = 0x1EE0;
/// `[1EDEh]`: the animation it is playing.
const A_PLAYING: u16 = 0x1EDE;
/// `[1ED6h]`: whether objects 11 and 203 speak.
const B_SPEAKS: u16 = 0x1ED6;
/// `[1EDAh]`, `[1ED8h]`: their next and playing animation.
const B_NEXT: u16 = 0x1EDA;
const B_PLAYING: u16 = 0x1ED8;
/// `[1EE4h]`: the node (`BB8h` or `BB9h`) the frame loop waits for.
const TARGET_NODE: u16 = 0x1EE4;
/// `[1EE2h]`: set when the frame loop saw the node reached, cleared by
/// the animation hook.
const WORD_1EE2: u16 = 0x1EE2;
/// `DS:11DCh`: the record handed to `05F2:05DB`, `05F2:0F83` and
/// `05F2:0FDD` (`push ds ; push 11DCh`).
const WALKER: u16 = 0x11DC;
/// `[1204h]`, word `+28h` of that record, compared with the node waited
/// for (DS1 `[16BBh]`).
const WALKER_NODE: u16 = 0x1204;
/// `[1E86h]`, `[1E88h]`: two words rules `000C` and `0042` clear (DS1
/// `[28BBh]`, `[28BDh]`).
const WORD_1E86: u16 = 0x1E86;
const WORD_1E88: u16 = 0x1E88;
/// `[1E60h]`: the word rule `0136` compares with 80h (DS1 `[2895h]`).
const WORD_1E60: u16 = 0x1E60;

/// Everything before the frame loop, `0x12C12` to `0x12CE5`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x12C12
    let entry = api.word(ENTRY); // 0x12C17
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x12C1E
    api.visit_enter(loc, entry); // 0x12C22
    // 0x12C2A: states 1 to 4 through the table at 0x12D71.
    match api.byte(STATE) {
        1 => {
            // 0x12C41: only with item 8 carried.
            if api.inv_has(8) != 0 {
                // 0x12C50
                api.input_lock();
                // 0x12C5C
                api.node_block(api.ds(WALKER), 0x03E8);
                let rec = api.obj_show(OBJECT_128); // 0x12C69
                api.obj_anim(rec, 3); // 0x12C71
                api.set_byte(STATE, 2); // 0x12C79
                // 0x12C85
                api.interaction(0x80, -1, 1);
            }
        }
        2 => {
            // 0x12C8C
            // 0x12C93
            api.node_block(api.ds(WALKER), 0x03E8);
            let rec = api.obj_show(OBJECT_128); // 0x12CA0
            api.obj_anim(rec, 0); // 0x12CA8
            let rec = api.obj_show(OBJECT_201); // 0x12CD0
            api.obj_anim(rec, 0); // 0x12CD8
        }
        3 | 4 => {
            // 0x12CB7
            let rec = api.obj_show(OBJECT_11); // 0x12CBB
            api.obj_anim(rec, 0); // 0x12CC3
            let rec = api.obj_show(OBJECT_203); // 0x12CD0
            api.obj_anim(rec, 0); // 0x12CD8
        }
        _ => {} // 0x12CE0
    }
    api.visit_finish_entry(); // 0x12CE0
    api.set_word(ENTRY, 0); // 0x12CE5
}

/// One turn of the frame loop, `0x12CEB` to `0x12D51`: between dispatch
/// and frame pass, the walk to node `BB8h` or `BB9h` is watched.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x12CEB
    api.dispatch(); // 0x12CF0
    // 0x12CF5: `mov ax, [1EE4h]`, BB8h or BB9h; then `[1204h]` against it.
    let target = api.word(TARGET_NODE);
    if (target == 0x0BB8 || target == 0x0BB9) && api.word(WALKER_NODE) == api.word(TARGET_NODE) {
        // 0x12D04 for BB8h (object 11), 0x12D37 for BB9h (object 203).
        let id = if target == 0x0BB8 {
            OBJECT_11
        } else {
            OBJECT_203
        };
        api.obj_anim_id(id, 5); // 0x12D11
        api.play_sample(8); // 0x12D1B
        api.task_start(0); // 0x12D23
        api.set_word(TARGET_NODE, 0); // 0x12D29
        api.set_word(WORD_1EE2, 1); // 0x12D2F
    }
    api.frame_pass(); // 0x12D47
    api.word(ENTRY) == 0 // 0x12D4C
}

/// Everything after the frame loop, `0x12D53` to `0x12D70`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE); // 0x12D53
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x12D5E
    api.visit_leave(); // 0x12D66
    api.slot_unload(); // 0x12D6B
}

/// Both of the pair back to animation 0 and their two words cleared,
/// as `0x12F2E` and `0x12F95` do it in the same order.
fn pair_rest(api: &mut dyn Api) {
    api.set_word(B_NEXT, 0);
    api.set_word(B_PLAYING, 0);
    api.obj_anim_id(OBJECT_11, 0);
    api.obj_anim_id(OBJECT_203, 0);
}

/// The hook, `12AC:040C`: object 128 and the pair 11/203 follow the
/// dialogue, and six script events. Codes 0 to 9 through the table at
/// `0x1308E`; codes 1 to 4 and above 9 do nothing (`0x13080`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x12EE4: bit 1 of the speakers is object 128, bit 2 the pair.
            api.set_word(A_SPEAKS, arg & 2); // 0x12EE9
            api.set_word(B_SPEAKS, arg & 4); // 0x12EF1
            api.set_word(A_NEXT, 0); // 0x12EF6
            api.set_word(A_PLAYING, 0); // 0x12EF9
            api.set_word(B_NEXT, 0); // 0x12EFE
            api.set_word(B_PLAYING, 0); // 0x12F01
        }
        5 | 6 => {
            // 0x12F07: neither speaker word is cleared here.
            if api.word(A_SPEAKS) != 0 {
                api.set_word(A_NEXT, 0); // 0x12F10
                api.set_word(A_PLAYING, 0); // 0x12F13
                api.obj_anim_id(OBJECT_128, 0); // 0x12F1A
            }
            // 0x12F22
            if api.word(B_SPEAKS) != 0 {
                pair_rest(api); // 0x12F2E to 0x12F44
            }
        }
        8 => {
            // 0x12F4F
            if api.word(A_SPEAKS) != 0 {
                api.set_word(A_NEXT, arg.wrapping_add(1)); // 0x12F59
            }
            // 0x12F5C
            if api.word(B_SPEAKS) != 0 {
                api.set_word(B_NEXT, arg.wrapping_add(1)); // 0x12F69
            }
        }
        9 => {
            // 0x12F6F: object 128 takes its next animation.
            let next = api.word(A_NEXT);
            if next != api.word(A_PLAYING) {
                api.set_word(A_PLAYING, next); // 0x12F78
                api.obj_anim_id(OBJECT_128, num::signed(next)); // 0x12F7F
            }
            // 0x12F87: the pair, when its two words differ, has both
            // cleared and goes to animation 0 — not to the next one.
            if api.word(B_NEXT) != api.word(B_PLAYING) {
                pair_rest(api); // 0x12F95 to 0x12FAB
            }
        }
        7 => match arg {
            // 0x12FB6: events 10 to 15 through the table at 0x13082.
            0x0A => api.set_word(A_NEXT, 5), // 0x12FCA
            0x0B => {
                api.set_word(A_NEXT, 4); // 0x12FD3
                // 0x12FD9
                api.input_unlock();
            }
            0x0C => {
                api.set_byte(STATE, 3); // 0x12FE1
                api.play_sample(0x16); // 0x12FE8
                // 0x12FF5
                api.node_open(api.ds(WALKER), 0x03E8);
                api.task_arm(0); // 0x12FFF
                api.set_word(A_NEXT, 6); // 0x13005
            }
            0x0D => {
                api.play_sample(7); // 0x1300F
                let rec = api.obj_show(OBJECT_11); // 0x13019
                api.obj_anim(rec, 4); // 0x13021
                let rec = api.obj_show(OBJECT_203); // 0x1302E
                api.obj_anim(rec, 4); // 0x13036
            }
            0x0E => {
                api.task_arm(0); // 0x13042
                // 0x13048
                api.input_lock();
                api.set_word(TARGET_NODE, 0x0BB8); // 0x13070
                let rec = api.ds(WALKER);
                api.walker_walk_to(rec, 0x0BB8, 0); // 0x13078
            }
            0x0F => {
                api.task_arm(0); // 0x13056
                // 0x1305C: `cmp [1EE2h], 0 ; je`.
                if api.word(WORD_1EE2) != 0 {
                    api.set_word(TARGET_NODE, 0x0BB9); // 0x13063
                } else {
                    api.set_word(TARGET_NODE, 0x0BB9); // 0x13070
                    let rec = api.ds(WALKER);
                    api.walker_walk_to(rec, 0x0BB9, 0); // 0x13078
                }
            }
            _ => {}
        },
        _ => {}
    }
}

/// The animation hook, `12AC:02B9`: codes 64h to 6Ah through the table
/// at `0x12EBE`; anything else does nothing (`0x12EBC`).
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        0x64 => {
            // 0x12D91
            let rec = api.obj_show(OBJECT_201); // 0x12D96
            api.obj_anim(rec, 0); // 0x12D9E
            api.play_sample(0x0A); // 0x12DA8
            api.play_sample(0x0A); // 0x12DB0
            api.play_sample(7); // 0x12DB8
        }
        0x65 => {
            // 0x12DC1
            api.obj_anim_id(OBJECT_100, 2); // 0x12DC5
            api.play_sample(4); // 0x12DCF
            api.play_sample(5); // 0x12DD7
            api.play_sample(6); // 0x12DDF
        }
        0x66 => {
            // 0x12DE8
            api.obj_hide(OBJECT_128); // 0x12DEB
            api.set_word(A_SPEAKS, 0); // 0x12DF3
            api.set_word(A_PLAYING, 0); // 0x12DF6
            api.set_word(A_NEXT, 0); // 0x12DF9
            api.play_sample(7); // 0x12DFE
            api.play_sample(9); // 0x12E06
            api.play_sample(8); // 0x12E0E
            api.task_start(0); // 0x12E16
        }
        0x67 => {
            // 0x12E5A: `cmp [1EE4h], BB9h ; jne`.
            if api.word(TARGET_NODE) == 0x0BB9 {
                let rec = api.ds(WALKER);
                api.walker_walk_to(rec, 0x0BB9, 0); // 0x12E6B
            }
            api.play_sample(7); // 0x12E75
            api.obj_hide(OBJECT_11); // 0x12E7D
            api.set_word(B_SPEAKS, 0); // 0x12E85
            api.set_word(B_PLAYING, 0); // 0x12E88
            api.set_word(B_NEXT, 0); // 0x12E8B
            api.set_word(WORD_1EE2, 0); // 0x12EB6
        }
        0x68 => {
            // 0x12E90
            api.play_sample(7); // 0x12E92
            api.obj_hide(OBJECT_203); // 0x12E9B
            api.inv_give(0x0B); // 0x12EA3
            let s = api.byte(STATE).wrapping_add(2); // 0x12EA9
            api.set_byte(STATE, s); // 0x12EAE
            // 0x12EB1
            api.input_unlock();
            api.set_word(WORD_1EE2, 0); // 0x12EB6
        }
        0x69 => {
            // 0x12E1F
            api.obj_hide(OBJECT_201); // 0x12E22
            api.obj_show(OBJECT_202); // 0x12E2B
            api.play_sample(7); // 0x12E33
            api.play_sample(8); // 0x12E3B
            // 0x12E41
            api.input_unlock();
        }
        0x6A => {
            // 0x12E48
            api.play_sample(7); // 0x12E4A
            api.play_sample(8); // 0x12E52
        }
        _ => {}
    }
}

/// The Saldo's rule handlers by their offset in segment `12ACh`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 12AC:000C, verb 7 on object 100: scene 2 in states 2, 3 and 5.
        0x000C => match api.byte(STATE) {
            2 | 3 | 5 => {
                api.scene_play(2); // 0x12AE5
                api.set_word(WORD_1E88, 0); // 0x12AEB
                api.set_word(WORD_1E86, 0); // 0x12AF1
                0x0A // 0x12AFD
            }
            _ => 0, // 0x12AF9
        },
        // 12AC:0042, verb 7 on object 101: states 2 to 6 through the
        // table at 0x12B3A — 4 and 6 answer 0, every other state (the
        // `ja` ones too) plays scene 3.
        0x0042 => match api.byte(STATE) {
            4 | 6 => 0, // 0x12B1D
            _ => {
                api.scene_play(3); // 0x12B23
                api.set_word(WORD_1E88, 0); // 0x12B29
                api.set_word(WORD_1E86, 0); // 0x12B2F
                0x0A // 0x12B35
            }
        },
        // 12AC:0084, verb 4 on object 200: states 2 to 6 through the
        // table at 0x12B92.
        0x0084 => {
            match api.byte(STATE) {
                2 => {
                    api.scene_play(4); // 0x12B5D
                }
                3 | 5 => {
                    // 0x12B61
                    let s = api.byte(STATE).wrapping_add(1);
                    api.set_byte(STATE, s);
                    api.obj_anim_id(OBJECT_100, 1); // 0x12B69
                    api.obj_anim_id(OBJECT_101, 1); // 0x12B75
                    api.scene_play(5); // 0x12B7D
                }
                4 | 6 => {
                    api.scene_play(6); // 0x12B81
                }
                _ => {
                    api.scene_play(7); // 0x12B85
                }
            }
            0x0A // 0x12B8D
        }
        // 12AC:00DC, verbs 9 and 5 with mask 20h (rows 3 and 4).
        0x00DC => {
            api.set_byte(STATE, 3); // 0x12B9F
            // 0x12BA4
            api.input_lock();
            api.scene_play(8); // 0x12BAB
            0x0A // 0x12BB1
        }
        // 12AC:00F6, verbs 2 and 0Ch on object 11: scene 9 unless item 11
        // is carried.
        0x00F6 => {
            if api.inv_has(0x0B) != 0 {
                0 // 0x12BC5
            } else {
                api.scene_play(9); // 0x12BCB
                0x0A // 0x12BD1
            }
        }
        // 12AC:0116, the row with mask 80h.
        0x0116 => {
            api.scene_play(4); // 0x12BDB
            0x0A // 0x12BE1
        }
        // 12AC:0126, verb 2 on object 128.
        0x0126 => {
            api.scene_play(0x0A); // 0x12BEB
            0x0A // 0x12BF1
        }
        // 12AC:0136, verbs 5 and 9 with mask 20h (rows 5 and 6).
        0x0136 => {
            // 0x12BF9: `cmp [1E60h], 80h ; jne`.
            if api.word(WORD_1E60) == 0x80 {
                api.scene_play(0x0B); // 0x12C03
                0x0A // 0x12C09
            } else {
                0 // 0x12C0E
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
