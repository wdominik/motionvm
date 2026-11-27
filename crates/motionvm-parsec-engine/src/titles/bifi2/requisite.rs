//! Location 16, the Requisite, a row of kind 1 with one exit, `6Fh`, to
//! the Flur 1. Stock (location 19). The module is named after the location
//! as the game itself names it in its location table (`Requisite`, the
//! record's `+37h`, `250Ah:0125h`).
//!
//! Re-expressed from the handler at `1485h:00D0h` (image `0x14920` to its
//! `retf` at `0x14AA4`), its hook `1485h:02C8h` (`0x14B18`, the record's
//! `+3Fh`), its animation hook `1485h:0255h` (`0x14AA5`, the record's
//! `+43h`) and its four rule handlers `1485h:000Bh` to `00BFh`
//! (`0x1485B`–`0x1491F`), against [`Api`] alone. The location's rules are
//! the four records at `2446h:05B0h` (image `0x24A10`, the record's
//! `+3Bh`), fourteen bytes each:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 3 | 4 | `92h` | — | `0020h` |
//! | 9 | `20h` | `12h` | `92h` | `000Bh` |
//! | 3 | 4 | `93h` | — | `0056h` |
//! | 4 | 4 | `12h` | — | `00BFh` |
//!
//! The verbs are, by the verb text row at `DS:116Ah`: 3 `Sprich mit`, 4
//! `Benutze`, 9 `Gib … an`. `Benutze` on item `12h`, the `Perücke`, turns
//! into `Gib … an` and asks for a second object (rule `00BFh`); the
//! `Perücke` given to `92h` plays scene 4 and sets `[2BBBh]` (rule
//! `000Bh`). Object `93h` is shown on frame 1 on every entry. From then on
//! object `92h` is hidden and object `93h`, kept at `[2EFAh]`, takes the
//! animation its state `[2BCEh]` names (0 becomes 1 with scene 6, 6 is
//! animation 4). Before that object `92h` is shown and kept at `[2EFEh]`.
//!
//! The hook's code 7 with argument `0Bh` hides object `92h`, keeps its
//! walking record at `[2EF6h]` (`06E3h:04EFh`, `0x14CC5`), binds it to
//! graph 1 on node `0BB8h` and sends it to `0BB9h`; the frame loop then
//! watches the record's node (`+28h`): on `0BB9h` the record walks on to
//! `0BBAh` and door `6Fh` takes animation 1, on `0BBAh` it walks on to
//! `0BBBh` and the door takes animation 0, on `0BBBh` the story goes on to
//! the Kaffee (location 1) with entry 8, whose row at `DS:182Ch` leads
//! back to location 16.
//!
//! The bytes of the logic's saved state block (`DS:2B98h`, `4Eh` bytes) the
//! location reads or writes: `[2BBBh]` (the `Perücke` given), `[2BBDh]`
//! (rule `0020h`'s first talk) and `[2BCEh]` (object `93h`'s state, 0 to
//! 6; segment `109Ah` raises it to 2, `0x109E3`, and segment `1746h` to 3,
//! `0x174F1`). The items the logic names are, by the item row at
//! `DS:10AAh`: `12h` `Perücke`, `13h` `Leuchtfarbe`, `1Bh`
//! `Labyrinth-Karte`, `29h` `Wasserpistole`, `2Bh` `Ente`.
//!
//! The record that walks by id is `06E3h:04EFh` (`0x0731F`, Dunkle
//! Schatten's `0BB6h:05B0h`, [`Api::mover_find`]), its graph `05DCh:0DB4h`
//! (`0x06B74`, Dunkle Schatten's `0A9Fh:0EA6h`, [`Api::walker_bind_graph`]).

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x1485;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x14A8D`).
const OBJECT_STATES: u16 = 0x2018;

/// Object `92h`, and the walking record of the same id.
const OBJECT_92: u16 = 0x92;
/// Object `93h`.
const OBJECT_93: u16 = 0x93;
/// Door `6Fh`, the row's one exit, to the Flur 1. Stock.
const DOOR_6F: u16 = 0x6F;

/// `[2EF6h]`/`[2EF8h]`: the walking record `92h` (`0x14CCB`, `0x14CCF`).
const MOVER_92: u16 = 0x2EF6;
/// `[2EFAh]`/`[2EFCh]`: object `93h`'s record (`0x1496C`, `0x14970`).
const REC_93: u16 = 0x2EFA;
/// `[2EFEh]`/`[2F00h]`: object `92h`'s record (`0x149C7`, `0x149CB`).
const REC_92: u16 = 0x2EFE;

/// The nodes of the record's walk: bound on `0BB8h`, sent to `0BB9h`,
/// then to `0BBAh` and `0BBBh`.
const NODE_BOUND: u16 = 0x0BB8;
const NODE_FIRST: u16 = 0x0BB9;
const NODE_SECOND: u16 = 0x0BBA;
const NODE_OUT: u16 = 0x0BBB;
/// The node event `14h` sends the player to (`push 93h`, `0x14D56`).
const NODE_93: u16 = 0x93;
/// The walking record's node, `+28h` (`0x14A05`).
const MOVER_NODE: u16 = 0x28;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BBBh]`: the `Perücke` given to `92h`.
const BYTE_2BBB: u16 = 0x2BBB;
/// `[2BBDh]`: rule `0020h`'s first talk done.
const BYTE_2BBD: u16 = 0x2BBD;
/// `[2BCEh]`: object `93h`'s state.
const BYTE_2BCE: u16 = 0x2BCE;

