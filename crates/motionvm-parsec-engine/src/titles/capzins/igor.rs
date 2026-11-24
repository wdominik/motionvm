//! Location 17, `Igor`, as the game names it in its location table
//! (record at image `0x1EE9C`).
//!
//! Re-expressed from the handler at `1406:0006` (image `0x14066`), its hook
//! at `1406:01FA` (`0x1425A`) and its animation hook at `1406:01A5`
//! (`0x14205`), against [`Api`] alone. The location has no rules of its
//! own (rules pointer `0000:0000`). Addresses in comments are offsets in
//! segment `1406h`.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 200.
const OBJECT_C8: u16 = 0xC8;
/// Object 208.
const OBJECT_D0: u16 = 0xD0;

/// `[1CFBh]`: this location's state byte.
const STATE: u16 = 0x1CFB;
/// `[1CECh]`: a state byte; at 2 the handler leaves through script exit
/// 64h.
const STATE_1CEC: u16 = 0x1CEC;
/// `[1F14h]`: the base animation of object 200 while pages are spoken, or
/// 0.
const BASE_C8: u16 = 0x1F14;
/// `[1F10h]`: the animation object 200 is asked to take.
const WANT_C8: u16 = 0x1F10;
/// `[1F12h]`: the animation object 200 was last given.
const ANIM_C8: u16 = 0x1F12;
/// `[1F0Eh]`: the base animation of object 208, or 0.
const BASE_D0: u16 = 0x1F0E;
/// `[1F0Ah]`: the animation object 208 is asked to take.
const WANT_D0: u16 = 0x1F0A;
/// `[1F0Ch]`: the animation object 208 was last given.
const ANIM_D0: u16 = 0x1F0C;
/// `[1F08h]`: the node the frame loop waits for, or 0.
const WAIT_NODE: u16 = 0x1F08;
/// `[1204h]`: the walker record's word `+28h` (`DS:11DCh + 28h`; DS1
/// `[16BBh]`).
const WALKER_28: u16 = 0x1204;

/// Everything before the frame loop, `0006` to `0152`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0006
    let entry = api.word(ENTRY); // 000B
    let loc = api.word(LOCATION); // 000F
    api.set_word(HERE, loc); // 0012
    api.visit_enter(loc, entry); // 0016

    // 001E: by `[1CFBh]`: 0, 1, 2; anything else goes straight to 012F.
    match api.byte(STATE) {
        0 => {
            // 003A: with item 17 in the inventory the state moves to 1.
            if api.inv_has(0x11) != 0 {
                api.set_byte(STATE, 1); // 0046
                both_shown(api); // 004B
                api.scene_play(0); // 008F
            } else {
                api.set_word(BASE_C8, 5); // 009A
                let rec = api.obj_show(OBJECT_C8); // 00A0
                api.slot_set_object(1, 0x10, rec); // 00A9
                api.set_word(ANIM_C8, 4); // 00B7
                api.set_word(WANT_C8, 4);
                api.obj_anim_id(OBJECT_C8, 4); // 00C0
                api.scene_play(1); // 00CC -> 0091
            }
        }
        1 => {
            both_shown(api); // 00D0
            api.scene_play(2); // 0114 -> 0091
        }
        2 => {
            // 0119: all six words cleared.
            api.set_word(BASE_C8, 0);
            api.set_word(ANIM_C8, 0);
            api.set_word(WANT_C8, 0);
            api.set_word(BASE_D0, 0); // 0124
            api.set_word(ANIM_D0, 0);
            api.set_word(WANT_D0, 0);
        }
        _ => {}
    }
    // 012F
    if api.byte(STATE_1CEC) == 2 {
        api.input_lock();
        api.hover_off();
        api.script_exit(0x64); // 0140
    }
    api.visit_finish_entry(); // 0148
    api.set_word(ENTRY, 0); // 014D
}

/// `004B`/`00D0`, the same sequence twice in the listing: both bases 1,
/// both objects shown and on animation 0.
fn both_shown(api: &mut dyn Api) {
    api.set_word(BASE_C8, 1); // 004B, 00D0
    api.set_word(BASE_D0, 1); // 0051, 00D6
    api.set_word(ANIM_C8, 0); // 0059, 00DE
    api.set_word(WANT_C8, 0);
    let rec = api.obj_show(OBJECT_C8); // 0060, 00E5
    api.obj_anim(rec, 0); // 006B, 00F0
    api.set_word(ANIM_D0, 0); // 0075, 00FA
    api.set_word(WANT_D0, 0);
    let rec = api.obj_show(OBJECT_D0); // 007C, 0101
    api.obj_anim(rec, 0); // 0087, 010C
}

