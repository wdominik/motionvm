//! Location 15, the Technik, a row of kind 1 with one exit, `6Eh`, to the
//! Flur 1. Stock (location 19). The module is named after the location as
//! the game itself names it in its location table (`Technik`, the record's
//! `+37h`, `250Ah:011Dh`).
//!
//! Re-expressed from the handler at `145Bh:00F1h` (image `0x146A1` to its
//! `retf` at `0x14741`), its hook `145Bh:019Eh` (`0x1474E` to `0x14846`,
//! the record's `+3Fh`) and its four rule handlers `145Bh:000Dh` to
//! `00D1h` (`0x145BD`–`0x146A0`), against [`Api`] alone. The record's
//! `+43h` is null: the library's empty animation hook. The location's rules
//! are the six records at `2446h:0540h` (image `0x249A0`, the record's
//! `+3Bh`), fourteen bytes each:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 3 | 4 | `91h` | — | `006Eh` |
//! | 9 | `20h` | `20h` | `91h` | `000Dh` |
//! | 9 | `20h` | `16h` | `91h` | `006Eh` |
//! | 4 | 4 | `20h` | — | `00C0h` |
//! | 4 | 4 | `16h` | — | `00C0h` |
//! | 1 | 4 | `40h` | — | `00D1h` |
//!
//! The verbs are, by the verb text row at `DS:116Ah`: 1 `Betrachte`, 3
//! `Sprich mit`, 4 `Benutze`, 9 `Gib … an`. The items the logic names are,
//! by the item row at `DS:10AAh`: 3 `VIP-Karte`, 5 `Schlüssel`, `16h`
//! `Brille`, `20h` `Nachricht`. `Benutze` on the `Brille` or the
//! `Nachricht` becomes `Gib … an` (rule `00C0h`).
//!
//! Object `91h` is the one the player talks to; its record is kept at
//! `[2EE8h]`, the animation it rests on at `[2EECh]`. Two bytes of the
//! logic's saved state block (`DS:2B98h`, `4Eh` bytes) hold the location's
//! progress, read and written in segment `145Bh` alone: `[2BB6h]`, the
//! state (0 to 5), and `[2BBAh]`, set once scene 4 has played. States 0 and
//! 1 rest `91h` on animation 3, states 2 and 3 on animation 0; the hook's
//! event `13h` (the `Brille` taken) moves state 0 or 1 to 2, and talking
//! moves 0 to 1 and 2 to 3. The script event `0Ah` leaves for the Kaffee
//! (location 1) by entry 7 in state 4; the Kaffee's entry 7 leads back here
//! (row `DS:182Ch`, entry 7: location `0Fh`), where `91h` is hidden, and
//! `Betrachte` on object `40h` plays scene `0Ah` once, for state 5.

use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x145B;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x1472A`).
const OBJECT_STATES: u16 = 0x2018;

/// Object `91h`, the one the player talks to.
const OBJECT_91: u16 = 0x91;

/// The items the logic moves (the item row at `DS:10AAh`).
const VIP_KARTE: u16 = 0x03;
const SCHLUESSEL: u16 = 0x05;
const BRILLE: u16 = 0x16;
const NACHRICHT: u16 = 0x20;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BB6h]`: the location's state, 0 to 5.
const STATE: u16 = 0x2BB6;
/// `[2BBAh]`: scene 4 played.
const TOLD: u16 = 0x2BBA;

/// The handler's and the hook's words, read and written in segment `145Bh`
/// alone.
/// `[2EE2h]`: the animation last given to `91h`, `FFFFh` none.
const GIVEN: u16 = 0x2EE2;
/// `[2EE4h]`: the animation wanted for `91h`, `FFFFh` none.
const WANTED: u16 = 0x2EE4;
/// `[2EE6h]`: bit 1 of code 0's argument, `91h` speaking.
const SPEAKING: u16 = 0x2EE6;
/// `[2EE8h]`/`[2EEAh]`: object `91h`'s record (`0x146E3`, `0x146DF`).
const REC_91: u16 = 0x2EE8;
/// `[2EECh]`: the animation `91h` rests on.
const REST: u16 = 0x2EEC;

