//! Location 20, the Tontechnik, a row of kind 1 with one exit, `6Eh`, to
//! the Flur 2. Stock (location 31). The module is named after the location
//! as the game itself names it in its location table (`Tontechnik`, the
//! record's `+37h`, `250Ah:0158h`).
//!
//! Re-expressed from the handler at `1523h:00C0h` (image `0x152F0` to its
//! `retf` at `0x15401`), its hook `1523h:01DAh` (`0x1540A` to its `retf`
//! at `0x154E0`, the record's `+3Fh`), its gate `1523h:0009h` (`0x15239`
//! to `0x15282`, installed by rule `0053h`) and its two rule handlers
//! `1523h:0053h` and `00AFh` (`0x15283`–`0x152EF`), against [`Api`] alone.
//! The record's `+43h` is null: the library's empty animation hook. The
//! location's rules are the three records at `2446h:06F0h` (image
//! `0x24B50`, the record's `+3Bh`), fourteen bytes each:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 3 | 4 | `96h` | — | `0053h` |
//! | 9 | `20h` | `11h` | `96h` | `0053h` |
//! | 4 | 4 | `11h` | — | `00AFh` |
//!
//! The verbs are, by the verb text row at `DS:116Ah`: 3 `Sprich mit`, 4
//! `Benutze`, 9 `Gib … an`. The items the logic names are, by the item row
//! at `DS:10AAh`: `11h` `Tonspule`, `20h` `Nachricht`. `Benutze` on the
//! `Tonspule` becomes `Gib … an` (rule `00AFh`); giving it to object `96h`
//! is talking to it (rule `0053h`).
//!
//! Object `96h` is the one the player talks to; its record is kept at
//! `[2F16h]`, and zeroed while it is hidden. Bytes of the logic's saved
//! state block (`DS:2B98h`, `4Eh` bytes) the location reads or writes:
//! `[2BC0h]`, its state (0 to 4), which the Enterhaken - Brücke (location
//! 10) reads (`cmp byte [2BC0h], 4`, `0x13470`); `[2BC1h]`, the counter of
//! the gate's scene sequence; and `[2BB4h]`, which the Enterhaken -
//! Maschinenraum (location 11) sets (`0x1385F`) and which brings object
//! `96h` in: in state 0 with `[2BB4h]` at 1 the entry shows it and moves on
//! to state 1. States 1 to 3 show it (3 falling back to 2), states above 3
//! hide it. Talking in state 1 or 2 plays scene 0 or 1 and installs the
//! gate; when the mode machine asks the gate with 0, state 3 (the hook's
//! event `16h`) becomes 2 with the next of scenes 3 to 6, any other state
//! becomes 4 with scene 2. Talking in state 3 plays scene 7, in state 4
//! scene 8.

use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers and the gate.
pub(super) const SEGMENT: u16 = 0x1523;
/// The gate's offset (`mov word [2D66h], 9`, `0x152A8`, `0x152BE`).
const GATE: u16 = 0x0009;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x153EA`).
const OBJECT_STATES: u16 = 0x2018;

/// Object `96h`, the one the player talks to.
const OBJECT_96: u16 = 0x96;

/// The items the logic moves (the item row at `DS:10AAh`).
const TONSPULE: u16 = 0x11;
const NACHRICHT: u16 = 0x20;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BB4h]`: set by the Enterhaken - Maschinenraum (`0x1385F`).
const BYTE_2BB4: u16 = 0x2BB4;
/// `[2BC0h]`: the location's state, 0 to 4.
const STATE: u16 = 0x2BC0;
/// `DS:2BC1h`: the counter of the gate's scene sequence (`0x15268`).
const SEQUENCE: u16 = 0x2BC1;

/// The handler's and the hook's words, read and written in segment `1523h`
/// alone and outside the `38h` bytes from `DS:2D38h` the visit clears.
/// `[2F10h]`: the animation last given to `96h`, `FFFFh` none.
const GIVEN: u16 = 0x2F10;
/// `[2F12h]`: the animation wanted for `96h`.
const WANTED: u16 = 0x2F12;
/// `[2F14h]`: bit 1 of code 0's argument, `96h` speaking.
const SPEAKING: u16 = 0x2F14;
/// `[2F16h]`/`[2F18h]`: object `96h`'s record (`0x1532F`, `0x1532B`).
const REC_96: u16 = 0x2F16;

