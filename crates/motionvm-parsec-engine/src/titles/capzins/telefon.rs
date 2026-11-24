//! Location 4, the Telefon (`Telefon` in the game's location table).
//!
//! Re-expressed from the handler at `11C7:008F` (image `0x11CFF`), its
//! hook at `11C7:0169` (`0x11DD9`), its animation hook at `11C7:010C`
//! (`0x11D7C`) and its one rule handler at `11C7:0000` (`0x11C70`),
//! against [`Api`] alone. Addresses are those of *Captain Zins*'
//! `XP01.EXE`, `DGROUP` `1FC4h`.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 200, the one the dialogue animates.
const OBJECT_200: u16 = 0xC8;

/// `[1CEFh]`: this location's state, stepped by the rule.
const STATE: u16 = 0x1CEF;
/// `[1CF0h]`, `[1CF1h]` (the PC's state), `[1CF2h]`: states the rule reads.
const BYTE_1CF0: u16 = 0x1CF0;
const PC_STATE: u16 = 0x1CF1;
const BYTE_1CF2: u16 = 0x1CF2;
/// `[1CECh]`: at 2 the handler leaves at once through exit 100.
const BYTE_1CEC: u16 = 0x1CEC;
/// `[1EAEh]`: whether object 200 speaks in the dialogue that is up; the
/// script events 10 and 11 set it to 6 and 1.
const IT_SPEAKS: u16 = 0x1EAE;
/// `[1EACh]`: the animation it is to play at the next page's end.
const NEXT_ANIM: u16 = 0x1EAC;
/// `[1EAAh]`: the animation it is playing.
const PLAYING_ANIM: u16 = 0x1EAA;
/// `[1EA8h]`: set while an event's own animation runs; hook code 9 leaves
/// the object alone until animation code 100 clears it.
const HELD: u16 = 0x1EA8;

/// Everything before the frame loop, `0x11CFF` to `0x11D45`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x11CFF
    let entry = api.word(ENTRY); // 0x11D04
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x11D0B
    api.visit_enter(loc, entry); // 0x11D0F
    // 0x11D17: `cmp [1CECh], 2 ; jne`.
    if api.byte(BYTE_1CEC) == 2 {
        api.input_lock(); // 0x11D1E
        api.hover_off(); // 0x11D23
        api.script_exit(0x64); // 0x11D2A
    }
    api.visit_finish_entry(); // 0x11D30
    api.set_word(ENTRY, 0); // 0x11D35
    api.obj_anim_id(OBJECT_200, 0); // 0x11D40
}

/// One turn of the frame loop, `0x11D48` to `0x11D5C`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x11D48
    api.dispatch(); // 0x11D4D
    api.frame_pass(); // 0x11D52
    api.word(ENTRY) == 0 // 0x11D57
}

/// Everything after the frame loop, `0x11D5E` to `0x11D7B`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5)); // 0x11D61
    api.objs_save_state(out); // 0x11D69
    api.visit_leave(); // 0x11D71
    api.slot_unload(); // 0x11D76
}

