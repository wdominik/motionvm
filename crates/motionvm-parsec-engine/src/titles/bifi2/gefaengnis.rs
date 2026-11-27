//! Location 41, Schlönstadt's Gefängnis, a row of kind 4 with three exits:
//! `6Eh` and `72h` to location 6, `73h` to location 46. The module is
//! named after the location as the game itself names it in its location
//! table (`Schlönstadt's Gefängnis`, the record's `+37h`, `250Ah:1202h`).
//!
//! Re-expressed from the handler at `1776h:0022h` (image `0x17782` to its
//! `retf` at `0x1791F`), its hook `1776h:01C0h` (`0x17920` to its `retf`
//! at `0x17A7A`, the record's `+3Fh`; its tables at `cs:031Bh`, `0x17A7B`,
//! and `cs:0325h`, `0x17A85`) and its two rule handlers `1776h:0002h` and
//! `0012h` (`0x17762`–`0x17781`), against [`Api`] alone. The record's
//! animation hook `+43h` is `0000h:0000h`. The immediate `1776h` occurs
//! only in the location record (`0x22329`, `0x22335`) and the rule table
//! (`0x2504C`, `0x2505A`): the location has no gate. The location's rules
//! are the two records at `2446h:0BE0h` (image `0x25040`, the record's
//! `+3Bh`), fourteen bytes each:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 1 | 4 | `1Ah` | — | `0002h` |
//! | 1 | 4 | `1Bh` | — | `0012h` |
//!
//! Verb 1 is `Betrachte`, by the verb text row at `DS:116Ah`; the items
//! are, by the item row at `DS:10AAh`, `1Ah` `Stadtplan` and `1Bh`
//! `Labyrinth-Karte`. The labyrinth's rows (40, 42 to 45, 47 to 90) share
//! the `Stadtplan` rule's twin `1746h:0006h` (scene 3, `0x17466`); for the
//! `Labyrinth-Karte` they have `1746h:0016h` (`0x17476`), which reads the
//! labyrinth's own table, where the Gefängnis has scene 9 alone.
//!
//! Row 46 is the only row whose exits lead here (`6Fh` and `73h`). Its
//! door `73h` opens by its rule `17A9h:0005h` (`0x17A95`) once `[2BD6h]`
//! is 1, which its rule `17A9h:004Ah` (`0x17ADA`, `Benutze` object `F9h`
//! with the `Labyrinth-Karte` carried) sets. The labyrinth's distances are
//! counted from this row (`push 29h ; call 7556h`, `0x1767C`).
//!
//! `[2BD0h]` in the logic's saved state block (`DS:2B98h`, `4Eh` bytes)
//! is the first visit: object `F4h` is shown on animation 0 and named by
//! subject slot 1, `F8h` hidden, the input taken, and the script's
//! interaction with node `0FA0h` plays scene 2 (`0x177F2`); the Bi-Fi
//! (location 12) reads the byte (`cmp byte [2BD0h], 1`, `0x13D3E`).
//! Every later visit has `F4h` hidden and `F8h` shown. Object `72h` is
//! shown on frame 1 and made a door, kind 4 (`0x17832`), for the visit,
//! and set back to kind 0 before the objects' states are kept
//! (`0x178FD`).
//!
//! The hook keeps a state of its own in `[2F9Ch]`: 1 and 0 by code 0,
//! 2 by the script event `0Dh` (`F4h` hidden, `F8h` and `F6h` shown, `F6h`
//! named by subject slot 1, the player walking to node `0FA0h`), 3 by the
//! event `0Eh`, which binds the walking record `FEh` to graph 1 on node
//! `0BB8h`, names it by subject slot 1 and sends it to node `0BB9h`. In
//! state 3 the frame loop takes the record on: at `0BB9h` to `0BBAh` with
//! `72h` on animation 1 and the player facing 1, at `0BBAh` to `0BBBh`
//! with `72h` on animation 0, and at `0BBBh` it is taken out of the draw
//! chain, the state goes back to 0 and the input is given back.
//!
//! The visit writes `[200Eh]` after the entry's end (`0x1783E`, after
//! `09B6h:00BBh` at `0x17836`), as location 6 does: the entry's end reads
//! the location run last against this one.
//!
//! The words the location keeps, each read or written in segment `1776h`
//! alone, lie outside every block the save copies: `[2F9Ch]` (cleared on
//! entry, `0x177A0`), `[2F9Eh]`/`[2FA0h]` and `[2FA2h]`/`[2FA4h]`. The
//! latter is written only on the first visit (`0x177BA`) and by event
//! `0Dh` (`0x179F7`); hook codes 0 to 3 on a later visit act on the record
//! a visit before left there.

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x1776;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x17907`).
const OBJECT_STATES: u16 = 0x2018;

/// The objects the logic names.
const OBJECT_72: u16 = 0x72;
const OBJECT_F4: u16 = 0xF4;
const OBJECT_F6: u16 = 0xF6;
const OBJECT_F8: u16 = 0xF8;
/// The walking record `FEh` (`push 0FEh ; call far 06E3h:04EFh`,
/// `0x17A2F`).
const MOVER_FE: u16 = 0xFE;

/// The nodes: `0FA0h` the first visit's interaction (`0x177EF`) and event
/// `0Dh`'s walk (`0x179C5`), `0F8h` event `0Ah`'s (`0x17990`), and the
/// walking record's `0BB8h` to `0BBBh` (`0x17A4F`, `0x17863`–`0x1786D`).
const NODE_FA0: u16 = 0x0FA0;
const NODE_F8: u16 = 0xF8;
const NODE_BB8: u16 = 0x0BB8;
const NODE_BB9: u16 = 0x0BB9;
const NODE_BBA: u16 = 0x0BBA;
const NODE_BBB: u16 = 0x0BBB;

/// `[2BD0h]`: the first visit done (inside the saved block at
/// `DS:2B98h`); read by location 12 (`0x13D3E`).
const BYTE_2BD0: u16 = 0x2BD0;

/// `[2F9Ch]`: the hook's state, 0 to 3.
const STATE: u16 = 0x2F9C;
/// `[2F9Eh]`/`[2FA0h]`: the walking record `FEh` (`0x17A38`, `0x17A3C`).
const REC_FE: u16 = 0x2F9E;
/// `[2FA2h]`/`[2FA4h]`: object `F4h`'s record (`0x177B6`, `0x177BA`), then
/// `F6h`'s (`0x179F3`, `0x179F7`).
const REC_SHOWN: u16 = 0x2FA2;

/// The handler's own stack frame: `[bp-4]`/`[bp-2]`, the far pointer
/// `0F48h:096Eh` answered for object `72h` (`0x17822`, `0x1781F`).
const REC_72_OFF: usize = 0;
const REC_72_SEG: usize = 1;

/// The object record's kind byte `+0` (`mov byte es:[bx], 4`, `0x17832`):
/// 4 a door, which `0F48h:074Bh` answers by its `+2` bit 1 (`0x0FBEB`–
/// `0x0FBFF`), any other kind by 3 (`0x0FBF1`).
const KIND_DOOR: u8 = 4;
const KIND_PLAIN: u8 = 0;

/// A walking record's node, `+28h` (`mov ax, es:[bx+28h]`, `0x1785F`).
const MOVER_NODE: u16 = 0x28;

/// The mode block's wait bits `[2D46h]` and walk target `[2D4Ah]`.
const BITS: u16 = 0x2D46;
const WALK_TARGET: u16 = 0x2D4A;

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

fn rec_72(api: &dyn Api) -> Far {
    Far {
        off: api.local(REC_72_OFF),
        seg: api.local(REC_72_SEG),
    }
}

/// `push k ; push word [2FA4h] ; push word [2FA2h] ; call far
/// 03F7h:1CF9h`: the shown record on animation `k` (`0x17943`, `k` 0;
/// `0x17A08`, `k` 1).
fn anim_shown(api: &mut dyn Api, k: i16) {
    let r = rec(api, REC_SHOWN);
    api.obj_anim(r, k);
}

