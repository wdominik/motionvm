//! Location 25, the VIP-Lounge, a row of kind 1: the room behind door
//! `78h` of the Flur Erdgeschoß (location 18, which the `VIP-Karte` opens,
//! see [`super::flur`]), with door `79h` on to the Vorführraum (location
//! 26). The module is named after the location as the game itself names
//! it in its location table (`VIP-Lounge`, the record's `+37h`,
//! `250Ah:0188h`).
//!
//! Re-expressed from the handler at `1624h:0157h` (image `0x16397` to its
//! `retf` at `0x164C0`), its hook `1624h:032Dh` (`0x1656D`, the record's
//! `+3Fh`), its animation hook `1624h:0281h` (`0x164C1`, the record's
//! `+43h`) and its eight rule handlers `1624h:0007h` to `013Ch`
//! (`0x16247`–`0x16396`), against [`Api`] alone. The location's rules are
//! the ten records at `2446h:08C0h` (image `0x24D20`, the record's
//! `+3Bh`), fourteen bytes each; a class is stored complemented (`FFFBh`
//! class 4, `FFFCh` class 3):
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 6 | 4 | `79h` | — | `0007h` |
//! | 3 | 4 | `79h` | — | `004Ch` |
//! | 5 | `20h` | 4 | `79h` | `0072h` |
//! | 5 | `20h` | class 3 | `79h` | `0098h` |
//! | 5 | `20h` | class 4 | `79h` | `00C7h` |
//! | 5 | `20h` | `0Dh` | `79h` | `00FBh` |
//! | 5 | `20h` | `0Dh`, class 3, class 4 | 0 | `013Ch` |
//! | 3 | 4 | `9Eh` | — | `0121h` |
//!
//! The verbs are, by the verb text row at `DS:116Ah` (indexed by `[2D38h]`
//! at `0x0B02D`): 3 `Sprich mit`, 5 `Benutze … mit`, 6 `Öffne`. Door `79h`
//! is locked while `[2BC9h]` is 0: `Öffne` on it stops there (rule
//! `0007h`) until item 4, the `Schlüssel`, used on it sets the byte (rule
//! `0072h`, `0x162C4`); then `Öffne` and `Sprich mit` leave the door to
//! the rules after them, and every other rule on the door plays scene 5.
//! Five bytes
//! of the logic's saved state block (`DS:2B98h`, `4Eh` bytes) hold the
//! location's progress: `[2BC9h]` the door, `[2BCAh]` and `[2BD8h]` the
//! counters of two scene sequences, `[2BD7h]` the first visit, and
//! `[2BA8h]`, set to 1 on the first visit and carried on by other
//! locations: 12 (`Bi-Fi`) compares it with 1 (`0x13CF7`) and writes 2
//! (`0x13D2E`), 22 (`Maske`) compares it with 2 (`0x1596D`) and writes 3
//! (`0x15A45`), 8 (`Jones 1`) compares it with 3 (`0x125A2`). The items the logic names are, by the item row at
//! `DS:10AAh`: 4 `Schlüssel`, `0Dh` `Haarnadel`, `1Ch`
//! `Bedienungsanleitung`.
//!
//! Four objects have a part: `1Ch` (shown with the door unlocked while
//! item `1Ch` is not carried), `40h`, `9Eh` (its record kept at `[2F68h]`)
//! and `F1h` (shown by the hook, its record kept at `[2F64h]`). The hook
//! keeps a state of its own in `[2F60h]` (0, 1, 3, 4, 5) with the frame
//! wanted `[2F5Ch]` and given `[2F5Eh]`, and the frame loop plays the
//! delayed samples of a script event left in `[2F62h]`, timed by the
//! hundredths `[1CF6h]`.
//!
//! The first visit, with the door still locked, opens with the script's
//! own interaction with door `79h` (`09B6h:16C4h(79h, −1, 6)`,
//! `0x163C9`). That routine is Dunkle Schatten's `0EB9h:177Ah`
//! (`0x1030A`) instruction for instruction but for its constants, one of
//! which is the verb it writes: `mov word [2D38h], 3` (`0x0B2F8`) where
//! Dunkle Schatten and Captain Zins write 2 ([`Api::interaction`] takes
//! the build's own); this build's 3 is `Sprich mit`.

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x1624;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x164A9`).
const OBJECT_STATES: u16 = 0x2018;

/// Door `79h`, to the Vorführraum.
const DOOR_79: u16 = 0x79;
/// Object `1Ch`, the id of item `1Ch` (`Bedienungsanleitung`).
const OBJECT_1C: u16 = 0x1C;
const OBJECT_40: u16 = 0x40;
const OBJECT_9E: u16 = 0x9E;
const OBJECT_F1: u16 = 0xF1;
/// `[2F68h]`/`[2F6Ah]`: object `9Eh`'s record (`0x16431`, `0x1642D`).
const REC_9E: u16 = 0x2F68;
/// `[2F64h]`/`[2F66h]`: object `F1h`'s record (`0x1669E`, `0x1669A`).
const REC_F1: u16 = 0x2F64;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BA8h]`: set on the first visit, read by locations 8, 12 and 22.
const BYTE_2BA8: u16 = 0x2BA8;
/// `[2BC9h]`: door `79h` unlocked.
const BYTE_2BC9: u16 = 0x2BC9;
/// `[2BCAh]`: rule `0007h`'s sequence counter.
const BYTE_2BCA: u16 = 0x2BCA;
/// `[2BD7h]`: the first visit's interaction done.
const BYTE_2BD7: u16 = 0x2BD7;
/// `[2BD8h]`: rule `0098h`'s sequence counter.
const BYTE_2BD8: u16 = 0x2BD8;

