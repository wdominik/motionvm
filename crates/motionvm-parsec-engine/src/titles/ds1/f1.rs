//! Location 27, the F1. The module is named after the location as the
//! game itself names it in its location table (`F1`).
//!
//! Re-expressed from the handler at `03FE:000F` (image `0x3FEF`) and its
//! hook at `03FE:033A` (`0x431A`), against [`Api`] alone. The location
//! has no rules and no animation hook of its own. Person 200 walks on
//! walk graph 1 between nodes 190h to 193h, and the hook's events start
//! and stop the music.
//!
//! The prologue differs from the standard one: `[1F4Ch]` is written only
//! after `0EB9:00B8` (`0x41D3`).

use super::logic::Logic;
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Person 200, the one who walks.
const PERSON: u16 = 0xC8;
/// Object 204, whose animation the events set.
const OBJECT_204: u16 = 0xCC;
/// Objects 128 and 132, which play along while the person speaks.
const OBJECT_128: u16 = 0x80;
const OBJECT_132: u16 = 0x84;
/// The objects woken at the entry, after the person (`0x40B9`).
const WOKEN: [u16; 13] = [
    0xCC, 0x84, 0x80, 0xD3, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDB, 0xDD, 0xDF, 0xE0,
];
/// The objects `03FE:02DC` sets on one animation together.
const CHORUS: [u16; 7] = [0xD3, 0xD5, 0xD6, 0xD7, 0xD8, 0xD9, 0xDB];
/// The person's walk graph and its nodes.
const GRAPH: u16 = 1;
const NODE_190: u16 = 0x190;
const NODE_191: u16 = 0x191;
const NODE_192: u16 = 0x192;
const NODE_193: u16 = 0x193;
/// A walking record's node word, `+28h`.
const NODE_FIELD: u16 = 0x28;

/// `[2726h]`: the sub-state the entry switches on and the scene sequence
/// steps.
const SUBSTATE: u16 = 0x2726;
/// `[193Ch]`: the person's walking record (`0BB6:05B0`), offset and
/// segment.
const PERSON_REC: u16 = 0x193C;
/// `[194Ah]`: what the frame loop waits for (1 to 4).
const WAITING: u16 = 0x194A;
/// `[1948h]`: who speaks in the dialogue that is up — bit 1 the person,
/// bit 2 objects 128 and 132.
const SPEAKERS: u16 = 0x1948;
/// `[1946h]`: the animation to play at the next page's end; `[1944h]`:
/// the one playing.
const NEXT_ANIM: u16 = 0x1946;
const PLAYING_ANIM: u16 = 0x1944;
/// `[285Dh]`: the song loaded last, offset and segment.
const SONG: u16 = 0x285D;
/// `[1C82h]`: the hundredths the scene machine counts.
const HUNDREDTHS: u16 = 0x1C82;
/// `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`, `DS:1F56h`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
const HERE: u16 = 0x1F4C;
const OBJECT_STATES: u16 = 0x1F56;

/// The far pointer kept at `DS:at`, offset word first.
fn far_at(api: &dyn Api, at: u16) -> Far {
    Far {
        off: api.word(at),
        seg: api.word(at.wrapping_add(2)),
    }
}

fn set_far_at(api: &mut dyn Api, at: u16, p: Far) {
    api.set_word(at.wrapping_add(2), p.seg);
    api.set_word(at, p.off);
}

/// The node the person's walking record stands on.
fn person_node(api: &mut dyn Api) -> u16 {
    let rec = far_at(api, PERSON_REC);
    api.far_word(Far {
        off: rec.off.wrapping_add(NODE_FIELD),
        seg: rec.seg,
    })
}