/// `push k ; push word [bp-2] ; push word [bp-4] ; call far 03F7h:1CF9h`.
fn anim_72(api: &mut dyn Api, k: i16) {
    let r = rec_72(api);
    api.obj_anim(r, k);
}

/// `les bx, [bp-4] ; mov byte es:[bx], v`: object `72h`'s kind.
fn set_kind_72(api: &mut dyn Api, v: u8) {
    let r = rec_72(api);
    api.set_far_byte(r, v);
}

/// `push dir ; push ds ; push 176Eh ; call far 05DCh:0B56h`.
fn face(api: &mut dyn Api, dir: i16) {
    let player = api.ds(WALKER);
    api.figure_face(player, dir);
}

/// `push facing ; push node ; push word [2FA0h] ; push word [2F9Eh] ; call
/// far 05DCh:05C8h`: the walking record `FEh` on to a node; the answer is
/// not read.
fn mover_walk(api: &mut dyn Api, node: u16, facing: i16) {
    let m = rec(api, REC_FE);
    api.walker_walk_to(m, node, facing);
}

/// `les bx, [2F9Eh] ; mov ax, es:[bx+28h]`.
fn mover_node(api: &mut dyn Api) -> u16 {
    let m = rec(api, REC_FE);
    api.far_word(Far {
        off: m.off.wrapping_add(MOVER_NODE),
        seg: m.seg,
    })
}

