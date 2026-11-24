//! Location 13, the Werkstatt. The module is named after the location as
//! the game itself names it in its location table (`Werkstatt`).
//!
//! Re-expressed from the handler at `02FD:001E` (image `0x2FEE`), its hook
//! at `02FD:01D9` (`0x31A9`), its animation hook at `02FD:01D4` (`0x31A4`)
//! and its rule handler at `02FD:000E`, against [`Api`] alone.
//!
//! Object 201 is a person who walks (`0BB6:05B0`); script event 0Ch sends
//! him from node 12Ch to 12Dh and keeps his record in `[190Ah]`/`[190Ch]`,
//! and the frame loop waits for his walk to end.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 129, the one who speaks with 201; object 202, what 201 becomes.
const SPEAKER: u16 = 0x81;
const FIGURE: u16 = 0xCA;
/// `[190Ah]`, `[190Ch]`: offset and segment of 201's record.
const RECORD_OFF: u16 = 0x190A;
const RECORD_SEG: u16 = 0x190C;
/// `[191Ch]`: 1 while the frame loop waits for his walk.
const WAITING: u16 = 0x191C;
/// `[191Ah]` the object the second speaker is, `[1918h]` whether it
/// speaks, `[1916h]` its next animation, `[1914h]` the one it plays.
const SECOND: u16 = 0x191A;
const SECOND_SPEAKS: u16 = 0x1918;
const SECOND_NEXT: u16 = 0x1916;
const SECOND_PLAYING: u16 = 0x1914;
/// `[1912h]` object 129's base animation while he speaks, `[1910h]` what
/// event 8 adds, `[190Eh]` the one he plays.
const BASE: u16 = 0x1912;
const STEP: u16 = 0x1910;
const PLAYING: u16 = 0x190E;
/// `[272Bh]`, `[272Ch]`: the Werkstatt's state and sub-state.
const STATE: u16 = 0x272B;
const SUBSTATE: u16 = 0x272C;

/// The record in `[190Ah]`/`[190Ch]`.
fn record(api: &dyn Api) -> Far {
    Far {
        off: api.word(RECORD_OFF),
        seg: api.word(RECORD_SEG),
    }
}

/// `0x3113` on: the chapter state after a scene.
fn chapter(api: &mut dyn Api, state: u8) {
    api.set_byte(SUBSTATE, 0);
    api.set_byte(STATE, 1);
    api.set_byte(0x2724, 0);
    api.set_byte(0x271B, state);
}

/// Everything before the frame loop, `0x2FEE` to `0x3118`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup();
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x2FFA
    api.visit_enter(loc, entry); // 0x2FFE
    api.set_word(SECOND, 0); // 0x3006
    match api.byte(STATE) {
        0 => {
            // 0x301D
            api.obj_wake(SPEAKER);
            api.walk_to_node(SPEAKER);
            api.scene_play(0);
            chapter(api, 4);
        }
        2 => {
            // 0x304E: by the sub-state.
            match api.byte(SUBSTATE) {
                0 => {
                    // 0x3064
                    api.set_byte(SUBSTATE, 1);
                    api.obj_wake(0xC8);
                    api.walk_to_node(0xC8);
                    api.scene_play(4);
                }
                1 => {
                    // 0x307F
                    api.set_byte(SUBSTATE, 2);
                    api.obj_wake(SPEAKER);
                    api.walk_to_node(SPEAKER);
                    api.scene_play(5);
                }
                2 => {
                    // 0x3099
                    api.obj_wake(0xC8);
                    api.walk_to_node(0xC8);
                    api.scene_play(6);
                    chapter(api, 3);
                }
                _ => {}
            }
        }
        _ => {
            // 0x30C9
            api.obj_wake(SPEAKER);
            if api.byte(0x2714) == 4 {
                // 0x30D9
                api.walk_to_node(SPEAKER);
                api.scene_play(7);
                api.set_byte(0x272E, 0);
                api.set_byte(0x272D, 5);
                api.set_byte(0x2700, 1);
                api.set_byte(0x2714, 1);
            } else {
                let n = api.random(1, 3); // 0x3100
                api.scene_play(num::signed(n));
            }
        }
    }
    api.visit_finish_entry(); // 0x3113
    api.set_word(ENTRY, 0); // 0x3118
}

