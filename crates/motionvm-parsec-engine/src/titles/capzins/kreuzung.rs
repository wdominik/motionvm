//! Location 3, the Kreuzung (`Kreuzung` in the game's location table).
//!
//! Re-expressed from the handler at `119D:002B` (image `0x119FB`), its
//! hook at `119D:0173` (`0x11B43`), its animation hook at `119D:00F1`
//! (`0x11AC1`) and its two rule handlers at `119D:000B` and `119D:001B`,
//! against [`Api`] alone. Addresses are those of *Captain Zins*'
//! `XP01.EXE`, `DGROUP` `1FC4h`.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 129, the one the dialogue animates.
const OBJECT_129: u16 = 0x81;
/// Object 202, woken by script event 10.
const OBJECT_202: u16 = 0xCA;

/// `[1CEAh]`: at 2 the entry plays scene 1 and steps it to 3.
const BYTE_1CEA: u16 = 0x1CEA;
/// `[1CECh]`: at 2 the entry plays scene 4 and leaves through exit 101.
const BYTE_1CEC: u16 = 0x1CEC;
/// `[1CEFh]`: the Telefon's state; at 6 object 129 is here, and
/// animation code 103 steps it to 7.
const TELEFON_STATE: u16 = 0x1CEF;
/// `[11DEh]`: byte 2 of the record at `DS:11DCh` (the Schranke hands that
/// record to the walker routines); the logic sets and clears its bit 0.
const RECORD_11DC_FLAGS: u16 = 0x11DE;
/// `[1EA4h]`: whether object 129 speaks in the dialogue that is up.
const IT_SPEAKS: u16 = 0x1EA4;
/// `[1EA0h]`: the animation it is to play at the next page's end.
const NEXT_ANIM: u16 = 0x1EA0;
/// `[1EA2h]`: the animation it is playing; 3 and above (signed) hold it.
const PLAYING_ANIM: u16 = 0x1EA2;

/// Everything before the frame loop, `0x119FB` to `0x11A87`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x119FB
    let entry = api.word(ENTRY); // 0x11A00
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x11A07
    api.visit_enter(loc, entry); // 0x11A0B
    if api.byte(BYTE_1CEA) == 2 {
        // 0x11A1A
        api.slot_set_raw(2, 0x10, 0xA0, 0); // 0x11A23
        api.set_byte(BYTE_1CEA, 3); // 0x11A2B
        let flags = api.byte(RECORD_11DC_FLAGS) & 0xFE;
        api.set_byte(RECORD_11DC_FLAGS, flags); // 0x11A30
        api.scene_play(1); // 0x11A37
    } else if api.byte(TELEFON_STATE) == 6 {
        // 0x11A45
        let rec = api.obj_show(OBJECT_129); // 0x11A4A
        api.obj_anim(rec, 0); // 0x11A52
    } else if api.byte(BYTE_1CEC) == 2 {
        // 0x11A63
        api.input_lock(); // 0x11A63
        api.hover_off(); // 0x11A68
        let flags = api.byte(RECORD_11DC_FLAGS) & 0xFE;
        api.set_byte(RECORD_11DC_FLAGS, flags); // 0x11A6D
        api.scene_play(4); // 0x11A74
        api.script_exit(0x65); // 0x11A7C
    }
    api.visit_finish_entry(); // 0x11A82
    api.set_word(ENTRY, 0); // 0x11A87
}

/// One turn of the frame loop, `0x11A8D` to `0x11AA1`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x11A8D
    api.dispatch(); // 0x11A92
    api.frame_pass(); // 0x11A97
    api.word(ENTRY) == 0 // 0x11A9C
}

/// Everything after the frame loop, `0x11AA3` to `0x11AC0`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5)); // 0x11AA6
    api.objs_save_state(out); // 0x11AAE
    api.visit_leave(); // 0x11AB6
    api.slot_unload(); // 0x11ABB
}

