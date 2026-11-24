//! Location 22, the Stammtisch: the regulars' table. The module is named
//! after the location as the game itself names it in its location table
//! (`Stammtisch`).
//!
//! Re-expressed from the handler at `03E6:000E` (image `0x3E6E`) and its
//! hook at `03E6:0072` (`0x3ED2`), against [`Api`] alone. The location has
//! no rules of its own.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::Api;

/// Objects 200, 202 and 204, which the hook animates together.
const OBJECT_200: u16 = 0xC8;
const OBJECT_202: u16 = 0xCA;
const OBJECT_204: u16 = 0xCC;

/// Everything before the frame loop, `0x3E6E` to `0x3E98`: hover off and
/// scene 0 at once.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x3E6E
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x3E7A
    api.visit_enter(loc, entry); // 0x3E7E
    api.hover_off(); // 0x3E86
    api.scene_play(0); // 0x3E8D
    api.visit_finish_entry(); // 0x3E93
    api.set_word(ENTRY, 0); // 0x3E98
}

/// One turn of the frame loop, `0x3E9E` to `0x3EB2`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x3EB4` to `0x3ED1`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `03E6:0072`: only code 9, a page's end, does anything — the
/// three objects play animation `arg + 1` (code 7 jumps to the same end,
/// `0x3EDF`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    if code == 9 {
        let anim = num::signed(arg.wrapping_add(1)); // 0x3EE8
        api.obj_anim_id(OBJECT_200, anim); // 0x3EED
        api.obj_anim_id(OBJECT_202, anim); // 0x3EFC
        api.obj_anim_id(OBJECT_204, anim); // 0x3F0B
    }
}

/// The location has no rules of its own (its record's `+2Bh` is null).
pub(crate) fn rule(_api: &mut dyn Api, _off: u16, _flags: u16) -> Option<i16> {
    None
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
