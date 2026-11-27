//! Location 27, Öliens, a row of kind 1: the room between door `6Eh` on to
//! the Flur 7. Stock (location 30) and door `7Bh` on to the Öliens-Halle
//! (location 28). The module is named after the location as the game
//! itself names it in its location table (`Öliens`, the record's `+37h`,
//! `250Ah:019Fh`).
//!
//! Re-expressed from the handler at `16A0h:00E9h` (image `0x16AE9` to its
//! `retf` at `0x16C53`), its hook `16A0h:0271h` (`0x16C71`, the record's
//! `+3Fh`), its animation hook `16A0h:025Ch` (`0x16C5C`, the record's
//! `+43h`) and its six rule handlers `16A0h:000Ch` to `00C9h`
//! (`0x16A0C`–`0x16AE8`), against [`Api`] alone. The segment's code ends
//! with the hook's table at `cs:0365h` (`0x16D65`–`0x16D78`); its first
//! twelve bytes end location 26's hook (`167Bh:01FEh`, `0x169AE` to
//! `0x16A0B`), and `0x16D79` is
//! `16D7h:0009h`, the Öliens-Halle's first rule. The location's rules are
//! the six records at `2446h:0A00h` (image `0x24E60`, the record's
//! `+3Bh`), fourteen bytes each:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 3 | 4 | `A1h` | — | `000Ch` |
//! | 3 | 4 | `F2h` | — | `0027h` |
//! | `0Dh` | 4 | `F0h` | — | `0042h` |
//! | 4 | 2 | `F0h` | — | `0084h` |
//! | 6 | 4 | `7Bh` | — | `0092h` |
//! | 7 | 4 | `7Bh` | — | `00C9h` |
//!
//! The verbs are, by the verb text row at `DS:116Ah`: 3 `Sprich mit`, 4
//! `Benutze`, 6 `Öffne`, 7 `Schließe`, `0Dh` `Drücke`. `Benutze` on `F0h`
//! becomes `Drücke` (rule `0084h`), and `Drücke` on `F0h` sets door
//! `7Bh` on animation 2 while the door object is live
//! (`0F48h:074Bh`), else on animation 1 — the first time, with `[2BD3h]`
//! still 0, scene `12h` in its place and `[2BD3h]` set (rule `0042h`).
//! `Öffne` on the door plays a scene while it is not live (`13h` once
//! `[2BD3h]` is 1, else `!1`) and clears the gate; `Schließe` plays scene
//! `13h` while it is live; otherwise both answer 0 and leave the door to
//! the rules after them.
//!
//! Three bytes of the logic's saved state block (`DS:2B98h`, `4Eh`
//! bytes) are written here: `[2BCCh]`, the location's state, `[2BD3h]`,
//! the button pressed once, and `[2BB8h]`, set to 1 on the first visit
//! (`0x16B37`) and read by no instruction of the program by its address.
//! `[2BCCh]` is shared with the Öliens-Halle: its rule `16D7h:0009h`
//! (`Benutze` item 5, `Schlüssel`, `mit` object `A0h`) writes 2
//! (`0x16D7C`), and its rules `0033h` and `0082h` compare it with 2
//! (`0x16DA6`, `0x16DF5`). On entry state 0 shows object `A1h`, hides
//! `F1h` and `F2h`, opens with the script's interaction with `A1h` and
//! becomes 1; state 1 shows `A1h` and hands one of scenes 1 to 4 to task
//! 1; state 2 hides `A1h`, shows `F1h` and `F2h`, names `F2h` in subject
//! slot 1, blocks nodes `3E8h` and `3E9h`, opens with the interaction
//! with `F2h` and becomes 3; state 3 hides `F2h` and becomes 4; above 3
//! nothing.
//!
//! The hook keeps the record it moves in `[2F78h]`/`[2F7Ah]` (object
//! `A1h`'s in states 0 and 1, `F2h`'s in state 2, null from state 3 on)
//! and three words of its own: `[2F76h]` bit 1 of code 0's argument,
//! `[2F74h]` the animation wanted and `[2F72h]` the animation given. All
//! of them lie outside the saved block and outside the `38h` bytes from
//! `DS:2D38h` the visit clears, and keep their values from visit to
//! visit; an entry in a state above 3 writes no record, so the null one
//! of state 3 stays. `03F7h:1CF9h` does nothing for a null record
//! (`0x05C73`), and [`Api::obj_anim`] skips a record that is not an
//! object's.
//!
//! An object's subject slot is `080Ch:1972h(slot, code, rec)`
//! (`0x09A32`), Captain Zins's `0849h:1A4Fh` (`0x09EDF`) instruction for
//! instruction but for the slot table (`1DD0h` against `17C8h`,
//! [`Api::slot_set_object`]).

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x16A0;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x16C3C`).
const OBJECT_STATES: u16 = 0x2018;

/// Door `7Bh`, to the Öliens-Halle.
const DOOR_7B: u16 = 0x7B;
/// Object `A1h`, spoken to by rule `000Ch`.
const OBJECT_A1: u16 = 0xA1;
const OBJECT_F1: u16 = 0xF1;
/// Object `F2h`, spoken to by rule `0027h`.
const OBJECT_F2: u16 = 0xF2;
/// Item `26h`, `Geld`, by the item row at `DS:10AAh`.
const ITEM_GELD: u16 = 0x26;

/// Nodes `3E8h` and `3E9h`, blocked in state 2 (`0x16BD3`, `0x16BE2`).
const NODE_3E8: u16 = 0x3E8;
const NODE_3E9: u16 = 0x3E9;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BB8h]`: set on the first visit.
const BYTE_2BB8: u16 = 0x2BB8;
/// `[2BCCh]`: the location's state, 0 to 4, written to 2 by location 28.
const STATE: u16 = 0x2BCC;
/// `[2BD3h]`: the button pressed once.
const BYTE_2BD3: u16 = 0x2BD3;