/// The location's own words, read and written in segment `1485h` alone.
/// `[2EEEh]`: bit 2 of code 0's argument, object `93h` tracked.
const TRACK_93: u16 = 0x2EEE;
/// `[2EF0h]`: bit 1 of code 0's argument, object `92h` tracked.
const TRACK_92: u16 = 0x2EF0;
/// `[2EF2h]`: the animation wanted.
const ANIM_WANTED: u16 = 0x2EF2;
/// `[2EF4h]`: the animation last given.
const ANIM_GIVEN: u16 = 0x2EF4;
/// `[2F02h]`: the meeting of a script and an animation: `FFFFh` the
/// script waits with its task armed, 1 the animation came first.
const MEETING: u16 = 0x2F02;
/// `[2F04h]`: the record's walk out is watched (`0x14CBC`), cleared on
/// entry (`0x14938`).
const WALKING_OUT: u16 = 0x2F04;

/// The items the hook's code 4 and the entry name.
const ITEM_PERUECKE: u16 = 0x12;
const ITEM_LEUCHTFARBE: u16 = 0x13;
const ITEM_LABYRINTH_KARTE: u16 = 0x1B;
const ITEM_WASSERPISTOLE: u16 = 0x29;
const ITEM_ENTE: u16 = 0x2B;

/// The Kaffee, location 1, and the entry the walk out sends it with
/// (`0x14A6C`, `0x14A72`).
const KAFFEE: u16 = 1;
const KAFFEE_ENTRY: u16 = 8;

/// The mode block's verb `[2D38h]` and the verb as chosen `[2D48h]`.
const VERB: u16 = 0x2D38;
const VERB_CHOSEN: u16 = 0x2D48;

fn rec(api: &dyn Api, at: u16) -> Far {
    Far {
        off: api.word(at),
        seg: api.word(at.wrapping_add(2)),
    }
}

fn keep(api: &mut dyn Api, at: u16, r: Far) {
    api.set_word(at.wrapping_add(2), r.seg);
    api.set_word(at, r.off);
}

/// `push dir ; push ds ; push 176Eh ; call far 05DCh:0B56h`.
fn face(api: &mut dyn Api, dir: i16) {
    let player = api.ds(WALKER);
    api.figure_face(player, dir);
}

/// `push k ; push 6Fh ; call far 06E3h:047Eh ; pop cx ; push dx ; push ax
/// ; call far 03F7h:1CF9h`: door `6Fh` set on animation `k`.
fn door(api: &mut dyn Api, k: i16) {
    let r = api.obj_find(DOOR_6F);
    api.obj_anim(r, k);
}

