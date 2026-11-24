//! *Captain Zins*' global rules: the handlers the table at `1F1Dh:0080h`
//! (image `0x1F250`, fourteen bytes to a row) names, which the rule search
//! asks after a location's own.
//!
//! Twenty-three of the twenty-four rows name a handler in the segment
//! `10C3h`, beside the location code; twenty-one handlers in all, each
//! re-expressed here against [`Api`] ([`api_rule`]), so that they run
//! against the same recording stand-in as the locations. Four of them speak
//! the hotspot's own line first (`10C3:0203`, `027F`, `0326`, `0376`), and
//! two ask the visit's callback (`10C3:011A`, `0141`).
//!
//! The last row, `0A01:0548` (verb 6, the options' gate), is the library's
//! own: the generation's [`Adventure::options_rule`], Dunkle Schatten's
//! `0EB9h:04BDh` but for the addresses ([`rule`]).
//!
//! Addresses are those of the unpacked `XP01.EXE`, `DGROUP` `1FC4h`; the
//! comments give image addresses (`10C3h:x` is `0x10C30 + x`).

use super::logic::{OBJECT, WALKER, WALKER_28};
use crate::num;
use crate::titles::parsec1994::{Adventure, Api, Far};

/// The segment of the global rule handlers.
const RULES: u16 = 0x10C3;

/// `[1E58h]`, the mode block's verb.
const VERB: u16 = 0x1E58;
/// `[1E60h]`, the second object.
const SECOND: u16 = 0x1E60;
/// `[1E62h]`, the second object's class.
const SECOND_CLASS: u16 = 0x1E62;
/// `[1E6Ah]`, the node the figure walks to.
const WALK_TARGET: u16 = 0x1E6A;
/// `[1E86h]`, `[1E88h]`: the gate the mode machine asks.
const GATE_OFF: u16 = 0x1E86;
const GATE_SEG: u16 = 0x1E88;
/// `[1E8Ch]`: the animation code a door's rule waits for.
const WAIT_CODE: u16 = 0x1E8C;
/// `[1990h]`: the door the location was entered through.
const ENTRY_DOOR: u16 = 0x1990;
/// `[1CE8h]` (word), `[1CF9h]`, `[1CFCh]` (bytes): story state the class
/// −6 rules and the rule of items 14 and 16 keep.
const WORD_1CE8: u16 = 0x1CE8;
const BYTE_1CF9: u16 = 0x1CF9;
const BYTE_1CFC: u16 = 0x1CFC;

/// A field of a far record.
fn at(rec: Far, field: u16) -> Far {
    Far {
        off: rec.off.wrapping_add(field),
        seg: rec.seg,
    }
}

