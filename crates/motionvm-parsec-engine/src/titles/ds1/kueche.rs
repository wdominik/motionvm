//! Location 3, the Küche: the kitchen, with Karsten's mother at the stove.
//! The module is named after the location as the game itself names it in
//! its location table (`Küche`).
//!
//! Re-expressed from the handler at `00D2:00DC` (image `0xDFC`), its hook
//! at `00D2:0205` (`0xF25`) and its four rule handlers at `00D2:0004`,
//! `00D2:004B`, `00D2:0068` and `00D2:00B4`, against [`Api`] alone.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::Api;

/// The mother, object 200.
const MOTHER: u16 = 0xC8;
/// Object 202, the one the day-4 arm walks to.
const OBJECT_202: u16 = 0xCA;

/// `[2725h]`: the location state.
const STATE: u16 = 0x2725;
/// `[2726h]`: a sub-state.
const SUBSTATE: u16 = 0x2726;
/// `[2715h]`: the mother's state; `80h` and above, she is not here.
const MOTHER_STATE: u16 = 0x2715;
/// `[2716h]`: the father's state.
const FATHER_STATE: u16 = 0x2716;
/// `[271Eh]`.
const BYTE_271E: u16 = 0x271E;
/// `[271Fh]`: the counter the door's scene sequence steps.
const DOOR_SEQUENCE: u16 = 0x271F;
/// `[1898h]`: whether the mother speaks in the dialogue that is up.
const SHE_SPEAKS: u16 = 0x1898;
/// `[1894h]`: the animation she is to play at the next page's end.
const NEXT_ANIM: u16 = 0x1894;
/// `[1896h]`: the animation she is playing.
const PLAYING_ANIM: u16 = 0x1896;

/// Everything before the frame loop, `0xDFC` to `0xEDD`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0xDFC
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0xE08
    api.visit_enter(loc, entry); // 0xE0C
    // 0xE14: `cmp [2715h], 80h ; jb` — below 80h she stands on animation 3.
    if api.byte(MOTHER_STATE) < 0x80 {
        let mother = api.obj_show(MOTHER); // 0xE2B
        api.obj_anim(mother, 3); // 0xE33
    } else {
        api.obj_hide(MOTHER); // 0xE1E
    }
    // 0xE3B: seven arms through the table at 0xF17.
    match api.byte(STATE) {
        0 => {
            // 0xE51
            api.walk_to_node(MOTHER);
            api.scene_play(0);
            api.set_byte(SUBSTATE, 0);
            api.set_byte(STATE, 1);
        }
        2 => {
            // 0xE8B
            api.walk_to_node(MOTHER);
            api.scene_play(5);
        }
        3 => match api.byte(SUBSTATE) {
            // 0xE98
            0 => {
                // 0xEA8
                let s = api.byte(SUBSTATE).wrapping_add(1);
                api.set_byte(SUBSTATE, s);
                api.walk_to_node(MOTHER);
                api.set_byte(BYTE_271E, 0);
                api.set_byte(MOTHER_STATE, 4);
                api.scene_play(0x11);
            }
            2 => api.script_exit(0x64), // 0xEC3
            _ => {}
        },
        4 => {
            // 0xE6E
            api.walk_to_node(OBJECT_202);
            api.scene_play(6);
            api.set_byte(SUBSTATE, 0);
            api.set_byte(STATE, 9);
        }
        5 => {
            api.scene_play(0x16); // 0xED0
        }
        6 => {
            api.scene_play(0x15); // 0xECC
        }
        _ => {} // 1 and above 6: 0xED8
    }
    api.visit_finish_entry(); // 0xED8
    api.set_word(ENTRY, 0); // 0xEDD
}

/// One turn of the frame loop, `0xEE3` to `0xEF7`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0xEF9` to `0xF16`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `00D2:0205`: the mother's animations follow the dialogue,
/// and four script events. Codes through the table at `0xFED`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0xF3D: a dialogue comes up; bit 1 of its speakers is hers.
            api.set_word(SHE_SPEAKS, arg & 2);
            api.set_word(NEXT_ANIM, 0xFFFF);
            api.set_word(PLAYING_ANIM, 0xFFFF);
        }
        5 | 6 => {
            // 0xF51: if she spoke, back to animation 3.
            if api.word(SHE_SPEAKS) != 0 {
                api.obj_anim_id(MOTHER, 3);
                api.set_word(SHE_SPEAKS, 0);
            }
        }
        8 => api.set_word(NEXT_ANIM, arg.wrapping_add(1)), // 0xF70
        9 => {
            // 0xF78: a new animation to play, if she speaks.
            if api.word(SHE_SPEAKS) != 0 && api.word(PLAYING_ANIM) != api.word(NEXT_ANIM) {
                let next = api.word(NEXT_ANIM);
                api.set_word(PLAYING_ANIM, next);
                api.obj_anim_id(MOTHER, num::signed(next));
            }
        }
        7 => match arg {
            // 0xF9C: events 11 to 14 through the table at 0xFE5.
            11 => api.set_byte(0x270F, 1), // 0xFAD
            12 => {
                // 0xFB4
                api.inv_give(0x0B);
                api.set_byte(0x2731, 0);
                api.set_byte(0x2728, 3);
                api.set_byte(0x26FF, 1);
                api.set_byte(BYTE_271E, 0);
                api.set_byte(MOTHER_STATE, 4);
            }
            13 => {
                api.inv_give(0x10); // 0xFD7
            }
            14 => {
                api.inv_give(4); // 0xFDB
            }
            _ => {}
        },
        _ => {}
    }
}

/// The Küche's rule handlers by their offset in segment `00D2h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 00D2:0004, the door to the Wohnzimmer, by the father's state:
        // 0 scene 1; 4 and 5 a two-scene sequence stepped by [271Fh];
        // anything else nothing of its own.
        0x0004 => match api.byte(FATHER_STATE) {
            0 => {
                api.scene_play(1); // 0xD3C
                0x0A
            }
            4 => {
                let counter = api.ds(DOOR_SEQUENCE);
                api.scene_sequence(counter, &[0x0D, 0x0E]); // 0xD46
                0x0A
            }
            5 => {
                let counter = api.ds(DOOR_SEQUENCE);
                api.scene_sequence(counter, &[0x0F, 0x10]); // 0xD4E
                0x0A
            }
            _ => 0, // 0xD62
        },
        // 00D2:004B, the Treppenhaus: scene 2 while the state is 1.
        0x004B => {
            if api.byte(STATE) == 1 {
                api.scene_play(2); // 0xD75
                0x0A
            } else {
                0
            }
        }
        // 00D2:0068, Sprich mit the mother, by her state.
        0x0068 => {
            match api.byte(MOTHER_STATE) {
                0 => {
                    api.scene_play(3); // 0xDA0
                }
                2 => {
                    api.scene_play(7); // 0xDA4
                }
                5 => {
                    // 0xDA8
                    api.scene_play(0x17);
                    api.set_byte(BYTE_271E, 0);
                    api.set_byte(MOTHER_STATE, 0x80);
                }
                _ => {
                    // 0xDBC: one of scenes 10 to 12.
                    let n = api.random(0x0A, 0x0C);
                    api.scene_play(num::signed(n));
                }
            }
            0x0A
        }
        // 00D2:00B4, Betrachte object 203: item 4 and scene 18h, unless
        // item 4 is carried.
        0x00B4 => {
            if api.inv_has(4) != 0 {
                0 // 0xDE3
            } else {
                api.inv_give(4); // 0xDE7
                api.scene_play(0x18);
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
