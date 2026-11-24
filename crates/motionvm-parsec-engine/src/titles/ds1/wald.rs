//! Location 23, the Wald: the forest, a location of kind 3. The module is
//! named after the location as the game itself names it in its location
//! table (`Wald`).
//!
//! Re-expressed from the handler at `03F1:0061` (image `0x3F71`), its hook
//! at `03F1:00D2` (`0x3FE2`), its animation hook at `03F1:00CD`
//! (`0x3FDD`) and its three rule handlers at `03F1:0008`, `03F1:0018` and
//! `03F1:0041`, against [`Api`] alone.
//!
//! The handler's prologue differs from the standard one: it hands
//! `[1F4Ah]` to the shared entry directly and writes `[1F4Ch]` only after
//! `0EB9:00B8`.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::titles::parsec1994::{Api, Far};

/// Object 215, set on animation 0 on every visit.
const OBJECT_215: u16 = 0xD7;
/// Object 105, which the rules for `03F1:0018` and `03F1:0041` name
/// (`69h` at `+6` of their records) and rule `03F1:0008` walks to.
const OBJECT_105: u16 = 0x69;

/// `[270Bh]`: set once scene 0 of rule `03F1:0041` has played.
const BYTE_270B: u16 = 0x270B;
/// `[270Dh]`: set on every visit.
const BYTE_270D: u16 = 0x270D;
/// `[28BBh]`, `[28BDh]`: the mode machine's gate, a far routine it asks
/// before it goes on (see `parsec1994/mode.rs`); rule `03F1:0018` clears it.
const GATE_OFF: u16 = 0x28BB;
const GATE_SEG: u16 = 0x28BD;

/// Everything before the frame loop, `0x3F71` to `0x3F93`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x3F71
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.visit_enter(loc, entry); // 0x3F7E
    api.obj_anim_id(OBJECT_215, 0); // 0x3F8B
    api.set_byte(BYTE_270D, 1); // 0x3F93
    api.visit_finish_entry(); // 0x3F98
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x3FA0
    api.set_word(ENTRY, 0); // 0x3FA3
}

/// One turn of the frame loop, `0x3FA9` to `0x3FBD`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x3FBF` to `0x3FDC`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `03F1:00D2`: a comparison with code 7 whose branch lands
/// where it falls through (`0x3FEB`, `jne` to the next instruction) — it
/// does nothing.
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// The animation hook, `03F1:00CD`: an empty routine.
pub(crate) fn anim_hook(_api: &mut dyn Api, _rec: Far, _code: u16) {}

/// The forest's rule handlers by their offset in segment `03F1h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 03F1:0008: the player walks to object 105.
        0x0008 => {
            api.walk_to_node(OBJECT_105); // 0x3F1D
            9
        }
        // 03F1:0018: before [270Bh] is set, scene 1 and the gate cleared.
        0x0018 => {
            if api.byte(BYTE_270B) != 0 {
                0 // 0x3F48
            } else {
                api.scene_play(1); // 0x3F34
                api.set_word(GATE_SEG, 0); // 0x3F3A
                api.set_word(GATE_OFF, 0); // 0x3F40
                0x0A
            }
        }
        // 03F1:0041: scene 0 the first time, [270Bh] set; scene 4 after.
        0x0041 => {
            if api.byte(BYTE_270B) == 0 {
                api.set_byte(BYTE_270B, 1); // 0x3F5B
                api.scene_play(0);
            } else {
                api.scene_play(4); // 0x3F64
            }
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
