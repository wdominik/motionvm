//! Location 17, the Denkfabrik, a row of kind 1 with one exit: door `70h`
//! to the Flur 1. Stock (location 19, the row's `+9h` and `+19h`). The
//! module is named after the location as the game itself names it in its
//! location table (`Denkfabrik`, the record's `+37h`, `250Ah:012Fh`).
//!
//! Re-expressed from the handler at `14DEh:0059h` (image `0x14E39` to its
//! `retf` at `0x14F12`), its hook `14DEh:0133h` (`0x14F13` to its `retf`
//! at `0x14F80`, the record's `+3Fh`) and its three rule handlers
//! `14DEh:0000h`, `001Bh` and `0027h` (`0x14DE0`–`0x14E38`), against
//! [`Api`] alone. The row's `+43h` is null, so the animation hook is the
//! library's empty one. The location's rules are the five records at
//! `2446h:0600h` (image `0x24A60`, the record's `+3Bh`), fourteen bytes
//! each as in the global table:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 3 | 4 | `42h` | — | `0000h` |
//! | 3 | 4 | `43h` | — | `0000h` |
//! | 3 | 4 | `44h` | — | `0000h` |
//! | 1 | 4 | `F0h` | — | `001Bh` |
//! | 3 | 4 | `94h` | — | `0027h` |
//!
//! Three bytes of the logic's saved state block (`DS:2B98h`, `4Eh` bytes)
//! carry the location: `[2BBEh]`, its own state 0 to 4, which Rambowsky
//! (location 7) moves from 0 to 1 (`0x11DC0`) and the Kantine (location
//! 23) reads (`0x15A77`); `[2BA6h]`, which the Kantine writes (2 at
//! `0x15A7E`, 3 at `0x15B8C`); and `[2BA7h]`, which rule `001Bh` sets and
//! Rambowsky reads (`0x11F69`).
//!
//! In state 1 the entry plays scene 3 as an interaction with object `94h`
//! and moves on to state 2; in state 2, once `[2BA6h]` is 2, it moves on
//! to 3; once `[2BA6h]` is 3 (and the state not yet 4), object `F0h` is
//! shown and the state is 4. Object `94h` stands in states 1 to 3 and
//! takes the animation the speaker's change names (the hook's code 8).

