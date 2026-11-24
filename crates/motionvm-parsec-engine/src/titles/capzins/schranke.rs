//! Location 8, the Schranke (`Schranke` in the game's location table).
//!
//! Re-expressed from the handler at `1254:00D7` (image `0x12617`), its
//! hook at `1254:0384` (`0x128C4`), its two rule handlers at `1254:0045`
//! and `1254:00A4`, the far routine `1254:0055` the handler installs at
//! `[1980h]`, and the near routines `1254:000F` and `1254:02C3`, against
//! [`Api`] alone. The location has no animation hook. Addresses are those
//! of *Captain Zins*' `XP01.EXE`, `DGROUP` `1FC4h`.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 20, which takes four fields from the record that walks.
const OBJECT_20: u16 = 0x14;
/// The record that walks here, found by id 200.
const MOVER_200: u16 = 0xC8;
/// Objects 201 and 204.
const OBJECT_201: u16 = 0xC9;
const OBJECT_204: u16 = 0xCC;

/// `[1CE8h]`, a word rule `00A4h` clears.
const WORD_1CE8: u16 = 0x1CE8;
/// `[1CF6h]`: 0 until the record has reached node 20000; then 1.
const STATE: u16 = 0x1CF6;
/// `[1CFCh]`: whether rule `00A4h` has played its first scene.
const SPOKEN: u16 = 0x1CFC;
/// `[1EC2h]`: the animation the record takes when its walk is through
/// (its word `+28h` no longer `FFFFh` is what the frame loop waits for).
const PENDING_ANIM: u16 = 0x1EC2;
/// `[1EC4h]`: the node the record stands on or walks to, 10000 and up;
/// 20000 ends it.
const NODE: u16 = 0x1EC4;
/// `[1EC6h]`: a direction, 0 to 3.
const DIRECTION: u16 = 0x1EC6;
/// `[1EC8h]`, `[1ECAh]`: object 20's record, a far pointer.
const OBJECT_20_REC: u16 = 0x1EC8;
/// `[1ECCh]`, `[1ECEh]`: the walking record, a far pointer.
const MOVER_REC: u16 = 0x1ECC;
/// `DS:11DCh`: the record the walker routines are handed elsewhere.
const RECORD_11DC: u16 = 0x11DC;
/// `[1980h]`, `[1982h]`: a far pointer to a routine `(id)`; the visit's
/// setup (`0A01:003B`) writes `0A01:0001` there, this handler its own.
const CALLBACK_OFF: u16 = 0x1980;
const CALLBACK_SEG: u16 = 0x1982;

/// A field of a far record.
fn at(rec: Far, field: u16) -> Far {
    Far {
        off: rec.off.wrapping_add(field),
        seg: rec.seg,
    }
}

/// The far pointer two words of `DGROUP` hold, offset first.
fn far_at(api: &dyn Api, addr: u16) -> Far {
    Far {
        off: api.word(addr),
        seg: api.word(addr.wrapping_add(2)),
    }
}

/// `1254:000F(k)`, near: the direction steps on, and the walking record
/// takes animation `(direction − 1 & 3) · 6 + k`, `k` below 2 raised by 4.
fn turn(api: &mut dyn Api, k: i16) {
    let d = api.word(DIRECTION).wrapping_add(1) & 3; // 0x12555, 0x12559
    api.set_word(DIRECTION, d);
    // 0x1255E: `cmp dx, 2 ; jge` — signed.
    let k = if k < 2 { k.wrapping_add(4) } else { k };
    let anim = ((d.wrapping_sub(1) & 3).wrapping_mul(6)).wrapping_add(num::unsigned(k)); // 0x12566
    api.figure_anim(far_at(api, MOVER_REC), num::signed(anim)); // 0x1257B
}

/// `1254:0055(id)`, the routine installed at `[1980h]`: the object with
/// this id, or the walking record when there is none; −1 when that is
/// null or bit 0 of its byte `+2` is clear, 1 when its byte `+9` is below
/// 4, else 0.
pub(crate) fn object_state(api: &mut dyn Api, id: u16) -> i16 {
    let mut rec = api.obj_find(id); // 0x1259C
    if rec == Far::NULL {
        rec = far_at(api, MOVER_REC); // 0x125AD
    }
    if rec == Far::NULL || api.far_byte(at(rec, 2)) & 1 == 0 {
        return -1; // 0x125CC
    }
    // 0x125D4: `cmp es:[bx+9], 4 ; jae`.
    i16::from(api.far_byte(at(rec, 9)) < 4)
}