/// One turn of the frame loop, `0153` to `0185`. Answers whether the loop
/// goes on.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0153
    api.dispatch(); // 0158
    // 015D: once `[1204h]` equals the awaited node, task 0 starts.
    let node = api.word(WAIT_NODE);
    if node != 0 && api.word(WALKER_28) == node {
        api.set_word(WAIT_NODE, 0); // 016D
        api.task_start(0); // 0173
    }
    api.frame_pass(); // 017B
    api.word(ENTRY) == 0 // 0180
}

/// Everything after the frame loop, `0187` to `01A4`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE); // 0187
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0192
    api.visit_leave(); // 019A
    api.slot_unload(); // 019F
}

/// The hook, `1406:01FA`: `code` at `[bp+6]`, `arg` at `[bp+8]`; codes 7,
/// 8 and 9 by compare (`0204`), anything else returns.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0216: each object with a base is asked for `base + arg`.
        8 => {
            let base = api.word(BASE_C8);
            if base != 0 {
                api.set_word(WANT_C8, base.wrapping_add(arg)); // 021D
            }
            let base = api.word(BASE_D0); // 0225
            if base != 0 {
                api.set_word(WANT_D0, base.wrapping_add(arg)); // 022F
            }
        }
        // 023A: each object follows what it was asked for.
        9 => {
            let want = api.word(WANT_C8);
            if want != api.word(ANIM_C8) {
                api.set_word(ANIM_C8, want); // 0243
                api.obj_anim_id(OBJECT_C8, num::signed(want)); // 0246
            }
            let want = api.word(WANT_D0); // 0252
            if want != api.word(ANIM_D0) {
                api.set_word(ANIM_D0, want); // 025E
                api.obj_anim_id(OBJECT_D0, num::signed(want)); // 0261 -> 02A3
            }
        }
        // 0267: script events, a sparse switch on `arg`: seven values at
        // `cs:0368` (0Ah, 0Bh, 0Dh, 0Eh, 0Fh, 10h, 6Eh), targets at
        // `cs:0376`.
        7 => match arg {
            0x0A => api.script_exit(0x65), // 0284
            0x0B => {
                api.set_byte(STATE, 2); // 028E
                api.set_word(BASE_C8, 0); // 0295
                api.set_word(ANIM_C8, 0);
                api.set_word(WANT_C8, 0);
                api.obj_anim_id(OBJECT_C8, 7); // 029E
            }
            0x0D => {
                api.walk_to_node(0x0BB8); // 02AE
            }
            0x0E => {
                api.set_word(WAIT_NODE, 0xC8); // 02B9
                api.walk_to_node(0xC8); // 02BF
                api.task_arm(0); // 02C6
            }
            0x0F => {
                // 02D0
                if api.word(BASE_D0) != 5 {
                    api.set_word(BASE_D0, 5);
                    let want = api.word(WANT_D0).wrapping_add(4); // 02E0
                    api.set_word(WANT_D0, want);
                }
            }
            0x10 => {
                // 02E7
                if api.word(BASE_D0) != 1 {
                    api.set_word(BASE_D0, 1);
                    let want = api.word(WANT_D0).wrapping_sub(4); // 02F4
                    api.set_word(WANT_D0, want);
                }
            }
            0x6E => {
                // 02FB: each object with a base is asked for `base - 1`.
                let base = api.word(BASE_C8);
                if base != 0 {
                    api.set_word(WANT_C8, base.wrapping_sub(1)); // 0302
                }
                let base = api.word(BASE_D0); // 0309
                if base != 0 {
                    api.set_word(WANT_D0, base.wrapping_sub(1)); // 0310
                }
                // 0317: `random(0, 6)`, five arms through the table at
                // `cs:035E`; above 4 no first sample (`ja 0354`).
                let first = match api.random(0, 6) {
                    0 => Some(0x23),             // 0331
                    1 => Some(0x24),             // 0335
                    2 => Some(0x25),             // 0339
                    3 => Some(0x0A),             // 033D
                    4 => Some(api.random(4, 6)), // 0341
                    _ => None,
                };
                if let Some(k) = first {
                    api.play_sample(k); // 034E
                }
                api.play_sample(0x23); // 0354
            }
            _ => {} // 027D
        },
        _ => {} // 0213
    }
}

/// The animation hook, `1406:01A5`: codes 64h, 65h and 66h by compare.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        0x64 => {
            api.obj_hide(OBJECT_C8); // 01BC
        }
        0x65 => {
            let k = api.random(3, 0x0A); // 01C6
            api.play_sample(k); // 01D3
            let k = api.random(7, 9); // 01D9
            api.play_sample(k); // 01E6 -> 01F2
        }
        0x66 => {
            api.play_sample(0x16); // 01E8
            api.play_sample(0x16); // 01F0
        }
        _ => {} // 01BA
    }
}

/// Igor has no rule handlers of its own.
pub(crate) fn rule(_api: &mut dyn Api, _off: u16, _flags: u16) -> Option<i16> {
    None
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
