//! Locations 8, 24, 25 and 26, the JZ: one handler serves all four (the
//! location table names them `JZ`, `JZ(1)`, `JZ(2)` and `JZ(3)` and gives
//! each the same handler, hook and animation hook; none has rules of its
//! own). The module is named after the location as the game itself names
//! it in its location table (`JZ`).
//!
//! Re-expressed from the handler at `01AD:000F` (image `0x1ADF`), its hook
//! at `01AD:035F` (`0x1E2F`) and its animation hook at `01AD:0337`
//! (`0x1E07`), against [`Api`] alone; the story routine `1586:06B1` both
//! the handler and the hook call is [`story::routine_06b1`]. The handler's
//! prologue is the standard one.

use super::logic::Logic;
use super::story;
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 200, the one the hook animates while bit 1 of a dialogue's
/// speakers is set.
const OBJECT_200: u16 = 0xC8;
/// Object 207, the same for bit 2.
const OBJECT_207: u16 = 0xCF;

/// `[271Ch]`: the location state the handler's arms are chosen by.
const STATE: u16 = 0x271C;
/// `[272Ah]`: a counter the state-6 arm steps.
const COUNTER: u16 = 0x272A;
/// `[2714h]`: tested for 4 by the state-7 arm.
const BYTE_2714: u16 = 0x2714;
/// `[18CCh]` whether object 200 speaks, `[18C8h]` its next animation,
/// `[18CAh]` the one it is playing.
const SPEAKS_200: u16 = 0x18CC;
const NEXT_200: u16 = 0x18C8;
const PLAYING_200: u16 = 0x18CA;
/// `[18C6h]` whether object 207 speaks, `[18C2h]` its next animation,
/// `[18C4h]` the one it is playing.
const SPEAKS_207: u16 = 0x18C6;
const NEXT_207: u16 = 0x18C2;
const PLAYING_207: u16 = 0x18C4;
/// `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`, `DS:1F56h`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
const HERE: u16 = 0x1F4C;
const OBJECT_STATES: u16 = 0x1F56;

/// Everything before the frame loop, `0x1ADF` to `0x1DBB`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x1ADF
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x1AEB
    api.visit_enter(loc, entry); // 0x1AEF
    by_state(api); // 0x1AF7
    // 0x1D89: the five objects every arm ends with.
    for id in [0xCB, 0xCD, 0xCE, 0xD1, 0xD2] {
        api.obj_wake(id);
    }
    api.visit_finish_entry(); // 0x1DB6
    api.set_word(ENTRY, 0); // 0x1DBB
}

/// `0x1AF7`: what the visit does by the location's state byte, nine arms
/// through the table at `0x1DF5`; above 8, none.
fn by_state(api: &mut dyn Api) {
    match api.byte(STATE) {
        0 => {
            // 0x1B0D
            api.scene_play(0);
            api.set_byte(COUNTER, 0);
            api.set_byte(STATE, 1);
            api.set_byte(0x271E, 0);
            api.set_byte(0x2715, 0x80);
            api.set_byte(0x271F, 0);
            api.set_byte(0x2716, 0x80);
            api.set_byte(0x2726, 0);
            api.set_byte(0x2725, 9);
            api.set_byte(0x26FC, 1);
            api.set_byte(0x2701, 1);
            api.set_byte(0x2704, 1);
            api.set_byte(0x2707, 1);
            api.set_byte(0x2730, 0x10);
        }
        1 => {
            // 0x1B59: scene 2 with item 1 carried, else scene 1.
            let n = if api.inv_has(1) != 0 { 2 } else { 1 };
            api.scene_play(n); // 0x1B6B
            wake_both(api); // 0x1B71
        }
        2 => {
            // 0x1B74
            api.obj_hide(0xD6);
            api.scene_play(3);
            api.set_byte(0x271F, 0);
            api.set_byte(0x2716, 1);
            api.set_byte(COUNTER, 0);
            api.set_byte(STATE, 3);
            api.set_byte(0x2712, 3);
            api.set_byte(0x26FE, 6);
            story::routine_06b1(api); // 0x1BA3
            wake_both(api); // 0x1BA8
        }
        3 => {
            // 0x1BAB
            api.obj_hide(0xD6);
            api.obj_hide(0xD7);
            api.scene_play(5);
            wake_both(api); // 0x1BC5
        }
        4 => {
            // 0x1BC8: scene 7 with item 8 carried, else scene 6.
            api.obj_hide(0xD8);
            let n = if api.inv_has(8) != 0 { 7 } else { 6 };
            api.scene_play(n); // 0x1BE3
            wake_both(api); // 0x1BE9
        }
        5 => {
            // 0x1BEC: scene 8 with item 15 carried, else a rolled one.
            let n = if api.inv_has(0x0F) != 0 {
                8
            } else {
                rolled_scene(api) // 0x1BFC
            };
            api.scene_play(n); // 0x1C1F
            wake_rolled(api); // 0x1C25
        }
        6 => {
            // 0x1C55: the counter stepped; 1 and 2 hide objects 218 and
            // 219, 3 hides 219, anything else is held at 4.
            let c = api.byte(COUNTER).wrapping_add(1);
            api.set_byte(COUNTER, c);
            match c {
                1 | 2 => {
                    api.obj_hide(0xDA); // 0x1C70
                    api.obj_hide(0xDB); // 0x1C79
                }
                3 => {
                    api.obj_hide(0xDB); // 0x1C79
                }
                _ => api.set_byte(COUNTER, 4), // 0x1C84
            }
            // 0x1C89: scene 14 with item 7 carried, else object 217 hidden
            // and a rolled scene.
            let n = if api.inv_has(7) != 0 {
                0x0E
            } else {
                api.obj_hide(0xD9); // 0x1C99
                rolled_scene(api)
            };
            api.scene_play(n); // 0x1CC5
            wake_rolled(api); // 0x1CCB
        }
        7 => {
            // 0x1CF6
            if api.byte(BYTE_2714) == 4 {
                api.scene_play(0x0F); // 0x1CFD
                api.set_byte(0x272E, 0);
                api.set_byte(0x272D, 5);
                api.set_byte(0x2700, 1);
                api.set_byte(BYTE_2714, 1);
            } else {
                let n = rolled_scene(api); // 0x1D1B
                api.scene_play(n); // 0x1D3E
            }
            wake_rolled(api); // 0x1D44
        }
        8 => {
            // 0x1D7D
            api.set_word(LOCATION, 0xFFFF);
            api.set_word(ENTRY, 1);
        }
        _ => {}
    }
}

