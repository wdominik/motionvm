//! Location 6, the Straße. The module is named after the location as the
//! game itself names it in its location table (`Straße`).
//!
//! Re-expressed from the handler at `0158:00A7` (image `0x1627`), its hook
//! at `0158:01D2` (`0x1752`), its animation hook at `0158:01AE`
//! (`0x172E`) and its one rule handler at `0158:000E`, against [`Api`]
//! alone.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Id 128, the person the handler looks up as a record that walks.
const PERSON: u16 = 0x80;
/// Object 100, whose animations the hook sets.
const OBJECT_100: u16 = 0x64;
/// Node 0BB8h, the one the player walks to.
const NODE_BB8: u16 = 0x0BB8;

/// `[2717h]`: the state the rule switches on.
const STATE: u16 = 0x2717;
/// `[2720h]`: the counter the rule's scene sequences step.
const SEQUENCE: u16 = 0x2720;
/// `[18ACh]`, `[18AEh]`: the far pointer to person 128's record.
const PERSON_OFF: u16 = 0x18AC;
const PERSON_SEG: u16 = 0x18AE;
/// `[18B0h]`: the node task 0 waits for, the player or person 128 on it.
const WAIT_NODE: u16 = 0x18B0;
/// `[18B2h]` object 100's next animation, `[18B4h]` the one it plays,
/// `[18B6h]` whether the hook follows the dialogue with it.
const NEXT_ANIM: u16 = 0x18B2;
const PLAYING_ANIM: u16 = 0x18B4;
const FOLLOWS: u16 = 0x18B6;
/// `[16BBh]`: the player's node (`DS:1693h + 28h`).
const PLAYER_NODE: u16 = 0x16BB;
/// `[1EDEh]`: the door the player came in through.
const ENTRY_DOOR: u16 = 0x1EDE;
/// `si`: the entry door as the handler read it after the entry finished,
/// kept until it is closed.
const DOOR_LOCAL: usize = 0;

/// The far pointer in `[18ACh]`/`[18AEh]`.
fn person(api: &dyn Api) -> Far {
    Far {
        off: api.word(PERSON_OFF),
        seg: api.word(PERSON_SEG),
    }
}

/// Everything before the frame loop, `0x1627` to `0x16B5`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x1628
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x1634
    api.visit_enter(loc, entry); // 0x1638
    let rec = api.mover_find(PERSON); // 0x1643
    api.set_word(PERSON_SEG, rec.seg); // 0x1649
    api.set_word(PERSON_OFF, rec.off); // 0x164D
    // 0x1652: with item 5 carried.
    if api.inv_has(5) != 0 {
        api.walk_to_node(NODE_BB8); // 0x165F
        api.scene_play(0x0F); // 0x1667
        let rec = person(api);
        api.walker_bind_graph(rec, 1, 0x12E, 3); // 0x167C
        let rec = person(api);
        api.walker_walk_to(rec, 0x12D, 3); // 0x1691
        // 0x1699
        if api.byte(0x271C) == 4 {
            api.set_byte(0x26FB, 1);
        } else {
            api.set_byte(0x26FB, 0x18); // 0x16A7
        }
    }
    api.visit_finish_entry(); // 0x16AC
    let door = api.word(ENTRY_DOOR); // 0x16B1
    api.set_local(DOOR_LOCAL, door);
    api.set_word(ENTRY, 0); // 0x16B5
}

/// One turn of the frame loop, `0x16BB` to `0x170D`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x16BB
    api.dispatch(); // 0x16C0
    // 0x16C5: task 0 starts once the player or person 128 is on the node.
    let wait = api.word(WAIT_NODE);
    if wait != 0 {
        let on = api.word(PLAYER_NODE) == wait || {
            let rec = person(api); // 0x16D5
            api.far_word(Far {
                off: rec.off.wrapping_add(0x28),
                seg: rec.seg,
            }) == wait
        };
        if on {
            api.task_start(0); // 0x16E5
            api.set_word(WAIT_NODE, 0); // 0x16EB
        }
    }
    // 0x16F1: the door come in through closes once, with the player on a
    // node.
    if api.local(DOOR_LOCAL) != 0 && api.word(PLAYER_NODE) != 0xFFFF {
        api.set_local(DOOR_LOCAL, 0); // 0x16FC
        api.entry_door_close(); // 0x16FE
    }
    api.frame_pass(); // 0x1703
    api.word(ENTRY) == 0 // 0x1708
}