/// The mode block's verb, verb as chosen and gate.
const VERB: u16 = 0x2D38;
const VERB_CHOSEN: u16 = 0x2D48;
const GATE_OFF: u16 = 0x2D66;
const GATE_SEG: u16 = 0x2D68;

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

/// `push k ; push word [2F18h] ; push word [2F16h] ; call far
/// 03F7h:1CF9h` (`0x1547A`–`0x15482`): object `96h`'s kept record on
/// animation `k`. A zeroed record is let alone (`0x05C76`).
fn anim_96(api: &mut dyn Api, k: u16) {
    let r = rec(api, REC_96);
    api.obj_anim(r, num::signed(k));
}

/// `push 96h ; call far 0F48h:0784h`, kept, then animation 1
/// (`0x15322`–`0x15336`, `0x1534C`–`0x15360`).
fn show_96(api: &mut dyn Api) {
    let r = api.obj_show(OBJECT_96); // 0x15325, 0x1534F
    keep(api, REC_96, r); // 0x1532B, 0x1532F
    api.obj_anim(r, 1); // 0x15336, 0x15360
}

/// Everything before the frame loop, `0x152F0` to `0x153C8`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x152F0
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x152FC
    api.visit_enter(loc, entry); // 0x15300
    // 0x15308: by the state through the table at cs:01D2h (0x15402);
    // states above 3, and state 0 without [2BB4h], hide 96h (0x1536A).
    let shown = match api.byte(STATE) {
        0 if api.byte(BYTE_2BB4) == 1 => {
            // 0x1531B
            show_96(api);
            api.set_byte(STATE, 1); // 0x1533E
            true
        }
        1 | 2 => {
            show_96(api); // 0x1534C
            true
        }
        3 => {
            api.set_byte(STATE, 2); // 0x15347
            show_96(api);
            true
        }
        _ => false,
    };
    if !shown {
        api.obj_hide(OBJECT_96); // 0x1536D
        keep(api, REC_96, Far::NULL); // 0x15373, 0x15379
    }
    // 0x1537F–0x153BB: subject slots 2 to 5 points at x C8h, 96h, C8h, FAh.
    api.slot_set_raw(2, 0, 0xC8, 0); // 0x15388
    api.slot_set_raw(3, 0, 0x96, 0); // 0x15399
    api.slot_set_raw(4, 0, 0xC8, 0); // 0x153AA
    api.slot_set_raw(5, 0, 0xFA, 0); // 0x153BB
    api.visit_finish_entry(); // 0x153C3
    api.set_word(WORDS.entry, 0); // 0x153C8
}

/// One turn of the frame loop, `0x153CE` to `0x153E2`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x153CE
    api.dispatch(); // 0x153D3
    api.frame_pass(); // 0x153D8
    api.word(WORDS.entry) == 0 // 0x153DD
}

/// Everything after the frame loop, `0x153E4` to `0x15401`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x153EF
    api.visit_leave(); // 0x153F7
    api.slot_unload(); // 0x153FC
}