/// One turn of the frame loop, `0x311E` to `0x3184`: while it waits and
/// 201's record has a node again (not `FFFFh`), he becomes object 202 and
/// task 0 starts.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    if api.word(WAITING) == 1 {
        // 0x3130
        let rec = record(api);
        let node = api.far_word(Far {
            off: rec.off.wrapping_add(0x28),
            seg: rec.seg,
        });
        if node != 0xFFFF {
            // 0x313B
            api.set_word(WAITING, 0);
            api.walker_unlink(rec);
            let next = api.word(SECOND_NEXT); // 0x314E
            api.set_word(SECOND_PLAYING, next);
            api.set_word(SECOND, FIGURE);
            let shown = api.obj_show(FIGURE);
            api.obj_anim(shown, num::signed(next)); // 0x3164
            api.set_word(SECOND_SPEAKS, 1);
            api.task_start(0);
        }
    }
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x3186` to `0x31A3`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `02FD:01D9`: codes 0 to 9 through the table at cs:0359h.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x31C1: speaker bit 1 is the second speaker, bit 2 object 129.
            api.set_word(SECOND_SPEAKS, arg & 2);
            api.set_word(BASE, u16::from(arg & 4 != 0));
            api.set_word(SECOND_PLAYING, 0); // 0x31DB
            api.set_word(SECOND_NEXT, 0);
            api.set_word(PLAYING, 0); // 0x31E3
            api.set_word(STEP, 0);
        }
        5 | 6 => {
            // 0x31EC: whoever spoke goes back to standing.
            if api.word(SECOND_SPEAKS) != 0 {
                api.set_word(SECOND_NEXT, 0);
                api.set_word(SECOND_PLAYING, 0);
                api.set_word(SECOND_SPEAKS, 0);
                let second = api.word(SECOND);
                api.obj_anim_id(second, 0); // 0x3203
            }
            let base = api.word(BASE);
            if base != 0 {
                // 0x3215
                api.obj_anim_id(SPEAKER, num::signed(base.wrapping_sub(1)));
                api.set_word(PLAYING, 0);
                api.set_word(STEP, 0);
                api.set_word(BASE, 0);
            }
        }
        8 => {
            // 0x3233
            api.set_word(STEP, arg);
            api.set_word(SECOND_NEXT, arg.wrapping_add(1));
        }
        9 => {
            // 0x323F: a page's end — each speaker's next animation.
            if api.word(SECOND_SPEAKS) != 0 && api.word(SECOND_PLAYING) != api.word(SECOND_NEXT) {
                let next = api.word(SECOND_NEXT); // 0x324F
                api.set_word(SECOND_PLAYING, next);
                let second = api.word(SECOND);
                api.obj_anim_id(second, num::signed(next));
            }
            let base = api.word(BASE);
            if base != 0 {
                let anim = base.wrapping_add(api.word(STEP)); // 0x326C
                if anim != api.word(PLAYING) {
                    api.set_word(PLAYING, anim); // 0x327C
                    api.obj_anim_id(SPEAKER, num::signed(anim));
                }
            }
        }
        7 => event(api, arg), // 0x3295
        _ => {}
    }
}

/// Code 7, `0x3295`: script events 0Ah to 11h through the table at
/// cs:0349h.
fn event(api: &mut dyn Api, n: u16) {
    match n.wrapping_sub(0x0A) {
        0 => api.set_word(BASE, 6), // 0x32A6
        1 => api.set_word(BASE, 1), // 0x32AE
        2 => {
            // 0x32B6: 201 bound to graph 1 on node 12Ch, walking to 12Dh.
            let rec = api.mover_find(0xC9);
            api.set_word(RECORD_SEG, rec.seg);
            api.set_word(RECORD_OFF, rec.off);
            api.walker_bind_graph(rec, 1, 0x12C, 3); // 0x32CF
            let rec = record(api);
            api.walker_walk_to(rec, 0x12D, 3); // 0x32E4
        }
        3 => {
            // 0x32EE
            api.set_word(WAITING, 1);
            api.task_arm(0);
        }
        4 => api.script_exit(0x6D), // 0x32FD
        5 => {
            api.inv_give(0x0E); // 0x3306
        }
        7 => {
            api.walk_to_node(0x6D); // 0x330F
        }
        _ => {}
    }
}

/// The animation hook, `02FD:01D4`: it does nothing (`push bp ; mov bp,
/// sp ; pop bp ; retf`).
pub(crate) fn anim_hook(_api: &mut dyn Api, _rec: Far, _code: u16) {}

/// The Werkstatt's rule handler by its offset in segment `02FDh`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    match off {
        // 02FD:000E (0x2FDE)
        0x000E => {
            api.scene_play(0);
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