/// A global rule read against [`Api`], by its handler's far address;
/// `None` for one that is not among them. `flags` is the phase the search
/// runs with, which none of them reads.
pub(crate) fn api_rule(api: &mut dyn Api, far: (u16, u16), _flags: u16) -> Option<i16> {
    if far.0 != RULES {
        return None;
    }
    Some(match far.1 {
        // 0x10D05, verb 2 on class −6: place 0 for something carried;
        // else the object is kept in `[1CE8h]`, place 1 the first time,
        // then one of places 2 to 8.
        0x00D5 => {
            let n = if api.inv_has(api.word(OBJECT)) != 0 {
                !0 // 0x10D16
            } else {
                let obj = api.word(OBJECT); // 0x10D1A
                api.set_word(WORD_1CE8, obj);
                if api.byte(BYTE_1CFC) == 0 {
                    // 0x10D20
                    api.set_byte(BYTE_1CFC, 1); // 0x10D27
                    !1
                } else {
                    !num::signed(api.random(2, 8)) // 0x10D34
                }
            };
            api.scene_play(n); // 0x10D3F
            0x0A
        }
        0x011A => kept_unless_called_back(api),
        0x0141 => carried_or_called_back(api),
        0x0203 => look(api),
        0x027F => take(api),
        0x0326 => use_(api),
        0x0376 => line_2(api),
        // 0x10DA4, verb 5 on items 14 and 16 (either order): both carried,
        // they go for item 17 and `[1CF9h]` becomes 2; else place 21.
        0x0174 => {
            if api.inv_has(0x0E) != 0 && api.inv_has(0x10) != 0 {
                // 0x10DA9, 0x10DB5
                api.inv_take(0x0E); // 0x10DC1
                api.inv_take(0x10); // 0x10DC9
                api.inv_give(0x11); // 0x10DD1
                api.set_byte(BYTE_1CF9, 2); // 0x10DD7
                1
            } else {
                api.scene_play(!21); // 0x10DE3
                0x0A
            }
        }
        // 0x10DEE, verb 9 on any two: place 12 for a carried second
        // object; place 13 for one of class −3, named in slot 9 first;
        // else place 14.
        0x01BE => {
            let n = if api.inv_has(api.word(SECOND)) != 0 {
                !12 // 0x10DF5
            } else if api.word(SECOND_CLASS) == 0xFFFD {
                // 0x10E03
                let rec = api.obj_find(api.word(SECOND)); // 0x10E0E
                api.slot_set_object(9, 0, rec); // 0x10E1A
                !13
            } else {
                !14 // 0x10E26
            };
            api.scene_play(n); // 0x10E28
            0x0A
        }
        // 0x10E8D, verb 2 on anything: place 15 for something carried,
        // 16 otherwise.
        0x025D => {
            let n = if api.inv_has(api.word(OBJECT)) != 0 {
                !15 // 0x10E94
            } else {
                !16
            };
            api.scene_play(n); // 0x10EA4
            0x0A
        }
        // 0x10FE4, verb 10 on anything: the walk verb's rule.
        0x03B4 => walk_rule(api),
        // The hover rules, phase 1 (0x1104C, 0x110AF, 0x110EF): the verb a
        // class of object means when none is picked.
        0x041C => verb(api, 1, 1),
        0x047F | 0x04BF => verb(api, 0x0A, 1),
        // 0x1105A, verb 7 on anything: the door's rule.
        0x042A => door_rule(api),
        // The phase-2 rules of verb 4 on the classes −4, −2 and −5
        // (0x110BD, 0x110CB, 0x11210): the verb becomes the walk verb — or
        // 7 for an object not live, 8 for a live class −5.
        0x048D => verb(api, 0x0A, 5),
        0x049B => {
            let live = api.obj_state(api.word(OBJECT)) != 0; // 0x110D2
            verb(api, if live { 0x0A } else { 7 }, 5)
        }
        0x05E0 => {
            let live = api.obj_state(api.word(OBJECT)) != 0; // 0x11217
            verb(api, if live { 8 } else { 7 }, 5)
        }
        // 0x110FD, verb 8 on anything: the eighth verb's rule.
        0x04CD => eighth_verb(api),
        // 0x11160, verb 5 and verb 4 on object 10: place 23.
        0x0530 => {
            api.scene_play(!23); // 0x11165
            0x0A
        }
        // 0x11170, verb 3 phase 2 on anything: 9 for something carried,
        // 12 otherwise.
        0x0540 => {
            let carried = api.inv_has(api.word(OBJECT)) != 0; // 0x11177
            verb(api, if carried { 9 } else { 0x0C }, 5)
        }
        _ => return None,
    })
}

/// `10C3h:011Ah` (0x10D4A): the object kept in `[1CE8h]` and place 22
/// when the visit's callback answers 0 for it (`call far [1980h]`,
/// `0x10D51`); else the search goes on.
fn kept_unless_called_back(api: &mut dyn Api) -> i16 {
    let obj = api.word(OBJECT);
    if api.callback(obj) != 0 {
        return 0; // 0x10D6D
    }
    api.set_word(WORD_1CE8, api.word(OBJECT)); // 0x10D5D
    api.scene_play(!22); // 0x10D62
    0x0A
}

