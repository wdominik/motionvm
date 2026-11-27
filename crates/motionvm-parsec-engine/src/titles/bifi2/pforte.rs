//! Location 4, the Pforte: the gate the Kaffee's first scene leads to
//! (entry 0 of the row at `DS:182Ch`). The module is named after the
//! location as the game itself names it in its location table (`Pforte`,
//! the record's `+37h`, `250Ah:00B4h`).
//!
//! Re-expressed from the handler at `10BCh:0196h` (image `0x10D56` to its
//! `retf` at `0x10EB6`), its hook `10BCh:030Ch` (`0x10ECC`, the record's
//! `+3Fh`), its animation hook `10BCh:02F7h` (`0x10EB7`, the record's
//! `+43h`) and its eight rule handlers, against [`Api`] alone. The
//! location's rules are the eleven records at `2446h:0000h` (image
//! `0x24460`, the record's `+3Bh`, handed to the installer `0E40h:0031h`
//! at `0x09DAB`–`0x09DB3`), fourteen bytes each as in Dunkle Schatten:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 3 | 4 | `A6h` | — | `000Eh` |
//! | 1 | 4 | `81h` | — | `00ABh` |
//! | 3 | 4 | `81h` | — | `00C9h` |
//! | 9 | `20h` | `15h` | `81h` | `0120h` |
//! | 9 | `20h` | 0 | `81h` | `0144h` |
//! | 9 | `20h` | `21h`, `2Ah`, `29h` | `A6h` | `000Eh` |
//! | 9 | `20h` | 0 | `A6h` | `009Bh` |
//! | 4 | 4 | `15h` | — | `0162h` |
//! | 4 | 4 | −1 | — | `0173h` |
//!
//! Two objects of the room carry the location: `81h`, whose record the
//! handler keeps at `[2D90h]` (`0x10D7C`), and `A6h`, shown on every visit
//! and kept at `[2D8Ch]` (`0x10DC7`). Four bytes of the logic's saved
//! state block (`DS:2B98h`, `4Eh` bytes, copied at `0x0F2ED`/`0x0F3EB`)
//! hold the location's progress: `[2B98h]` (cleared on every visit),
//! `[2B99h]` and `[2B9Ah]` around object `81h`, and `[2BD1h]` around
//! object `A6h` — which is also the sequence counter rule `000Eh` hands
//! `09B6h:15DBh`. The items the logic asks about or moves are, by the
//! item row at `DS:10AAh` (`0x0A59E`–`0x0A5AC`): 1 `Infrarotclip`, 2
//! `Besucherausweis`, `15h` `Gagbrille`, `16h` `Brille`, `1Eh` `Ball`,
//! `21h` `Autogramm`, `29h`/`2Ah` `Wasserpistole`, `2Bh` `Ente`.
//!
//! When `[2B9Ah]` is 0, the visit opens with the player walking to node
//! `81h` and rule `00C9h`'s scene played under mode 7 (`0x10E54`–`0x10E82`).
//!
//! An object's frame is `03F7h:03A0h(rec, k)` (`0x04310`), Dunkle
//! Schatten's `0883h:03EBh` with the same writes, only the rectangle
//! marked before the change there and after it here ([`Api::obj_frame`]).

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x10BC;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x10E9F`).
const OBJECT_STATES: u16 = 0x2018;

/// The two objects the location keeps records of.
const OBJECT_81: u16 = 0x81;
const OBJECT_A6: u16 = 0xA6;
/// `[2D90h]`/`[2D92h]`: object `81h`'s record (`0x10D7C`, `0x10D80`).
const REC_81: u16 = 0x2D90;
/// `[2D8Ch]`/`[2D8Eh]`: object `A6h`'s record (`0x10DC7`, `0x10DCB`).
const REC_A6: u16 = 0x2D8C;

/// Object `16h`, shown and hidden by the hook's code 4.
const OBJECT_16: u16 = 0x16;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
const BYTE_2B98: u16 = 0x2B98;
const BYTE_2B99: u16 = 0x2B99;
const BYTE_2B9A: u16 = 0x2B9A;
const BYTE_2BD1: u16 = 0x2BD1;

/// The hook's words, read and written in segment `10BCh` alone.
/// `[2D84h]`: the frame object `81h` (or `A6h`) is to take.
const FRAME_WANTED: u16 = 0x2D84;
/// `[2D86h]`: the frame last given.
const FRAME_GIVEN: u16 = 0x2D86;
/// `[2D88h]`: bit 2 of code 0's argument (`0x10EE7`).
const TRACK_A6: u16 = 0x2D88;
/// `[2D8Ah]`: set by code 7 with argument `0Ah` (`0x1101D`).
const TRACK_81: u16 = 0x2D8A;

/// The mode block: the verb `[2D38h]`, the mode `[2D3Ah]`, the object
/// chosen `[2D3Ch]`, the wait bits `[2D46h]`, the verb as chosen `[2D48h]`
/// (`0x0ABA4`: `mov [2D48h], ax ; mov [2D38h], ax` as a verb is chosen).
const VERB: u16 = 0x2D38;
const MODE: u16 = 0x2D3A;
const OBJECT: u16 = 0x2D3C;
const BITS: u16 = 0x2D46;
const VERB_CHOSEN: u16 = 0x2D48;
/// `[2D4Ah]`: the node the figure walks to.
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

/// Everything before the frame loop, `0x10D56` to `0x10E82`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x10D56
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x10D62
    api.visit_enter(loc, entry); // 0x10D66
    api.set_byte(BYTE_2B98, 0); // 0x10D6E
    let r81 = api.obj_find(OBJECT_81); // 0x10D76
    keep(api, REC_81, r81);
    // 0x10D83: below 2 in [2B99h] frame 5 and animation 0, else frame 12h
    // and animation 1.
    if api.byte(BYTE_2B99) < 2 {
        api.obj_frame(r81, 5); // 0x10D8E
        api.obj_anim(r81, 0); // 0x10DB6
    } else {
        let r = rec(api, REC_81);
        api.obj_frame(r, 0x12); // 0x10DA4
        api.obj_anim(r, 1); // 0x10DB6
    }
    let ra6 = api.obj_show(OBJECT_A6); // 0x10DC1
    keep(api, REC_A6, ra6);
    // 0x10DCE: object A6h's animation by [2BD1h].
    let state = api.byte(BYTE_2BD1);
    if state <= 3 {
        api.obj_anim(ra6, 1); // 0x10DD9
        if api.inv_has(0x21) != 0 {
            api.set_byte(BYTE_2BD1, 3); // 0x10DED
        }
    } else if state <= 8 {
        if state == 4 {
            api.set_byte(BYTE_2BD1, 5); // 0x10E02
        }
        let r = rec(api, REC_A6);
        api.obj_anim(r, 2); // 0x10E10
        if api.inv_has(0x2A) != 0 || api.inv_has(0x29) != 0 {
            // 0x10E1A, 0x10E26
            api.set_byte(BYTE_2BD1, 8); // 0x10E30
        }
    } else {
        let r = rec(api, REC_A6);
        api.obj_anim(r, 0); // 0x10E41
    }
    api.visit_finish_entry(); // 0x10E49
    api.set_word(WORDS.entry, 0); // 0x10E4E
    // 0x10E54: the first time round, a walk to node 81h and rule 00C9h's
    // scene under mode 7.
    if api.byte(BYTE_2B9A) == 0 {
        api.set_word(WALK_TARGET, OBJECT_81); // 0x10E60
        let player = api.ds(WALKER);
        api.walker_walk_to(player, OBJECT_81, 0); // 0x10E68
        api.set_word(MODE, 7); // 0x10E70
        api.set_word(BITS, 0x10); // 0x10E76
        rule_00c9(api); // 0x10E7F, push 0 ; push cs ; call 00C9h
    }
}

/// One turn of the frame loop, `0x10E83` to `0x10E97`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x10E83
    api.dispatch(); // 0x10E88
    api.frame_pass(); // 0x10E8D
    api.word(WORDS.entry) == 0 // 0x10E92
}

/// Everything after the frame loop, `0x10E99` to `0x10EB6`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x10EA4
    api.visit_leave(); // 0x10EAC
    api.slot_unload(); // 0x10EB1
}

/// The hook, `10BCh:030Ch`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read. Codes above 9 and codes 1 to 3 do nothing
/// (`0x10ED6`, the table at `cs:059Dh`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    let sarg = num::signed(arg);
    match code {
        // 0x10EE5: bit 2 of the argument kept, both frames forgotten.
        0 => {
            api.set_word(TRACK_A6, arg & 4); // 0x10EEA
            api.set_word(FRAME_WANTED, 0xFFFF); // 0x10EF0
            api.set_word(FRAME_GIVEN, 0xFFFF); // 0x10EF3
        }
        4 => hook_items(api, arg),
        // 0x10EF9.
        5 | 6 => {
            if api.word(TRACK_81) != 0 {
                let k = if api.byte(BYTE_2B99) < 2 { 3 } else { 4 };
                let r = rec(api, REC_81);
                api.obj_anim(r, k); // 0x10F15
            }
            api.set_word(TRACK_81, 0); // 0x10F1D
            if api.word(TRACK_A6) != 0 {
                // 0x10F2A, the table at cs:0589h.
                let k = match api.byte(BYTE_2BD1) {
                    0..=3 => Some(1), // 0x10F41
                    4 | 9 => Some(0), // 0x10F3D
                    5..=8 => Some(2), // 0x10F45
                    _ => None,
                };
                if let Some(k) = k {
                    let r = rec(api, REC_A6);
                    api.obj_anim(r, k); // 0x10F4F
                }
            }
            api.set_word(TRACK_A6, 0); // 0x10F57
        }
        // 0x11000.
        7 => match arg {
            5 => api.set_word(FRAME_WANTED, 7), // 0x11014
            0x0A => {
                api.set_word(TRACK_81, 1); // 0x1101D
                api.set_word(FRAME_WANTED, 2); // 0x11023
            }
            // 0x1102C: only while [2D8Ah] is set.
            0x0B if api.word(TRACK_81) != 0 => {
                api.set_word(TRACK_81, 0); // 0x11036
                let r = rec(api, REC_81);
                api.obj_anim(r, 3); // 0x110D8
            }
            _ => {}
        },
        // 0x10F60.
        8 => {
            if api.word(TRACK_81) != 0 {
                let k = if sarg > 0 { 5 } else { 6 };
                api.set_word(FRAME_WANTED, k); // 0x10F73
            }
            if api.word(TRACK_A6) == 0 {
                return; // 0x10F7D
            }
            // 0x10F80, the table at cs:0575h.
            match api.byte(BYTE_2BD1) {
                0..=3 => api.set_word(FRAME_WANTED, 1), // 0x10F9B
                4 | 9 => api.set_word(FRAME_WANTED, 0), // 0x10F93
                5..=8 => api.set_word(FRAME_WANTED, 2), // 0x10FA3
                _ => {}
            }
            if sarg > 1 {
                let k = api.word(FRAME_WANTED).wrapping_add(3);
                api.set_word(FRAME_WANTED, k); // 0x10FB1
            }
        }
        // 0x10FB9: two checks one after the other; the first one's write
        // makes the second one's frames equal.
        9 => {
            if api.word(TRACK_81) != 0 && api.word(FRAME_GIVEN) != api.word(FRAME_WANTED) {
                let k = api.word(FRAME_WANTED);
                api.set_word(FRAME_GIVEN, k); // 0x10FCC
                let r = rec(api, REC_81);
                api.obj_anim(r, num::signed(k)); // 0x10FD8
            }
            if api.word(TRACK_A6) == 0 {
                return; // 0x10FE7
            }
            if api.word(FRAME_GIVEN) == api.word(FRAME_WANTED) {
                return; // 0x10FF3
            }
            let k = api.word(FRAME_WANTED);
            api.set_word(FRAME_GIVEN, k); // 0x10FF9
            let r = rec(api, REC_A6);
            api.obj_anim(r, num::signed(k)); // 0x110D8
        }
        _ => {}
    }
}

/// Code 4, `0x11049`: the argument looked up in the row at `cs:054Dh`
/// (1 to 6, `2Bh` to `2Eh`), its arm in the row after it (`cs:0561h`).
fn hook_items(api: &mut dyn Api, arg: u16) {
    match arg {
        // 0x11066: Infrarotclip and Besucherausweis given.
        1 => {
            api.inv_give(1); // 0x11068
            api.inv_give(2); // 0x11070
        }
        // 0x11077: Gagbrille taken.
        2 => {
            api.inv_take(0x15); // 0x110E4
        }
        // 0x1107B.
        3 => api.set_byte(BYTE_2B99, 1),
        // 0x11083.
        4 => {
            api.set_word(TRACK_81, 0);
            let r = rec(api, REC_81);
            api.obj_anim(r, 8); // 0x11093
            api.obj_show(OBJECT_16); // 0x1109D
        }
        // 0x110A4.
        5 => {
            api.set_word(TRACK_81, 0);
            let r = rec(api, REC_81);
            api.obj_anim(r, 4); // 0x110D8
        }
        // 0x110B6: object 16h hidden, Brille given.
        6 => {
            api.obj_hide(OBJECT_16); // 0x110B8, 0F48h:08AAh
            api.inv_give(0x16); // 0x11070
        }
        // 0x110C2: Autogramm taken.
        0x2B => {
            api.inv_take(0x21); // 0x110E4
        }
        // 0x110C6: Ball given, A6h on animation 3.
        0x2C => {
            api.inv_give(0x1E); // 0x110C8
            let r = rec(api, REC_A6);
            api.obj_anim(r, 3); // 0x110D8
        }
        // 0x110E2: Wasserpistole (2Ah) taken.
        0x2D => {
            api.inv_take(0x2A); // 0x110E4
        }
        // 0x110EC: Ente given, A6h on animation 3, [2BD1h] 9.
        0x2E => {
            api.inv_give(0x2B); // 0x110EE
            let r = rec(api, REC_A6);
            api.obj_anim(r, 3); // 0x110FE
            api.set_byte(BYTE_2BD1, 9); // 0x11106
        }
        _ => {}
    }
}

/// The animation hook, `10BCh:02F7h`: the record in `[bp+6]`, the code in
/// `[bp+0Ah]` (`0x0B6D3`–`0x0B6DA`); code `64h` plays sample `0Eh`.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    if code == 0x64 {
        api.play_sample(0x0E); // 0x10EC4
    }
}

/// Rule `10BCh:00C9h`, also called by the entry (`0x10E7F`).
fn rule_00c9(api: &mut dyn Api) -> i16 {
    if api.byte(BYTE_2B99) >= 2 {
        api.scene_play(0x0B); // 0x10C95
    } else if api.byte(BYTE_2B9A) > 5 || api.byte(BYTE_2B98) == 1 {
        // 0x10CAB: one of scenes 6 to 8.
        let n = api.random(6, 8); // 0x10CAF
        api.scene_play(num::signed(n)); // 0x10C95
    } else {
        api.set_byte(BYTE_2B98, 1); // 0x10CBA
        let counter = api.ds(BYTE_2B9A);
        api.scene_sequence(counter, &[0, 1, 2, 3, 4, 5, 6]); // 0x10CD3
    }
    0x0A
}

/// The verb made 9, as chosen and as it is, and the answer 6.
fn verb_9(api: &mut dyn Api) -> i16 {
    api.set_word(VERB_CHOSEN, 9);
    api.set_word(VERB, 9);
    6
}

/// The Pforte's rule handlers by their offset in segment `10BCh`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 10BC:000E (0x10BCE): by [2BD1h], the table at cs:0087h; the
        // sequence counter is [2BD1h] itself. Above 9 nothing.
        0x000E => {
            let counter = api.ds(BYTE_2BD1);
            match api.byte(BYTE_2BD1) {
                0..=2 => api.scene_sequence(counter, &[0x11, 0x12, 0x13]), // 0x10BF0
                3 | 5..=7 => {
                    api.scene_sequence(counter, &[0x11, 0x12, 0x13, 0x14, 0x1A, 0x15, 0x16, 0x17]);
                } // 0x10C10
                4 | 9 => {
                    api.scene_play(0x1A); // 0x10C3C
                }
                8 => {
                    // 0x10C1E
                    if api.inv_has(0x29) != 0 {
                        api.scene_play(0x18);
                    } else if api.inv_has(0x2A) != 0 {
                        // 0x10C30
                        api.scene_play(0x19);
                    }
                }
                _ => {}
            }
            0x0A // 0x10C42
        }
        // 10BC:009B (0x10C5B): scene 1Bh.
        0x009B => {
            api.scene_play(0x1B); // 0x10C60
            0x0A
        }
        // 10BC:00AB (0x10C6B): scene 0Dh, or 0Eh from [2B99h] 2 on.
        0x00AB => {
            let n = if api.byte(BYTE_2B99) < 2 { 0x0D } else { 0x0E };
            api.scene_play(n); // 0x10C7E
            0x0A
        }
        // 10BC:00C9 (0x10C89).
        0x00C9 => rule_00c9(api),
        // 10BC:0120 (0x10CE0): scene 9 while [2B99h] is 0, scene 0Ah
        // queued, [2B99h] 2.
        0x0120 => {
            if api.byte(BYTE_2B99) < 1 {
                api.scene_play(9); // 0x10CEC
            }
            api.scene_enqueue(0x0A); // 0x10CF4
            api.set_byte(BYTE_2B99, 2); // 0x10CFA
            0x0A
        }
        // 10BC:0144 (0x10D04): scene 0Fh, or 10h from [2B99h] 2 on.
        0x0144 => {
            let n = if api.byte(BYTE_2B99) < 2 { 0x0F } else { 0x10 };
            api.scene_play(n); // 0x10D17
            0x0A
        }
        // 10BC:0162 (0x10D22): verb 9, answer 6.
        0x0162 => verb_9(api), // 0x10D28, 0x10D2B
        // 10BC:0173 (0x10D33): verb 9 and answer 6 for an item carried,
        // else 0.
        0x0173 => {
            let obj = api.word(OBJECT);
            if api.inv_has(obj) != 0 {
                // 0x10D3A
                verb_9(api) // 0x10D47, 0x10D4A
            } else {
                0 // 0x10D52
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