/// The mode block's verb and verb as chosen.
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

/// `push k ; push word [2EEAh] ; push word [2EE8h] ; call far
/// 03F7h:1CF9h`: object `91h`'s kept record on animation `k`.
fn anim_91(api: &mut dyn Api, k: u16) {
    let r = rec(api, REC_91);
    api.obj_anim(r, num::signed(k));
}

/// Everything before the frame loop, `0x146A1` to `0x14708`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x146A1
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x146AD
    api.visit_enter(loc, entry); // 0x146B1
    match api.byte(STATE) {
        // 0x146B9: by the state through the table at cs:0192h (0x14742);
        // states above 5 skip to 0x14703.
        state @ 0..=3 => {
            // 0x146CC for states 0 and 1, 0x146F2 for 2 and 3.
            api.set_word(REST, if state < 2 { 3 } else { 0 }); // 0x146CC, 0x146F2
            let rest = api.word(REST); // 0x146D2
            let r = api.obj_show(OBJECT_91); // 0x146D9
            keep(api, REC_91, r); // 0x146DF, 0x146E3
            api.obj_anim(r, num::signed(rest)); // 0x146E8
        }
        4 | 5 => {
            api.obj_hide(OBJECT_91); // 0x146FD
        }
        _ => {}
    }
    api.visit_finish_entry(); // 0x14703
    api.set_word(WORDS.entry, 0); // 0x14708
}

/// One turn of the frame loop, `0x1470E` to `0x14722`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x1470E
    api.dispatch(); // 0x14713
    api.frame_pass(); // 0x14718
    api.word(WORDS.entry) == 0 // 0x1471D
}

/// Everything after the frame loop, `0x14724` to `0x14741`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x1472F
    api.visit_leave(); // 0x14737
    api.slot_unload(); // 0x1473C
}

/// The hook, `145Bh:019Eh`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read. Codes above 9 and codes 1 to 3 do nothing
/// (`0x14757`, the table at `cs:0297h`, `0x14847`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0x14766: a dialogue comes up; bit 1 is object 91h, which goes to
        // its resting animation.
        0 => {
            api.set_word(GIVEN, 0xFFFF); // 0x14766
            api.set_word(SPEAKING, arg & 2); // 0x14771
            if api.word(SPEAKING) != 0 {
                // 0x14774
                let rest = api.word(REST);
                anim_91(api, rest); // 0x1478A
            }
        }
        // 0x147EA: the script's item events 10h, 11h and 13h.
        4 => match arg {
            0x10 => {
                api.inv_give(VIP_KARTE); // 0x1481D, 0x14823
            }
            0x11 => {
                api.inv_give(SCHLUESSEL); // 0x14821, 0x14823
            }
            0x13 => {
                api.inv_take(BRILLE); // 0x147FF
                api.set_byte(STATE, 2); // 0x14805
                api.set_word(REST, 0); // 0x1480A
                api.set_word(WANTED, 2); // 0x14813
                api.set_word(GIVEN, 2); // 0x14816
                anim_91(api, 2); // 0x14819, 0x1478A
            }
            _ => {}
        },
        // 0x14795: the dialogue over; a speaking 91h back on its resting
        // animation.
        5 | 6 => {
            if api.word(SPEAKING) != 0 {
                // 0x14795
                let rest = api.word(REST);
                anim_91(api, rest); // 0x147A8
            }
            api.set_word(SPEAKING, 0); // 0x147B0
            api.set_word(WANTED, 0xFFFF); // 0x147B9
            api.set_word(GIVEN, 0xFFFF); // 0x147BC
        }
        // 0x1482B: script event 0Ah leaves for the Kaffee by entry 7.
        7 => {
            if arg == 0x0A {
                api.set_word(WORDS.location, 1); // 0x14834
                api.set_word(WORDS.entry, 7); // 0x1483A
                api.set_byte(STATE, 4); // 0x14840
            }
        }
        // 0x147C2: a speaking 91h wants the animation after its resting
        // one by the argument.
        8 => {
            if api.word(SPEAKING) != 0 {
                let k = arg.wrapping_add(api.word(REST)).wrapping_add(1);
                api.set_word(WANTED, k); // 0x147C9–0x147D0
            }
        }
        // 0x147D5: a new animation wanted goes to a speaking 91h.
        9 => {
            if api.word(SPEAKING) == 0 {
                return; // 0x147DA
            }
            let k = api.word(WANTED);
            if api.word(GIVEN) == k {
                return; // 0x147E3
            }
            api.set_word(GIVEN, k); // 0x14816
            anim_91(api, k); // 0x14819, 0x1478A
        }
        _ => {}
    }
}

