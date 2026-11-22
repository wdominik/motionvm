//! Location 9, the Laden: the shop, with a second record that walks (id
//! 131) besides the player. The module is named after the location as the
//! game itself names it in its location table (`Laden`).
//!
//! Re-expressed from the handler at `020C:0071` (image `0x2131`), its
//! rule handler at `020C:0002` (`0x20C2`), its hook at `020C:0313`
//! (`0x23D3`) and its animation hook at `020C:0296` (`0x2356`), against
//! [`Api`] alone.
//!
//! The prologue differs from the standard one: with `[2728h]` at 3 the
//! handler sets the entry word `[1F48h]` to 6Ch before it hands it to the
//! shared entry (`0x2141`).

use super::logic::Logic;
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 200, animated while bit 0 of a dialogue's speakers is set.
const OBJECT_200: u16 = 0xC8;
/// Object 202; the record that walks for id 131 is bound and walked in its
/// place.
const OBJECT_202: u16 = 0xCA;

/// `[2728h]`: the location state.
const STATE: u16 = 0x2728;
/// `DS:2731h`: the counter the rule's scene sequence steps.
const SEQUENCE: u16 = 0x2731;
/// `[18CEh]`:`[18D0h]`: the record that walks for id 131, as the handler
/// found it (`0x2179`).
const MOVER_OFF: u16 = 0x18CE;
const MOVER_SEG: u16 = 0x18D0;
/// `[18D2h]`: the node the frame loop waits for someone to reach.
const AWAITED: u16 = 0x18D2;
/// `[18D8h]`: object 202's base animation, `[18D4h]` what code 8 adds,
/// `[18D6h]` the one it is playing.
const BASE_202: u16 = 0x18D8;
const STEP_202: u16 = 0x18D4;
const PLAYING_202: u16 = 0x18D6;
/// `[18DEh]` whether object 200 speaks, `[18DAh]` its next animation,
/// `[18DCh]` the one it is playing.
const SPEAKS_200: u16 = 0x18DE;
const NEXT_200: u16 = 0x18DA;
const PLAYING_200: u16 = 0x18DC;
/// `DS:1693h`, the player's walker record, and `[16BBh]`, its `+28h`
/// node id.
const WALKER: u16 = 0x1693;
const WALKER_NODE: u16 = 0x16BB;
/// `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`, `DS:1F56h`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
const HERE: u16 = 0x1F4C;
const OBJECT_STATES: u16 = 0x1F56;

/// Handler locals 0 and 1: `[bp-4]`:`[bp-2]`, object 202's record as the
/// entry found it (`0x216A`); the frame loop writes its words `+4` and `+6`
/// through it (`0x2219`). The handler keeps it on its stack for the visit.
const RECORD_202_OFF: usize = 0;
const RECORD_202_SEG: usize = 1;

/// Everything before the frame loop, `0x2131` to `0x21C1`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x2135
    if api.byte(STATE) == 3 {
        api.set_word(ENTRY, 0x6C); // 0x2141
    }
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x214E
    api.visit_enter(loc, entry); // 0x2152
    api.obj_anim_id(OBJECT_200, 0); // 0x215F
    let rec = api.obj_find(OBJECT_202); // 0x216A
    api.set_local(RECORD_202_OFF, rec.off);
    api.set_local(RECORD_202_SEG, rec.seg);
    let mover = api.mover_find(0x83); // 0x2179
    api.set_word(MOVER_SEG, mover.seg); // 0x217F
    api.set_word(MOVER_OFF, mover.off);
    if api.byte(STATE) == 3 {
        // 0x2192
        api.obj_wake(OBJECT_202);
        api.obj_show(0xC9);
        api.set_word(BASE_202, 1);
        api.set_byte(SEQUENCE, 0);
        api.set_byte(STATE, 1);
        api.scene_play(2); // 0x21B6
    }
    api.visit_finish_entry(); // 0x21BC
    api.set_word(ENTRY, 0); // 0x21C1
}