/// The hook, `1523h:01DAh`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read. Codes above 9 and codes 1 to 3 do nothing
/// (`0x15413`, the table at `cs:02B1h`, `0x154E1`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0x15422: a dialogue comes up; bit 1 is object 96h, which then
        // wants animation 1 with none given; else 96h goes to animation 0.
        0 => {
            api.set_word(SPEAKING, arg & 2); // 0x15427
            if api.word(SPEAKING) != 0 {
                // 0x1542A
                api.set_word(WANTED, 1); // 0x15431
                api.set_word(GIVEN, 0xFFFF); // 0x15437
            } else {
                anim_96(api, 0); // 0x15440, 0x15482
            }
        }
        // 0x1548C: the script's item events 15h to 17h.
        4 => match arg {
            0x15 => {
                api.inv_take(TONSPULE); // 0x154A1
            }
            0x16 => {
                api.set_byte(STATE, 3); // 0x154A8
            }
            0x17 => {
                api.inv_give(NACHRICHT); // 0x154B1
            }
            _ => {}
        },
        // 0x15444: the dialogue over; a speaking 96h back on animation 0.
        // The speaking word is left as it is.
        5 | 6 => {
            if api.word(SPEAKING) != 0 {
                // 0x15444
                anim_96(api, 0); // 0x15440, 0x15482
            }
        }
        // 0x154B8: the script's events 0Ah, 1Eh and 1Fh.
        7 => match arg {
            0x0A => api.set_word(WANTED, 0), // 0x154CB
            0x1E => api.play_sample(0x1A),   // 0x154D3, 0x154D9
            0x1F => api.play_sample(0x1B),   // 0x154D7, 0x154D9
            _ => {}
        },
        // 0x15450: animation 2 wanted for an argument of 1, else 1.
        8 => {
            let k = if arg == 1 { 2 } else { 1 };
            api.set_word(WANTED, k); // 0x15455, 0x1545E
        }
        // 0x15466: a new animation wanted goes to a speaking 96h.
        9 => {
            if api.word(SPEAKING) == 0 {
                return; // 0x1546B
            }
            let k = api.word(WANTED);
            if api.word(GIVEN) == k {
                return; // 0x15474
            }
            api.set_word(GIVEN, k); // 0x15476
            anim_96(api, k); // 0x15482
        }
        _ => {}
    }
}

/// The gate, `1523h:0009h` (`0x15239`): asked with 0 it plays what follows
/// the talk — in state 3 state 2 and the next of scenes 3 to 6 by the
/// counter at `DS:2BC1h`, else state 4 and scene 2 — and clears itself;
/// asked with anything else it does nothing. It answers 0 either way
/// (`0x1527F`).
pub(crate) fn gate(api: &mut dyn Api, off: u16, arg: i16) -> Option<i16> {
    if off != GATE {
        return None;
    }
    if arg != 0 {
        return Some(0); // 0x15240
    }
    if api.byte(STATE) == 3 {
        // 0x15242
        api.set_byte(STATE, 2); // 0x15258
        let counter = api.ds(SEQUENCE);
        api.scene_sequence(counter, &[3, 4, 5, 6]); // 0x1526B
    } else {
        api.set_byte(STATE, 4); // 0x15249
        api.scene_play(2); // 0x15250
    }
    api.set_word(GATE_SEG, 0); // 0x15273
    api.set_word(GATE_OFF, 0); // 0x15279
    Some(0) // 0x1527F
}

/// `mov word [2D68h], 1523h ; mov word [2D66h], 9`: the gate installed
/// (`0x152A2`, `0x152B8`).
fn install_gate(api: &mut dyn Api) {
    api.set_word(GATE_SEG, SEGMENT);
    api.set_word(GATE_OFF, GATE);
}

/// The Tontechnik's rule handlers by their offset in segment `1523h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 1523:0053 (0x15283), Sprich mit object 96h, and Gib the Tonspule
        // an it: by the state less 1 through the table at cs:00A7h
        // (0x152D7); state 0 and states above 4 nothing.
        0x0053 => {
            match api.byte(STATE) {
                1 => {
                    api.scene_play(0); // 0x1529C
                    install_gate(api); // 0x152A2
                }
                2 => {
                    api.scene_play(1); // 0x152B2
                    install_gate(api); // 0x152B8
                }
                3 => {
                    api.scene_play(7); // 0x152C6, 0x152CC
                }
                4 => {
                    api.scene_play(8); // 0x152CA, 0x152CC
                }
                _ => {} // 0x15291
            }
            0x0A // 0x152D2
        }
        // 1523:00AF (0x152DF), Benutze the Tonspule: the verb becomes
        // Gib … an.
        0x00AF => {
            api.set_word(VERB_CHOSEN, 9); // 0x152E5
            api.set_word(VERB, 9); // 0x152E8
            6 // 0x152EB
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