/// The hook's words, read and written in segment `16A0h` alone.
/// `[2F72h]`: the animation last given.
const ANIM_GIVEN: u16 = 0x2F72;
/// `[2F74h]`: the animation wanted.
const ANIM_WANTED: u16 = 0x2F74;
/// `[2F76h]`: code 0's argument `and 2`.
const ARG_BIT: u16 = 0x2F76;
/// `[2F78h]`/`[2F7Ah]`: the record the hook moves.
const REC: u16 = 0x2F78;

/// The mode block's verb `[2D38h]`.
const VERB: u16 = 0x2D38;
/// The mode block's gate `[2D66h]`/`[2D68h]` and the entry door `[1F3Ah]`.
const GATE_OFF: u16 = 0x2D66;
const GATE_SEG: u16 = 0x2D68;
const ENTRY_DOOR: u16 = 0x1F3A;

fn rec(api: &dyn Api) -> Far {
    Far {
        off: api.word(REC),
        seg: api.word(REC + 2),
    }
}

fn keep(api: &mut dyn Api, r: Far) {
    api.set_word(REC + 2, r.seg);
    api.set_word(REC, r.off);
}

/// `push k ; push id ; call far 06E3h:047Eh ; pop cx ; push dx ; push ax ;
/// call far 03F7h:1CF9h`: the object found and set on animation `k`.
fn anim(api: &mut dyn Api, id: u16, k: i16) {
    let r = api.obj_find(id);
    api.obj_anim(r, k);
}

/// `push k ; push word [2F7Ah] ; push word [2F78h] ; call far
/// 03F7h:1CF9h` (`0x16D53`–`0x16D5B`): the kept record on animation `k`.
fn anim_kept(api: &mut dyn Api, k: u16) {
    let r = rec(api);
    api.obj_anim(r, num::signed(k));
}

/// `push hi ; push lo ; call far 203Dh:0024h ; push ax ; call far
/// 080Ch:164Ch`: the roll is the scene.
fn roll(api: &mut dyn Api, lo: u16, hi: u16) {
    let n = api.random(lo, hi);
    api.scene_play(num::signed(n));
}

/// Whether door `7Bh` is live (`push 7Bh ; call far 0F48h:074Bh ; or ax,
/// ax`).
fn door_live(api: &mut dyn Api) -> bool {
    api.obj_state(DOOR_7B) != 0
}