/// One turn of the frame loop, `0x21C7` to `0x232A`: while `[18D2h]`
/// names a node, the frame waits for the walking record (first) or the
/// player (else) to stand on it.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x21C7
    api.dispatch(); // 0x21CC
    let awaited = api.word(AWAITED);
    if awaited != 0 {
        // 0x21DB: the walking record's +28h, the node it stands on.
        let m = mover(api);
        let node = api.far_word(Far {
            off: m.off.wrapping_add(0x28),
            seg: m.seg,
        });
        if node == awaited {
            mover_arrived(api, awaited);
        } else if awaited == api.word(WALKER_NODE) {
            // 0x22E0: the player arrived.
            match awaited {
                0x6D => {
                    api.obj_anim_id(0x6D, 0); // 0x22FA
                    api.set_word(AWAITED, 0); // 0x2318
                }
                0xC8 => {
                    api.face(3); // 0x2308
                    api.task_start(0); // 0x2310
                    api.set_word(AWAITED, 0);
                }
                0x0BB8 => {
                    api.task_start(0); // 0x2310
                    api.set_word(AWAITED, 0);
                }
                _ => {}
            }
        }
    }
    api.frame_pass(); // 0x231E
    api.word(ENTRY) == 0
}

/// `0x21EC`: the walking record stands on the awaited node, 300 to 304
/// through the table at `0x234C`.
fn mover_arrived(api: &mut dyn Api, node: u16) {
    match node.wrapping_sub(0x12C) {
        0 => {
            // 0x2203: the record unlinked; words +4 and +6 of object 202's
            // record set to 99h and 2Fh, and the object shown on
            // animation 7.
            api.set_word(BASE_202, 5);
            let m = mover(api);
            api.walker_unlink(m); // 0x2211
            let rec = Far {
                off: api.local(RECORD_202_OFF),
                seg: api.local(RECORD_202_SEG),
            };
            api.set_far_word(
                Far {
                    off: rec.off.wrapping_add(4),
                    seg: rec.seg,
                },
                0x99,
            ); // 0x221C
            api.set_far_word(
                Far {
                    off: rec.off.wrapping_add(6),
                    seg: rec.seg,
                },
                0x2F,
            ); // 0x2222
            let shown = api.obj_show(OBJECT_202); // 0x222D
            api.obj_anim(shown, 7); // 0x2235
            api.set_word(AWAITED, 0); // 0x223F
            api.task_start(0); // 0x2243
        }
        1 => {
            // 0x224C
            api.set_word(AWAITED, 0x12E);
            let m = mover(api);
            api.walker_walk_to(m, 0x12E, 3); // 0x225D
            api.face(3); // 0x2267
        }
        2 => {
            // 0x2270
            api.set_word(AWAITED, 0x12F);
            let m = mover(api);
            api.walker_walk_to(m, 0x12F, 3); // 0x2281
        }
        3 => {
            // 0x228C
            api.obj_anim_id(0x6D, 1);
            api.set_word(AWAITED, 0x130);
            let m = mover(api);
            api.walker_walk_to(m, 0x130, 3); // 0x22A9
            api.walk_to_node(0xC8); // 0x22B4
        }
        4 => {
            // 0x22BC
            let m = mover(api);
            api.walker_unlink(m); // 0x22C4
            api.obj_anim_id(0x6D, 0); // 0x22D0
            api.set_word(AWAITED, 0xC8); // 0x22D8
        }
        _ => {}
    }
}

/// Everything after the frame loop, `0x232D` to `0x234B`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

fn mover(api: &dyn Api) -> Far {
    Far {
        off: api.word(MOVER_OFF),
        seg: api.word(MOVER_SEG),
    }
}

/// The hook, `020C:0313`: codes 0 to 10 through the table at `0x25D4`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x23EC
            api.set_word(SPEAKS_200, arg & 1);
            api.set_word(NEXT_200, 0);
            api.set_word(PLAYING_200, 0);
            api.set_word(STEP_202, 0); // 0x23FC
            api.set_word(PLAYING_202, 0);
        }
        5 | 6 => {
            // 0x2407
            if api.word(SPEAKS_200) != 0 {
                api.set_word(SPEAKS_200, 0); // 0x2411
                api.set_word(NEXT_200, 0);
                api.set_word(PLAYING_200, 0);
                api.obj_anim_id(OBJECT_200, 0); // 0x2479
            }
        }
        8 => {
            // 0x2422
            if api.word(SPEAKS_200) != 0 {
                api.set_word(NEXT_200, arg.wrapping_add(1)); // 0x2429
            }
            api.set_word(STEP_202, arg); // 0x242F
        }
        9 | 10 => {
            // 0x2436: a page's end.
            if api.word(PLAYING_200) != api.word(NEXT_200) {
                let next = api.word(NEXT_200); // 0x243F
                api.set_word(PLAYING_200, next);
                api.obj_anim_id(OBJECT_200, num::signed(next));
            }
            let base = api.word(BASE_202); // 0x2451
            if base != 0 {
                let anim = base.wrapping_add(api.word(STEP_202)); // 0x245B
                if anim != api.word(PLAYING_202) {
                    api.set_word(PLAYING_202, anim); // 0x246B
                    api.obj_anim_id(OBJECT_202, num::signed(anim));
                }
            }
        }
        7 => event(api, arg), // 0x2484
        _ => {}
    }
}