/// `145Bh:000Dh` (`0x145BD`), `Gib` the `Nachricht` `an` object 91h: with
/// the `Brille` carried scene 8 (9 once scene 4 has played), sample 0Ah and
/// the `Nachricht` taken; without it scene 4, sample 0Ah and scene 4
/// marked, or scene 5 after it.
fn rule_000d(api: &mut dyn Api) -> i16 {
    if api.inv_has(BRILLE) != 0 {
        // 0x145C2, 0x145CC
        let scene = if api.byte(TOLD) == 0 { 8 } else { 9 };
        api.scene_play(scene); // 0x145DC
        api.play_sample(0x0A); // 0x145E4
        api.inv_take(NACHRICHT); // 0x145EC
    } else if api.byte(TOLD) == 0 {
        // 0x145F3
        api.scene_play(4); // 0x145FC
        api.play_sample(0x0A); // 0x14604
        api.set_byte(TOLD, 1); // 0x1460A
    } else {
        api.scene_play(5); // 0x14613
    }
    0x0A // 0x14619
}

/// The Technik's rule handlers by their offset in segment `145Bh`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        0x000D => rule_000d(api),
        // 145B:006E (0x1461E), Sprich mit object 91h, and Gib the Brille an
        // it: with the Nachricht carried, rule 000Dh, its answer dropped
        // (`push word [bp+6] ; push cs ; call 000Dh`, 0x1462D–0x14631);
        // else by the state through the table at cs:00B8h (0x14668):
        // state 0 becomes 1 and state 2 becomes 3 with scene 1, states 1
        // and 3 play scene 2, states above 3 nothing.
        0x006E => {
            if api.inv_has(NACHRICHT) != 0 {
                // 0x14623
                rule_000d(api); // 0x14631
            } else {
                match api.byte(STATE) {
                    // 0x14636
                    0 => {
                        api.set_byte(STATE, 1); // 0x14649
                        api.scene_play(1); // 0x1465D
                    }
                    2 => {
                        api.set_byte(STATE, 3); // 0x14654
                        api.scene_play(1); // 0x1465D
                    }
                    1 | 3 => {
                        api.scene_play(2); // 0x14652, 0x1465D
                    }
                    _ => {} // 0x14640
                }
            }
            0x0A // 0x14663
        }
        // 145B:00C0 (0x14670), Benutze the Nachricht or the Brille: the
        // verb becomes Gib … an.
        0x00C0 => {
            api.set_word(VERB_CHOSEN, 9); // 0x14676
            api.set_word(VERB, 9); // 0x14679
            6 // 0x1467C
        }
        // 145B:00D1 (0x14681), Betrachte object 40h: in state 4 scene 0Ah
        // and state 5; else the search goes on.
        0x00D1 => {
            if api.byte(STATE) == 4 {
                // 0x14684
                api.scene_play(0x0A); // 0x1468D
                api.set_byte(STATE, 5); // 0x14693
                0x0A // 0x14698
            } else {
                0 // 0x1469D
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
    anim_hook: None,
    rule,
};
