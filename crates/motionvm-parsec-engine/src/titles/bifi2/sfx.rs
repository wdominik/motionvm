//! Location 21, the SFX, a row of kind 1 with one exit, `70h`, to the
//! Flur 2. Stock (location 31). The module is named after the location as
//! the game itself names it in its location table (`SFX`, the record's
//! `+37h`, `250Ah:0163h`).
//!
//! Re-expressed from the handler at `154Fh:00BDh` (image `0x155AD` to its
//! `retf` at `0x15635`), its hook `154Fh:0146h` (`0x15636` to `0x156A5`,
//! the record's `+3Fh`) and its two rule handlers `154Fh:0005h` and
//! `0059h` (`0x154F5`–`0x155AC`), against [`Api`] alone. The record's
//! `+43h` is null: the library's empty animation hook. The location's rules
//! are the two records at `2446h:0730h` (image `0x24B90`, the record's
//! `+3Bh`), fourteen bytes each:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 3 | 4 | `97h` | — | `0059h` |
//! | 0 | `80h` | 0 | — | `0005h` |
//!
//! Verb 3 is `Sprich mit` by the verb text row at `DS:116Ah`. Phase `80h`
//! with verb 0 and object 0 is matched by the search a walk stopped short
//! of its target runs (`push 1080h ; call far 0E40h:009Ch`, `0x0A4CB`),
//! for any verb and any object; its answer `0Ah` is non-zero, so the mode
//! block is reset after it (`0x0A4E8`, `0x09BA2`). Until `[2BC3h]` is set the
//! player's graph has node `3E8h` blocked (`0x155D3`), and a walk that
//! needs it ends in rule `0005h`, an interaction with object `97h`.
//!
//! Object `97h` is the one the player talks to; its record is kept at
//! `[2F1Ch]`, and the hook's code 0 bit 1 says it speaks. Item 8 is the
//! third of the three item rows (6, 7 and 8) that the item row at
//! `DS:10AAh` names `Sicherheitsausweis` (`250Ah:11DBh` for all three).
//! Held, being stopped short plays the interaction's scene 4, and talking
//! to `97h` scene 5 while `[2BC3h]` is clear; either sets `[2BC3h]` and
//! opens node `3E8h` again. Talking afterwards plays one of scenes 6 to 9. Without it, talking
//! plays scenes `0Ah` to `0Dh` and being stopped short scenes 0 to 3, both
//! in turn by the one counter `[2BC2h]`. The hook's code 4 with argument
//! `18h` leaves through exit `70h`.
//!
//! Two bytes of the logic's saved state block (`DS:2B98h`, `4Eh` bytes)
//! hold the location's progress, read and written in segment `154Fh`
//! alone: `[2BC2h]`, the sequences' counter, and `[2BC3h]`, set with item
//! 8 carried after scene 5 or the interaction's scene 4, node `3E8h` open.
//!
//! A node blocked and opened are `05DCh:0EFDh` and `05DCh:0F57h`
//! (`0x06CBD`, `push 3FFh` at `0x06D0B`; `0x06D17`, `push 0` at `0x06D65`;
//! Dunkle Schatten's `0A9Fh:1002h` and `0A9Fh:105Ch`, [`Api::node_block`],
//! [`Api::node_open`]). The script exit is `0FE3h:088Ah` (`0x106BA`,
//! Dunkle Schatten's `1586h:05E1h` less its hover call, [`Api::script_exit`]).

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x154F;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x1561E`).
const OBJECT_STATES: u16 = 0x2018;

/// Object `97h`, the one the player talks to.
const OBJECT_97: u16 = 0x97;

/// Item 8, the third `Sicherheitsausweis` (the item row at `DS:10AAh`).
const SICHERHEITSAUSWEIS_8: u16 = 0x08;

/// The player's graph node blocked until `[2BC3h]` is set.
const NODE_3E8: u16 = 0x03E8;

/// Exit `70h`, the row's one exit, to the Flur 2. Stock.
const EXIT_70: u16 = 0x70;

/// The facing the rule `0005h`'s interactions send the player to.
const FACING: i16 = 3;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BC2h]`: the counter both scene sequences step.
const SEQUENCE: u16 = 0x2BC2;
/// `[2BC3h]`: set with item 8 carried after scene 5 or the interaction's
/// scene 4; node `3E8h` open.
const NODE_OPEN: u16 = 0x2BC3;

/// The hook's words, read and written in segment `154Fh` alone.
/// `[2F1Ah]`: bit 1 of code 0's argument, `97h` speaking.
const SPEAKING: u16 = 0x2F1A;
/// `[2F1Ch]`/`[2F1Eh]`: object `97h`'s record (`0x155EA`, `0x155E6`).
const REC_97: u16 = 0x2F1C;

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