/// Code 7, `0x2484`: script events through the value table at `0x25A8`.
fn event(api: &mut dyn Api, n: u16) {
    match n {
        0x0A => {
            api.inv_give(0x16); // 0x24A3
            api.set_byte(0x26FF, 0x1C); // 0x24B8
        }
        0x0B => {
            api.inv_give(0x0C); // 0x24AD
            api.set_byte(0x26FE, 1);
            api.set_byte(0x26FF, 0x1C);
        }
        0x0C => {
            // 0x24C0
            api.set_word(BASE_202, 0);
            api.obj_anim_id(OBJECT_202, 0x0A);
            api.task_arm(0); // 0x24D5
        }
        0x0D => {
            // 0x24DD
            api.set_word(BASE_202, 0);
            api.obj_anim_id(OBJECT_202, 8);
            api.obj_hide(0xC9); // 0x24F3
            api.task_arm(0);
        }
        0x0E => {
            // 0x24FB
            api.obj_wake(0xCD);
            api.play_sample(8);
            api.obj_anim_id(OBJECT_202, 9); // 0x250C, through 0x2408
            api.task_arm(0);
        }
        0x0F => {
            // 0x2510: object 109 on animation 1, the player walks to node
            // 109.
            api.obj_anim_id(0x6D, 1);
            let walker = api.ds(WALKER);
            api.walker_walk_to(walker, 0x6D, 1); // 0x2524
            api.set_word(AWAITED, 0x6D);
        }
        0x10 => {
            // 0x2534
            api.set_word(AWAITED, 0x0BB8);
            api.walk_to_node(0x0BB8);
            api.task_arm(0); // 0x2541, through 0x2413
        }
        0x11 => {
            // 0x2543
            api.set_word(BASE_202, 0);
            let rec = mover(api);
            api.walker_bind_graph(rec, 1, 0x12C, 3); // 0x2558
            let rec = mover(api);
            api.walker_walk_to(rec, 0x12D, 3); // 0x256D
            api.obj_hide(OBJECT_202); // 0x2578
            api.set_word(AWAITED, 0x12D);
            api.task_arm(0); // 0x2584, through 0x2413
        }
        0x12 => {
            api.inv_take(0x0B); // 0x2589
        }
        0x13 => {
            api.inv_give(0x0B); // 0x2592
        }
        0x6E => {
            // 0x259A
            api.set_word(NEXT_200, 0);
            api.set_word(STEP_202, 0);
        }
        _ => {}
    }
}

/// The animation hook, `020C:0296`: codes 100 to 103 through the table at
/// `0x23CB`.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        0x64 => {
            api.play_sample(9); // 0x236D
            api.task_start(0); // 0x23C3
        }
        0x65 => {
            // 0x2375: the mover bound to graph 1 on node 301 and walked to
            // node 300; object 202 hidden.
            let rec = mover(api);
            api.walker_bind_graph(rec, 1, 0x12D, 1); // 0x2384
            let rec = mover(api);
            api.walker_walk_to(rec, 0x12C, 1); // 0x2399
            api.obj_hide(OBJECT_202); // 0x23A4
            api.set_word(AWAITED, 0x12C);
        }
        0x66 => api.play_sample(0x14), // 0x23B4
        0x67 => {
            api.set_word(BASE_202, 1); // 0x23BB
            api.task_start(0);
        }
        _ => {}
    }
}

/// The Laden's rule handler, `020C:0002`, by `[2728h]` through the table
/// at `0x2127`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    if off != 0x0002 {
        return None;
    }
    Some(match api.byte(STATE) {
        0 | 2 => {
            // 0x20D8
            let counter = api.ds(SEQUENCE);
            api.scene_sequence(counter, &[0, 1]);
            api.set_byte(0x26FE, 1);
            api.set_byte(0x26FF, 4);
            api.set_byte(0x272A, 0);
            api.set_byte(0x271C, 2);
            0x0A
        }
        1 => {
            api.scene_play(3); // 0x2102
            0x0A
        }
        4 => {
            // 0x210A
            api.scene_play(6);
            api.set_byte(0x26FF, 0x1C);
            api.set_byte(STATE, 5);
            0x0A
        }
        _ => 0, // 0x211E
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