/// The hook's words, read and written in segment `1624h` alone.
/// `[2F5Ch]`: the frame wanted.
const FRAME_WANTED: u16 = 0x2F5C;
/// `[2F5Eh]`: the frame last given.
const FRAME_GIVEN: u16 = 0x2F5E;
/// `[2F60h]`: the hook's state.
const STATE: u16 = 0x2F60;
/// `[2F62h]`: the script event whose samples the frame loop owes, 0 none
/// (`0x163AF`, `0x1676A`).
const PENDING: u16 = 0x2F62;

/// `[1CF6h]`: the hundredths (`super::build`'s `hundredths`).
const HUNDREDTHS: u16 = 0x1CF6;
/// `cmp word [1CF6h], 3Ch ; jna` (`0x16467`).
const PENDING_DELAY: u16 = 0x3C;

/// The mode block's gate `[2D66h]`/`[2D68h]` and the entry door `[1F3Ah]`.
const GATE_OFF: u16 = 0x2D66;
const GATE_SEG: u16 = 0x2D68;
const ENTRY_DOOR: u16 = 0x1F3A;

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

/// `push k ; push id ; call far 06E3h:047Eh ; pop cx ; push dx ; push ax ;
/// call far 03F7h:1CF9h`: the object found and set on animation `k`.
fn anim(api: &mut dyn Api, id: u16, k: i16) {
    let r = api.obj_find(id);
    api.obj_anim(r, k);
}

/// `push k ; push id ; call far 0F48h:0784h ; pop cx ; push dx ; push ax ;
/// call far 03F7h:1CF9h`: the object shown and set on animation `k`.
fn show(api: &mut dyn Api, id: u16, k: i16) {
    let r = api.obj_show(id);
    api.obj_anim(r, k);
}

/// `push dir ; push ds ; push 176Eh ; call far 05DCh:0B56h`.
fn face(api: &mut dyn Api, dir: i16) {
    let player = api.ds(WALKER);
    api.figure_face(player, dir);
}

