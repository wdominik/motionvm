//! Location 16, `Wucher`, as the game names it in its location table
//! (record at image `0x1EE70`).
//!
//! Re-expressed from the handler at `13BC:00B1` (image `0x13C71`), its hook
//! at `13BC:02A3` (`0x13E63`), its animation hook at `13BC:01BE`
//! (`0x13D7E`) and its four rule handlers at `13BC:0003`, `13BC:002B`,
//! `13BC:003B` and `13BC:00A1` (seven rules at `1F73:04A0`), against
//! [`Api`] alone. Addresses in comments are offsets in segment `13BCh`.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 200.
const OBJECT_C8: u16 = 0xC8;
/// Object 201.
const OBJECT_C9: u16 = 0xC9;
/// Object 202.
const OBJECT_CA: u16 = 0xCA;
/// Object 6.
const OBJECT_06: u16 = 6;

/// `DS:11DCh`: the walker record (DS1 `DS:1693h`, by `[1204h]` = DS1
/// `[16BBh]`).
const WALKER: u16 = 0x11DC;
/// `[1204h]`: the walker record's word `+28h` (DS1 `[16BBh]`).
const WALKER_28: u16 = 0x1204;
/// `[1CF7h]`: this location's state byte.
const STATE: u16 = 0x1CF7;
/// `[1CECh]`: a state byte; at 2 the handler leaves through script exit
/// 65h.
const STATE_1CEC: u16 = 0x1CEC;
/// `[1E6Ah]`: cleared on the first visit (DS1 `[289Fh]`).
const WORD_1E6A: u16 = 0x1E6A;
/// `[1EF8h]`: non-zero while a spoken page drives object 6.
const TALK_06: u16 = 0x1EF8;
/// `[1EF4h]`: the animation object 6 is asked to take.
const WANT_06: u16 = 0x1EF4;
/// `[1EF6h]`: the animation object 6 was last given.
const ANIM_06: u16 = 0x1EF6;
/// `[1EFEh]`: non-zero while a spoken page drives objects 200 and 201.
const TALK_C8: u16 = 0x1EFE;
/// `[1EFAh]`: the animation objects 200 and 201 are asked to take.
const WANT_C8: u16 = 0x1EFA;
/// `[1EFCh]`: the animation objects 200 and 201 were last given.
const ANIM_C8: u16 = 0x1EFC;
/// `[1F00h]`: the upper bound of the animation hook's sample draw.
const DRAW_BOUND: u16 = 0x1F00;
/// `[1F02h]`: the node the frame loop waits for, or 0.
const WAIT_NODE: u16 = 0x1F02;
/// `[1F04h]`: a counter script event 0Eh steps.
const STEP: u16 = 0x1F04;
/// `[1F06h]`: the threshold the animation hook holds the record's word
/// `+4` against.
const THRESHOLD: u16 = 0x1F06;

/// Everything before the frame loop, `00B1` to `016B`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 00B1
    let entry = api.word(ENTRY); // 00B6
    let loc = api.word(LOCATION); // 00BA
    api.set_word(HERE, loc); // 00BD
    api.visit_enter(loc, entry); // 00C1

    // 00C9: by `[1CF7h]`: 0 falls into the arm of 1; anything else goes
    // to 0148.
    let state = api.byte(STATE);
    if state == 0 {
        api.scene_play(1); // 00D9
        api.walk_to_node(6); // 00E1
        api.set_word(WORD_1E6A, 0); // 00E9
    }
    if state <= 1 {
        let rec = api.obj_find(OBJECT_C8); // 00EF
        api.slot_set_object(2, 0x40, rec); // 00F8
        let rec = api.obj_find(OBJECT_C9); // 0106
        api.slot_set_object(3, 0x40, rec); // 010F

        api.node_block(api.ds(WALKER), 0x03E8);
        api.obj_anim_id(OBJECT_C8, 0); // 012C
        api.obj_anim_id(OBJECT_C9, 0); // 0139
    } else if api.byte(STATE_1CEC) == 2 {
        // 0148
        api.input_lock();
        api.hover_off();
        api.script_exit(0x65); // 0159
    }
    api.visit_finish_entry(); // 0161
    api.set_word(ENTRY, 0); // 0166
}