/// Everything before the frame loop, `0x3FEF` to `0x41DF`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x3FF3
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.visit_enter(loc, entry); // 0x4000
    api.hover_off(); // 0x4008
    api.set_word(0x167D, 0x67); // 0x400D
    api.set_word(0x167F, 7);
    api.set_word(0x1681, 0x0A);
    api.set_word(0x1683, 0x03E8);
    let rec = api.mover_find(PERSON); // 0x4028
    set_far_at(api, PERSON_REC, rec);
    let obj = api.obj_find(PERSON); // 0x4038
    // 0x404B: seven arms through the table at 0x42AE.
    match api.byte(SUBSTATE) {
        0 => {
            api.obj_wake(PERSON); // 0x4057
        }
        1..=3 => {
            // 0x4062
            api.set_far_word(
                Far {
                    off: obj.off.wrapping_add(4),
                    seg: obj.seg,
                },
                0x64,
            );
            api.obj_wake(PERSON);
            api.slot_set_object(1, 0, obj); // 0x407E
        }
        4..=6 => {
            // 0x4085: the person on the walk graph instead of the object.
            api.obj_hide(PERSON);
            let rec = far_at(api, PERSON_REC);
            api.walker_bind_graph(rec, GRAPH, NODE_193, 1); // 0x409D
            let rec = far_at(api, PERSON_REC);
            api.slot_set_figure(1, 0, rec); // 0x40B1
        }
        _ => {}
    }
    for id in WOKEN {
        api.obj_wake(id); // 0x40B9 to 0x4128
    }
    // 0x412E: subject slots 4 to 9, points.
    api.slot_set_raw(4, 3, 0x96, 0x50);
    api.slot_set_raw(5, 3, 0xAA, 0x55);
    api.slot_set_raw(6, 3, 0x82, 0x3C);
    api.slot_set_raw(7, 3, 0xBE, 0x41);
    api.slot_set_raw(8, 3, 0x96, 0x46);
    api.slot_set_raw(9, 3, 0xA0, 0x40);
    let counter = api.ds(SUBSTATE);
    api.scene_sequence(counter, &[0, 1, 2, 3, 6, 7]); // 0x41A6
    api.visit_finish_entry(); // 0x41AE
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x41B6
    api.set_word(ENTRY, 0); // 0x41B9
}

