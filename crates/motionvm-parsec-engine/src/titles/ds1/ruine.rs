//! Location 18, the Ruine: the ruin from outside. The module is named
//! after the location as the game itself names it in its location table
//! (`Ruine`).
//!
//! Re-expressed from the handler at `03B9:003E` (image `0x3BCE`) and its
//! two rule handlers at `03B9:0003` and `03B9:001E`, against [`Api`]
//! alone. The location has no hook and no animation hook.

use super::logic::Logic;
use crate::titles::parsec1994::Api;

/// Objects 103, 201 and 200, there unless `[271Dh]` is 1 or above 2.
const OBJECT_103: u16 = 0x67;
const OBJECT_201: u16 = 0xC9;
const OBJECT_200: u16 = 0xC8;

/// `[271Dh]`: the ruin's state.
const RUIN_STATE: u16 = 0x271D;
/// `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`, `DS:1F56h`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
const HERE: u16 = 0x1F4C;
const OBJECT_STATES: u16 = 0x1F56;

/// Everything before the frame loop, `0x3BCE` to `0x3C43`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x3BCE
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x3BDA
    api.visit_enter(loc, entry); // 0x3BDE
    // 0x3BE6: by [271Dh]: 0 the objects; 2 two points in slots 3 and 4,
    // the ambient scenes 0 to 2 and the objects; anything else nothing.
    let state = api.byte(RUIN_STATE);
    if state == 0 || state == 2 {
        if state == 2 {
            api.slot_set_raw(3, 0, 0xA5, 0x32); // 0x3BF4
            api.slot_set_raw(4, 0, 0x82, 0x32); // 0x3C05
            api.ambient_play(0, 2, 1); // 0x3C16
        }
        api.obj_wake(OBJECT_103); // 0x3C24
        api.obj_wake(OBJECT_201);
        api.obj_show(OBJECT_200); // 0x3C35
    }
    api.visit_finish_entry(); // 0x3C3E
    api.set_word(ENTRY, 0); // 0x3C43
}

/// One turn of the frame loop, `0x3C49` to `0x3C5D`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x3C5F` to `0x3C7C`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The Ruine has no hook of its own.
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// The Ruine's rule handlers by their offset in segment `03B9h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    match off {
        // 03B9:0003, verb 0Ah on object 103: scene 5, unless [271Dh] is 1.
        0x0003 => {
            if api.byte(RUIN_STATE) == 1 {
                Some(0) // 0x3B9D
            } else {
                api.scene_play(5); // 0x3BA1
                Some(0x0A)
            }
        }
        // 03B9:001E, verbs 1 and 4 on object 17: scene 6, a walk to node
        // 65h and exit 65h.
        0x001E => {
            api.scene_play(6); // 0x3BB1
            api.walk_to_node(0x65); // 0x3BB9
            api.script_exit(0x65); // 0x3BC1
            Some(0x0A)
        }
        _ => None,
    }
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