/// Everything before the frame loop, `0x12617` to `0x1271B`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x12617
    api.set_word(CALLBACK_SEG, 0x1254); // 0x1261C
    api.set_word(CALLBACK_OFF, 0x0055); // 0x12622
    let entry = api.word(ENTRY); // 0x12628
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x1262F
    api.visit_enter(loc, entry); // 0x12633
    let object = api.obj_find(OBJECT_20); // 0x1263D
    api.set_word(OBJECT_20_REC.wrapping_add(2), object.seg); // 0x12643
    api.set_word(OBJECT_20_REC, object.off); // 0x12647
    let mover = api.mover_find(MOVER_200); // 0x1264D
    api.set_word(MOVER_REC.wrapping_add(2), mover.seg); // 0x12653
    api.set_word(MOVER_REC, mover.off); // 0x12657
    // 0x1265A: object 20 takes the mover's word +0Ch, byte +15h, byte +9
    // and word +0Ah, in this order.
    let w = api.far_word(at(mover, 0x0C));
    api.set_far_word(at(object, 0x0C), w); // 0x12666
    let b = api.far_byte(at(mover, 0x15));
    api.set_far_byte(at(object, 0x15), b); // 0x12676
    let b = api.far_byte(at(mover, 9));
    api.set_far_byte(at(object, 9), b); // 0x12686
    let w = api.far_word(at(mover, 0x0A));
    api.set_far_word(at(object, 0x0A), w); // 0x12696
    api.set_word(PENDING_ANIM, 0); // 0x1269A
    if api.byte(STATE) == 0 {
        // 0x126A7
        let rec = api.obj_show(OBJECT_201); // 0x126AC
        api.obj_anim(rec, 0); // 0x126B4
        api.node_block(api.ds(RECORD_11DC), 0x03E8); // 0x126C3
        api.set_word(NODE, 0x2710); // 0x126CB
        let node = api.word(NODE);
        api.walker_bind_graph(mover, 1, node, 0); // 0x126E1
        api.set_word(DIRECTION, 2); // 0x126E9
        turn(api, 1); // 0x126F1
    } else {
        api.obj_hide(OBJECT_201); // 0x126FA
        api.set_far_byte(at(mover, 2), 0); // 0x12704
        api.hotspot_follow(mover); // 0x1270E
    }
    api.visit_finish_entry(); // 0x12716
    api.set_word(ENTRY, 0); // 0x1271B
}

/// One turn of the frame loop, `0x12721` to `0x127E2`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x12721
    api.dispatch(); // 0x12726
    let mover = far_at(api, MOVER_REC);
    // 0x1272B: an animation is pending and the mover's word +28h is not
    // FFFFh.
    if api.word(PENDING_ANIM) != 0 && api.far_word(at(mover, 0x28)) != 0xFFFF {
        if api.word(NODE) == 0x4E20 {
            // 0x1274B: node 20000 reached.
            api.set_byte(STATE, 1);
            api.node_open(api.ds(RECORD_11DC), 0x03E8); // 0x12757
            let k = object_state(api, MOVER_200); // 0x12763
            let rec = api.obj_show(OBJECT_20); // 0x1276A
            api.obj_anim(rec, k); // 0x12772
            api.obj_invalidate(mover); // 0x12782
            api.set_far_byte(at(mover, 2), 0); // 0x1278E
            api.hotspot_follow(mover); // 0x12798
            api.obj_hide(OBJECT_201); // 0x127A3
            api.obj_hide(OBJECT_204); // 0x127AC
            api.play_sample(0x14); // 0x127B4
        } else {
            api.figure_anim(mover, num::signed(api.word(PENDING_ANIM))); // 0x127C8
        }
        api.set_word(PENDING_ANIM, 0); // 0x127D0
    }
    api.frame_pass(); // 0x127D6
    api.word(ENTRY) == 0 // 0x127DB
}