/// The hook, `11C7:0169`: object 200's animations follow the dialogue,
/// and four script events. Codes 0 to 9 through the table at `0x11F13`
/// (0 `0182h`; 1 to 4 `0291h`, the return; 5 and 6 `01A5h`; 7 `0203h`;
/// 8 `01C9h`; 9 `01DBh`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x11DF2: `test dx, 2`.
            if arg & 2 != 0 {
                api.set_word(IT_SPEAKS, 1); // 0x11DF8
                api.set_word(PLAYING_ANIM, 0xFFFF); // 0x11DFE
            } else {
                clear(api); // 0x11E07
            }
        }
        5 | 6 => {
            // 0x11E15
            if api.word(IT_SPEAKS) != 0 {
                // 0x11E1F: `cmp [1EAEh], 1 ; jle` — signed.
                let k = if num::signed(api.word(IT_SPEAKS)) > 1 {
                    4
                } else {
                    0
                };
                api.obj_anim_id(OBJECT_200, k); // 0x11E2F
                clear(api); // 0x11E37 -> 0x11E07
            }
        }
        8 => {
            // 0x11E39
            if api.word(IT_SPEAKS) != 0 {
                let speaks = api.word(IT_SPEAKS);
                api.set_word(NEXT_ANIM, speaks.wrapping_add(arg)); // 0x11E43, 0x11EFE
            }
        }
        9 => {
            // 0x11E4B: a new animation, unless an event's own one holds.
            let next = api.word(NEXT_ANIM);
            if next != api.word(PLAYING_ANIM) && api.word(HELD) == 0 {
                api.set_word(PLAYING_ANIM, next); // 0x11E61
                api.obj_anim_id(OBJECT_200, num::signed(next)); // 0x11E68
            }
        }
        7 => match arg {
            // 0x11E73: four values at 0x11F03 (10, 11, 12, 110), their
            // targets behind them (021Fh, 0253h, 0280h, 028Ah).
            0x0A => {
                // 0x11E8F
                api.set_word(PLAYING_ANIM, 3);
                api.obj_anim_id(OBJECT_200, 3); // 0x11E99
                api.play_sample(3); // 0x11EA3
                api.set_word(HELD, 1); // 0x11EA9
                api.set_word(IT_SPEAKS, 6); // 0x11EAF
                let next = api.word(NEXT_ANIM).wrapping_add(5);
                api.set_word(NEXT_ANIM, next); // 0x11EB5
                api.task_arm(0); // 0x11EBC
            }
            0x0B => {
                // 0x11EC3
                api.set_word(PLAYING_ANIM, 4);
                api.obj_anim_id(OBJECT_200, 4); // 0x11ECD
                api.play_sample(0x13); // 0x11ED7
                api.set_word(HELD, 1); // 0x11EDD
                api.set_word(IT_SPEAKS, 1); // 0x11EE3
                let next = api.word(NEXT_ANIM).wrapping_sub(5);
                api.set_word(NEXT_ANIM, next); // 0x11EE9
                api.task_arm(0); // 0x11EBC
            }
            0x0C => {
                api.inv_take(7); // 0x11EF2
            }
            0x6E => {
                // 0x11EFA
                let speaks = api.word(IT_SPEAKS);
                api.set_word(NEXT_ANIM, speaks.wrapping_sub(1)); // 0x11EFE
            }
            _ => {}
        },
        _ => {} // 1 to 4 and above 9: 0x11F01
    }
}

/// `0x11E07`: nobody speaks, both animation words zero.
fn clear(api: &mut dyn Api) {
    api.set_word(IT_SPEAKS, 0); // 0x11E07
    api.set_word(PLAYING_ANIM, 0); // 0x11E0F
    api.set_word(NEXT_ANIM, 0); // 0x11EFE
}

/// The animation hook, `11C7:010C`: codes 100 to 104 through the table
/// at `0x11DCF` (100 `0124h`; 101 to 104 `0148h`).
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        100 => {
            // 0x11D94: the event's animation is through; the dialogue's
            // animation and task 0 go on.
            api.set_word(HELD, 0);
            let next = api.word(NEXT_ANIM);
            api.set_word(PLAYING_ANIM, next); // 0x11D9D
            api.obj_anim_id(OBJECT_200, num::signed(next)); // 0x11DA4
            api.task_start(0); // 0x11DAE
            api.play_sample(4); // 0x11DC6
        }
        101..=104 => {
            api.play_sample(5); // 0x11DBA
            // 0x11DC0: `add ax, FFAAh` — samples 15 to 18.
            api.play_sample(code.wrapping_add(0xFFAA)); // 0x11DC6
        }
        _ => {} // 0x11DCC
    }
}

/// The Telefon's rule handlers by their offset in segment `11C7h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 11C7:0000 (the rule with word +2 = 2, mask 4, word +6 = C8h), by
        // the location's state: seven arms through the table at 0x11CF1,
        // the rest by `[1CF2h]`.
        0x0000 => {
            match api.byte(STATE) {
                0 => {
                    api.set_byte(STATE, 1); // 0x11C86
                    api.scene_play(0); // 0x11C8B
                }
                1 | 2 => {
                    // 0x11C95: `cmp [1CF1h], 2 ; jne`.
                    if api.byte(PC_STATE) == 2 {
                        api.set_byte(STATE, 3); // 0x11C9C
                        api.scene_play(2);
                    } else {
                        api.set_byte(STATE, 2); // 0x11CA5
                        api.scene_play(1);
                    }
                }
                3 => {
                    api.scene_play(3); // 0x11CAE
                }
                4 => {
                    // 0x11CB2: `cmp [1CF0h], 2 ; jb`.
                    if api.byte(BYTE_1CF0) >= 2 {
                        api.set_byte(STATE, 5); // 0x11CB9
                        api.scene_play(4);
                    } else {
                        api.scene_play(5); // 0x11CC2
                    }
                }
                5 => {
                    api.set_byte(STATE, 6); // 0x11CC6
                    api.scene_play(6);
                }
                6 => {
                    api.scene_play(7); // 0x11CCF
                }
                _ => {
                    // 0x11CD3: above 6.
                    if api.byte(BYTE_1CF2) == 1 {
                        api.set_byte(BYTE_1CF2, 2); // 0x11CDF
                        api.scene_play(8);
                    } else {
                        api.scene_play(9); // 0x11CE8
                    }
                }
            }
            0x0A // 0x11CEC
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