/// Everything after the frame loop, `0x170F` to `0x172D`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `0158:01D2`: codes 0 to 9 through the table at `0x18A5`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x176A
            api.set_word(NEXT_ANIM, 0);
            api.set_word(PLAYING_ANIM, 0);
            api.set_word(FOLLOWS, 0);
        }
        8 => api.set_word(NEXT_ANIM, arg.wrapping_add(4)), // 0x1778
        9 => {
            // 0x1783: a new animation for object 100, if it follows.
            if api.word(FOLLOWS) != 0 && api.word(NEXT_ANIM) != api.word(PLAYING_ANIM) {
                let next = api.word(NEXT_ANIM);
                api.set_word(PLAYING_ANIM, next); // 0x1799
                api.obj_anim_id(OBJECT_100, num::signed(next));
            }
        }
        7 => event(api, arg), // 0x17AA
        _ => {}               // 1 to 6 and above 9: 0x1891
    }
}

/// Code 7, `0x17AA`: script events 10 to 18 through the table at `0x1893`.
fn event(api: &mut dyn Api, n: u16) {
    match n {
        0x0A | 0x0B => {
            // 0x17BE; event 11 enters at 0x17C4, past the flag.
            if n == 0x0A {
                api.set_word(FOLLOWS, 1);
            }
            api.obj_anim_id(OBJECT_100, 1); // 0x17C8
            let next = api.word(NEXT_ANIM);
            api.set_word(PLAYING_ANIM, next); // 0x17D3
        }
        0x0C => {
            api.obj_anim_id(OBJECT_100, 0); // 0x17DD
            api.set_word(FOLLOWS, 0); // 0x17E5
        }
        0x0D => {
            // 0x17EE: out through exit 100, the door remembered.
            api.obj_anim_id(OBJECT_100, 7);
            api.set_word(FOLLOWS, 0); // 0x17FA
            api.set_word(ENTRY_DOOR, 0x64); // 0x1803
            api.script_exit(0x64); // 0x1807
        }
        0x0E => {
            // 0x180F
            api.scene_enqueue(0x0E);
            api.set_byte(0x272F, 0);
            api.set_byte(0x2729, 3);
            api.set_byte(0x271E, 0);
            api.set_byte(0x2715, 2);
            api.set_byte(0x271F, 0);
            api.set_byte(0x2716, 4);
            api.set_byte(0x26FE, 2);
            let rec = person(api);
            api.walker_bind_graph(rec, 1, 0x12C, 3); // 0x1849
            let rec = person(api);
            api.walker_walk_to(rec, 0x12D, 3); // 0x1875, through 0x186D
        }
        0x0F => {
            api.task_arm(0); // 0x185A
            api.set_word(WAIT_NODE, 0x12D); // 0x1860
        }
        0x10 => {
            let rec = person(api);
            api.walker_walk_to(rec, 0x12C, 1); // 0x1875
        }
        0x11 => {
            api.inv_take(5); // 0x1881
        }
        0x12 => {
            api.walk_to_node(NODE_BB8); // 0x188B
        }
        _ => {}
    }
}

/// The animation hook, `0158:01AE`: at code 2, while object 100 follows
/// the dialogue, the reporting record plays `[18B4h]`.
pub(crate) fn anim_hook(api: &mut dyn Api, rec: Far, code: u16) {
    if code == 2 && api.word(FOLLOWS) != 0 {
        let k = api.word(PLAYING_ANIM);
        api.obj_anim(rec, num::signed(k)); // 0x1748
    }
}

/// The Straße's rule handler by its offset in segment `0158h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    match off {
        // 0158:000E, by [2717h] - 1 through the table at 0x1615; every arm
        // answers 0Ah (0x1610).
        0x000E => {
            let counter = api.ds(SEQUENCE);
            match u16::from(api.byte(STATE)).wrapping_sub(1) {
                0 => {
                    // 0x15A5, into the next arm.
                    api.set_byte(STATE, 2);
                    api.scene_sequence(counter, &[6, 7, 8]);
                }
                1 => api.scene_sequence(counter, &[6, 7, 8]), // 0x15AA
                2 => {
                    api.scene_play(0x15); // 0x15B4
                }
                3 => api.scene_sequence(counter, &[3, 4, 5]), // 0x15BE
                4 => {
                    api.scene_play(0x16); // 0x15C8
                }
                5 => api.scene_sequence(counter, &[0x10, 0x11, 0x12]), // 0x15CC
                6 => api.scene_sequence(counter, &[9, 0x0A, 0x0B]),    // 0x15D6
                7 => api.scene_sequence(counter, &[0x0C, 0x0D]),       // 0x15E0
                8 => api.scene_sequence(counter, &[0x13, 0x14]),       // 0x15E8
                _ => api.scene_sequence(counter, &[0, 1, 2]),          // 0x15FC
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