/// `0x1BFC` (and `0x1CA2`, `0x1D1B`): one of scenes 19 to 22 or, on the
/// first roll's 0, one of scenes 9 to 12.
fn rolled_scene(api: &mut dyn Api) -> i16 {
    let n = if api.random(0, 1) != 0 {
        api.random(0x13, 0x16) // 0x1C0C
    } else {
        api.random(9, 0x0C) // 0x1C12
    };
    num::signed(n)
}

/// `0x1C25` (and `0x1CCB`, `0x1D44`): a roll of 1 to 3 wakes object 212
/// (1), object 213 (2) or both (3).
fn wake_rolled(api: &mut dyn Api) {
    match api.random(1, 3) {
        1 => {
            api.obj_wake(0xD4); // 0x1C49, through 0x1D75
        }
        2 => {
            api.obj_wake(0xD5); // 0x1D72
        }
        3 => wake_both(api), // 0x1D69
        _ => {}
    }
}

/// `0x1D69`: objects 212 and 213 woken.
fn wake_both(api: &mut dyn Api) {
    api.obj_wake(0xD4);
    api.obj_wake(0xD5); // 0x1D72
}

/// One turn of the frame loop, `0x1DC1` to `0x1DD5`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x1DD7` to `0x1DF4`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `01AD:035F`: codes 0 to 9 through the table at `0x20AE`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x1E49: a dialogue comes up; the player walks by speaker
            // bits 1 and 2 (the table at 0x20A0), bit 1 is object 200,
            // bit 2 object 207.
            let node = match arg & 6 {
                0 => 0x0BBA, // 0x1E5A
                2 => OBJECT_200,
                4 => OBJECT_207,
                _ => 0x0BB8, // 0x1E69
            };
            api.walk_to_node(node); // 0x1E6C
            api.set_word(SPEAKS_200, arg & 2); // 0x1E77
            if api.word(SPEAKS_200) != 0 {
                api.obj_wake(OBJECT_200); // 0x1E84
            }
            api.set_word(SPEAKS_207, arg & 4); // 0x1E8F
            if api.word(SPEAKS_207) != 0 {
                api.obj_wake(OBJECT_207); // 0x1E9C
            }
            api.set_word(NEXT_200, 0); // 0x1EA4
            api.set_word(PLAYING_200, 0);
            api.set_word(NEXT_207, 0);
            api.set_word(PLAYING_207, 0);
        }
        5 | 6 => {
            // 0x1EB3: whoever spoke goes back to animation 0.
            if api.word(SPEAKS_200) != 0 {
                api.set_word(SPEAKS_200, 0); // 0x1EBC
                api.set_word(NEXT_200, 0);
                api.set_word(PLAYING_200, 0);
                api.obj_anim_id(OBJECT_200, 0); // 0x1EC9
            }
            if api.word(SPEAKS_207) != 0 {
                api.set_word(SPEAKS_207, 0); // 0x1EDD
                api.set_word(NEXT_207, 0);
                api.set_word(PLAYING_207, 0); // 0x1F26
                api.obj_anim_id(OBJECT_207, 0); // 0x1F2D
            }
        }
        8 => {
            // 0x1EE5
            let next = arg.wrapping_add(1);
            api.set_word(NEXT_200, next); // 0x2073
            api.set_word(NEXT_207, next);
        }
        9 => {
            // 0x1EEB: a page's end — each speaker's next animation.
            if api.word(SPEAKS_200) != 0 && api.word(PLAYING_200) != api.word(NEXT_200) {
                let next = api.word(NEXT_200); // 0x1EFB
                api.set_word(PLAYING_200, next);
                api.obj_anim_id(OBJECT_200, num::signed(next));
            }
            if api.word(SPEAKS_207) != 0 && api.word(PLAYING_207) != api.word(NEXT_207) {
                let next = api.word(NEXT_207); // 0x1F23
                api.set_word(PLAYING_207, next);
                api.obj_anim_id(OBJECT_207, num::signed(next));
            }
        }
        7 => event(api, arg), // 0x1F38
        _ => {}
    }
}