/// The hook, `119D:0173`: object 129's animations follow the dialogue,
/// and three script events. Codes 0 to 9 through the table at `0x11C5C`
/// (0 `018Bh`; 1 to 4 `028Ah`, the return; 5 and 6 `019Eh`; 7 `01FDh`;
/// 8 `01C2h`; 9 `01D5h`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x11B5B: bit 1 of the speakers is object 129's.
            api.set_word(IT_SPEAKS, arg & 2); // 0x11B60
            api.set_word(NEXT_ANIM, 0); // 0x11B65
            api.set_word(PLAYING_ANIM, 0); // 0x11B68
        }
        5 | 6 => {
            // 0x11B6E
            if api.word(IT_SPEAKS) != 0 {
                api.set_word(NEXT_ANIM, 0); // 0x11B7A
                api.set_word(PLAYING_ANIM, 0); // 0x11B7D
                api.set_word(IT_SPEAKS, 0); // 0x11B80
                api.obj_anim_id(OBJECT_129, 0); // 0x11B87
            }
        }
        8 => {
            // 0x11B92
            if api.word(IT_SPEAKS) != 0 {
                api.set_word(NEXT_ANIM, arg.wrapping_add(1)); // 0x11B9F
            }
        }
        9 => {
            // 0x11BA5: `cmp [1EA2h], 3 ; jl` — signed.
            let next = api.word(NEXT_ANIM);
            if next != api.word(PLAYING_ANIM) && num::signed(api.word(PLAYING_ANIM)) < 3 {
                api.set_word(PLAYING_ANIM, next); // 0x11BBB
                api.obj_anim_id(OBJECT_129, num::signed(next)); // 0x11BC2
            }
        }
        7 => match arg {
            // 0x11BCD: a compare chain, events 10 to 12.
            0x0A => {
                api.task_arm(0); // 0x11BE2
                let rec = api.obj_show(OBJECT_202); // 0x11BED
                api.obj_anim(rec, 0); // 0x11BF5
                api.play_sample(0x15); // 0x11BFD, 0x11C2D
            }
            0x0B => {
                api.inv_take(6); // 0x11C03
                api.set_word(PLAYING_ANIM, 3); // 0x11C0C
                api.obj_anim_id(OBJECT_129, 3); // 0x11C13
                api.play_sample(7); // 0x11C1D
                api.play_sample(8); // 0x11C25
                api.play_sample(3); // 0x11C2D
            }
            0x0C => {
                api.obj_anim_id(OBJECT_129, 4); // 0x11C3A
                api.play_sample(0x16); // 0x11C44
                api.input_lock(); // 0x11C4A
                api.set_word(NEXT_ANIM, 0); // 0x11C51
                api.set_word(PLAYING_ANIM, 0); // 0x11C54
                api.set_word(IT_SPEAKS, 0); // 0x11C57
            }
            _ => {}
        },
        _ => {} // 1 to 4 and above 9: 0x11C5A
    }
}

/// The animation hook, `119D:00F1`: codes 100 to 103 through the table
/// at `0x11B3B` (`0106h`, `0115h`, `0126h`, `0156h`).
pub(crate) fn anim_hook(api: &mut dyn Api, rec: Far, code: u16) {
    match code {
        100 => {
            // 0x11AD6
            let flags = api.byte(RECORD_11DC_FLAGS) | 1;
            api.set_byte(RECORD_11DC_FLAGS, flags);
            api.face(3); // 0x11ADD
        }
        101 => {
            // 0x11AE5: bit 0 of the record's byte 2 cleared, task 0 on.
            let at = Far {
                off: rec.off.wrapping_add(2),
                seg: rec.seg,
            };
            let b = api.far_byte(at) & 0xFE;
            api.set_far_byte(at, b); // 0x11AE8
            api.task_start(0); // 0x11AEF
        }
        102 => {
            api.inv_give(6); // 0x11AF8
            api.inv_give(8); // 0x11B00
            api.play_sample(7); // 0x11B08
            api.play_sample(9); // 0x11B10
            api.page_release(1); // 0x11B18
            api.set_word(PLAYING_ANIM, 0xFFFF); // 0x11B1E
        }
        103 => {
            api.set_byte(TELEFON_STATE, 7); // 0x11B26
            api.obj_hide(OBJECT_129); // 0x11B2E
            api.input_unlock(); // 0x11B34
        }
        _ => {} // 0x11B39
    }
}

/// The Kreuzung's rule handlers by their offset in segment `119Dh`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 119D:000B (the rule with word +2 = 2, mask 4, word +6 = 81h):
        // scene 3.
        0x000B => {
            api.scene_play(3); // 0x119E0
            0x0A
        }
        // 119D:001B (the rules with word +2 = 5 and 9, mask 20h, word +6 = 6,
        // word +8 = 81h): scene 2.
        0x001B => {
            api.scene_play(2); // 0x119F0
            0x0A
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
