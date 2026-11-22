//! Location 4, the Wohnzimmer: the living room — the father at the
//! television, Mike as a guest, and the telephone. The module is named
//! after the location as the game itself names it in its location table
//! (`Wohnzimmer`).
//!
//! Re-expressed from the handler at `0100:0073` (image `0x1073`), its hook
//! at `0100:0240` (`0x1240`), its animation hook at `0100:01FC`
//! (`0x11FC`) and its two rule handlers at `0100:0001` and `0100:0058`,
//! against [`Api`] alone.

use super::logic::Logic;
use super::logic::WALKER;
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The father, object 200.
const FATHER: u16 = 0xC8;
/// Mike, object 130.
const MIKE: u16 = 0x82;
/// The television, object 205.
const TELEVISION: u16 = 0xCD;
/// Object 204, shown without an animation.
const OBJECT_204: u16 = 0xCC;
/// The telephone, object 202.
const TELEPHONE: u16 = 0xCA;

/// `[2725h]`, `[2726h]`: location state and sub-state.
const STATE: u16 = 0x2725;
const SUBSTATE: u16 = 0x2726;
/// `[2716h]`: the father's state; `80h`, he is out.
const FATHER_STATE: u16 = 0x2716;
/// `[2715h]`: the mother's state.
const MOTHER_STATE: u16 = 0x2715;
/// `[2714h]`: who is on the telephone while the father is out.
const CALLER: u16 = 0x2714;
/// `[271Eh]`.
const BYTE_271E: u16 = 0x271E;
/// `[271Fh]`: counts the visits that find the father with Mike.
const VISITS: u16 = 0x271F;
/// `DS:1060h`: the null-terminated row of far pointers a wrong number asks
/// for; `DS:1689h`: substitution 0, the one the scene's text names.
const WRONG_NUMBERS: u16 = 0x1060;
const SUBSTITUTION_0: u16 = 0x1689;
/// `[18A4h]` the father's base animation while he speaks, `[18A2h]` what
/// event 8 adds, `[18A0h]` the one he is playing.
const FATHER_BASE: u16 = 0x18A4;
const FATHER_STEP: u16 = 0x18A2;
const FATHER_PLAYING: u16 = 0x18A0;
/// `[189Eh]` whether Mike speaks, `[189Ch]` his next animation, `[189Ah]`
/// the one he is playing.
const MIKE_SPEAKS: u16 = 0x189E;
const MIKE_NEXT: u16 = 0x189C;
const MIKE_PLAYING: u16 = 0x189A;
/// `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`, `DS:1F56h`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
const HERE: u16 = 0x1F4C;
const OBJECT_STATES: u16 = 0x1F56;

/// Everything before the frame loop, `0x1073` to `0x11BA`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup();
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x107F
    api.visit_enter(loc, entry); // 0x1083
    // 0x108B: by the father's state.
    match api.byte(FATHER_STATE) {
        3 => {
            // 0x109C: the father with Mike.
            api.obj_wake(MIKE);
            api.obj_wake(FATHER);
            let visits = api.byte(VISITS); // 0x10AE
            api.set_byte(VISITS, visits.wrapping_add(1));
            if visits != 0 {
                api.hover_off(); // 0x10B9
                api.set_byte(SUBSTATE, 0);
                api.set_byte(STATE, 5);
                api.scene_play(0x0E);
            } else {
                api.scene_play(9); // 0x10D3
            }
        }
        0x80 => {
            // 0x10F3: the father is out; the telephone, by [2714h] - 1
            // through the table at 0x11F4.
            api.obj_hide(FATHER);
            match u16::from(api.byte(CALLER)).wrapping_sub(1) {
                1 => {
                    // 0x1110
                    api.set_byte(0x2700, 1);
                    if api.byte(0x272D) != 2 {
                        api.set_byte(0x272E, 0);
                        api.set_byte(0x272D, 1);
                    }
                    api.scene_play(4);
                    api.set_byte(CALLER, 1); // 0x114B
                }
                2 => {
                    api.scene_play(8); // 0x1130
                    api.set_byte(CALLER, 1);
                }
                3 => {
                    // 0x1134
                    api.scene_play(0x0A);
                    api.set_byte(0x272E, 0);
                    api.set_byte(0x272D, 5);
                    api.set_byte(0x2700, 1);
                    api.set_byte(CALLER, 1);
                }
                0 => {
                    // 0x1152: one in three rings for a wrong number.
                    if api.random(0, 2) == 0 {
                        api.set_byte(CALLER, 0);
                        wrong_number(api);
                    }
                }
                _ => {
                    // 0x1169: one in five.
                    if api.random(0, 4) == 0 {
                        wrong_number(api);
                    }
                }
            }
        }
        _ => {
            // 0x10D7: the father at the television.
            api.obj_wake(FATHER);
            api.obj_wake(TELEVISION);
            api.obj_show(OBJECT_204);
        }
    }
    api.visit_finish_entry(); // 0x11B5
    api.set_word(ENTRY, 0); // 0x11BA
}

/// `0x1179`: a wrong number — one of the row's names into substitution 0,
/// then one of scenes 11 to 13.
fn wrong_number(api: &mut dyn Api) {
    let row = api.ds(WRONG_NUMBERS);
    let n = api.farptr_count(row);
    let i = api.random(0, n.wrapping_sub(1)); // 0x1189
    let at = WRONG_NUMBERS.wrapping_add(i << 2);
    let off = api.word(at);
    let seg = api.word(at.wrapping_add(2));
    api.set_word(SUBSTITUTION_0.wrapping_add(2), seg); // 0x119E
    api.set_word(SUBSTITUTION_0, off); // 0x11A1
    let scene = api.random(0x0B, 0x0D); // 0x11A9
    api.scene_play(num::signed(scene));
}