/// Everything before the frame loop, `0x17782` to `0x17841`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x17786
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.visit_enter(loc, entry); // 0x17793
    // 0x1779B: `09B6h:1820h` is `or word [2D46h], 8 ; retf` (0x0B380).
    let bits = api.word(BITS);
    api.set_word(BITS, bits | 8);
    api.set_word(STATE, 0); // 0x177A0
    if api.byte(BYTE_2BD0) == 0 {
        // 0x177A6: the first visit.
        let r = api.obj_show(OBJECT_F4); // 0x177B0
        keep(api, REC_SHOWN, r); // 0x177B6, 0x177BA
        api.slot_set_object(1, 0, r); // 0x177C3, 080Ch:1972h
        api.input_lock(); // 0x177CB
        anim_shown(api, 0); // 0x177DA
        api.obj_hide(OBJECT_F8); // 0x177E5
        api.interaction(NODE_FA0, 0, 2); // 0x177F2, 09B6h:16C4h
        api.set_byte(BYTE_2BD0, 1); // 0x177FA
    } else {
        api.obj_hide(OBJECT_F4); // 0x17804
        api.obj_show(OBJECT_F8); // 0x1780D, answer not read
    }
    let r = api.obj_show_frame(OBJECT_72, 1); // 0x17817
    api.set_local(REC_72_SEG, r.seg); // 0x1781F
    api.set_local(REC_72_OFF, r.off); // 0x17822
    api.hotspot_follow(r); // 0x17827, 0F48h:06B9h
    set_kind_72(api, KIND_DOOR); // 0x17832
    api.visit_finish_entry(); // 0x17836
    // 0x1783B: here is written after the entry's end.
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x1783E
    api.set_word(WORDS.entry, 0); // 0x17841
}

/// One turn of the frame loop, `0x17847` to `0x178F7`: in state 3 the
/// walking record `FEh` is taken on by the node it stands on.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x17847
    api.dispatch(); // 0x1784C
    if api.word(STATE) == 3 {
        // 0x17851
        match mover_node(api) {
            // 0x17874
            NODE_BB9 => {
                mover_walk(api, NODE_BBA, 1); // 0x17881
                face(api, 1); // 0x1788F
                anim_72(api, 1); // 0x1789F
            }
            // 0x178A9
            NODE_BBA => {
                anim_72(api, 0); // 0x178B1
                mover_walk(api, NODE_BBB, 1); // 0x178C6
            }
            // 0x178D0
            NODE_BBB => {
                let m = rec(api, REC_FE);
                api.walker_detach(m); // 0x178D8, 05DCh:0E45h
                api.set_word(STATE, 0); // 0x178E0
                api.input_unlock(); // 0x178E6
            }
            _ => {}
        }
    }
    api.frame_pass(); // 0x178EB
    api.word(WORDS.entry) == 0 // 0x178F0
}

/// Everything after the frame loop, `0x178FA` to `0x1791F`.
pub(crate) fn leave(api: &mut dyn Api) {
    set_kind_72(api, KIND_PLAIN); // 0x178FD
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x1790C
    api.visit_leave(); // 0x17914
    api.slot_unload(); // 0x17919
}