/// One turn of the frame loop, `0x41BF` to `0x4282`: what the person's
/// walk was waiting for, by `[194Ah]` through the table at `0x42A6`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    match api.word(WAITING).wrapping_sub(1) {
        0 => {
            // 0x41DD: at node 191h, on to 192h; the player to node BB9h.
            if person_node(api) == NODE_191 {
                let rec = far_at(api, PERSON_REC);
                api.walker_walk_to(rec, NODE_192, 0); // 0x41F6
                api.walk_to_node(0x0BB9); // 0x4201
                api.set_word(WAITING, 0); // 0x4207
                api.task_start(0);
            }
        }
        1 => {
            // 0x4217: at node 191h, the player to node 65h and the person on.
            if person_node(api) == NODE_191 {
                api.walk_to_node(0x65); // 0x4225
                let rec = far_at(api, PERSON_REC);
                api.walker_walk_to(rec, NODE_192, 0); // 0x4238
                api.set_word(WAITING, 0); // 0x4227
                api.task_start(0);
            }
        }
        2 => {
            // 0x4242: at node 192h, off to location 1Dh, entry 65h.
            if person_node(api) == NODE_192 {
                api.set_word(WAITING, 0); // 0x424E
                api.set_word(LOCATION, 0x1D);
                api.set_word(ENTRY, 0x65);
            }
        }
        3 => {
            // 0x4262: after five seconds.
            if api.word(HUNDREDTHS) > 0x01F4 {
                api.task_start(0); // 0x426C
                api.set_word(WAITING, 0);
            }
        }
        _ => {}
    }
    api.frame_pass(); // 0x4278
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x4287` to `0x42A5`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// `03FE:02DC(k)`: the chorus set on animation `k`.
fn chorus(api: &mut dyn Api, k: i16) {
    for id in CHORUS {
        api.obj_anim_id(id, k);
    }
}

/// The song `n` loaded into `[285Dh]` and played.
fn song(api: &mut dyn Api, n: u16, a: i16) {
    let res = api.song_load(n);
    set_far_at(api, SONG, res);
    let res = far_at(api, SONG);
    api.music_play(res, a);
}

/// The hook, `03FE:033A`: codes 0 to 10 through the table at `0x46B0`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x4333: a dialogue comes up; while the sub-state is 4 or
            // less, its speakers' bits 1 and 2 are kept.
            let speakers = if api.byte(SUBSTATE) <= 4 { arg & 6 } else { 0 };
            api.set_word(SPEAKERS, speakers);
            api.set_word(NEXT_ANIM, 0);
            api.set_word(PLAYING_ANIM, 0);
        }
        5 | 6 => {
            // 0x4355: back to standing.
            api.obj_wake(OBJECT_132);
            api.obj_wake(OBJECT_128);
            if api.word(SPEAKERS) & 0xFFFD != 0 {
                api.obj_wake(PERSON); // 0x4372
            }
            api.set_word(SPEAKERS, 0); // 0x444B
        }
        8 => {
            // 0x4381: the next animation, 0 for anything above 3.
            let k = if num::signed(arg) > 3 { 0 } else { arg };
            api.set_word(NEXT_ANIM, k.wrapping_add(1));
        }
        9 | 10 => {
            // 0x4391: a page's end — each speaker's next animation.
            if api.word(PLAYING_ANIM) == api.word(NEXT_ANIM) {
                return;
            }
            let k = api.word(NEXT_ANIM);
            api.set_word(PLAYING_ANIM, k);
            if api.word(SPEAKERS) & 4 != 0 {
                api.obj_anim_id(OBJECT_128, num::signed(k)); // 0x43AF
                let k = api.word(PLAYING_ANIM);
                api.obj_anim_id(OBJECT_132, num::signed(k));
            }
            if api.word(SPEAKERS) & 2 != 0 {
                let k = api.word(PLAYING_ANIM);
                api.obj_anim_id(PERSON, num::signed(k)); // 0x43D1
            }
        }
        7 => event(api, arg), // 0x43DB
        _ => {}
    }
}

/// Code 7, `0x43DB`: script events through the value table at `0x465C`.
fn event(api: &mut dyn Api, n: u16) {
    match n {
        0x0A => {
            api.walk_to_node(0x80); // 0x43F8
        }
        0x0B => {
            api.walk_to_node(0x64); // 0x4403
        }
        0x0C => {
            api.walk_to_node(0xC8); // 0x4407
        }
        0x0D => {
            api.walk_to_node(0x65); // 0x440C
        }
        0x11..=0x13 => person(api, n),
        0x14 => api.obj_anim_id(OBJECT_204, 2), // 0x44A2
        0x15 => {
            // 0x44A6: off to location 1Ch, entry 64h.
            api.set_word(LOCATION, 0x1C);
            api.set_word(ENTRY, 0x64);
            api.task_arm(0);
        }
        0x16 => {
            // 0x44B5: off to location 1Dh, entry 65h.
            api.set_word(LOCATION, 0x1D);
            api.set_word(ENTRY, 0x65);
            api.task_arm(0);
        }
        0x19 => api.obj_anim_id(OBJECT_204, 1), // 0x44C4
        0x1A..=0x20 => band(api, n),
        0x21 => song(api, 0x0F, -1), // 0x461B
        0x28 => {
            // 0x463E: location and entry −1, which ends the main loop
            // (0x4FE9); a location of −1 is the road to the end sequence
            // (0x4FFD).
            api.set_word(LOCATION, 0xFFFF);
            api.set_word(ENTRY, 0xFFFF);
            api.task_arm(0);
        }
        0x6E => api.set_word(NEXT_ANIM, 0), // 0x4654
        _ => {}
    }
}

/// Events 11h–13h of the value table at `0x465C`: the person walking
/// off the stage and back.
fn person(api: &mut dyn Api, n: u16) {
    match n {
        0x11 => {
            // 0x4410: the person out of the object onto the walk graph,
            // walking to node 191h.
            let rec = far_at(api, PERSON_REC);
            api.walker_bind_graph(rec, GRAPH, NODE_190, 0);
            let rec = far_at(api, PERSON_REC);
            api.walker_walk_to(rec, NODE_191, 0); // 0x4434
            api.obj_hide(PERSON); // 0x443F
            api.set_word(WAITING, 1);
            api.set_word(SPEAKERS, 0); // 0x444B
        }
        0x12 => {
            // 0x4454
            if api.word(WAITING) == 1 {
                api.set_word(WAITING, 2); // 0x445B
                api.task_arm(0); // 0x464C
            } else {
                api.walk_to_node(0x65); // 0x4466
                let rec = far_at(api, PERSON_REC);
                api.walker_walk_to(rec, NODE_192, 0); // 0x4479
            }
        }
        0x13 => {
            // 0x4484
            let rec = far_at(api, PERSON_REC);
            api.walker_walk_to(rec, NODE_192, 0);
            api.set_word(WAITING, 3);
            api.task_arm(0); // 0x464C
        }
        _ => {}
    }
}

/// Events 1Ah–20h of the value table at `0x465C`: the band's songs and
/// the chorus over them.
fn band(api: &mut dyn Api, n: u16) {
    match n {
        0x1A => {
            // 0x44CC: song 5.
            api.obj_anim_id(OBJECT_204, 1);
            song(api, 5, 0); // 0x44DB
        }
        0x1B => {
            // 0x44ED: song 14, and the frame loop waits five seconds.
            api.task_arm(0);
            api.obj_anim_id(OBJECT_204, 1);
            api.set_word(WAITING, 4); // 0x4502
            api.set_word(HUNDREDTHS, 0);
            song(api, 0x0E, -1); // 0x461D
        }
        0x1C => {
            // 0x4513: the music faded out.
            api.obj_anim_id(0xDD, 2);
            api.obj_anim_id(0xDF, 2);
            api.obj_anim_id(0xE0, 0);
            api.music_fade(0x0FA0); // 0x453D
        }
        0x1D => {
            // 0x4545: the music stopped, the song's block handed back,
            // song 15.
            api.obj_anim_id(0xDF, 2);
            api.obj_anim_id(0xE0, 2);
            api.music_stop(); // 0x455F
            let res = far_at(api, SONG);
            if res.off | res.seg != 0 {
                // 0x4578: `1EF8:0720` is handed the song's far pointer
                // after a null test.
                api.heap_0720(res);
            }
            song(api, 0x0F, -1); // 0x461B
        }
        0x1E => {
            // 0x4586
            chorus(api, 0);
            if api.word(SPEAKERS) & 4 == 0 {
                api.obj_anim_id(OBJECT_128, 0); // 0x4598
                api.obj_anim_id(OBJECT_132, 0);
            }
        }
        0x1F => {
            // 0x45A9
            chorus(api, 1);
            api.obj_anim_id(0xDD, 1);
            api.obj_anim_id(0xDF, 1);
            api.obj_anim_id(0xE0, 1);
            if api.word(SPEAKERS) & 4 == 0 {
                api.obj_anim_id(OBJECT_128, 5); // 0x45DF
                api.obj_anim_id(OBJECT_132, 5);
            }
        }
        0x20 => {
            // 0x45F0
            chorus(api, 2);
            if api.word(SPEAKERS) & 4 == 0 {
                api.obj_anim_id(OBJECT_128, 6); // 0x45FF
                api.obj_anim_id(OBJECT_132, 6);
            }
        }
        _ => {}
    }
}

/// No rules of its own (`+2Bh` is null).
pub(crate) fn rule(_api: &mut dyn Api, _off: u16, _flags: u16) -> Option<i16> {
    None
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
