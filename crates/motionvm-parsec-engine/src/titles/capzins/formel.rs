//! Location 7, the Formel (`Formel` in the game's location table).
//!
//! Re-expressed from the handler at `1246:0022` (image `0x12482`), its
//! hook at `1246:00B4` (`0x12514`) and the one rule handler both of its
//! rules share, `1246:0001` (`0x12461`), against [`Api`] alone. The
//! location has no animation hook. Addresses are those of *Captain Zins*'
//! `XP01.EXE`, `DGROUP` `1FC4h`.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::titles::parsec1994::Api;

/// Objects 200 and 201, both set on animation 0 on entry.
const OBJECT_200: u16 = 0xC8;
const OBJECT_201: u16 = 0xC9;

/// `[1CF5h]`: the state the rule and script event 10 share (the Schranke's
/// segment reads it too).
const STATE: u16 = 0x1CF5;
/// `[1EC0h]`: the ambient scene's latch: positive, the frame loop starts
/// it and clears the word; `FFFFh` after the rule; set to 1 again when
/// task 0's scene ends.
const AMBIENT: u16 = 0x1EC0;

/// Everything before the frame loop, `0x12482` to `0x124BF`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x12482
    let entry = api.word(ENTRY); // 0x12487
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x1248E
    api.visit_enter(loc, entry); // 0x12492
    api.set_word(AMBIENT, 1); // 0x1249A
    api.obj_anim_id(OBJECT_200, 0); // 0x124A5
    api.obj_anim_id(OBJECT_201, 0); // 0x124B2
    api.visit_finish_entry(); // 0x124BA
    api.set_word(ENTRY, 0); // 0x124BF
}

/// One turn of the frame loop, `0x124C5` to `0x124F4`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x124C5
    api.dispatch(); // 0x124CA
    // 0x124CF: `cmp [1EC0h], 0 ; jle` — signed.
    if crate::num::signed(api.word(AMBIENT)) > 0 {
        api.ambient_play(3, 5, 1); // 0x124DC
        api.set_word(AMBIENT, 0); // 0x124E4
    }
    api.frame_pass(); // 0x124EA
    api.word(ENTRY) == 0 // 0x124EF
}

/// Everything after the frame loop, `0x124F6` to `0x12513`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5)); // 0x124F9
    api.objs_save_state(out); // 0x12501
    api.visit_leave(); // 0x12509
    api.slot_unload(); // 0x1250E
}

/// The hook, `1246:00B4`: a compare chain on the code (5, 6, 7).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, task: u16) {
    match code {
        5 | 6 => {
            // 0x1252B: task 0's dialogue ends and the latch is not zero:
            // the ambient scene is due again.
            if task == 0 && api.word(AMBIENT) != 0 {
                api.set_word(AMBIENT, 1); // 0x12538
            }
        }
        7 => {
            // 0x12540: event 10.
            if arg == 0x0A {
                api.set_byte(STATE, 2); // 0x12548
            }
        }
        _ => {}
    }
}

/// The Formel's rule handlers by their offset in segment `1246h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 1246:0001, both rules' handler (word +2 = 2, mask 4, word +6 =
        // C8h and C9h).
        0x0001 => {
            // 0x12464: `cmp [1CF5h], 1 ; jb`.
            if api.byte(STATE) >= 1 {
                api.scene_play(1); // 0x1246B
            } else {
                api.scene_play(2); // 0x1246F
            }
            api.set_word(AMBIENT, 0xFFFF); // 0x12477
            0x0A // 0x1247D
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
