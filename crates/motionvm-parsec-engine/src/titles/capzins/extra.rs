//! Location 15, `Extra`, as the game names it in its location table
//! (record at image `0x1EE44`).
//!
//! Re-expressed from the handler at `13A5:0047` (image `0x13A97`), its hook
//! at `13A5:00E9` (`0x13B39`), its animation hook at `13A5:00B8`
//! (`0x13B08`) and its one rule handler at `13A5:0006` (`0x13A56`),
//! against [`Api`] alone. Every branch carries the address it was read
//! from; addresses in comments are offsets in segment `13A5h`.
//!
//! The original handler owns its frame loop; here it is cut into
//! [`enter`] (everything before the loop), [`frame`] (one turn of it) and
//! [`leave`] (everything after).

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 200.
const OBJECT_C8: u16 = 0xC8;
/// Object 202.
const OBJECT_CA: u16 = 0xCA;

/// `[1CF2h]`: this location's state byte, the rule's switch.
const STATE: u16 = 0x1CF2;
/// `[1EF2h]`: non-zero while a spoken page drives object 200.
const TALKING: u16 = 0x1EF2;
/// `[1EF0h]`: the animation object 200 was last given by the hook.
const TALK_ANIM: u16 = 0x1EF0;

/// Everything before the frame loop, `0047` to `0083`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0047
    let entry = api.word(ENTRY); // 004C
    let loc = api.word(LOCATION); // 0050
    api.set_word(HERE, loc); // 0053
    api.visit_enter(loc, entry); // 0057
    api.obj_anim_id(OBJECT_C8, 0); // 005F
    api.obj_anim_id(OBJECT_CA, 0); // 006C
    api.visit_finish_entry(); // 0079
    api.set_word(ENTRY, 0); // 007E
}

/// One turn of the frame loop, `0084` to `0098`. Answers whether the loop
/// goes on.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0084
    api.dispatch(); // 0089
    api.frame_pass(); // 008E
    api.word(ENTRY) == 0 // 0093
}

/// Everything after the frame loop, `009A` to `00B7`.
pub(crate) fn leave(api: &mut dyn Api) {
    // 009A: this location's object states kept in its 32 bytes.
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 00A5
    api.visit_leave(); // 00AD
    api.slot_unload(); // 00B2
}

/// The hook, `13A5:00E9`: `code` at `[bp+6]`, `arg` at `[bp+8]`. Ten arms
/// through the table at `cs:015F` (0: `00FE`, 5 and 6: `0120`, 7: `0144`,
/// 9: `012F`, the rest and anything above 9: the return at `015D`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 00FE: `[1EF2h] = arg & 2`; when set, object 200 starts from
        // animation 0.
        0 => {
            api.set_word(TALKING, arg & 2);
            if api.word(TALKING) != 0 {
                talk_anim(api, 0); // 010D
            }
        }
        // 0120: the flag cleared, and object 200 back on animation 0.
        5 | 6 => {
            if api.word(TALKING) != 0 {
                api.set_word(TALKING, 0); // 0127
                talk_anim(api, 0); // 012D -> 010D
            }
        }
        // 012F: while the flag is set, object 200 follows `arg + 1`.
        9 => {
            if api.word(TALKING) != 0 {
                let k = arg.wrapping_add(1); // 0136
                if k != api.word(TALK_ANIM) {
                    talk_anim(api, k); // 013F -> 010F
                }
            }
        }
        // 0144: script event 10 — sample 7, and item 12 given.
        7 if arg == 0x0A => {
            api.play_sample(7); // 014D
            api.inv_give(0x0C); // 0155
        }
        _ => {} // 015D
    }
}

/// `010F`: `[1EF0h] = k`, and object 200 on animation `k`.
fn talk_anim(api: &mut dyn Api, k: u16) {
    api.set_word(TALK_ANIM, k); // 010F
    api.obj_anim_id(OBJECT_C8, num::signed(k)); // 0112
}

/// The animation hook, `13A5:00B8`: `code - 64h` as an unsigned word, four
/// arms through the table at `cs:00E1`.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code.wrapping_sub(0x64) {
        0 | 1 => api.play_sample(0x21), // 00CD; 00D1 jumps to 00CD
        2 => api.play_sample(0x17),     // 00D3
        3 => api.play_sample(9),        // 00D7
        _ => {}                         // 00DF
    }
}

/// The rule handlers by their offset in segment `13A5h`. Answers what the
/// original answers the rule search with, or `None` for an offset that is
/// not one of them.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 13A5:0006 (rule at `1F73:0480`: verb 2, mask 4, object 200):
        // by `[1CF2h]`, four arms through the table at `cs:003F`; above 3
        // no scene plays (`ja 003A`).
        0x0006 => {
            match api.byte(STATE) {
                0 => {
                    api.set_byte(STATE, 1); // 001C
                    api.scene_play(0); // 0021
                }
                1 => {
                    api.scene_play(1); // 0025
                }
                2 => {
                    api.set_byte(STATE, 3); // 0029
                    api.scene_play(2); // 002E
                }
                3 => {
                    api.scene_play(3); // 0032
                }
                _ => {}
            }
            0x0A // 003A
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
