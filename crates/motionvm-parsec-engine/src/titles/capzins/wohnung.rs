//! Location 1, the Wohnung, where a fresh game opens. The module is named
//! after the location as the game itself names it in its location table
//! (`Wohnung`, the record's `+1Fh` name).
//!
//! Re-expressed from the handler at `112E:012A` (image `0x1140A`), its
//! hook at `112E:01DF` (`0x114BF`) and its six rule handlers at
//! `112E:0008`, `003B`, `00AD`, `00CF`, `00DE` and `00F5`, against [`Api`]
//! alone. Every branch carries the address it
//! was read from.
//!
//! The original handler owns its frame loop; here it is cut into
//! [`enter`] (everything before the loop), [`frame`] (one turn of it) and
//! [`leave`] (everything after), as Dunkle Schatten's locations are.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT, OBJECT_STATES, WALKER, WALKER_28};
use crate::titles::parsec1994::{Api, Far};

/// Object 201.
const OBJECT_201: u16 = 0xC9;
/// Object 202, whose record's `+2` bit 0 the second rule toggles.
const OBJECT_202: u16 = 0xCA;
/// Object 203.
const OBJECT_203: u16 = 0xCB;
/// Node 204, where the hook's event 11 walks to and event 12 waits for.
const NODE_204: u16 = 0xCC;

/// `[1CEAh]`: the opening has run.
const OPENED: u16 = 0x1CEA;
/// `[1CEBh]`: object 201's first scene has run.
const SEEN_201: u16 = 0x1CEB;
/// `[1E92h]`: the value of the walker's word `+28h` the frame loop waits
/// for, or 0.
const WAIT_FOR: u16 = 0x1E92;
/// `[1E90h]`: cleared by the hook's code 0 and its event 13.
const WORD_1E90: u16 = 0x1E90;

/// Everything before the frame loop, `0x1140A` to `0x11450`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x1140A
    let entry = api.word(ENTRY); // 0x1140F
    let loc = api.word(LOCATION); // 0x11413
    api.set_word(HERE, loc); // 0x11416
    api.visit_enter(loc, entry); // 0x1141A
    api.set_word(WAIT_FOR, 0); // 0x11422
    // 0x11428: the opening, once.
    if api.byte(OPENED) == 0 {
        api.set_byte(OPENED, 1); // 0x1142F
        api.input_lock(); // 0x11434
        api.hover_off(); // 0x11439
        api.scene_play(1); // 0x11440
    }
    api.visit_finish_entry(); // 0x11446
    api.set_word(ENTRY, 0); // 0x1144B
}

/// One turn of the frame loop, `0x11451` to `0x1149F`. Answers whether
/// the loop goes on.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x11451
    api.dispatch(); // 0x11456
    // 0x1145B: once the walker's word +28h is the value waited for.
    let wait = api.word(WAIT_FOR);
    if wait != 0 && api.word(WALKER_28) == wait {
        api.face(0); // 0x1146D
        let walker = api.ds(WALKER);
        api.figure_anim(walker, 3); // 0x11479
        api.set_word(WAIT_FOR, 0); // 0x11481
        api.set_word(WORD_1E90, 0); // 0x11487
        api.task_start(0); // 0x1148F
    }
    api.frame_pass(); // 0x11495
    api.word(ENTRY) == 0 // 0x1149A
}

/// Everything after the frame loop, `0x114A1` to `0x114BE`.
pub(crate) fn leave(api: &mut dyn Api) {
    // 0x114A1: this location's object states kept in its 32 bytes.
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x114AC
    api.visit_leave(); // 0x114B4
    api.slot_unload(); // 0x114B9
}

/// The hook, `112E:01DF`: what the scene machine hands the location.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0x114D0
        0 => api.set_word(WORD_1E90, 0),
        // 0x114D8: script events 10 to 20 through the table at 0x1154E;
        // 15 to 19 and anything outside do nothing.
        7 => match arg {
            10 => api.play_sample(0x0E), // 0x114EA
            11 => {
                api.walk_to_node(NODE_204); // 0x114F3
            }
            12 => {
                // 0x114FD
                api.set_word(WAIT_FOR, NODE_204);
                api.play_sample(4);
                api.play_sample(6);
                api.task_arm(0); // 0x11515
            }
            13 => {
                // 0x1151C
                api.set_word(WORD_1E90, 0);
                api.play_sample(0x0A);
                api.face(1); // 0x1152C
            }
            14 => api.script_exit(0x64), // 0x11533
            20 => {
                // 0x1153C
                api.play_sample(8);
                api.inv_give(4); // 0x11546
            }
            _ => {}
        },
        _ => {}
    }
}

/// Byte `+2` of an object record.
fn flags_of(rec: Far) -> Far {
    Far {
        off: rec.off.wrapping_add(2),
        seg: rec.seg,
    }
}

/// The Wohnung's rule handlers by their offset in segment `112Eh`.
/// Answers what the original answers the rule search with, or `None` for
/// an offset that is not one of them.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 112E:0008, object 201: only while it is live (0x112EE); scene 2
        // the first time, 3 after.
        0x0008 => {
            if api.obj_state(OBJECT_201) == 0 {
                0 // 0x11312
            } else {
                if api.byte(SEEN_201) != 0 {
                    api.scene_play(3); // 0x1130E
                } else {
                    api.set_byte(SEEN_201, 1); // 0x112FF
                    api.scene_play(2);
                }
                0x0A
            }
        }
        // 112E:003B, object 202: samples 5 and 6, then by bit 0 of the
        // record's byte +2 (0x11341).
        0x003B => {
            let rec = api.obj_find(OBJECT_202); // 0x11322
            api.play_sample(5);
            api.play_sample(6);
            let flags = api.far_byte(flags_of(rec));
            if flags & 1 != 0 {
                api.play_sample(8); // 0x1134A
                api.obj_invalidate(rec); // 0x11356
                let flags = api.far_byte(flags_of(rec));
                api.set_far_byte(flags_of(rec), flags & 0xFE); // 0x11361
                api.scene_play(6); // 0x11366
            } else {
                api.set_far_byte(flags_of(rec), flags | 1); // 0x1136D
                api.obj_anim(rec, 0); // 0x11378
                api.scene_play(5); // 0x11380
            }
            0x0A
        }
        // 112E:00AD: scene 4 while the object the pointer chose is live.
        0x00AD => {
            let chosen = api.word(OBJECT); // 0x11390
            if api.obj_state(chosen) == 0 {
                0 // 0x113AB
            } else {
                api.scene_play(4); // 0x113A0
                0x0A
            }
        }
        // 112E:00CF: sample 31, and the search goes on.
        0x00CF => {
            api.play_sample(0x1F); // 0x113B4
            0
        }
        // 112E:00DE: samples 10 and 8, and the search goes on.
        0x00DE => {
            api.play_sample(0x0A); // 0x113C3
            api.play_sample(8); // 0x113CB
            0
        }
        // 112E:00F5, exit 100: scene 0 while object 202's bit 0 is set or
        // object 203 is live; otherwise the search goes on.
        0x00F5 => {
            let rec = api.obj_find(OBJECT_202); // 0x113DB
            if api.far_byte(flags_of(rec)) & 1 != 0 || api.obj_state(OBJECT_203) != 0 {
                api.scene_play(0); // 0x113FB
                0x0A
            } else {
                0 // 0x11406
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
    anim_hook: None,
    rule,
};
