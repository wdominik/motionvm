//! Location 14, the Bogenstraße. The module is named after the location
//! as the game itself names it in its location table (`Bogenstraße`).
//!
//! Re-expressed from the handler at `0333:0082` (image `0x33B2`), its hook
//! at `0333:015C` (`0x348C`), its animation hook at `0333:00E8`
//! (`0x3418`) and its rule handler at `0333:000D` (`0x333D`), against
//! [`Api`] alone.
//!
//! The handler is the standard one: nothing of its own between the
//! shared entry and `0EB9:00B8`, and no check in its frame loop.

use super::logic::Logic;
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 132, the one the hook's animations play on.
const OBJECT_132: u16 = 0x84;
/// Object 201, which the animation hook shows and hides.
const OBJECT_201: u16 = 0xC9;
/// Object 202, woken and hidden together with the walker's bit.
const OBJECT_202: u16 = 0xCA;

/// `[2719h]`: the location state the rule switches on.
const STATE: u16 = 0x2719;
/// `DS:2722h`: the counter the rule's scene sequences step.
const SEQUENCE: u16 = 0x2722;
/// `[191Eh]`: where script event 0Dh and animation code 65h meet —
/// 3E8h set by event 0Ch, 3E9h by whichever of the two comes first,
/// 3EAh once the animation has reported, 0 at its end (code 67h).
const MEETING: u16 = 0x191E;
/// `[1920h]` the animation object 132 plays, `[1922h]` the next one,
/// `[1924h]` whether it is to follow the pages.
const PLAYING: u16 = 0x1920;
const NEXT: u16 = 0x1922;
const FOLLOWS: u16 = 0x1924;
/// `[1695h]`: byte `+2` of the player's walker record (`DS:1693h`); the
/// location sets and clears its bit 0 around object 202.
const WALKER_BYTE_2: u16 = 0x1695;
/// `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`, `DS:1F56h`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
const HERE: u16 = 0x1F4C;
const OBJECT_STATES: u16 = 0x1F56;

/// Everything before the frame loop, `0x33B2` to `0x33CF`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x33B2
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x33BE
    api.visit_enter(loc, entry); // 0x33C2
    api.visit_finish_entry(); // 0x33CA
    api.set_word(ENTRY, 0); // 0x33CF
}

/// One turn of the frame loop, `0x33D5` to `0x33E9`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x33EB` to `0x3408`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// `0333:00D9` (`0x3409`): object 202 woken, bit 0 of the walker's byte
/// `+2` cleared.
fn object_202_back(api: &mut dyn Api) {
    api.obj_wake(OBJECT_202);
    let b = api.byte(WALKER_BYTE_2);
    api.set_byte(WALKER_BYTE_2, b & 0xFE); // 0x3412
}

/// The hook, `0333:015C`: codes 0 to 9 through the table at `0x35B5`;
/// codes 1 to 6 do nothing.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x34A5
            api.set_word(FOLLOWS, 0);
            api.set_word(NEXT, 0);
            api.set_word(PLAYING, 0);
        }
        8 => api.set_word(NEXT, arg.wrapping_add(1)), // 0x34B3
        9 => {
            // 0x34BC: a page's end — the next animation, if it follows.
            if api.word(FOLLOWS) != 0 && api.word(PLAYING) != api.word(NEXT) {
                let next = api.word(NEXT); // 0x34D2
                api.set_word(PLAYING, next);
                api.obj_anim_id(OBJECT_132, num::signed(next));
            }
        }
        7 => event(api, arg), // 0x34E7
        _ => {}
    }
}

/// Code 7, `0x34E7`: script events through the value table at `0x3599`.
fn event(api: &mut dyn Api, n: u16) {
    match n {
        0x0A => {
            // 0x3504: walk to object 132; it shows on animation 3 and
            // follows the pages.
            api.walk_to_node(OBJECT_132);
            api.set_word(NEXT, 3);
            api.set_word(PLAYING, 3);
            // 0x3516: the 3 pushed for 0883:1F82 is pushed before the
            // id for 14DD:084A and stays on the stack for the second call.
            let rec = api.obj_show(OBJECT_132);
            api.obj_anim(rec, 3);
            api.set_word(FOLLOWS, 1); // 0x352A
        }
        0x0C => {
            // 0x3532
            api.set_word(NEXT, 5);
            api.set_word(MEETING, 0x3E8);
        }
        0x0D => {
            // 0x3540: if the animation has reported already, object 202
            // is back; else this event was first.
            if api.word(MEETING) == 0x3EA {
                object_202_back(api);
            } else {
                api.set_word(MEETING, 0x3E9); // 0x354D
            }
            api.task_arm(0); // 0x3553
        }
        0x0E => api.set_word(NEXT, 6), // 0x355C
        0x0F => {
            // 0x3564
            api.set_word(NEXT, 4);
            api.set_word(PLAYING, 4);
            api.obj_anim_id(OBJECT_132, 4);
            api.set_word(FOLLOWS, 0);
        }
        0x10 => {
            // 0x3581
            api.set_word(NEXT, 6);
            api.inv_give(8);
        }
        0x6E => api.set_word(NEXT, 0), // 0x3591
        _ => {}
    }
}

/// The animation hook, `0333:00E8`: codes 64h to 67h through the table at
/// `0x3484`.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        0x64 => {
            // 0x342D: the 0 pushed first is 0883:1F82's animation, left
            // on the stack under 14DD:0A06's two arguments.
            let rec = api.obj_show_frame(OBJECT_201, -1);
            api.obj_anim(rec, 0);
        }
        0x65 => {
            // 0x3448: if event 0Dh came first, object 202 is back.
            if api.word(MEETING) == 0x3E9 {
                object_202_back(api);
            }
            api.set_word(MEETING, 0x3EA); // 0x3453
        }
        0x66 => {
            api.obj_hide(OBJECT_201); // 0x345B
        }
        0x67 => {
            // 0x3466
            api.set_word(MEETING, 0);
            api.task_start(0);
            api.obj_hide(OBJECT_202);
            let b = api.byte(WALKER_BYTE_2);
            api.set_byte(WALKER_BYTE_2, b | 1); // 0x347D
        }
        _ => {}
    }
}

/// The Bogenstraße's rule handler by its offset in segment `0333h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    match off {
        // 0333:000D, verb 4 on object 200, by the state through the table
        // at 0x33A0; every arm but state 0 answers 0Ah.
        0x000D => {
            let counter = api.ds(SEQUENCE);
            match api.byte(STATE) {
                0 => return Some(0), // 0x3353
                1 => {
                    // 0x3357
                    api.scene_play(0);
                    api.set_byte(SEQUENCE, 0);
                    api.set_byte(STATE, 2);
                }
                2 => api.scene_sequence(counter, &[1, 4, 5]), // 0x336B
                3 => api.scene_sequence(counter, &[2, 3]),    // 0x3381
                8 => api.scene_sequence(counter, &[6, 7]),    // 0x3389
                _ => {}                                       // 4 to 7, above 8: 0x339B
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
    anim_hook: Some(anim_hook),
    rule,
};