/// The hook, `1776h:01C0h`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read. Codes 0 to 7 go through the table at `cs:0325h`
/// (`0x17933`); codes 4 to 6 and every code above 7 do nothing
/// (`0x17A79`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0x17938: with bit 1 of the argument state 1 and the shown record
        // on animation 0, else state 0.
        0 => {
            if arg & 2 != 0 {
                api.set_word(STATE, 1); // 0x1793D
                anim_shown(api, 0); // 0x1794D
            } else {
                api.set_word(STATE, 0); // 0x17958
            }
        }
        // 0x17961: in state 2 the shown record on animation 1.
        1 | 3 => {
            if api.word(STATE) == 2 {
                anim_shown(api, 1); // 0x17A08, 0x1794D
            }
        }
        // 0x1796E: in state 2 the shown record on animation 0.
        2 => {
            if api.word(STATE) == 2 {
                anim_shown(api, 0); // 0x17943, 0x1794D
            }
        }
        7 => hook_events(api, arg),
        _ => {}
    }
}

/// Code 7, `0x1797A`: the argument less `0Ah`, 0 to 4, through the table
/// at `cs:031Bh` (`0x17989`); any other argument does nothing.
fn hook_events(api: &mut dyn Api, arg: u16) {
    match arg {
        // 0x1798E: the player walks to node F8h.
        0x0A => {
            api.set_word(WALK_TARGET, NODE_F8); // 0x17993
            let player = api.ds(WALKER);
            api.walker_walk_to(player, NODE_F8, 0); // 0x17A71, answer not read
        }
        // 0x1799E: the input given back, turn 0, the shown record on
        // animation 1.
        0x0B => {
            api.input_unlock(); // 0x1799E
            api.figure_turn(0); // 0x179A5, answer not read
            anim_shown(api, 1); // 0x17A08, 0x1794D
        }
        // 0x179AD: the player faces 0, the shown record on animation 0.
        0x0C => {
            face(api, 0); // 0x179B3
            anim_shown(api, 0); // 0x17943, 0x1794D
        }
        // 0x179BD: state 2; the player walks to node 0FA0h; F4h hidden,
        // F8h shown, F6h shown, kept, named by subject slot 1 and on
        // animation 1.
        0x0D => {
            api.set_word(STATE, 2); // 0x179BD
            api.set_word(WALK_TARGET, NODE_FA0); // 0x179C8
            let player = api.ds(WALKER);
            api.walker_walk_to(player, NODE_FA0, 3); // 0x179D0, answer not read
            api.obj_hide(OBJECT_F4); // 0x179DB
            api.obj_show(OBJECT_F8); // 0x179E4, answer not read
            let r = api.obj_show(OBJECT_F6); // 0x179ED
            keep(api, REC_SHOWN, r); // 0x179F3, 0x179F7
            api.slot_set_object(1, 0, r); // 0x17A00, 080Ch:1972h
            anim_shown(api, 1); // 0x17A08, 0x1794D
        }
        // 0x17A0D: state 3; the input taken, the player faces 0, F6h
        // hidden; the walking record FEh kept, named by subject slot 1,
        // bound to graph 1 on node 0BB8h and sent to node 0BB9h.
        0x0E => {
            api.set_word(STATE, 3); // 0x17A0D
            api.input_lock(); // 0x17A13
            face(api, 0); // 0x17A1E
            api.obj_hide(OBJECT_F6); // 0x17A29
            let m = api.mover_find(MOVER_FE); // 0x17A32, 06E3h:04EFh
            keep(api, REC_FE, m); // 0x17A38, 0x17A3C
            api.slot_set_figure(1, 0, m); // 0x17A45, 080Ch:19B4h
            let m = rec(api, REC_FE);
            api.walker_bind_graph(m, 1, NODE_BB8, 1); // 0x17A5C, 05DCh:0DB4h
            mover_walk(api, NODE_BB9, 1); // 0x17A71
        }
        _ => {}
    }
}

/// The location's rule handlers by their offset in segment `1776h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 1776:0002 (0x17762), Betrachte the Stadtplan: scene 3.
        0x0002 => {
            api.scene_play(3); // 0x17767
            0x0A
        }
        // 1776:0012 (0x17772), Betrachte the Labyrinth-Karte: scene 9.
        0x0012 => {
            api.scene_play(9); // 0x17777
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