/// `push k ; push word [2F1Eh] ; push word [2F1Ch] ; call far
/// 03F7h:1CF9h`: object `97h`'s kept record on animation `k`.
fn anim_97(api: &mut dyn Api, k: i16) {
    let r = rec(api, REC_97);
    api.obj_anim(r, k);
}

/// `push 3E8h ; push ds ; push 176Eh ; call far 05DCh:0F57h`: node `3E8h`
/// of the player's graph open again.
fn open_node(api: &mut dyn Api) {
    let player = api.ds(WALKER);
    api.node_open(player, NODE_3E8);
}

/// Everything before the frame loop, `0x155AD` to `0x155FC`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x155AD
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x155B9
    api.visit_enter(loc, entry); // 0x155BD
    if api.byte(NODE_OPEN) == 0 {
        // 0x155C5
        let player = api.ds(WALKER);
        api.node_block(player, NODE_3E8); // 0x155D3
    }
    // 0x155DB: the `push 0` is obj_anim's argument, pushed before the
    // search.
    let r = api.obj_find(OBJECT_97); // 0x155E0
    keep(api, REC_97, r); // 0x155E6, 0x155EA
    api.obj_anim(r, 0); // 0x155EF
    api.visit_finish_entry(); // 0x155F7
    api.set_word(WORDS.entry, 0); // 0x155FC
}

/// One turn of the frame loop, `0x15602` to `0x15616`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x15602
    api.dispatch(); // 0x15607
    api.frame_pass(); // 0x1560C
    api.word(WORDS.entry) == 0 // 0x15611
}

/// Everything after the frame loop, `0x15618` to `0x15635`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x15623
    api.visit_leave(); // 0x1562B
    api.slot_unload(); // 0x15630
}

/// The hook, `154Fh:0146h`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read. Codes above 9, codes 1 to 3 and codes 7 and 8 do
/// nothing (`0x1563F`, the table at `cs:01B6h`, `0x156A6`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0x1564B: a dialogue comes up; bit 1 is object 97h.
        0 => api.set_word(SPEAKING, arg & 2), // 0x15650
        // 0x15693: the script's event 18h leaves through exit 70h.
        4 if arg == 0x18 => api.script_exit(EXIT_70), // 0x15695, 0x1569E
        // 0x15655: the dialogue over; a speaking 97h back on animation 0.
        5 | 6 => {
            if api.word(SPEAKING) != 0 {
                // 0x15655
                anim_97(api, 0); // 0x15666
            }
            api.set_word(SPEAKING, 0); // 0x1566E
        }
        // 0x15676: a speaking 97h on the animation after the argument.
        9 if api.word(SPEAKING) != 0 => {
            anim_97(api, num::signed(arg.wrapping_add(1))); // 0x1567F, 0x15689
        }
        _ => {}
    }
}

/// The SFX's rule handlers by their offset in segment `154Fh`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 154F:0005 (0x154F5), a walk stopped short: with the
        // Sicherheitsausweis scene 4 as an interaction with object 97h,
        // [2BC3h] set and node 3E8h open; without it scenes 0 to 3 in turn
        // as the interaction.
        0x0005 => {
            if api.inv_has(SICHERHEITSAUSWEIS_8) != 0 {
                // 0x154FA, 0x15502
                api.interaction(OBJECT_97, FACING, 4); // 0x1550B, 09B6h:16C4h
                api.set_byte(NODE_OPEN, 1); // 0x15513
                open_node(api); // 0x1551F
            } else {
                let counter = api.ds(SEQUENCE);
                // 09B6h:164Dh, Dunkle Schatten's 0EB9h:1703h.
                api.interaction_sequence(OBJECT_97, FACING, counter, &[0, 1, 2, 3]); // 0x1553C
            }
            0x0A // 0x15544
        }
        // 154F:0059 (0x15549), Sprich mit object 97h: with the
        // Sicherheitsausweis the first time scene 5, [2BC3h] set and node
        // 3E8h open, then one of scenes 6 to 9; without it scenes 0Ah to
        // 0Dh in turn.
        0x0059 => {
            if api.inv_has(SICHERHEITSAUSWEIS_8) != 0 {
                // 0x1554E, 0x15556
                if api.byte(NODE_OPEN) == 0 {
                    // 0x15558
                    api.scene_play(5); // 0x15561
                    api.set_byte(NODE_OPEN, 1); // 0x15567
                    open_node(api); // 0x15573
                } else {
                    let n = api.random(6, 9); // 0x15581
                    api.scene_play(num::signed(n)); // 0x1558A
                }
            } else {
                let counter = api.ds(SEQUENCE);
                api.scene_sequence(counter, &[0x0A, 0x0B, 0x0C, 0x0D]); // 0x155A0
            }
            0x0A // 0x155A8
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
