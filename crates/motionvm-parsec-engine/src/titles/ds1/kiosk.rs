//! Location 17, the Kiosk. The module is named after the location as the
//! game itself names it in its location table (`Kiosk`).
//!
//! Re-expressed from the handler at `0395:00A8` (image `0x39F8`), its hook
//! at `0395:015C` (`0x3AAC`) and its rule handler at `0395:0006`
//! (`0x3956`), against [`Api`] alone.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::Api;

/// Object 201, which speaks on speaker bit 1 and is the rule's object.
const OBJECT_201: u16 = 0xC9;
/// Object 203, which speaks on speaker bit 2.
const OBJECT_203: u16 = 0xCB;
/// Object 200, shown in state 9.
const OBJECT_200: u16 = 0xC8;

/// `[2729h]`: the location state.
const STATE: u16 = 0x2729;
/// `DS:272Fh`: the counter the rule's scene sequences step.
const SEQUENCE: u16 = 0x272F;
/// `[193Ah]` whether object 201 speaks, `[1938h]` whether object 203
/// does, `[1934h]` the next animation, `[1936h]` the one object 201
/// plays.
const SPEAKS_201: u16 = 0x193A;
const SPEAKS_203: u16 = 0x1938;
const NEXT: u16 = 0x1934;
const PLAYING: u16 = 0x1936;

/// Everything before the frame loop, `0x39F8` to `0x3A62`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x39F8
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x3A04
    api.visit_enter(loc, entry); // 0x3A08
    // 0x3A10: by the state less 2 through the table at 0x3A9C; states 0
    // and 1 wrap above 7 and take the default.
    match u16::from(api.byte(STATE)).wrapping_sub(2) {
        0 => {
            // 0x3A3E: state 2
            api.set_byte(SEQUENCE, 0);
            api.set_byte(STATE, 0);
            api.obj_wake(OBJECT_201);
        }
        2 | 5 => {
            // 0x3A4A: states 4 and 7
            api.set_byte(SEQUENCE, 0);
            api.set_byte(STATE, 5);
            api.obj_wake(OBJECT_201);
        }
        6 => {
            // 0x3A30: state 8
            api.obj_wake(OBJECT_201);
            api.obj_wake(OBJECT_203); // 0x3A39
        }
        7 => {
            api.obj_show(OBJECT_200); // 0x3A26: state 9
        }
        _ => {
            api.obj_wake(OBJECT_201); // 0x3A54
        }
    }
    api.visit_finish_entry(); // 0x3A5D
    api.set_word(ENTRY, 0); // 0x3A62
}

/// One turn of the frame loop, `0x3A68` to `0x3A7C`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x3A7E` to `0x3A9B`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `0395:015C`: codes 0 to 9 through the table at `0x3B7F`;
/// codes 1 to 4 do nothing.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x3AC4: a dialogue comes up; bit 1 is object 201, bit 2
            // object 203.
            api.set_word(SPEAKS_201, arg & 2);
            api.set_word(SPEAKS_203, arg & 4);
            api.set_word(NEXT, 0);
            api.set_word(PLAYING, 0);
        }
        5 | 6 => {
            // 0x3ADF: whoever spoke goes back to animation 0.
            if api.word(SPEAKS_201) != 0 {
                api.set_word(NEXT, 0);
                api.set_word(PLAYING, 0);
                api.set_word(SPEAKS_201, 0);
                api.obj_anim_id(OBJECT_201, 0); // 0x3AF5
            }
            if api.word(SPEAKS_203) != 0 {
                api.set_word(SPEAKS_203, 0); // 0x3B04
                api.obj_anim_id(OBJECT_203, 0);
            }
        }
        8 => api.set_word(NEXT, arg.wrapping_add(1)), // 0x3B0C
        9 => {
            // 0x3B14: a page's end — each speaker's next animation.
            let next = api.word(NEXT);
            if next == api.word(PLAYING) {
                return;
            }
            if api.word(SPEAKS_201) != 0 {
                api.set_word(PLAYING, next); // 0x3B24
                api.obj_anim_id(OBJECT_201, num::signed(next));
            }
            if api.word(SPEAKS_203) != 0 {
                api.obj_anim_id(OBJECT_203, num::signed(api.word(NEXT))); // 0x3B3A
            }
        }
        7 => {
            // 0x3B4B: events 0Ah to 0Dh through the table at 0x3B77.
            match arg {
                0x0A => {
                    api.inv_give(5); // 0x3B5C
                }
                0x0B => {
                    api.inv_give(0x12); // 0x3B65
                }
                0x0C => {
                    api.inv_give(9); // 0x3B69
                }
                0x0D => {
                    api.inv_take(0x0B); // 0x3B6D
                }
                _ => {}
            }
        }
        _ => {}
    }
}

/// The Kiosk's rule handler by its offset in segment `0395h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    match off {
        // 0395:0006, Sprich mit object 201, by the state through the table
        // at 0x39E4; answers 0Ah whatever the state.
        0x0006 => {
            let counter = api.ds(SEQUENCE);
            match api.byte(STATE) {
                0 => {
                    api.scene_play(0); // 0x396C
                }
                1 => {
                    // 0x3970: on into state 2's sequence.
                    api.set_byte(0x26FF, 1);
                    api.set_byte(STATE, 2);
                    api.scene_sequence(counter, &[1, 2]);
                }
                2 => api.scene_sequence(counter, &[1, 2]), // 0x397A
                3 | 4 => {
                    // 0x398E
                    api.scene_sequence(counter, &[4, 5]);
                    api.set_byte(STATE, 4);
                }
                5 => {
                    // 0x39A7: one of scenes 6 to 8.
                    let n = api.random(6, 8);
                    api.scene_play(num::signed(n));
                }
                6 | 7 => {
                    // 0x39B6
                    api.scene_sequence(counter, &[9, 0x0A]);
                    api.set_byte(STATE, 7);
                }
                8 => api.scene_sequence(counter, &[0x0B, 0x0C]), // 0x39CF
                9 => {
                    api.scene_play(3); // 0x39D7
                }
                _ => {}
            }
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