/// One turn of the frame loop, `016C` to `019E`. Answers whether the loop
/// goes on.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 016C
    api.dispatch(); // 0171
    // 0176: once `[1204h]` equals the awaited node, task 0 starts.
    let node = api.word(WAIT_NODE);
    if node != 0 && api.word(WALKER_28) == node {
        api.set_word(WAIT_NODE, 0); // 0186
        api.task_start(0); // 018C
    }
    api.frame_pass(); // 0194
    api.word(ENTRY) == 0 // 0199
}

/// Everything after the frame loop, `01A0` to `01BD`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE); // 01A0
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 01AB
    api.visit_leave(); // 01B3
    api.slot_unload(); // 01B8
}

/// `random(26h, 2Ch)` played as a sample, as at `01CC`, `025F`, `044A`
/// and `0463`.
fn sample_26_2c(api: &mut dyn Api) {
    let k = api.random(0x26, 0x2C);
    api.play_sample(k);
}

/// The hook, `13BC:02A3`: `code` at `[bp+6]`, `arg` at `[bp+8]`. Ten arms
/// through the table at `cs:0492` (0: `02BB`, 5 and 6: `02DE`, 7: `0369`,
/// 8: `0303`, 9: `031F`, the rest and anything above 9: the return at
/// `0484`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 02BB: both flags from the page's bits, all four counters
        // cleared.
        0 => {
            api.set_word(TALK_06, arg & 2);
            api.set_word(TALK_C8, arg & 0x0C); // 02C3
            api.set_word(ANIM_06, 0); // 02CB
            api.set_word(WANT_06, 0);
            api.set_word(ANIM_C8, 0); // 02D3
            api.set_word(WANT_C8, 0);
        }
        // 02DE: the pair's flag cleared, and both back on animation 0.
        5 | 6 => {
            if api.word(TALK_C8) != 0 {
                api.set_word(ANIM_C8, 0); // 02EA
                api.set_word(WANT_C8, 0);
                api.set_word(TALK_C8, 0);
                api.obj_anim_id(OBJECT_C8, 0); // 02F3
                api.obj_anim_id(OBJECT_C9, 0); // 02FF -> 035B
            }
        }
        // 0303: whoever speaks is asked for `arg + 1`.
        8 => {
            if api.word(TALK_06) != 0 {
                api.set_word(WANT_06, arg.wrapping_add(1)); // 030A
            }
            if api.word(TALK_C8) != 0 {
                api.set_word(WANT_C8, arg.wrapping_add(1)); // 031A -> 02D8
            }
        }
        // 031F: the objects follow what they were asked for.
        9 => {
            let want = api.word(WANT_06);
            if api.word(ANIM_06) != want {
                api.set_word(ANIM_06, want); // 0328
                api.obj_anim_id(OBJECT_06, num::signed(want)); // 032E
            }
            let want = api.word(WANT_C8); // 0339
            if api.word(ANIM_C8) != want {
                api.set_word(ANIM_C8, want); // 0345
                api.obj_anim_id(OBJECT_C8, num::signed(want)); // 034B
                api.obj_anim_id(OBJECT_C9, num::signed(want)); // 0357
            }
        }
        // 0369: script events, `arg - 0Ah` as an unsigned word, six arms
        // through the table at `cs:0486`.
        7 => match arg.wrapping_sub(0x0A) {
            0 => {
                // 037D
                if api.word(WALKER_28) != 6 {
                    api.set_word(WAIT_NODE, 6); // 0387
                    api.task_arm(0); // 038D -> 047C
                }
            }
            1 => {
                api.set_byte(STATE, 1); // 0390
                api.play_sample(7); // 0395
                api.play_sample(8); // 039D
                api.inv_give(6); // 03A5
                api.obj_hide(OBJECT_06); // 03AD
                api.set_word(ANIM_06, 0); // 03B5
                api.set_word(WANT_06, 0);
                api.set_word(TALK_06, 0);
            }
            2 => {
                api.walk_to_node(0x0BB8); // 03C3
            }
            3 => {
                api.inv_take(0x0B); // 03CE
                api.set_word(DRAW_BOUND, 2); // 03D6
                let rec = api.obj_show(OBJECT_CA); // 03DC
                api.obj_anim(rec, 0); // 03E7
                api.play_sample(7); // 03F1
                api.play_sample(8); // 03F9
                api.obj_hide(OBJECT_C8); // 0401
                api.obj_hide(OBJECT_C9); // 040A
                api.set_word(THRESHOLD, 0); // 0413
                api.set_word(ANIM_C8, 0); // 0419
                api.set_word(WANT_C8, 0);
                api.set_word(TALK_C8, 0);
                api.set_word(STEP, 0);
                let walker = api.ds(WALKER);
                api.walker_walk_to(walker, 0xC8, 1); // 0427
                api.task_arm(0); // 0438 -> 047C
            }
            4 => {
                let step = api.word(STEP).wrapping_add(1); // 043A
                api.set_word(STEP, step);
                // 043E: `imul ax, 1Eh ; add ax, 8Ch`.
                api.set_word(THRESHOLD, step.wrapping_mul(0x1E).wrapping_add(0x8C));
                sample_26_2c(api); // 044A
                let bound = api.word(DRAW_BOUND).wrapping_add(1); // 045D
                api.set_word(DRAW_BOUND, bound);
                api.task_arm(0); // 0461 -> 047C
            }
            5 => {
                sample_26_2c(api); // 0463
                api.set_word(THRESHOLD, 0x12C); // 0476
                api.task_arm(0); // 047C
            }
            _ => {} // 0373
        },
        _ => {} // 0484
    }
}