/// Everything before the frame loop, `0x16AE9` to `0x16C1A`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x16AE9
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x16AF5
    api.visit_enter(loc, entry); // 0x16AF9
    // 0x16B01: unless the figure comes through door 7Bh, the door shows
    // frame 1.
    if api.word(WORDS.entry) != DOOR_7B {
        api.obj_show_frame(DOOR_7B, 1); // 0x16B0C, 0F48h:096Eh
    }
    // 0x16B14: an entry door 7Bh is forgotten.
    if api.word(ENTRY_DOOR) == DOOR_7B {
        api.set_word(ENTRY_DOOR, 0); // 0x16B1B
    }
    // 0x16B21: the state through the table at cs:0254h; above 3 nothing.
    match api.byte(STATE) {
        // 0x16B37
        0 => {
            api.set_byte(BYTE_2BB8, 1); // 0x16B37
            api.set_byte(STATE, 1); // 0x16B3C
            api.obj_hide(OBJECT_F1); // 0x16B44
            api.obj_hide(OBJECT_F2); // 0x16B4D
            let r = api.obj_show(OBJECT_A1); // 0x16B56
            keep(api, r); // 0x16B5C, 0x16B60
            api.interaction(OBJECT_A1, 3, 0); // 0x16B6A, 09B6h:16C4h
        }
        // 0x16B75
        1 => {
            let r = api.obj_show(OBJECT_A1); // 0x16B78
            keep(api, r); // 0x16B7E, 0x16B82
            let n = api.random(1, 4); // 0x16B8B
            api.ambient_play_one(num::signed(n), 0); // 0x16B94, answer not read
        }
        // 0x16B9E
        2 => {
            api.set_byte(STATE, 3); // 0x16B9E
            api.obj_hide(OBJECT_A1); // 0x16BA6
            api.obj_show(OBJECT_F1); // 0x16BAF, the record not kept
            let r = api.obj_show(OBJECT_F2); // 0x16BB8
            keep(api, r); // 0x16BBE, 0x16BC2
            api.slot_set_object(1, 0, r); // 0x16BCB, 080Ch:1972h
            let player = api.ds(WALKER);
            api.node_block(player, NODE_3E8); // 0x16BDA
            api.node_block(player, NODE_3E9); // 0x16BE9
            api.interaction(OBJECT_F2, 3, 9); // 0x16BF8 into 0x16B6A
        }
        // 0x16BFB
        3 => {
            api.set_byte(STATE, 4); // 0x16BFB
            api.obj_hide(OBJECT_F2); // 0x16C03
            keep(api, Far::NULL); // 0x16C09, 0x16C0F
        }
        _ => {}
    }
    api.visit_finish_entry(); // 0x16C15
    api.set_word(WORDS.entry, 0); // 0x16C1A
}

/// One turn of the frame loop, `0x16C20` to `0x16C34`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x16C20
    api.dispatch(); // 0x16C25
    api.frame_pass(); // 0x16C2A
    api.word(WORDS.entry) == 0 // 0x16C2F
}

/// Everything after the frame loop, `0x16C36` to `0x16C53`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x16C41
    api.visit_leave(); // 0x16C49
    api.slot_unload(); // 0x16C4E
}

/// The hook, `16A0h:0271h`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read. Codes above 9 and codes 1 to 3 do nothing
/// (`0x16C7A`, the table at `cs:0365h`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0x16C89: with bit 1 of the argument the animations start over;
        // without it a kept record goes to animation 0.
        0 => {
            api.set_word(ARG_BIT, arg & 2); // 0x16C8E
            if api.word(ARG_BIT) != 0 {
                // 0x16C91
                api.set_word(ANIM_WANTED, 1); // 0x16C98
                api.set_word(ANIM_GIVEN, 0xFFFF); // 0x16C9E
            } else if rec(api) != Far::NULL {
                // 0x16CA7
                anim_kept(api, 0); // 0x16CB3 into 0x16D5B
            }
        }
        // 0x16CF7: argument 25h, the animation given forgotten and the
        // Geld given.
        4 => {
            if arg == 0x25 {
                api.set_word(ANIM_GIVEN, 0xFFFF); // 0x16D00
                api.inv_give(ITEM_GELD); // 0x16D08
            }
        }
        // 0x16CB8: with the bit, the kept record on animation 0.
        5 | 6 => {
            if api.word(ARG_BIT) != 0 {
                anim_kept(api, 0); // 0x16CC2 into 0x16D5B
            }
        }
        7 => hook_events(api, arg),
        // 0x16CC7: animation 2 wanted for argument 1, else 1.
        8 => {
            let k = if arg == 1 { 2 } else { 1 };
            api.set_word(ANIM_WANTED, k); // 0x16CCC, 0x16CD5
        }
        // 0x16CDE: with the bit, a new animation wanted goes to the kept
        // record, unless the one given is 4.
        9 => {
            if api.word(ARG_BIT) == 0 {
                return; // 0x16CE3
            }
            let k = api.word(ANIM_WANTED);
            let given = api.word(ANIM_GIVEN);
            if k == given || given == 4 {
                return; // 0x16CEC, 0x16CF3
            }
            api.set_word(ANIM_GIVEN, k); // 0x16D4F
            anim_kept(api, k); // 0x16D5B
        }
        _ => {}
    }
}

