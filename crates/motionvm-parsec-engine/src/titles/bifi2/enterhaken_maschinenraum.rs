//! Location 11, the Enterhaken - Maschinenraum, a row of kind 1 with one
//! exit, `6Fh`, to the Enterhaken - Brücke (location 10). The module is
//! named after the location as the game itself names it in its location
//! table (`Enterhaken - Maschinenraum`, the record's `+37h`, `250Ah:00F3h`).
//!
//! Re-expressed from the handler at `1379h:0015h` (image `0x137A5` to its
//! `retf` at `0x13904`), its hook `1379h:01CEh` (`0x1395E` to `0x13CA6`,
//! the record's `+3Fh`), its animation hook `1379h:0175h` (`0x13905` to
//! `0x1395D`, the record's `+43h`) and its one rule handler `1379h:0005h`
//! (`0x13795`–`0x137A4`), against [`Api`] alone. The location's rules are
//! the one record at `2446h:04C0h` (image `0x24920`, the record's `+3Bh`),
//! fourteen bytes, ended by the zero phase at `0x2492E`:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 0 | `80h` | 0 | — | `0005h` |
//!
//! Phase `80h` with verb 0 and object 0 is matched by the search a walk
//! stopped short of its target runs (`push 1080h ; call far 0E40h:009Ch`,
//! `0x0A4CB`), for any verb and any object; its answer `0Ah` is non-zero,
//! so the mode block is reset after it (`jnz` at `0x0A4D6` to `call 9BA2h`,
//! `0x0A4E8`).
//!
//! The location holds five figures, objects `F2h`, `F0h`, `F4h`, `F6h` and
//! `F8h` (also the row's bytes `+29h` on), their records kept at
//! `DS:2E96h` on, a far pointer each, and object `F9h`. On the first visit
//! (`[2BB4h]` clear) the player's graph has nodes `3E8h` to `3EBh` blocked,
//! so a walk that needs them ends in rule `0005h`, scene −13 (place 12 of
//! the shared bank); the location's own interaction with node `0FA0h`
//! (facing 3, scene 0) plays, `[2BB4h]` is set and object `F9h` is shown
//! on frame 0 with its state `+12h` cleared, still: the animator
//! (`03F7h`, the table at `cs:02BAh`, `0x422A`) does nothing for state 0
//! (`0x4202`), and in state 1 counts the frame `+9` round the object's
//! cels `+15h` (`0x40D8`). On every later visit the first four figures and
//! object `F9h` are hidden; figure `F8h` stays.
//!
//! The scene's hook drives the figures. Code 0, a scene claiming its
//! speakers (the argument their bits), with an argument above 1
//! starts the figures' play (`[18AAh]` 1); code 7's events `0Ah` and `0Bh`
//! move it on to 2 and 3, the others name the animation a figure takes
//! next; code 8 keeps the subject slot of the page it begins, and code
//! 9, at the page's end, gives each figure its next
//! animation. Event `18h` sends the player to node `0FA2h` and holds task 0
//! until the frame loop sees the player there. Event `10h` starts the
//! stage of figure 0 that hands over item `11h`, the `Tonspule` (the item
//! row at `DS:10AAh`): by the animation hook's code `FFFEh`, or at the
//! scene's end (code 5 or 6), which leads on to the Kaffee (location 1)
//! with entry 4, whose row at `DS:182Ch` names location 32, the Flur 3.
//! Stock.
//!
//! One byte of the logic's saved state block (`DS:2B98h`, `4Eh` bytes) is
//! the location's: `[2BB4h]`, set on the first visit (`0x1385F`); the
//! Tontechnik reads it (`0x1531B`).

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x1379;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x138ED`).
const OBJECT_STATES: u16 = 0x2018;

/// The five figures, in the order of their kept records (`0x137BD`–
/// `0x13800`).
const FIGURES: [u16; 5] = [0xF2, 0xF0, 0xF4, 0xF6, 0xF8];
/// The objects hidden on a later visit (`0x1387C`–`0x138A3`).
const HIDDEN_LATER: [u16; 5] = [0xF2, 0xF0, 0xF4, 0xF6, OBJECT_F9];
/// Object `F9h`, whose state `+12h` the handler and the hook write.
const OBJECT_F9: u16 = 0xF9;
/// An object record's state, `+12h` (`mov byte es:[bx+12h], …`,
/// `0x13875`, `0x13BCF`, `0x13C5D`).
const OBJECT_STATE: u16 = 0x12;

/// The player's graph nodes blocked on the first visit (`0x13814`–
/// `0x13848`).
const BLOCKED_NODES: [u16; 4] = [0x03E8, 0x03E9, 0x03EA, 0x03EB];
/// The node of the location's own interaction (`push 0FA0h`, `0x13854`).
const NODE_INTERACTION: u16 = 0x0FA0;
/// The nodes the hook's events `17h` and `18h` send the player to.
const NODE_17: u16 = 0x0FA1;
const NODE_18: u16 = 0x0FA2;
/// `[1796h]`: the player record's `+28h`, its node (`0x138C5`).
const PLAYER_NODE: u16 = WALKER + 0x28;

/// The logic's state byte (inside the saved block at `DS:2B98h`).
/// `[2BB4h]`: the first visit done.
const BYTE_2BB4: u16 = 0x2BB4;

/// The Tonspule, item `11h` (`0x13954`, `0x139E8`).
const TONSPULE: u16 = 0x11;

/// The Kaffee, location 1, and the entry the scene's end sends it with
/// (`0x139FC`, `0x13A02`).
const KAFFEE: u16 = 1;
const KAFFEE_ENTRY: u16 = 4;

/// The handler's and the hook's words, read and written in segment
/// `1379h` alone.
/// `[18AAh]`: the figures' play, 0 none, 1 to 3 its stages (`0x1397C`,
/// `0x13B9D`, `0x13BDF`); the image holds 1.
const PLAY: u16 = 0x18AA;
/// `DS:2E70h`: five words, the animation a figure falls back to.
const BASE: u16 = 0x2E70;
/// `DS:2E7Ah`: five words, the animation a figure takes next.
const PENDING: u16 = 0x2E7A;
/// `DS:2E84h`: five words, the animation a figure was last given.
const GIVEN: u16 = 0x2E84;
/// `[2E8Eh]`: the argument of the hook's last code 8, the subject slot of
/// the page that began.
const SLOT: u16 = 0x2E8E;
/// `[2E90h]`: cleared by code 0 (`0x13982`) and read nowhere.
const WORD_2E90: u16 = 0x2E90;
/// `[2E92h]`: the stage of figure 0's hand-over, 0 to 3.
const STAGE: u16 = 0x2E92;
/// `[2E94h]`: task 0 held until the player stands on node `0FA2h`.
const AWAITED: u16 = 0x2E94;
/// `DS:2E96h`: five far pointers, the figures' records.
const RECORDS: u16 = 0x2E96;

/// Figure `i`'s word in one of the three rows `BASE`, `PENDING`, `GIVEN`.
fn slot(row: u16, i: u16) -> u16 {
    row.wrapping_add(i << 1)
}

/// Figure `i`'s kept record (`[bx+2E96h]`, `[bx+2E98h]`, `bx = i × 4`).
fn record(api: &dyn Api, i: u16) -> Far {
    let at = RECORDS.wrapping_add(i << 2);
    Far {
        off: api.word(at),
        seg: api.word(at.wrapping_add(2)),
    }
}

fn keep(api: &mut dyn Api, at: u16, r: Far) {
    api.set_word(at.wrapping_add(2), r.seg);
    api.set_word(at, r.off);
}

/// `push k ; push [bx+2E98h] ; push [bx+2E96h] ; call far 03F7h:1CF9h`:
/// figure `i` on animation `k`.
fn anim(api: &mut dyn Api, i: u16, k: u16) {
    let r = record(api, i);
    api.obj_anim(r, num::signed(k));
}

/// `mov bx, ax ; mov es, dx ; mov byte es:[bx+12h], v`: the state of the
/// record just found or shown.
fn set_state(api: &mut dyn Api, r: Far, v: u8) {
    let at = Far {
        off: r.off.wrapping_add(OBJECT_STATE),
        seg: r.seg,
    };
    api.set_far_byte(at, v);
}

/// Figure `i`'s next animation given when it differs from the last one
/// (`mov ax, [PENDING+i] ; cmp ax, [GIVEN+i] ; jz`); answers whether it
/// did.
fn give(api: &mut dyn Api, i: u16) -> bool {
    let k = api.word(slot(PENDING, i));
    if k == api.word(slot(GIVEN, i)) {
        return false;
    }
    api.set_word(slot(GIVEN, i), k);
    anim(api, i, k);
    true
}

/// Everything before the frame loop, `0x137A5` to `0x138AE`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x137A5
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x137B1
    api.visit_enter(loc, entry); // 0x137B5
    for (i, id) in (0u16..).zip(FIGURES) {
        let r = api.obj_find(id); // 0x137C0, 0x137D0, 0x137E0, 0x137F0, 0x13800
        keep(api, RECORDS.wrapping_add(i << 2), r); // 0x137C6 … 0x1380A
    }
    if api.byte(BYTE_2BB4) == 0 {
        // 0x1380D: the first visit.
        for node in BLOCKED_NODES {
            let player = api.ds(WALKER);
            api.node_block(player, node); // 0x1381B, 0x1382A, 0x13839, 0x13848
        }
        api.interaction(NODE_INTERACTION, 3, 0); // 0x13857, 09B6h:16C4h
        api.set_byte(BYTE_2BB4, 1); // 0x1385F
        let r = api.obj_show_frame(OBJECT_F9, 0); // 0x13869
        set_state(api, r, 0); // 0x13875
    } else {
        for id in HIDDEN_LATER {
            api.obj_hide(id); // 0x1387F, 0x13888, 0x13891, 0x1389A, 0x138A3
        }
    }
    api.visit_finish_entry(); // 0x138A9
    api.set_word(WORDS.entry, 0); // 0x138AE
}

/// One turn of the frame loop, `0x138B4` to `0x138E5`: once the awaited
/// player stands on node `0FA2h`, task 0 starts.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x138B4
    api.dispatch(); // 0x138B9
    if api.word(AWAITED) != 0 && api.word(PLAYER_NODE) == NODE_18 {
        // 0x138BE, 0x138C5
        api.set_word(AWAITED, 0); // 0x138CD
        api.task_start(0); // 0x138D5
    }
    api.frame_pass(); // 0x138DB
    api.word(WORDS.entry) == 0 // 0x138E0
}

/// Everything after the frame loop, `0x138E7` to `0x13904`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x138F2
    api.visit_leave(); // 0x138FA
    api.slot_unload(); // 0x138FF
}

/// The animation hook, `1379h:0175h`: the code in `[bp+0Ah]`; codes
/// `FFFFh` and `FFFEh` have arms (`0x13908`–`0x13913`).
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        // 0x13915: figure 0 on animation 6 when its stage is 2; stage 3,
        // sample 7, object F9h hidden, the held page let go with 1.
        0xFFFF => {
            if api.word(STAGE) == 2 {
                anim(api, 0, 6); // 0x13926
            }
            api.set_word(STAGE, 3); // 0x1392E
            api.play_sample(7); // 0x13936
            api.obj_hide(OBJECT_F9); // 0x1393F
            api.page_release(1); // 0x13947, 080Ch:1A86h
        }
        // 0x1394E: stage 0 and the Tonspule given.
        0xFFFE => {
            api.set_word(STAGE, 0); // 0x1394E
            api.inv_give(TONSPULE); // 0x13956
        }
        _ => {}
    }
}

/// The hook, `1379h:01CEh`: code in `[bp+6]`, argument in `[bp+8]`, task
/// in `[bp+0Ah]` (not read). Codes above 9 and codes 1 to 3 do nothing
/// (`0x13968`, the table at `cs:0535h`, `0x13CC5`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0x13977: an argument above 1 starts the play, every figure on
        // animation 0 with base and last given 0, figure 1's base 1;
        // else no play.
        0 => {
            if num::signed(arg) <= 1 {
                api.set_word(PLAY, 0); // 0x139CE
                return;
            }
            api.set_word(PLAY, 1); // 0x1397C
            api.set_word(WORD_2E90, 0); // 0x13982
            api.set_word(STAGE, 0); // 0x13988
            for i in (0..5u16).rev() {
                // 0x1398E: si from 5, decremented before each turn.
                api.set_word(slot(BASE, i), 0); // 0x13997
                api.set_word(slot(GIVEN, i), 0); // 0x139A1
                anim(api, i, 0); // 0x139B6
            }
            api.set_word(slot(BASE, 1), 1); // 0x139C5
        }
        // 0x13B6C: argument 0Dh names animation 6 for figure 0.
        4 => {
            if arg == 0x0D {
                api.set_word(slot(PENDING, 0), 6); // 0x13B76
            }
        }
        // 0x139D7: the scene's end while the play runs: the Tonspule given
        // when figure 0's stage is not 0, no play, on to the Kaffee with
        // entry 4.
        5 | 6 => {
            if api.word(PLAY) == 0 {
                return; // 0x139DC
            }
            if api.word(STAGE) != 0 {
                api.inv_give(TONSPULE); // 0x139EA
                api.set_word(STAGE, 0); // 0x139F0
            }
            api.set_word(PLAY, 0); // 0x139F6
            api.set_word(WORDS.location, KAFFEE); // 0x139FC
            api.set_word(WORDS.entry, KAFFEE_ENTRY); // 0x13A02
        }
        7 => events(api, arg),        // 0x13B7F
        8 => api.set_word(SLOT, arg), // 0x13A0B
        9 => page_end(api),           // 0x13A12
        _ => {}
    }
}

/// Code 9, `0x13A12` to `0x13B69`: the figures' next animations, by the
/// play's stage.
fn page_end(api: &mut dyn Api) {
    match api.word(PLAY) {
        // 0x13A2A: slots 1, 3 and 4 raise the next animation of
        // figure slot − 1 from 0 to 1 (`[bx+2E78h]`, `bx` the slot
        // × 2); then figures 3 to 0 take theirs and fall back to their
        // base.
        1 | 3 => {
            let slot_k = api.word(SLOT);
            if matches!(slot_k, 1 | 3 | 4) {
                // 0x13A2D–0x13A3A
                let at = slot(PENDING, slot_k.wrapping_sub(1));
                if api.word(at) == 0 {
                    api.set_word(at, 1); // 0x13A51, by 0x13A44
                }
            }
            for i in (0..4u16).rev() {
                // 0x13A57: si from 4, decremented before each turn.
                give(api, i); // 0x13A68, 0x13A76, 0x13A88
                let base = api.word(slot(BASE, i));
                api.set_word(slot(PENDING, i), base); // 0x13A94, 0x13A9C
            }
        }
        2 => {
            // 0x13AAA: figure 1, falling back to its base only when given.
            if give(api, 1) {
                // 0x13AB3, 0x13ABF
                let base = api.word(slot(BASE, 1));
                api.set_word(slot(PENDING, 1), base); // 0x13AC7, 0x13ACA
            }
            // 0x13ACD: slot 2 rests figures 4 and 0 on 0, any other
            // gives figure 4 animation 1 and figure 0 animation 4.
            let (k4, k0) = if api.word(SLOT) == 2 { (0, 0) } else { (1, 4) };
            api.set_word(slot(PENDING, 4), k4); // 0x13AD6, 0x13AE4
            api.set_word(slot(PENDING, 0), k0); // 0x13ADC, 0x13AEA
            give(api, 4); // 0x13AF0, 0x13AF9, 0x13B05
            // 0x13B0D: figure 0 by its stage: in stage 0 given and its
            // next animation 0, in stage 1 the stage made 2, else left.
            if api.word(slot(PENDING, 0)) != api.word(slot(GIVEN, 0)) {
                match api.word(STAGE) {
                    0 => {
                        give(api, 0); // 0x13B27, 0x13B33
                        api.set_word(slot(PENDING, 0), 0); // 0x13B3B
                    }
                    1 => api.set_word(STAGE, 2), // 0x13B43
                    _ => {}
                }
            }
            give(api, 2); // 0x13B49, 0x13B55, 0x13B61
        }
        _ => {}
    }
}

/// Code 7, `0x13B7F`: while the play runs, events `0Ah` to `18h` through
/// the table at `cs:0517h` (`0x13CA7`); others do nothing.
fn events(api: &mut dyn Api, arg: u16) {
    if api.word(PLAY) == 0 {
        return; // 0x13B84
    }
    match arg {
        // 0x13B9D: stage 2; figure 1's base and next 2; figure 3 given and
        // on animation 0; object F9h's state 1; sample 5.
        0x0A => {
            api.set_word(PLAY, 2); // 0x13B9D
            api.set_word(slot(BASE, 1), 2); // 0x13BA6
            api.set_word(slot(PENDING, 1), 2); // 0x13BA9
            api.set_word(slot(GIVEN, 3), 0); // 0x13BAE
            anim(api, 3, 0); // 0x13BBA
            let r = api.obj_find(OBJECT_F9); // 0x13BC5
            set_state(api, r, 1); // 0x13BCF
            api.play_sample(5); // 0x13BD6
        }
        // 0x13BDF: stage 3, figure 1's base and next 1.
        0x0B => {
            api.set_word(PLAY, 3); // 0x13BDF
            base_1(api, 1); // 0x13BE5
        }
        0x0C => api.set_word(slot(PENDING, 1), 0), // 0x13BEA
        0x0D => base_1(api, 3),                    // 0x13BF3
        0x0E => base_1(api, 2),                    // 0x13BF8
        0x0F => api.set_word(slot(PENDING, 0), 3), // 0x13C04
        // 0x13C0D: figure 0's hand-over begins: stage 1, sample 1Ah, its
        // next animation 5.
        0x10 => {
            api.set_word(STAGE, 1); // 0x13C0D
            api.play_sample(0x1A); // 0x13C15
            api.set_word(slot(PENDING, 0), 5); // 0x13C1B
        }
        0x11 => api.set_word(slot(PENDING, 2), 2), // 0x13C24
        0x12 => api.set_word(slot(PENDING, 2), 3), // 0x13C2C
        0x13 => api.set_word(slot(PENDING, 2), 4), // 0x13C34
        0x14 => api.set_word(slot(PENDING, 2), 5), // 0x13C3C
        0x15 => api.set_word(slot(PENDING, 2), 6), // 0x13C44
        // 0x13C4C: object F9h shown on frame 0 with state 0, sample 6, figure 2's
        // next animation 7.
        0x16 => {
            let r = api.obj_show_frame(OBJECT_F9, 0); // 0x13C51
            set_state(api, r, 0); // 0x13C5D
            api.play_sample(6); // 0x13C64
            api.set_word(slot(PENDING, 2), 7); // 0x13C6A
        }
        // 0x13C72: the player walks to node 0FA1h facing 1; answer not
        // read.
        0x17 => {
            let player = api.ds(WALKER);
            api.walker_walk_to(player, NODE_17, 1); // 0x13C7B
        }
        // 0x13C85: the player walks to node 0FA2h facing 0, task 0 armed
        // and awaited there.
        0x18 => {
            let player = api.ds(WALKER);
            api.walker_walk_to(player, NODE_18, 0); // 0x13C8E
            api.task_arm(0); // 0x13C98
            api.set_word(AWAITED, 1); // 0x13C9E
        }
        _ => {}
    }
}

/// `0x13BFB`/`0x13BFE`: figure 1's base and next animation both `k`.
fn base_1(api: &mut dyn Api, k: u16) {
    api.set_word(slot(BASE, 1), k); // 0x13BFB
    api.set_word(slot(PENDING, 1), k); // 0x13BFE
}

/// The Enterhaken - Maschinenraum's rule handler by its offset in segment
/// `1379h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 1379:0005 (0x13795), a walk stopped short: scene −13.
        0x0005 => {
            api.scene_play(-13); // 0x1379A
            0x0A // 0x137A0
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