/// The animation hook, `13BC:01BE`: only code 64h does anything.
pub(crate) fn anim_hook(api: &mut dyn Api, rec: Far, code: u16) {
    if code != 0x64 {
        return; // 01C9
    }
    sample_26_2c(api); // 01CC
    // 01DF: `random(0, [1F00h])`, four arms through the table at
    // `cs:029B`; above 3 no second sample (`ja 021C`).
    let bound = api.word(DRAW_BOUND);
    let range = match api.random(0, bound) {
        0 | 3 => Some((4, 0x0A)), // 01FB jumps to 0209
        1 => Some((0x1A, 0x1D)),  // 01FD
        2 => Some((0x26, 0x2C)),  // 0203
        _ => None,
    };
    if let Some((lo, hi)) = range {
        let k = api.random(lo, hi); // 020D
        api.play_sample(k); // 0216
    }
    // 021C: slots 4, 5 and 6 on the hook's record, code 30h.
    api.slot_set_object(4, 0x30, rec);
    api.slot_set_object(5, 0x30, rec); // 022E
    api.slot_set_object(6, 0x30, rec); // 0240

    // 0252: the record's word `+4` above the threshold, unsigned (`jbe`).
    let at = Far {
        off: rec.off.wrapping_add(4),
        seg: rec.seg,
    };
    if api.far_word(at) > api.word(THRESHOLD) {
        sample_26_2c(api); // 025F
        if api.word(THRESHOLD) == 0x12C {
            api.play_sample(3); // 027A
            api.obj_hide(OBJECT_CA); // 0282
        }
        api.task_start(0); // 028B
        api.set_word(THRESHOLD, 0x1000); // 0293
    }
}

/// The rule handlers by their offset in segment `13BCh`. Answers what the
/// original answers the rule search with, or `None` for an offset that is
/// not one of them.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 13BC:0003 (verb 2, mask 4, object 200): scene `random(2, 5)`,
        // and above 3 (signed, `jle`) two more.
        0x0003 => {
            let mut n = num::signed(api.random(2, 5)); // 0009
            if n > 3 {
                n = n.wrapping_add(2); // 001C
            }
            api.scene_play(n); // 001F
            0x0A
        }
        // 13BC:002B (mask 80h): scene 4.
        0x002B => {
            api.scene_play(4); // 002E
            0x0A
        }
        // 13BC:003B (verb 4 on item 11; verbs 5 and 9 with item 11 on
        // object 200): the state moves to 2, object 202's record goes to
        // slots 4, 5 and 6 with code 30h, scene 5.
        0x003B => {
            api.set_byte(STATE, 2); // 003F
            let rec = api.obj_find(OBJECT_CA); // 0044
            api.slot_set_object(4, 0x30, rec); // 0053
            api.slot_set_object(5, 0x30, rec); // 0061
            api.slot_set_object(6, 0x30, rec); // 0073

            api.node_open(api.ds(WALKER), 0x03E8);
            api.scene_play(5); // 0094
            0x0A
        }
        // 13BC:00A1 (verbs 9 and 5 with item 6 on object 200): scene 0.
        0x00A1 => {
            api.scene_play(0); // 00A4
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