/// Code 7, `0x16D10`: the script's events `0Ah` to `0Ch`.
fn hook_events(api: &mut dyn Api, arg: u16) {
    match arg {
        // 0x16D23, 0x16D49: animation 3 or 4 wanted, given and handed to
        // the kept record.
        0x0A | 0x0C => {
            let k = if arg == 0x0A { 3 } else { 4 }; // 0x16D23, 0x16D49
            api.set_word(ANIM_WANTED, k); // 0x16D4C
            api.set_word(ANIM_GIVEN, k); // 0x16D4F
            anim_kept(api, k); // 0x16D5B
        }
        // 0x16D28: door 7Bh on animation 1, the figure faces 3.
        0x0B => {
            anim(api, DOOR_7B, 1); // 0x16D2C, 0x16D34
            let player = api.ds(WALKER);
            api.figure_face(player, 3); // 0x16D42
        }
        _ => {}
    }
}

/// The animation hook, `16A0h:025Ch`: the code in `[bp+0Ah]`; code `64h`
/// plays sample `24h` (`0x16C62`, `0x16C69`).
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    if code == 0x64 {
        api.play_sample(0x24); // 0x16C69
    }
}

/// The Öliens' rule handlers by their offset in segment `16A0h`. None
/// reads the phase it is called with.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 16A0:000C (0x16A0C), Sprich mit A1h: one of scenes 5 to 8.
        0x000C => {
            roll(api, 5, 8); // 0x16A13, 0x16A1C
            0x0A
        }
        // 16A0:0027 (0x16A27), Sprich mit F2h: one of scenes 0Ah to 0Ch.
        0x0027 => {
            roll(api, 0x0A, 0x0C); // 0x16A2E, 0x16A37
            0x0A
        }
        // 16A0:0042 (0x16A42), Drücke F0h: a live door on animation 2;
        // else the first press scene 12h, later ones animation 1.
        0x0042 => {
            if door_live(api) {
                anim(api, DOOR_7B, 2); // 0x16A51 into 0x16A6F, 0x16A77
            } else if api.byte(BYTE_2BD3) == 0 {
                // 0x16A55
                api.set_byte(BYTE_2BD3, 1); // 0x16A5C
                api.scene_play(0x12); // 0x16A63
            } else {
                anim(api, DOOR_7B, 1); // 0x16A6B, 0x16A6F, 0x16A77
            }
            0x0A
        }
        // 16A0:0084 (0x16A84), Benutze F0h: verb 0Dh, answer 5.
        0x0084 => {
            api.set_word(VERB, 0x0D); // 0x16A87
            5 // 0x16A8D
        }
        // 16A0:0092 (0x16A92), Öffne door 7Bh: while it is not live scene
        // 13h after the first press, else scene !1, the gate cleared; a
        // live door leaves the search going on (0x16AC5).
        0x0092 => {
            if door_live(api) {
                0
            } else {
                let n = if api.byte(BYTE_2BD3) == 1 { 0x13 } else { !1 }; // 0x16AA1
                api.scene_play(n); // 0x16AAE
                api.set_word(GATE_SEG, 0); // 0x16AB4
                api.set_word(GATE_OFF, 0); // 0x16ABA
                0x0A
            }
        }
        // 16A0:00C9 (0x16AC9), Schließe door 7Bh: a live door plays scene
        // 13h; else the search goes on (0x16AE5).
        0x00C9 => {
            if door_live(api) {
                api.scene_play(0x13); // 0x16ADA
                0x0A
            } else {
                0
            }
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