/// Everything before the frame loop, `0x16397` to `0x16450`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x16397
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x163A3
    api.visit_enter(loc, entry); // 0x163A7
    api.set_word(PENDING, 0); // 0x163AF
    if api.byte(BYTE_2BC9) == 0 && api.byte(BYTE_2BD7) == 0 {
        // 0x163B5, 0x163BC: the first visit, the door locked.
        api.interaction(DOOR_79, -1, 6); // 0x163C9, 09B6h:16C4h
        api.set_byte(BYTE_2BD7, 1); // 0x163D1
        api.set_byte(BYTE_2BA8, 1); // 0x163D6
        api.obj_hide(OBJECT_1C); // 0x16408, by 0x163DB
    } else if api.byte(BYTE_2BC9) == 1 && api.inv_has(OBJECT_1C) == 0 {
        // 0x163DD, 0x163E6
        show(api, OBJECT_1C, 0); // 0x163F4, 0x163FC
    } else {
        api.obj_hide(OBJECT_1C); // 0x16408
    }
    anim(api, OBJECT_40, 0); // 0x16412, 0x1641A
    let r9e = api.obj_find(OBJECT_9E); // 0x16427
    keep(api, REC_9E, r9e); // 0x1642D, 0x16431
    api.obj_anim(r9e, 0); // 0x16436
    // 0x1643E: an entry door 79h is forgotten.
    if api.word(ENTRY_DOOR) == DOOR_79 {
        api.set_word(ENTRY_DOOR, 0); // 0x16445
    }
    api.visit_finish_entry(); // 0x1644B
    api.set_word(WORDS.entry, 0); // 0x16450
}

/// One turn of the frame loop, `0x16456` to `0x164A1`: between the
/// dispatch and the frame pass, a pending script event's samples once the
/// hundredths are past `3Ch`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x16456
    api.dispatch(); // 0x1645B
    let pending = api.word(PENDING);
    if pending != 0 && api.word(HUNDREDTHS) > PENDING_DELAY {
        // 0x16460, 0x16467
        match pending {
            0x1F => {
                api.play_sample(0x0E); // 0x1647F
                api.play_sample(0x14); // 0x1648B
            }
            0x20 => {
                api.play_sample(0x0A); // 0x1648B
            }
            _ => {}
        }
        api.set_word(PENDING, 0); // 0x16491
    }
    api.frame_pass(); // 0x16497
    api.word(WORDS.entry) == 0 // 0x1649C
}

/// Everything after the frame loop, `0x164A3` to `0x164C0`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x164AE
    api.visit_leave(); // 0x164B6
    api.slot_unload(); // 0x164BB
}

/// The hook, `1624h:032Dh`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read. Codes above 9 and codes 1 to 4 do nothing
/// (`0x16578`, the table at `cs:055Fh`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0x16587: bit 1 of the argument before bit 2.
        0 => {
            if arg & 2 != 0 {
                api.set_word(STATE, 2); // 0x1658D
                api.set_word(FRAME_GIVEN, 0xFFFF); // 0x16593
                api.set_word(FRAME_WANTED, 0xFFFF); // 0x16599
            } else if arg & 4 != 0 {
                api.set_word(STATE, 1); // 0x165A8
                api.set_word(FRAME_GIVEN, 0xFFFF); // 0x165AE
            } else {
                api.set_word(STATE, 0); // 0x165B7
            }
        }
        // 0x165C0.
        5 | 6 => {
            if api.word(STATE) == 1 {
                let r = rec(api, REC_9E);
                api.obj_anim(r, 0); // 0x165D5
            }
        }
        7 => hook_events(api, arg),
        // 0x165E0: in state 1 frame 1 for argument 2, in state 3 for
        // argument 1; else frame 0.
        8 => {
            let on = match api.word(STATE) {
                1 => Some(2), // 0x165F0
                3 => Some(1), // 0x16607
                _ => None,
            };
            if let Some(on) = on {
                let k = u16::from(arg == on);
                api.set_word(FRAME_WANTED, k); // 0x165F5, 0x165FE, 0x1660C, 0x16615
            }
        }
        // 0x1661E: a new frame wanted goes to object 9Eh in state 1, to
        // object F1h in state 3.
        9 => {
            let at = match api.word(STATE) {
                1 => REC_9E, // 0x1662E
                3 => REC_F1, // 0x16651
                _ => return,
            };
            let k = api.word(FRAME_WANTED);
            if k == api.word(FRAME_GIVEN) {
                return; // 0x16635, 0x16658
            }
            api.set_word(FRAME_GIVEN, k); // 0x1663A, 0x1665D
            let r = rec(api, at);
            api.obj_anim(r, num::signed(k)); // 0x16646, 0x16669
        }
        _ => {}
    }
}

