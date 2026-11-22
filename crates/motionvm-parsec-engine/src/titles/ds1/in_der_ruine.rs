//! Location 19, In der Ruine: inside the ruin. The module is named after
//! the location as the game itself names it in its location table
//! (`In der Ruine`).
//!
//! Re-expressed from the handler at `03C7:0059` (image `0x3CC9`) and its
//! three rule handlers at `03C7:000D`, `03C7:0039` and `03C7:0049`,
//! against [`Api`] alone. The location has no hook and no animation hook;
//! the handler is the standard one.

use super::logic::Logic;
use crate::titles::parsec1994::Api;

/// `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`, `DS:1F56h`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
const HERE: u16 = 0x1F4C;
const OBJECT_STATES: u16 = 0x1F56;

/// Everything before the frame loop, `0x3CC9` to `0x3CE6`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x3CC9
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x3CD5
    api.visit_enter(loc, entry); // 0x3CD9
    api.visit_finish_entry(); // 0x3CE1
    api.set_word(ENTRY, 0); // 0x3CE6
}

/// One turn of the frame loop, `0x3CEC` to `0x3D00`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x3D02` to `0x3D1F`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// In der Ruine has no hook of its own.
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// In der Ruine's rule handlers by their offset in segment `03C7h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    match off {
        // 03C7:000D, verb 0Ch on object 20: state bytes only; answers 0,
        // so the rule search goes on.
        0x000D => {
            if api.byte(0x272D) != 3 {
                api.set_byte(0x2724, 0); // 0x3C87
                api.set_byte(0x271B, 6);
            }
            api.set_byte(0x26FF, 4); // 0x3C91
            api.set_byte(0x2706, 0x14);
            api.set_byte(0x272F, 0);
            api.set_byte(0x2729, 9); // 0x3CA0
            Some(0)
        }
        // 03C7:0039, Betrachte object 200.
        0x0039 => {
            api.scene_play(1); // 0x3CAC
            Some(0x0A)
        }
        // 03C7:0049, Betrachte object 201.
        0x0049 => {
            api.scene_play(2); // 0x3CBC
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