use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x14DE;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x14EFB`).
const OBJECT_STATES: u16 = 0x2018;

/// Object `94h`, shown in states 1 to 3 (`0x14EAF`) and hidden otherwise
/// (`0x14ECB`).
const OBJECT_94: u16 = 0x94;
/// Object `F0h`, shown once `[2BA6h]` is 3 (`0x14E5F`).
const OBJECT_F0: u16 = 0xF0;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
const BYTE_2BA6: u16 = 0x2BA6;
const BYTE_2BA7: u16 = 0x2BA7;
const BYTE_2BBE: u16 = 0x2BBE;

/// The handler's words, read and written in segment `14DEh` alone and
/// outside the `38h` bytes from `DS:2D38h` the visit clears.
/// `[2F06h]`: the argument of the hook's code 8 last followed, `FFFFh`
/// none (`0x14F30`, `0x14F66`).
const LAST_ARG: u16 = 0x2F06;
/// `[2F08h]`: bit 1 of the hook's code 0 argument, whether object `94h`
/// speaks (`0x14F2D`).
const SPEAKS: u16 = 0x2F08;
/// `[2F0Ah]`/`[2F0Ch]`: object `94h`'s record as `0F48h:0784h` answers it
/// (`0x14EB8`, `0x14EBC`).
const REC_94: u16 = 0x2F0A;

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

/// Everything before the frame loop, `0x14E39` to `0x14EDE`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x14E39
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x14E45
    api.visit_enter(loc, entry); // 0x14E49
    // 0x14E51–0x14E9A: the state moved on.
    let state = api.byte(BYTE_2BBE);
    if state != 4 && api.byte(BYTE_2BA6) == 3 {
        // 0x14E51, 0x14E58.
        api.obj_show(OBJECT_F0); // 0x14E62
        api.set_byte(BYTE_2BBE, 4); // 0x14E68
    } else if state == 1 {
        // 0x14E6F: scene 3 as an interaction with object 94h, facing 1;
        // 09B6h:16C4h (0x14E7D), Dunkle Schatten's 0EB9h:177Ah.
        api.interaction(OBJECT_94, 1, 3); // 0x14E7D
        api.set_byte(BYTE_2BBE, 2); // 0x14E85
    } else if state == 2 && api.byte(BYTE_2BA6) == 2 {
        // 0x14E8C, 0x14E93.
        api.set_byte(BYTE_2BBE, 3); // 0x14E9A
    }
    // 0x14E9F–0x14ECE: object 94h by the state.
    let state = api.byte(BYTE_2BBE);
    if state != 4 && state != 0 {
        // `push 0 ; push 94h ; call far 0F48h:0784h ; pop cx`: the 0 stays
        // on the stack as 03F7h:1CF9h's animation (`add sp, 6`, 0x14EC6).
        let r = api.obj_show(OBJECT_94); // 0x14EB2
        keep(api, REC_94, r); // 0x14EB8, 0x14EBC
        api.obj_anim(r, 0); // 0x14EC1
    } else {
        api.obj_hide(OBJECT_94); // 0x14ECE
    }
    api.visit_finish_entry(); // 0x14ED4
    api.set_word(WORDS.entry, 0); // 0x14ED9
}

/// One turn of the frame loop, `0x14EDF` to `0x14EF3`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x14EDF
    api.dispatch(); // 0x14EE4
    api.frame_pass(); // 0x14EE9
    api.word(WORDS.entry) == 0 // 0x14EEE
}

/// Everything after the frame loop, `0x14EF5` to `0x14F12`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x14F00
    api.visit_leave(); // 0x14F08
    api.slot_unload(); // 0x14F0D
}

/// The hook, `14DEh:0133h`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read. Codes above 8 do nothing (`ja`, `0x14F1F`); the
/// others go through the table at `cs:01A1h` (image `0x14F81`): 0
/// `0x14F28`, 5 and 6 `0x14F38`, 8 `0x14F59`, 1 to 4 and 7 the `retf`
/// path `0x14F7F`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0x14F28: bit 1 of the argument kept, no argument followed yet.
        0 => {
            api.set_word(SPEAKS, arg & 2); // 0x14F2D
            api.set_word(LAST_ARG, 0xFFFF); // 0x14F30
        }
        // 0x14F38: while speaking, object 94h back on animation 0 and the
        // speaking cleared.
        5 | 6 if api.word(SPEAKS) != 0 => {
            let r = rec(api, REC_94);
            api.obj_anim(r, 0); // 0x14F49
            api.set_word(SPEAKS, 0); // 0x14F51
        }
        // 0x14F59: while speaking, object 94h on animation `arg + 1`, once
        // for each new argument.
        8 if api.word(SPEAKS) != 0 && api.word(LAST_ARG) != arg => {
            api.set_word(LAST_ARG, arg); // 0x14F66
            let r = rec(api, REC_94);
            api.obj_anim(r, num::signed(arg.wrapping_add(1))); // 0x14F77
        }
        _ => {}
    }
}

/// `push hi ; push lo ; call far 203Dh:0024h ; add sp, 4 ; push ax ; call
/// far 080Ch:164Ch`: one of scenes `lo` to `hi`.
fn scene_between(api: &mut dyn Api, lo: u16, hi: u16) {
    let n = api.random(lo, hi);
    api.scene_play(num::signed(n));
}

/// The Denkfabrik's rule handlers by their offset in segment `14DEh`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 14DE:0000 (0x14DE0), verb 3 on objects 42h to 44h: one of scenes
        // 0 to 2.
        0x0000 => {
            scene_between(api, 0, 2); // 0x14DE7, 0x14DF0
            0x0A // 0x14DF6
        }
        // 14DE:001B (0x14DFB), verb 1 on object F0h: [2BA7h] set; answer 0.
        0x001B => {
            api.set_byte(BYTE_2BA7, 1); // 0x14DFE
            0 // 0x14E03
        }
        // 14DE:0027 (0x14E07), verb 3 on object 94h, by [2BBEh]: in state
        // 2 one of scenes 4 to 6, in state 3 one of 7 and 8, else none.
        0x0027 => {
            match api.byte(BYTE_2BBE) {
                2 => scene_between(api, 4, 6), // 0x14E1B, 0x14E25, 0x14E2E
                3 => scene_between(api, 7, 8), // 0x14E21, 0x14E25, 0x14E2E
                _ => {}                        // 0x14E19
            }
            0x0A // 0x14E34
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