/// `10C3h:0141h` (0x10D71): place 9 for something carried; else by the
/// callback (`0x10D8A`), place 10 for 0 and 11 for anything else.
fn carried_or_called_back(api: &mut dyn Api) -> i16 {
    let n = if api.inv_has(api.word(OBJECT)) != 0 {
        !9 // 0x10D82
    } else if api.callback(api.word(OBJECT)) == 0 {
        !10 // 0x10D93
    } else {
        !11 // 0x10D97
    };
    api.scene_play(n); // 0x10D99
    0x0A
}

/// `10C3h:0203h` (0x10E33): the hotspot's line 3 (`+22h`, `0x10E44`);
/// else for an item, `0 < id < 1Ah`, bank 2's place `~(id + 4000h)`
/// (`0x10E7B`), and for anything else the search goes on with 1
/// (`0x10E83`).
fn look(api: &mut dyn Api) -> i16 {
    if own_line(api, 3) {
        return 0x0A;
    }
    let id = num::signed(api.word(OBJECT));
    if (1..ITEMS_END).contains(&id) {
        api.scene_play(!(id.wrapping_add(0x4000))); // 0x10E7B
        0x0A
    } else {
        1
    }
}

/// `10C3h:027Fh` (0x10EAF): the hotspot's line 5 (`+2Ah`, `0x10EC0`);
/// else an item below `1Ah` that has a record and is live (`0x10EEB`,
/// `0x10EFE`) leaves the room — the live bit cleared between two of the
/// drawer's marks (`0x10F07`, `0x10F29`), its hotspot following it
/// (`0x10F1B`) — and goes into the player's hands (`0x10F35`); else one of
/// places 17 to 19, rolled (`0x10F40`).
fn take(api: &mut dyn Api) -> i16 {
    if !own_line(api, 5) && !take_item(api) {
        let n = !num::signed(api.random(0x11, 0x13)); // 0x10F40
        api.scene_play(n); // 0x10F4B
    }
    0x0A
}

/// `10C3h:0326h` (0x10F56): place 24 for something carried (`0x10F5E`);
/// else the hotspot's line 2 (`+1Eh`, `0x10F79`), else place 20.
fn use_(api: &mut dyn Api) -> i16 {
    if api.inv_has(api.word(OBJECT)) != 0 {
        api.scene_play(!0x18); // 0x10F9B
    } else if !own_line(api, 2) {
        api.scene_play(!0x14); // 0x10F9B
    }
    0x0A
}

/// `10C3h:0376h` (0x10FA6): the hotspot's line 2 (`+1Eh`, `0x10FB7`),
/// else place 21.
fn line_2(api: &mut dyn Api) -> i16 {
    if !own_line(api, 2) {
        api.scene_play(!0x15); // 0x10FD9
    }
    0x0A
}

/// The first id past the items, `1Ah` (`cmp word [1E5Ch], 1Ah`,
/// `0x10E64`, `0x10EE0`).
const ITEMS_END: i16 = 0x1A;

/// The chosen hotspot's line in slot `k`, played when there is one: the
/// record `[1E7Eh]` tested for null, then the far pointer at its `+16h +
/// 4k`, then `0849h:18A3h` (`0x10E37`–`0x10E5A`).
fn own_line(api: &mut dyn Api, k: u16) -> bool {
    let text = api.hotspot_own_text(k);
    if text == Far::NULL {
        return false;
    }
    api.say_text(text);
    true
}

/// The take of `10C3h:027Fh` (`0x10EE0`–`0x10F35`), when the object is an
/// item with a live record: whether it was taken.
fn take_item(api: &mut dyn Api) -> bool {
    let id = api.word(OBJECT);
    if num::signed(id) >= ITEMS_END {
        return false; // 0x10EE5
    }
    let rec = api.obj_find(id); // 0x10EEB
    if rec == Far::NULL || api.far_byte(at(rec, 2)) & 1 == 0 {
        return false; // 0x10EF9, 0x10F03
    }
    api.obj_invalidate(rec); // 0x10F07
    let flags = api.far_byte(at(rec, 2));
    api.set_far_byte(at(rec, 2), flags & 0xFE); // 0x10F12
    api.hotspot_follow(rec); // 0x10F1B
    api.obj_invalidate(rec); // 0x10F29
    api.inv_give(api.word(OBJECT)); // 0x10F35
    true
}