/// Everything before the frame loop, `0x14920` to `0x149EA`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x14920
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x1492C
    api.visit_enter(loc, entry); // 0x14930
    api.set_word(WALKING_OUT, 0); // 0x14938
    if api.byte(BYTE_2BBB) == 1 {
        // 0x1493E: the Perücke given.
        if api.inv_has(ITEM_LABYRINTH_KARTE) != 0 {
            api.set_byte(BYTE_2BCE, 6); // 0x14951, by 0x14947
        }
        api.obj_hide(OBJECT_92); // 0x14959
        let r93 = api.obj_show_frame(OBJECT_93, 1); // 0x14964
        keep(api, REC_93, r93); // 0x1496C, 0x14970
        match api.byte(BYTE_2BCE) {
            // 0x14983: state 1, animation 0, scene 6.
            0 => {
                api.set_byte(BYTE_2BCE, 1); // 0x14983
                api.obj_anim(r93, 0); // 0x14992
                api.scene_play(6); // 0x1499C
            }
            6 => api.obj_anim(r93, 4), // 0x149A4, 0x149B2
            _ => api.obj_anim(r93, 0), // 0x149A8, 0x149B2
        }
    } else {
        let r92 = api.obj_show(OBJECT_92); // 0x149C1
        keep(api, REC_92, r92); // 0x149C7, 0x149CB
        api.obj_anim(r92, 0); // 0x149D0
        api.obj_show_frame(OBJECT_93, 1); // 0x149DD, the record not kept
    }
    api.visit_finish_entry(); // 0x149E5
    api.set_word(WORDS.entry, 0); // 0x149EA
}

/// One turn of the frame loop, `0x149F0` to `0x14A84`: while `[2F04h]` is
/// set, the walking record's node is watched between the dispatch and the
/// frame pass.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x149F0
    api.dispatch(); // 0x149F5
    if api.word(WALKING_OUT) != 0 {
        // 0x149FA
        let mover = rec(api, MOVER_92);
        let node = api.far_word(Far {
            off: mover.off.wrapping_add(MOVER_NODE),
            seg: mover.seg,
        }); // 0x14A01, 0x14A05
        match node {
            // 0x14A1A: the player faces 1, the record walks on, the door
            // on animation 1.
            NODE_FIRST => {
                face(api, 1); // 0x14A20
                api.walker_walk_to(mover, NODE_SECOND, 1); // 0x14A35
                door(api, 1); // 0x14A41, 0x14A49
            }
            // 0x14A53: the record walks on, the door on animation 0.
            NODE_SECOND => {
                api.walker_walk_to(mover, NODE_OUT, 1); // 0x14A60
                door(api, 0); // 0x14A41, 0x14A49
            }
            // 0x14A6C: on to the Kaffee with entry 8.
            NODE_OUT => {
                api.set_word(WORDS.location, KAFFEE); // 0x14A6C
                api.set_word(WORDS.entry, KAFFEE_ENTRY); // 0x14A72
            }
            _ => {}
        }
    }
    api.frame_pass(); // 0x14A78
    api.word(WORDS.entry) == 0 // 0x14A7D
}

/// Everything after the frame loop, `0x14A87` to `0x14AA4`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x14A92
    api.visit_leave(); // 0x14A9A
    api.slot_unload(); // 0x14A9F
}

/// The hook, `1485h:02C8h`: code in `[bp+6]`, argument in `[bp+8]`, task
/// in `[bp+0Ah]`. Codes above 9 and codes 1 to 3 do nothing (`0x14B26`,
/// the table at `cs:057Ch`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, task: u16) {
    match code {
        // 0x14B35: bits 1 and 2 of the argument kept, each tracked object
        // on animation 0; object 93h only in states 1 to 5. The animation
        // wanted and given forgotten.
        0 => {
            api.set_word(TRACK_92, arg & 2); // 0x14B3A
            if api.word(TRACK_92) != 0 {
                let r = rec(api, REC_92);
                api.obj_anim(r, 0); // 0x14B4E
            }
            api.set_word(TRACK_93, arg & 4); // 0x14B5B
            if api.word(TRACK_93) != 0 {
                let state = api.byte(BYTE_2BCE);
                if state == 0 || state == 6 {
                    api.set_word(TRACK_93, 0); // 0x14B87, by 0x14B65, 0x14B6C
                } else {
                    let r = rec(api, REC_93);
                    api.obj_anim(r, 0); // 0x14B7D
                }
            }
            api.set_word(ANIM_WANTED, 0xFFFF); // 0x14B90
            api.set_word(ANIM_GIVEN, 0xFFFF); // 0x14B93
        }
        4 => items(api, arg), // 0x14C4C
        // 0x14B99: the tracked object back on animation 0 and no longer
        // tracked, 92h before 93h.
        5 | 6 => {
            if api.word(TRACK_92) != 0 {
                let r = rec(api, REC_92);
                api.obj_anim(r, 0); // 0x14BAA
                api.set_word(TRACK_92, 0); // 0x14BB2
            } else if api.word(TRACK_93) != 0 {
                let r = rec(api, REC_93);
                api.obj_anim(r, 0); // 0x14BCF
                api.set_word(TRACK_93, 0); // 0x14BD7
            }
        }
        7 => events(api, arg, task), // 0x14C92
        // 0x14BE0: the animation wanted, 2 for an argument above 0, else
        // the argument plus 1; left alone while 92h is tracked on 4.
        8 => {
            if api.word(TRACK_92) != 0 && api.word(ANIM_GIVEN) == 4 {
                return; // 0x14BEE
            }
            let k = if num::signed(arg) > 0 {
                2 // 0x14BFE
            } else {
                arg.wrapping_add(1) // 0x14BF5, 0x14BF8
            };
            api.set_word(ANIM_WANTED, k);
        }
        // 0x14C07: a new animation wanted goes to object 92h when it is
        // tracked, else to object 93h when it is; it is taken as given
        // either way.
        9 => {
            let k = api.word(ANIM_WANTED);
            if k == api.word(ANIM_GIVEN) {
                return; // 0x14C0E
            }
            api.set_word(ANIM_GIVEN, k); // 0x14C1A, 0x14C38, 0x14B93
            let at = if api.word(TRACK_92) != 0 {
                REC_92 // 0x14C13
            } else if api.word(TRACK_93) != 0 {
                REC_93 // 0x14C2E
            } else {
                return; // 0x14C46
            };
            let r = rec(api, at);
            api.obj_anim(r, num::signed(k)); // 0x14C26
        }
        _ => {}
    }
}

