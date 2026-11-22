//! Location 16, the Krankenhaus: the hospital. The module is named after
//! the location as the game itself names it in its location table
//! (`Krankenhaus`).
//!
//! Re-expressed from the handler at `0373:004B` (image `0x377B`), its hook
//! at `0373:00DA` (`0x380A`) and its two rule handlers at `0373:0006` and
//! `0373:0027`, against [`Api`] alone.

use super::logic::Logic;
use crate::num;
use crate::titles::parsec1994::Api;

/// Object 200, which speaks on speaker bit 2.
const OBJECT_200: u16 = 0xC8;
/// Object 201, hidden unless the state is 4.
const OBJECT_201: u16 = 0xC9;
/// Object 132, which speaks on speaker bit 1.
const OBJECT_132: u16 = 0x84;

/// `[2719h]`: the location state.
const STATE: u16 = 0x2719;
/// `DS:2722h`: the counter of the scene sequence object 200 answers with.
const SEQUENCE: u16 = 0x2722;
/// `[1932h]` whether object 200 speaks, `[192Eh]` the next animation,
/// `[192Ch]` the one playing.
const SPEAKS_200: u16 = 0x1932;
const NEXT: u16 = 0x192E;
const PLAYING: u16 = 0x192C;
/// `[1930h]`: object 132's base animation while it speaks; negative, it
/// does not.
const BASE_132: u16 = 0x1930;
/// `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`, `DS:1F56h`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
const HERE: u16 = 0x1F4C;
const OBJECT_STATES: u16 = 0x1F56;

/// Everything before the frame loop, `0x377B` to `0x37D0`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x377B
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x3787
    api.visit_enter(loc, entry); // 0x378B
    api.set_word(BASE_132, 0xFFFF); // 0x3793
    // 0x3799: by the state, 4 and 5 against the rest.
    match api.byte(STATE) {
        4 => {
            api.obj_wake(OBJECT_200); // 0x37AA
        }
        5 => {
            api.obj_hide(OBJECT_201); // 0x37B4
            api.obj_wake(OBJECT_132); // 0x37BD
        }
        _ => {
            api.obj_hide(OBJECT_201); // 0x37C2
        }
    }
    api.visit_finish_entry(); // 0x37CB
    api.set_word(ENTRY, 0); // 0x37D0
}

/// One turn of the frame loop, `0x37D6` to `0x37EA`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x37EC` to `0x3809`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `0373:00DA`: codes 0 to 9 through the table at `0x3942`;
/// codes 1 to 4 do nothing.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x3823: a dialogue comes up; bit 2 is object 200, bit 1
            // object 132 on base animation 0.
            api.set_word(SPEAKS_200, arg & 4);
            api.set_word(BASE_132, if arg & 2 != 0 { 0 } else { 0xFFFF });
            api.set_word(NEXT, 0); // 0x383B
            api.set_word(PLAYING, 0);
        }
        5 | 6 => {
            // 0x3846: whoever spoke goes back.
            if api.word(SPEAKS_200) != 0 {
                api.set_word(NEXT, 0); // 0x384D
                api.set_word(PLAYING, 0);
                api.set_word(SPEAKS_200, 0);
                api.obj_anim_id(OBJECT_200, 0);
            }
            // 0x3864: `jge` — a signed test.
            let base = num::signed(api.word(BASE_132));
            if base >= 0 {
                api.obj_anim_id(OBJECT_132, base); // 0x386E
                api.set_word(BASE_132, 0xFFFF);
            }
        }
        8 => api.set_word(NEXT, arg.wrapping_add(1)), // 0x3886
        9 => {
            // 0x388F: a page's end — each speaker's next animation.
            if api.word(PLAYING) == api.word(NEXT) {
                return;
            }
            if api.word(SPEAKS_200) != 0 {
                let next = api.word(NEXT); // 0x38A2
                api.set_word(PLAYING, next);
                api.obj_anim_id(OBJECT_200, num::signed(next));
            }
            // 0x38B4: `jl` — a signed test.
            let base = api.word(BASE_132);
            if num::signed(base) >= 0 {
                let anim = base.wrapping_add(api.word(NEXT)); // 0x38BB
                api.set_word(PLAYING, anim);
                api.obj_anim_id(OBJECT_132, num::signed(anim));
            }
        }
        7 => event(api, arg), // 0x38D3
        _ => {}
    }
}

/// Code 7, `0x38D3`: script events through the value table at `0x392A`.
fn event(api: &mut dyn Api, n: u16) {
    match n {
        0x0A => {
            // 0x38EF
            api.set_word(BASE_132, 3);
            api.inv_take(0x12);
        }
        0x0B => {
            // 0x38FE
            api.set_word(BASE_132, 6);
            api.inv_take(0x16);
        }
        0x0C => {
            api.inv_give(6); // 0x3908
        }
        0x0D => api.set_word(BASE_132, 3), // 0x3912
        0x0E => api.set_word(BASE_132, 6), // 0x391A
        0x6E => api.set_word(NEXT, 0),     // 0x3922
        _ => {}
    }
}

/// The Krankenhaus's rule handlers by their offset in segment `0373h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    match off {
        // 0373:0006, Sprich mit object 200: a three-scene sequence.
        0x0006 => {
            let counter = api.ds(SEQUENCE);
            api.scene_sequence(counter, &[0, 1, 2]); // 0x3739
            api.set_byte(0x270F, 1); // 0x374D
            Some(0x0A)
        }
        // 0373:0027, Sprich mit object 132.
        0x0027 => {
            api.scene_play(3); // 0x375A
            api.set_byte(0x2708, 1);
            api.set_byte(0x2706, 0x14);
            api.set_byte(SEQUENCE, 0);
            api.set_byte(STATE, 7); // 0x3771
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
