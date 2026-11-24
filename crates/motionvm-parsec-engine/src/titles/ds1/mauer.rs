//! Location 12, the Mauer. The module is named after the location as the
//! game itself names it in its location table (`Mauer`).
//!
//! Re-expressed from the handler at `02E5:0071` (image `0x2EC1`), its
//! animation hook at `02E5:017C` (`0x2FCC`) and its three rule handlers at
//! `02E5:0008`, `02E5:0042` and `02E5:005D`, against [`Api`] alone. The
//! location installs no hook of its own.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::titles::parsec1994::{Api, Far};

/// Object 201, the one rule `0008` wakes and the frame loop sets on
/// animation 1.
const OBJECT_201: u16 = 0xC9;
/// Objects 200 and 203, shown once the animation hook has heard 66h.
const OBJECT_200: u16 = 0xC8;
const OBJECT_203: u16 = 0xCB;

/// `[2713h]`: the location's state — 0, then 2 while object 201 runs,
/// 3 once it is on animation 1, 4 from the animation hook, 1 at the end.
const STATE: u16 = 0x2713;
/// `[2725h]`, `[2726h]`: the state and sub-state the Zimmer and the
/// Küche switch on.
const ROOM_STATE: u16 = 0x2725;
const ROOM_SUBSTATE: u16 = 0x2726;
/// `[1C82h]`: the hundredths since rule `0008` cleared them.
const HUNDREDTHS: u16 = 0x1C82;
/// `[1C76h]`: the ticks not yet taken.
const TICKS_TAKEN: u16 = 0x1C76;

/// Everything before the frame loop, `0x2EC1` to `0x2EEF`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x2EC1
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x2ECD
    api.visit_enter(loc, entry); // 0x2ED1
    api.slot_set_raw(0, 3, 0xA0, 0x40); // 0x2EE2
    api.visit_finish_entry(); // 0x2EEA
    api.set_word(ENTRY, 0); // 0x2EEF
}

/// One turn of the frame loop, `0x2EF5` to `0x2FAB`: by `[2713h]`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x2EF5
    api.dispatch(); // 0x2EFA
    match api.byte(STATE) {
        2 => {
            // 0x2F11: two seconds on, object 201 to animation 1.
            if api.word(HUNDREDTHS) > 0xC8 {
                api.set_byte(STATE, 3); // 0x2F1C
                api.obj_anim_id(OBJECT_201, 1);
            }
        }
        4 => {
            // 0x2F30: the picture changes under the reveal effect.
            api.obj_hide(OBJECT_201);
            api.obj_show(OBJECT_200); // 0x2F3C
            api.obj_show(OBJECT_203); // 0x2F45
            api.set_byte(0x2724, 0); // 0x2F4B
            api.set_byte(0x271B, 5);
            api.set_byte(ROOM_SUBSTATE, 0);
            api.set_byte(ROOM_STATE, 7);
            api.anim_pass(0); // 0x2F61
            api.room_redraw(); // 0x2F67
            api.panel_refresh(); // 0x2F6C
            api.fx_reveal(); // 0x2F71
            api.pointer_unfreeze(); // 0x2F76
            api.key_flush(); // 0x2F7B
            api.input_poll(); // 0x2F80
            api.input_poll(); // 0x2F85
            api.hover_on(); // 0x2F8A
            api.set_word(TICKS_TAKEN, 0); // 0x2F8F
            api.input_unlock(); // 0x2F95
            api.set_byte(STATE, 1); // 0x2F9A
        }
        _ => {} // 0x2F0E
    }
    api.frame_pass(); // 0x2F9F
    api.word(ENTRY) == 0 // 0x2FA4
}

/// Everything after the frame loop, `0x2FAE` to `0x2FCB`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// No hook is installed for the Mauer (the location record's `+2Fh` is
/// null).
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// The animation hook, `02E5:017C`: code 66h moves the state to 4.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    if code == 0x66 {
        api.set_byte(STATE, 4); // 0x2FD7
    }
}

/// The Mauer's rule handlers by their offset in segment `02E5h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 02E5:0008, the rules of verbs 4 and 5 on object 14: the first
        // time object 201 is woken, the clock cleared and the input taken
        // over, and it answers 1; after that, scene 2.
        0x0008 => {
            if api.byte(STATE) == 0 {
                api.set_byte(STATE, 2); // 0x2E62
                api.obj_wake(OBJECT_201); // 0x2E6A
                api.set_word(HUNDREDTHS, 0); // 0x2E70
                api.input_lock(); // 0x2E76
                api.hover_off(); // 0x2E7B
                1 // 0x2E80
            } else {
                api.scene_play(2); // 0x2E87
                0x0A
            }
        }
        // 02E5:0042, Betrachte object 201: scene 0 before, scene 1 after.
        0x0042 => {
            let n = if api.byte(STATE) == 0 { 0 } else { 1 }; // 0x2E95
            api.scene_play(n); // 0x2EA2
            0x0A
        }
        // 02E5:005D, Betrachte object 200: to location 11, entry 200.
        0x005D => {
            api.set_word(LOCATION, 0x0B); // 0x2EB0
            api.set_word(ENTRY, 0xC8); // 0x2EB6
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
