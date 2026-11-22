//! Location 11, the Schule. The module is named after the location as the
//! game itself names it in its location table (`Schule`).
//!
//! Re-expressed from the handler at `02AD:0101` (image `0x2BD1`), its hook
//! at `02AD:0219` (`0x2CE9`) and its five rule handlers at `02AD:0001`,
//! `002F`, `00B5`, `00DD` and `00F1`, against [`Api`] alone.
//!
//! Object 204 is a person who walks (`0BB6:05B0`); the handler keeps his
//! record in `[18FEh]`/`[1900h]`, and the frame loop waits for him to reach
//! node 0CAh.

use super::logic::Logic;
use super::story;
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The person who walks, object 204, and the object 202 he becomes.
const WALKER_ID: u16 = 0xCC;
const FIGURE: u16 = 0xCA;
/// `[18FEh]`, `[1900h]`: offset and segment of his record.
const RECORD_OFF: u16 = 0x18FE;
const RECORD_SEG: u16 = 0x1900;
/// `[1902h]`: where his walk stands (`FFFFh`, 0, 1, 2, 0C9h, `FFFEh`).
const WALK: u16 = 0x1902;
/// `[1908h]` whether 202 speaks, `[1906h]` his next animation, `[1904h]`
/// the one he plays.
const SPEAKS: u16 = 0x1908;
const NEXT: u16 = 0x1906;
const PLAYING: u16 = 0x1904;
/// `[271Bh]`: the chapter's state; `[2724h]` its scene counter.
const STATE: u16 = 0x271B;
const COUNTER: u16 = 0x2724;
/// `[2725h]`, `[2726h]`: the Wohnzimmer's state and sub-state.
const HOME_STATE: u16 = 0x2725;
const HOME_SUBSTATE: u16 = 0x2726;
/// `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`, `DS:1F56h`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
const HERE: u16 = 0x1F4C;
const OBJECT_STATES: u16 = 0x1F56;

/// The record in `[18FEh]`/`[1900h]`.
fn record(api: &dyn Api) -> Far {
    Far {
        off: api.word(RECORD_OFF),
        seg: api.word(RECORD_SEG),
    }
}

/// Everything before the frame loop, `0x2BD1` to `0x2C71`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup();
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x2BDD
    api.visit_enter(loc, entry); // 0x2BE1
    let rec = api.mover_find(WALKER_ID); // 0x2BEC
    api.set_word(RECORD_SEG, rec.seg);
    api.set_word(RECORD_OFF, rec.off);
    // 0x2BF9: by [271Bh] through the table at cs:0209h.
    match api.byte(STATE) {
        0 | 2 | 7 => {} // 0x2C20
        3 => {
            api.obj_show(0xC9); // 0x2C0C, into 0x2C17
        }
        _ => {
            // 0x2C0E
            api.obj_wake(FIGURE);
            api.obj_show(0xC9);
        }
    }
    if api.byte(HOME_STATE) == 7 {
        // 0x2C2C
        api.scene_play(9);
        api.set_byte(HOME_SUBSTATE, 0);
        api.set_byte(HOME_STATE, 9);
        api.set_byte(COUNTER, 0);
        api.set_byte(STATE, 3);
        api.obj_hide(FIGURE);
        api.set_word(WALK, 0xFFFF);
        api.set_byte(0x2703, 0x28);
    }
    if api.byte(0x2713) == 1 {
        api.obj_show(0xC8); // 0x2C63
    }
    api.visit_finish_entry(); // 0x2C6C
    api.set_word(ENTRY, 0); // 0x2C71
}