/// Everything after the frame loop, `0x127E5` to `0x12802`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5)); // 0x127E8
    api.objs_save_state(out); // 0x127F0
    api.visit_leave(); // 0x127F8
    api.slot_unload(); // 0x127FD
}

/// `1254:02C3(way, k)`, near: one step of the walking record. Four ways
/// through the table at `0x128BC` (`02D9h`, `02FFh`, `0315h`, `0326h`);
/// each either moves the node or, at its edge, only turns.
fn step(api: &mut dyn Api, way: u16, k: i16) {
    let node = num::signed(api.word(NODE));
    let walk = match way {
        // 0x12819: ten up to 10024, from where it is 20000.
        0 => {
            // `cmp [1EC4h], 271Eh ; jg` — signed.
            if node <= 0x271E {
                api.set_word(NODE, num::unsigned(node.wrapping_add(10))); // 0x12821
                true
            } else if node == 0x2728 {
                api.set_word(NODE, 0x4E20); // 0x12830
                true
            } else {
                false // 0x12838
            }
        }
        // 0x1283F: one up while the remainder by ten (`idiv`) is below 4.
        1 => {
            if node % 10 >= 4 {
                false
            } else {
                api.set_word(NODE, num::unsigned(node.wrapping_add(1))); // 0x1284D
                true
            }
        }
        // 0x12855: ten down, not from below 10010 (`jl`, signed).
        2 => {
            if node < 0x271A {
                false
            } else {
                api.set_word(NODE, num::unsigned(node.wrapping_sub(10))); // 0x1285D
                true
            }
        }
        // 0x12866: one down while the remainder by ten is not 0.
        3 => {
            if node % 10 == 0 {
                false
            } else {
                api.set_word(NODE, num::unsigned(node.wrapping_sub(1))); // 0x12873
                true
            }
        }
        _ => true, // 0x12810: above 3, straight to 0x1287B
    };
    if !walk {
        turn(api, k); // 0x12838
        return;
    }
    let mover = far_at(api, MOVER_REC);
    api.figure_anim(mover, k); // 0x12884
    let node = api.word(NODE);
    api.walker_walk_to(mover, node, 0); // 0x1289A
    let d = api.word(DIRECTION);
    // 0x128A2: the animation for the walk's end.
    api.set_word(
        PENDING_ANIM,
        d.wrapping_mul(6)
            .wrapping_add(num::unsigned(k))
            .wrapping_add(4),
    ); // 0x128AD
    api.set_word(DIRECTION, d.wrapping_add(1) & 3); // 0x128B0
}

/// The hook, `1254:0384`: script events 200 to 202 only, each nothing
/// once the node is 20000.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    // 0x128CA: `cmp ax, 7 ; jne`.
    if code != 7 {
        return;
    }
    // 0x128CF: a compare chain on the event; each arm first compares
    // `[1EC4h]` with 4E20h.
    match arg {
        0xC8 if api.word(NODE) != 0x4E20 => {
            let way = api.word(DIRECTION) ^ 2; // 0x128ED
            step(api, way, 0); // 0x12904
        }
        0xC9 if api.word(NODE) != 0x4E20 => {
            let way = api.word(DIRECTION); // 0x12900
            step(api, way, 1); // 0x12904
        }
        0xCA if api.word(NODE) != 0x4E20 => {
            let k = object_state(api, MOVER_200); // 0x12918
            turn(api, k.wrapping_add(2)); // 0x12920
        }
        _ => {} // 0x12924
    }
}

/// The Schranke's rule handlers by their offset in segment `1254h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 1254:0045: scene 0.
        0x0045 => {
            api.scene_play(0); // 0x1258A
            0x0A
        }
        // 1254:00A4: the first time scene −2, after that the complement
        // of a roll of 2 to 8 (scenes −3 to −9).
        0x00A4 => {
            api.set_word(WORD_1CE8, 0); // 0x125E7
            if api.byte(SPOKEN) == 0 {
                api.set_byte(SPOKEN, 1); // 0x125F4
                api.scene_play(-2); // 0x125F9, 0x1260C
            } else {
                let n = api.random(2, 8); // 0x12601
                api.scene_play(num::signed(!n)); // 0x12609, 0x1260C
            }
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