/// Code 4, `0x14C4C`: the argument looked up in the row at `cs:056Ch`
/// (`14h`, `31h` to `33h`), its arm in the row after it (`cs:0574h`).
fn items(api: &mut dyn Api, arg: u16) {
    match arg {
        // 0x14C69: the Leuchtfarbe given, sample 26h.
        0x14 => {
            api.inv_give(ITEM_LEUCHTFARBE); // 0x14C6B
            api.play_sample(0x26); // 0x14C73
        }
        // 0x14C7A: the Wasserpistole given.
        0x31 => {
            api.inv_give(ITEM_WASSERPISTOLE); // 0x14C7C
        }
        // 0x14C83: the Perücke taken.
        0x32 => {
            api.inv_take(ITEM_PERUECKE); // 0x14C89
        }
        // 0x14C87: the Ente taken.
        0x33 => {
            api.inv_take(ITEM_ENTE); // 0x14C89
        }
        _ => {}
    }
}

/// Code 7, `0x14C92`: the argument looked up in the row at `cs:053Ch`
/// (`0Ah` to `10h`, `14h` to `17h`, `50h`), its arm in the row after it
/// (`cs:0554h`).
fn events(api: &mut dyn Api, arg: u16, task: u16) {
    match arg {
        // 0x14CAF: the input given back.
        0x0A => api.input_unlock(),
        0x0B => walk_out(api), // 0x14CB7
        // 0x14D11 (by 0x14D67), 0x14D67.
        0x0C | 0x15 => api.set_word(ANIM_WANTED, 3),
        // 0x14D13: animation 4 wanted, sample 1Ch, the meeting cleared.
        0x0D => {
            api.set_word(ANIM_WANTED, 4); // 0x14D13
            api.play_sample(0x1C); // 0x14D1B
            api.set_word(MEETING, 0); // 0x14D3D
        }
        // 0x14D23, 0x14D36: when the animation came first the meeting is
        // cleared, else the task is armed and waits.
        0x0E | 0x10 => meet(api, task),
        // 0x14D2E: animation 5 wanted, the meeting cleared.
        0x0F => {
            api.set_word(ANIM_WANTED, 5); // 0x14D2E
            api.set_word(MEETING, 0); // 0x14D3D
        }
        // 0x14D54: the player walks to node 93h facing 0; answer not read.
        0x14 => {
            let player = api.ds(WALKER);
            api.walker_walk_to(player, NODE_93, 0); // 0x14D5D
        }
        0x16 => face(api, 3), // 0x14D6F, 0x14D79
        0x17 => face(api, 0), // 0x14D73, 0x14D79
        // 0x14D83.
        0x50 => api.set_word(ANIM_WANTED, 0),
        _ => {}
    }
}

/// `0x14D23`/`0x14D36` to `0x14D4C`: `cmp word [2F02h], 0 ; jng`.
fn meet(api: &mut dyn Api, task: u16) {
    if num::signed(api.word(MEETING)) > 0 {
        api.set_word(MEETING, 0); // 0x14D3D
    } else {
        api.task_arm(task); // 0x14D46
        api.set_word(MEETING, 0xFFFF); // 0x14D4C
    }
}