/// One turn of the frame loop, `0x2C77` to `0x2CB9`: once he has walked
/// (`[1902h]` above 1) and stands on node 0CAh, task 0 starts.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    if num::signed(api.word(WALK)) > 0 {
        // 0x2C88
        let rec = record(api);
        let node = api.far_word(Far {
            off: rec.off.wrapping_add(0x28),
            seg: rec.seg,
        });
        if node == 0xCA && num::signed(api.word(WALK)) > 1 {
            // 0x2C9B
            api.set_word(SPEAKS, 1);
            api.set_word(WALK, 0xFFFE);
            api.task_start(0);
        }
    }
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x2CBB` to `0x2CD8`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `02AD:0219`: codes 0 to 10 through the table at cs:0372h.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x2D01: speaker bit 1 is 202, unless he is walking.
            let speaks = if api.word(WALK) == 0 { arg & 2 } else { 0 };
            api.set_word(SPEAKS, speaks);
            api.set_word(PLAYING, 0); // 0x2D18
            api.set_word(NEXT, 0);
        }
        5 | 6 => {
            // 0x2D23
            if api.word(SPEAKS) != 0 {
                api.set_word(PLAYING, 0);
                api.set_word(NEXT, 0);
                api.set_word(SPEAKS, 0);
                api.obj_anim_id(FIGURE, 0);
            }
        }
        8 => api.set_word(NEXT, arg.wrapping_add(1)), // 0x2D47
        9 | 10 => {
            // 0x2D4C
            if api.word(SPEAKS) == 0 || api.word(PLAYING) == api.word(NEXT) {
                return;
            }
            let next = api.word(NEXT);
            api.set_word(PLAYING, next);
            if api.word(WALK) == 0xFFFE {
                // 0x2D69: he stops walking and is 202 again.
                let rec = api.obj_show(FIGURE);
                api.obj_anim(rec, num::signed(next));
                let walker = record(api);
                api.walker_unlink(walker);
            } else {
                api.obj_anim_id(FIGURE, num::signed(next)); // 0x2D96
            }
        }
        7 => event(api, arg), // 0x2DAB
        _ => {}
    }
}

/// Code 7, `0x2DAB`: script events 0Ah to 0Ch.
fn event(api: &mut dyn Api, n: u16) {
    match n {
        0x0A => {
            // 0x2DBF: the player to 202, and he walks from 0C9h to 0CAh.
            api.walk_to_node(FIGURE);
            api.set_word(WALK, 0xC9); // 0x2DCD
            let rec = record(api);
            api.walker_bind_graph(rec, 1, 0xC9, 1); // 0x2DDB
            let rec = record(api);
            api.walker_walk_to(rec, FIGURE, 1); // 0x2DF0
            api.set_word(WALK, 1);
        }
        0x0B => {
            // 0x2E00
            if num::signed(api.word(WALK)) > 0 {
                api.task_arm(0);
                api.set_word(WALK, 2);
            } else {
                api.set_word(WALK, 0xFFFE); // 0x2E17
                api.set_word(SPEAKS, 1);
            }
            api.set_word(NEXT, 3); // 0x2E23
            api.inv_give(7);
        }
        0x0C => {
            // 0x2E32
            api.set_word(NEXT, 3);
            api.inv_take(0x14);
        }
        _ => {}
    }
}

/// `02AD:0001`, also called near from `0x2B6A`.
fn rule_0001(api: &mut dyn Api) -> i16 {
    api.scene_play(0x0B);
    story::routine_06b1(api); // 0x2ADC
    api.set_byte(COUNTER, 0);
    api.set_byte(STATE, 7);
    api.set_byte(0x271D, 0);
    api.set_byte(0x271F, 0);
    api.set_byte(0x2716, 3);
    0x0A
}

/// The Schule's rule handlers by their offset in segment `02ADh`. Answers
/// what the original answers the rule search with, or `None` for an
/// offset that is not one of them.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        0x0001 => rule_0001(api),
        // 02AD:002F (0x2AFF): by [271Bh] - 1 through the table at cs:00A9h.
        0x002F => match api.byte(STATE).wrapping_sub(1) {
            0 => {
                // 0x2B16
                api.scene_play(0);
                api.set_byte(COUNTER, 0);
                api.set_byte(STATE, 2);
                api.set_byte(0x2705, 1);
                0x0A
            }
            3 => {
                // 0x2B2F
                let counter = api.ds(COUNTER);
                api.scene_sequence(counter, &[4, 5, 6, 7]);
                api.set_byte(0x2703, 1);
                if api.byte(0x272B) != 2 {
                    api.set_byte(0x272C, 0); // 0x2B51
                    api.set_byte(0x272B, 2);
                }
                0x0A
            }
            4 => {
                api.scene_play(0x0A); // 0x2B5D
                0x0A
            }
            5 => {
                // 0x2B66: `push cs ; call 0001`, its answer dropped.
                rule_0001(api);
                0x0A
            }
            _ => 0, // 0x2B70
        },
        // 02AD:00B5 (0x2B85)
        0x00B5 => {
            if api.byte(STATE) == 3 {
                let n = api.random(1, 3);
                api.scene_play(num::signed(n));
                0x0A
            } else {
                0
            }
        }
        // 02AD:00DD (0x2BAD): on to location 12, entry 64h.
        0x00DD => {
            api.set_word(LOCATION, 0x0C);
            api.set_word(ENTRY, 0x64);
            1
        }
        // 02AD:00F1 (0x2BC1)
        0x00F1 => {
            api.scene_play(0x0D);
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
    anim_hook: None,
    rule,
};