/// Code 7, `0x16674`: the argument looked up in the row at `cs:0543h`
/// (`0Ah` to `0Dh`, `1Eh` to `20h`), its arm in the row after it
/// (`cs:0551h`).
fn hook_events(api: &mut dyn Api, arg: u16) {
    match arg {
        // 0x16691: object F1h shown and kept, the figure walks to node
        // 0FA0h, the door opens, state 3.
        0x0A => {
            let r = api.obj_show(OBJECT_F1); // 0x16694
            keep(api, REC_F1, r); // 0x1669A, 0x1669E
            let player = api.ds(WALKER);
            api.walker_walk_to(player, 0x0FA0, 1); // 0x166AA, answer not read
            api.set_word(FRAME_WANTED, 3); // 0x166B2
            anim(api, DOOR_79, 1); // 0x166BC, 0x166C4
            api.set_word(STATE, 3); // 0x166CC
        }
        // 0x166D5.
        0x0B => api.set_word(FRAME_WANTED, 2),
        // 0x166DE: state 4, F1h on animation 4, 9Eh on animation 2.
        0x0C => {
            api.set_word(STATE, 4); // 0x166DE
            let r = rec(api, REC_F1);
            api.obj_anim(r, 4); // 0x166EE
            let r = rec(api, REC_9E);
            api.obj_anim(r, 2); // 0x16700
        }
        // 0x1670A: state 5, sample 2Dh, F1h on animation 5.
        0x0D => {
            api.set_word(STATE, 5); // 0x1670A
            api.play_sample(0x2D); // 0x16712
            let r = rec(api, REC_F1);
            api.obj_anim(r, 5); // 0x16722
        }
        // 0x16770: sample 2Ah twice.
        0x1E => {
            api.play_sample(0x2A); // 0x16772
            api.play_sample(0x2A); // 0x1677A
        }
        // 0x1672C: samples 0Eh, 0Eh, 14h, 14h; the hundredths from 0 and
        // the event owed.
        0x1F => {
            api.play_sample(0x0E); // 0x1672E
            api.play_sample(0x0E); // 0x16736
            api.play_sample(0x14); // 0x1673E
            api.play_sample(0x14); // 0x16746
            api.set_word(HUNDREDTHS, 0); // 0x1674C
            api.set_word(PENDING, arg); // 0x1676A
        }
        // 0x16754: samples 0Eh, 0Eh; the hundredths from 1Eh and the event
        // owed.
        0x20 => {
            api.play_sample(0x0E); // 0x16756
            api.play_sample(0x0E); // 0x1675E
            api.set_word(HUNDREDTHS, 0x1E); // 0x16764
            api.set_word(PENDING, arg); // 0x1676A
        }
        _ => {}
    }
}

/// The animation hook, `1624h:0281h`: the record in `[bp+6]`, the code in
/// `[bp+0Ah]`; codes −1 to −3 have arms (`0x164C4`–`0x164D6`).
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        // 0x164D9: samples 2Ch, 2Ch, 0Ah; object 1Ch shown on animation 1.
        0xFFFF => {
            api.play_sample(0x2C); // 0x164DB
            api.play_sample(0x2C); // 0x164E3
            api.play_sample(0x0A); // 0x164EB
            show(api, OBJECT_1C, 1); // 0x164F5, 0x164FD
        }
        // 0x16507: samples 0Eh, 0Eh, 0Ah; the figure faces 2.
        0xFFFE => {
            api.play_sample(0x0E); // 0x16509
            api.play_sample(0x0E); // 0x16511
            api.play_sample(0x0A); // 0x16519
            face(api, 2); // 0x16525
        }
        // 0x1652C: 9Eh on animation 0, 40h on animation 1, samples 2Ch,
        // 2Ch, object F1h hidden.
        0xFFFD => {
            let r = rec(api, REC_9E);
            api.obj_anim(r, 0); // 0x16536
            anim(api, OBJECT_40, 1); // 0x16542, 0x1654A
            api.play_sample(0x2C); // 0x16554
            api.play_sample(0x2C); // 0x1655C
            api.obj_hide(OBJECT_F1); // 0x16565
        }
        _ => {}
    }
}