/// Code 7 with argument `0Bh`, `0x14CB7` to `0x14D0F`: the input taken
/// over, the walk out watched, object `92h` no longer tracked and hidden,
/// its walking record kept, bound to graph 1 on node `0BB8h` and sent to
/// `0BB9h`, the player facing 2.
fn walk_out(api: &mut dyn Api) {
    api.input_lock(); // 0x14CB7
    api.set_word(WALKING_OUT, 1); // 0x14CBC
    let mover = api.mover_find(OBJECT_92); // 0x14CC5, 06E3h:04EFh
    keep(api, MOVER_92, mover); // 0x14CCB, 0x14CCF
    api.set_word(TRACK_92, 0); // 0x14CD2
    api.obj_hide(OBJECT_92); // 0x14CDB
    api.walker_bind_graph(mover, 1, NODE_BOUND, 1); // 0x14CF0, 05DCh:0DB4h
    api.walker_walk_to(mover, NODE_FIRST, 1); // 0x14D05, answer not read
    face(api, 2); // 0x14D0D, 0x14D79
}

/// The animation hook, `1485h:0255h`: the code in `[bp+0Ah]`; codes `64h`
/// and `65h` have arms (`0x14AA8`–`0x14AB3`).
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        // 0x14AB7: one of six samples rolled (the table at cs:02BCh), then
        // as 65h.
        0x64 => {
            const SAMPLES: [u16; 6] = [0x1C, 2, 0x0A, 0x0B, 0x0C, 0x0D];
            let n = api.random(0, 5); // 0x14ABB
            if let Some(&k) = SAMPLES.get(usize::from(n)) {
                api.play_sample(k); // 0x14AE7, by 0x14AC5
            }
            meet_anim(api);
        }
        0x65 => meet_anim(api), // 0x14AED
        _ => {}
    }
}

/// `0x14AED` to `0x14B04`: a waiting script's task 0 started and the
/// meeting cleared, or the animation's arrival kept.
fn meet_anim(api: &mut dyn Api) {
    if num::signed(api.word(MEETING)) < 0 {
        api.task_start(0); // 0x14AF6
        api.set_word(MEETING, 0); // 0x14AFC
    } else {
        api.set_word(MEETING, 1); // 0x14B04
    }
}

/// The Requisite's rule handlers by their offset in segment `1485h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 1485:000B (0x1485B), the Perücke given to 92h: scene 4 and
        // [2BBBh] set.
        0x000B => {
            api.scene_play(4); // 0x14860
            api.set_byte(BYTE_2BBB, 1); // 0x14866
            0x0A
        }
        // 1485:0020 (0x14870), Sprich mit 92h: the first time the input
        // taken over and scene 0, then one of scenes 1 and 2.
        0x0020 => {
            if api.byte(BYTE_2BBD) == 0 {
                api.input_lock(); // 0x1487A
                api.scene_play(0); // 0x14881
                api.set_byte(BYTE_2BBD, 1); // 0x14887
            } else {
                let n = api.random(1, 2); // 0x14892
                api.scene_play(num::signed(n)); // 0x1489B
            }
            0x0A
        }
        // 1485:0056 (0x148A6), Sprich mit 93h: by [2BCEh], the table at
        // cs:00B3h; in state 0 the search goes on, above 5 scene 0Dh.
        0x0056 => {
            let n = match api.byte(BYTE_2BCE) {
                0 => return Some(0), // 0x148BC
                1 => 7,              // 0x148C0
                2 => 8,              // 0x148C5
                3 => {
                    api.set_byte(BYTE_2BCE, 4); // 0x148CD
                    9 // 0x148CA
                }
                // 0x148D4: with the Ente state 5 and scene 0Bh, else 0Ah.
                4 => {
                    if api.inv_has(ITEM_ENTE) != 0 {
                        api.set_byte(BYTE_2BCE, 5); // 0x148E0
                        0x0B // 0x148E5
                    } else {
                        0x0A // 0x148EA
                    }
                }
                5 => 0x0C, // 0x148EF
                _ => 0x0D, // 0x148F4
            };
            api.scene_play(n); // 0x148F8
            0x0A
        }
        // 1485:00BF (0x1490F), Benutze on the Perücke: the verb made 9, as
        // chosen and as it is, and the answer 6, a second object wanted.
        0x00BF => {
            api.set_word(VERB_CHOSEN, 9); // 0x14915
            api.set_word(VERB, 9); // 0x14918
            6
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