/// One turn of the frame loop, `0x11C0` to `0x11D4`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x11D6` to `0x11F3`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `0100:0240`: codes 0 to 10 through the table at `0x13F3`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x1259: a dialogue comes up; speaker bit 1 is the father,
            // bit 2 Mike.
            api.set_word(FATHER_BASE, u16::from(arg & 2 != 0));
            api.set_word(FATHER_PLAYING, 0xFFFF);
            api.set_word(FATHER_STEP, 0xFFFF);
            api.set_word(MIKE_SPEAKS, arg & 4);
            api.set_word(MIKE_PLAYING, 0xFFFF);
            api.set_word(MIKE_NEXT, 0xFFFF);
        }
        5 | 6 => {
            // 0x1286: whoever spoke goes back to standing.
            let base = api.word(FATHER_BASE);
            if base != 0 {
                api.obj_anim_id(FATHER, num::signed(base.wrapping_sub(1)));
                api.set_word(FATHER_BASE, 0);
                api.set_word(FATHER_STEP, 0);
                api.set_word(FATHER_PLAYING, 0);
            }
            if api.word(MIKE_SPEAKS) != 0 {
                api.set_word(MIKE_SPEAKS, 0); // 0x12B5
                api.set_word(MIKE_NEXT, 0);
                api.set_word(MIKE_PLAYING, 0);
                api.obj_anim_id(MIKE, 0);
            }
        }
        8 => {
            // 0x12CF
            api.set_word(FATHER_STEP, arg);
            api.set_word(MIKE_NEXT, arg.wrapping_add(1));
        }
        9 | 10 => {
            // 0x12D8: a page's end — each speaker's next animation.
            if api.word(MIKE_SPEAKS) != 0 && api.word(MIKE_PLAYING) != api.word(MIKE_NEXT) {
                let next = api.word(MIKE_NEXT);
                api.set_word(MIKE_PLAYING, next);
                api.obj_anim_id(MIKE, num::signed(next));
            }
            let base = api.word(FATHER_BASE);
            if base != 0 {
                let anim = base.wrapping_add(api.word(FATHER_STEP)); // 0x1304
                if anim != api.word(FATHER_PLAYING) {
                    api.set_word(FATHER_PLAYING, anim);
                    api.obj_anim_id(FATHER, num::signed(anim));
                }
            }
        }
        7 => event(api, arg), // 0x132D
        _ => {}
    }
}

/// Code 7, `0x132D`: script events through the value table at `0x13CF`.
fn event(api: &mut dyn Api, n: u16) {
    match n {
        0x0A => api.set_word(FATHER_BASE, 5), // 0x134A
        0x0B => api.set_word(FATHER_BASE, 1), // 0x1352
        0x0C => {
            // 0x135A
            api.task_arm(0);
            api.play_sample(8);
            api.play_sample(9);
            api.set_word(FATHER_BASE, 0);
            api.obj_anim_id(FATHER, 8);
            api.set_byte(0x270F, 1);
        }
        0x0D => {
            let walker = api.ds(WALKER);
            api.walker_walk_to(walker, 0x69, 3); // 0x138C
        }
        0x0E => {
            let walker = api.ds(WALKER);
            api.walker_walk_to(walker, 0x65, 1); // 0x1392
        }
        0x0F => {
            api.walk_to_node(TELEPHONE); // 0x13A4
        }
        0x10 => {
            api.inv_take(0x13); // 0x13AE
        }
        0x11 => api.play_sample(0x0E), // 0x13B7
        0x6E => {
            // 0x13C1
            api.set_word(FATHER_STEP, 0xFFFF);
            api.set_word(MIKE_NEXT, 0);
        }
        _ => {}
    }
}

/// The animation hook, `0100:01FC`: what the father's animation 8 reports.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        0x64 => {
            // 0x120E
            api.task_start(0);
            api.play_sample(0x13);
            api.play_sample(0x13);
        }
        0x65 => {
            // 0x1228
            api.play_sample(9);
            api.play_sample(0x0A);
            api.set_word(FATHER_BASE, 1);
        }
        _ => {}
    }
}

/// The Wohnzimmer's rule handlers by their offset in segment `0100h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    match off {
        // 0100:0001, Sprich mit the father, by his state; answers 0Ah
        // whatever the state (0x1053).
        0x0001 => {
            match api.byte(FATHER_STATE) {
                1 => {
                    // 0x1015
                    api.scene_play(0);
                    api.set_byte(SUBSTATE, 0);
                    api.set_byte(STATE, 2);
                    api.set_byte(BYTE_271E, 0);
                    api.set_byte(MOTHER_STATE, 1);
                }
                2 => {
                    // 0x1033
                    api.scene_play(1);
                    api.set_byte(BYTE_271E, 0);
                    api.set_byte(MOTHER_STATE, 0x80);
                    api.set_byte(VISITS, 0);
                    api.set_byte(FATHER_STATE, 4);
                    let s = api.byte(SUBSTATE).wrapping_add(1);
                    api.set_byte(SUBSTATE, s);
                }
                _ => {}
            }
            Some(0x0A)
        }
        // 0100:0058, the telephone: one of scenes 5 to 7.
        0x0058 => {
            let n = api.random(5, 7);
            api.scene_play(num::signed(n));
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
    anim_hook: Some(anim_hook),
    rule,
};