/// Code 7, `0x1F38`: script events through the value table at `0x207C`.
fn event(api: &mut dyn Api, n: u16) {
    match n {
        0x0A => {
            // 0x1F55
            take_item(api, 1);
            api.set_byte(0x26FE, 2);
            api.set_byte(0x272F, 0);
            api.set_byte(0x2729, 1);
            api.set_byte(COUNTER, 0);
            api.set_byte(STATE, 3);
            api.set_byte(0x2712, 1);
        }
        0x0B => {
            // 0x1F7C
            take_item(api, 0x0C);
            api.set_byte(COUNTER, 0);
            api.set_byte(STATE, 4);
            if api.byte(0x26FB) != 0 {
                api.set_byte(0x26FB, 1); // 0x1F93
            }
            api.set_byte(0x2730, 0x11);
        }
        0x0C => {
            // 0x1FA0
            take_item(api, 8);
            story::routine_06b1(api); // 0x1FA6
            api.set_byte(0x271E, 0);
            api.set_byte(0x2715, 3);
            api.set_byte(0x271F, 0);
            api.set_byte(0x2716, 2);
            api.set_byte(0x2726, 0);
            api.set_byte(0x2725, 3);
            api.set_byte(0x2722, 0);
            api.set_byte(0x2719, 8);
            api.set_byte(COUNTER, 0);
            api.set_byte(STATE, 5);
        }
        0x0D => {
            // 0x1FE0
            take_item(api, 0x0F);
            api.set_byte(COUNTER, 0);
            api.set_byte(STATE, 6);
            api.set_byte(0x272E, 0);
            api.set_byte(0x272D, 2);
            api.set_byte(0x2730, 0x12);
        }
        0x0E => {
            // 0x2001
            take_item(api, 7);
            api.set_byte(0x271D, 1);
            api.set_byte(0x26FE, 6);
            api.set_byte(0x2700, 1);
            api.set_byte(0x272E, 0);
            api.set_byte(0x272D, 3);
            api.set_byte(COUNTER, 0);
            api.set_byte(STATE, 7);
        }
        0x0F => take_item(api, 0x0A),      // 0x202C
        0x10 => api.set_word(NEXT_200, 5), // 0x2034
        0x11 => {
            // 0x203C: the record that walks for id 202 bound to graph 1 on
            // node 300, then walked to node 301.
            let rec = api.mover_find(0xCA);
            api.walker_bind_graph(rec, 1, 0x12C, 1); // 0x2054
            api.walker_walk_to(rec, 0x12D, 1); // 0x2067
        }
        0x6E => {
            // 0x2071
            api.set_word(NEXT_200, 0);
            api.set_word(NEXT_207, 0);
        }
        _ => {}
    }
}

/// `01AD:033C` (`0x1E0C`), a near helper: the item taken away, and
/// animation 4 next for object 200 while it speaks, else for object 207.
fn take_item(api: &mut dyn Api, id: u16) {
    api.inv_take(id); // 0x1E12
    if api.word(SPEAKS_200) != 0 {
        api.set_word(NEXT_200, 4); // 0x1E1F
    } else {
        api.set_word(NEXT_207, 4); // 0x1E27
    }
}

/// The animation hook, `01AD:0337`: an empty routine.
pub(crate) fn anim_hook(_api: &mut dyn Api, _rec: Far, _code: u16) {}

/// The JZ has no rules of its own.
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