/// The gate cleared (`mov word [2D68h], 0 ; mov word [2D66h], 0`).
fn clear_gate(api: &mut dyn Api) {
    api.set_word(GATE_SEG, 0);
    api.set_word(GATE_OFF, 0);
}

/// `push hi ; push lo ; call far 203Dh:0024h ; push ax ; call far
/// 080Ch:164Ch`: the roll is the scene.
fn roll(api: &mut dyn Api, lo: u16, hi: u16) {
    let n = api.random(lo, hi);
    api.scene_play(num::signed(n));
}

/// Whether door `79h` is still locked (`cmp byte [2BC9h], 0`).
fn locked(api: &dyn Api) -> bool {
    api.byte(BYTE_2BC9) == 0
}

/// The VIP-Lounge's rule handlers by their offset in segment `1624h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 1624:0007 (0x16247), Öffne on door 79h: while it is locked the
        // figure faces 2, one of scenes 0 to 3 in turn, the gate cleared;
        // unlocked the search goes on (0x16283).
        0x0007 => {
            if locked(api) {
                face(api, 2); // 0x16257
                let counter = api.ds(BYTE_2BCA);
                api.scene_sequence(counter, &[0, 1, 2, 3]); // 0x1626D
                clear_gate(api); // 0x16275, 0x1627B
                0x0A
            } else {
                0
            }
        }
        // 1624:004C (0x1628C), Sprich mit door 79h: while it is locked one of
        // scenes 7 to 0Ah; unlocked the search goes on (0x162AE).
        0x004C => {
            if locked(api) {
                roll(api, 7, 0x0A); // 0x1629A, 0x162A3
                0x0A
            } else {
                0
            }
        }
        // 1624:0072 (0x162B2), the Schlüssel on door 79h: scene 4 and the
        // door unlocked, or scene 5.
        0x0072 => {
            if locked(api) {
                api.scene_play(4); // 0x162BE
                api.set_byte(BYTE_2BC9, 1); // 0x162C4
            } else {
                api.scene_play(5); // 0x162CD
            }
            0x0A
        }
        // 1624:0098 (0x162D8), class 3 on door 79h: one of scenes 0Bh to 0Eh
        // in turn, or scene 5.
        0x0098 => {
            if locked(api) {
                let counter = api.ds(BYTE_2BD8);
                api.scene_sequence(counter, &[0x0B, 0x0C, 0x0D, 0x0E]); // 0x162F0
            } else {
                api.scene_play(5); // 0x162FC
            }
            0x0A
        }
        // 1624:00C7 (0x16307), class 4 on door 79h: one of scenes 0Fh to
        // 11h, or the figure faces 0 and scene 5.
        0x00C7 => {
            if locked(api) {
                roll(api, 0x0F, 0x11); // 0x16315, 0x16330
            } else {
                face(api, 0); // 0x16326
                api.scene_play(5); // 0x16330
            }
            0x0A
        }
        // 1624:00FB (0x1633B), the Haarnadel on door 79h: one of scenes 12h
        // to 14h, or scene 5.
        0x00FB => {
            if locked(api) {
                roll(api, 0x12, 0x14); // 0x16349, 0x16356
            } else {
                api.scene_play(5); // 0x16356
            }
            0x0A
        }
        // 1624:0121 (0x16361), Sprich mit object 9Eh: one of scenes 15h to
        // 18h.
        0x0121 => {
            roll(api, 0x15, 0x18); // 0x16368, 0x16371
            0x0A
        }
        // 1624:013C (0x1637C), the Haarnadel, class 3 or class 4 on
        // nothing: one of scenes !0x24 to !0x22.
        0x013C => {
            roll(api, !0x24, !0x22); // 0x16383, 0x1638C
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
    anim_hook: Some(anim_hook),
    rule,
};
