//! Location 21, the Schuppen: the shed. The module is named after the
//! location as the game itself names it in its location table
//! (`Schuppen`).
//!
//! Re-expressed from the handler at `03D2:00F7` (image `0x3E17`) and its
//! four rule handlers at `03D2:0000`, `03D2:0032`, `03D2:005A` and
//! `03D2:00DC`, against [`Api`] alone. The location has no hook of its
//! own.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::titles::parsec1994::Api;

/// Object 10, shown by one rule and taken by another.
const OBJECT_10: u16 = 0x0A;

/// `[2711h]`: the shed's state, also the counter of its scene sequence;
/// 3 once object 10 has been shown.
const SHED_STATE: u16 = 0x2711;

/// Everything before the frame loop, `0x3E17` to `0x3E34`: the standard
/// prologue and nothing of its own.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x3E17
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x3E23
    api.visit_enter(loc, entry); // 0x3E27
    api.visit_finish_entry(); // 0x3E2F
    api.set_word(ENTRY, 0); // 0x3E34
}

/// One turn of the frame loop, `0x3E3A` to `0x3E4E`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x3E50` to `0x3E6D`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The location has no hook (its record's `+2Fh` is null).
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// The shed's rule handlers by their offset in segment `03D2h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 03D2:0000: the next of scenes 1, 1, 2 by [2711h]; without item 9
        // carried, [272Fh] and [2729h] are set.
        0x0000 => {
            let counter = api.ds(SHED_STATE);
            api.scene_sequence(counter, &[1, 1, 2]); // 0x3D2F
            if api.inv_has(9) == 0 {
                // 0x3D43
                api.set_byte(0x272F, 0);
                api.set_byte(0x2729, 8);
            }
            0x0A
        }
        // 03D2:0032: object 10 shown with scene 3, once.
        0x0032 => {
            if api.byte(SHED_STATE) == 3 {
                0 // 0x3D76
            } else {
                api.obj_show(OBJECT_10); // 0x3D5E
                api.scene_play(3);
                api.set_byte(SHED_STATE, 3);
                0x0A
            }
        }
        // 03D2:005A: object 10 taken — hidden, two samples, into the
        // inventory, and the story bytes it moves on.
        0x005A => {
            api.obj_hide(OBJECT_10); // 0x3D7F
            api.play_sample(8);
            api.play_sample(9);
            api.inv_give(OBJECT_10); // 0x3D97
            api.set_byte(0x26FB, 0x22); // 0x3D9D
            api.set_byte(0x2720, 0);
            api.set_byte(0x2717, 9);
            api.set_byte(0x272A, 0);
            api.set_byte(0x271C, 8);
            api.set_byte(0x2730, 0x13);
            api.set_byte(0x26FE, 1);
            api.set_byte(0x26FF, 4);
            api.set_byte(0x2700, 0x24);
            api.set_byte(0x26F9, 0x2A);
            api.set_byte(0x2703, 0x28);
            api.set_byte(0x2706, 1);
            api.set_byte(0x2722, 0);
            api.set_byte(0x2719, 7);
            api.set_byte(0x271D, 1);
            api.set_byte(0x272F, 0);
            api.set_byte(0x2729, 9); // 0x3DED
            api.pointer_shape_resync(); // 0x3DF2
            1
        }
        // 03D2:00DC: scene 0 until object 10 has been shown.
        0x00DC => {
            if api.byte(SHED_STATE) == 3 {
                0 // 0x3E13
            } else {
                api.scene_play(0); // 0x3E08
                0x0A
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