/// Every global rule of this program, by its handler's far address, for
/// the runtime; `None` for one this engine does not read.
pub(crate) fn rule(adv: &mut Adventure, far: (u16, u16), flags: u16) -> Option<i16> {
    match far {
        // `0A01h:0548h` (0x0A558): the sixth verb's rule installs the
        // options' gate and waits for the figure.
        (0x0A01, 0x0548) => Some(adv.options_rule()),
        _ => api_rule(adv, far, flags),
    }
}

/// `mov word [1E58h], verb ; mov ax, answer`.
fn verb(api: &mut dyn Api, verb: u16, answer: i16) -> i16 {
    api.set_word(VERB, verb);
    answer
}

/// `10C3h:03B4h` (0x10FE4), the walk verb's rule. It installs the gate
/// `10C3h:000Ch` first. The door the location was entered through, while
/// the figure does not stand at it, and an object not live are refused:
/// verb 7, answer −1. Anything else the figure walks to (node
/// `obj + 7D0h`); if it starts, the script takes the input and the node
/// is the walk's target. The answer is 9.
fn walk_rule(api: &mut dyn Api) -> i16 {
    api.set_word(GATE_SEG, RULES); // 0x10FE7
    api.set_word(GATE_OFF, 0x000C); // 0x10FED
    let obj = api.word(OBJECT);
    // 0x10FF3: `[1990h] == obj` and `[1204h] != obj` refuses.
    let refused = (api.word(ENTRY_DOOR) == obj && api.word(WALKER_28) != obj)
        || api.obj_state(api.word(OBJECT)) == 0; // 0x11009
    if refused {
        api.set_word(VERB, 7); // 0x1103C
        return -1; // 0x11042
    }
    let node = api.word(OBJECT).wrapping_add(0x07D0); // 0x11015
    let player = api.ds(WALKER);
    if api.walker_walk_to(player, node, -1) != 0 {
        // 0x11020, 0x11028
        api.input_lock(); // 0x1102C
        let node = api.word(OBJECT).wrapping_add(0x07D0); // 0x11031
        api.set_word(WALK_TARGET, node); // 0x11037
    }
    9 // 0x11047
}

/// `10C3h:042Ah` (0x1105A), verb 7: the object's record of kind 4 is a
/// door — its animation 1, `[1E8Ch] = 2`, answer 12. Anything else is
/// made live (bit 0 of `+2`) and marked for the drawer; answer 1. The
/// record is read without a null check (0x1106E).
fn door_rule(api: &mut dyn Api) -> i16 {
    let rec = api.obj_find(api.word(OBJECT)); // 0x11062
    if api.far_byte(rec) == 4 {
        // 0x11071
        api.obj_anim(rec, 1); // 0x11083
        api.set_word(WAIT_CODE, 2); // 0x1108B
        0x0C
    } else {
        let flags = api.far_byte(at(rec, 2));
        api.set_far_byte(at(rec, 2), flags | 1); // 0x11099
        api.obj_invalidate(rec); // 0x110A2
        1
    }
}

/// `10C3h:04CDh` (0x110FD), verb 8: a door plays its animation 0,
/// `[1E8Ch] = 3`, answer 12. Anything else is made not live between two
/// marks for the drawer; answer 1. The record is read without a null
/// check (0x11111).
fn eighth_verb(api: &mut dyn Api) -> i16 {
    let rec = api.obj_find(api.word(OBJECT)); // 0x11105
    if api.far_byte(rec) == 4 {
        // 0x11114
        api.obj_anim(rec, 0); // 0x11126
        api.set_word(WAIT_CODE, 3); // 0x1112E
        0x0C
    } else {
        api.obj_invalidate(rec); // 0x1113F
        let flags = api.far_byte(at(rec, 2));
        api.set_far_byte(at(rec, 2), flags & 0xFE); // 0x1114A
        api.obj_invalidate(rec); // 0x11153
        1
    }
}
