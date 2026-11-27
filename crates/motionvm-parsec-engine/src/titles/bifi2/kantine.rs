//! Location 23, the Kantine, a row of kind 1 with one exit: door `6Fh` to
//! the Flur 5. Stock (location 34, the row's `+9h` and `+19h`). The module
//! is named after the location as the game itself names it in its
//! location table (`Kantine`, the record's `+37h`, `250Ah:016Dh`).
//!
//! Re-expressed from the handler at `15A5h:0093h` (image `0x15AE3` to its
//! `retf` at `0x15CA3`), its hook `15A5h:0254h` (`0x15CA4` to its `retf`
//! at `0x15EA0`, the record's `+3Fh`) and its two rule handlers
//! `15A5h:0006h` and `0055h` (`0x15A56`–`0x15AE2`), against [`Api`] alone.
//! The row's `+43h` is null, so the animation hook is the library's empty
//! one, and the segment installs no gate. The location's rules are the two
//! records at `2446h:0840h` (image `0x24CA0`, the record's `+3Bh`),
//! fourteen bytes each; verb 3 is `Sprich mit` by the verb text row at
//! `DS:116Ah`:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 3 | 4 | `98h` | — | `0006h` |
//! | 3 | 4 | `F0h` | — | `0055h` |
//!
//! Four bytes of the logic's saved state block (`DS:2B98h`, `4Eh` bytes)
//! carry the location: `[2BA6h]`, Rambowsky's course (location 7), which
//! rule `0006h` moves from 0 to 1 (`0x15A6E`) and, once the Denkfabrik's
//! `[2BBEh]` is 2 or more (`0x15A77`), to 2 (`0x15A7E`), and the entry to
//! 3 (`0x15B8C`); `[2BC7h]`, which Schlönstadt's Büro (location 24) sets to
//! 1 (`0x15ED6`) and the entry moves on to 2 (`0x15B87`); and `[2BA1h]`,
//! which the Vorzimmer (location 5) sets.
//!
//! Object `F0h` is found and set on animation 0 on every visit; object
//! `9Ah` is found; the walking record `98h` is kept. Object `98h` is gone
//! from course 3 on. Below course 3 with `[2BC7h]` at 1 (which the Büro
//! sets in course 2, `0x15ED6`) the entry shows it,
//! takes the player out of the draw chain and plays scene `17h`; the
//! script's event `0Ch` then hides the object and sends its walking record
//! from node `0BB8h` to `0BB9h`, where door `6Fh` opens, and on to
//! `0BBAh`, where the visit goes on to Schlönstadt's Büro (location 24)
//! with entry `9Ah` (`0x15C28`–`0x15C74`). Objects `F1h` and `F2h` stand
//! by chance: `F1h` two times in three, `F2h` one time in four.
//!
//! The items the hook's code 4 gives are, by the item row at `DS:10AAh`:
//! `10h` `kandierten Apfel`, `21h` `Autogramm`, `22h` `Bi-Fi-Roll`.
//!
//! `05DCh:0E45h(176Eh)` (`0x06C05`) is Dunkle Schatten's `0A9Fh:0F42h`
//! (`0x0B932`) without its test for a null record ([`Api::walker_detach`]).

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x15A5;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x15C8C`).
const OBJECT_STATES: u16 = 0x2018;

/// Object `98h`, and the walking record of the same id (`0x15B20`).
const OBJECT_98: u16 = 0x98;
/// Object `F0h`, found on every visit (`0x15B04`).
const OBJECT_F0: u16 = 0xF0;
/// Object `9Ah`, found on every visit (`0x15C06`).
const OBJECT_9A: u16 = 0x9A;
/// Objects `F1h` and `F2h`, there by chance.
const OBJECT_F1: u16 = 0xF1;
const OBJECT_F2: u16 = 0xF2;
/// Door `6Fh`, the row's one exit id, to the Flur 5. Stock.
const DOOR_6F: u16 = 0x6F;

/// The walking record's nodes: bound on `0BB8h`, sent to `0BB9h` at the
/// door, then to `0BBAh`.
const NODE_BB8: u16 = 0x0BB8;
const NODE_BB9: u16 = 0x0BB9;
const NODE_BBA: u16 = 0x0BBA;
/// A walking record's node, `+28h` (`0x15C33`).
const MOVER_NODE: u16 = 0x28;

/// Where the walking record leads the visit: Schlönstadt's Büro with
/// entry `9Ah` (`0x15C6E`, `0x15C74`).
const BUERO: u16 = 0x18;
const BUERO_ENTRY: u16 = 0x9A;

/// The items the hook's code 4 gives (the item row at `DS:10AAh`).
const APFEL: u16 = 0x10;
const AUTOGRAMM: u16 = 0x21;
const BI_FI_ROLL: u16 = 0x22;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BA1h]`: set by the Vorzimmer.
const BYTE_2BA1: u16 = 0x2BA1;
/// `[2BA6h]`: Rambowsky's course.
const COURSE: u16 = 0x2BA6;
/// `[2BBEh]`: the Denkfabrik's state.
const BYTE_2BBE: u16 = 0x2BBE;
/// `[2BC7h]`: 1 from Schlönstadt's Büro, 2 from here, 3 from the Büro.
const BYTE_2BC7: u16 = 0x2BC7;

/// The location's own words, read and written in segment `15A5h` alone
/// and outside the `38h` bytes from `DS:2D38h` the visit clears.
/// `[2F24h]`: 1 while the walking record `98h` walks out (`0x15E4B`).
const WALKS_OUT: u16 = 0x2F24;
/// `[2F26h]`, `[2F28h]`, `[2F2Ah]`: the animation the hook's code 9 is to
/// give objects `98h`, `F0h` and `9Ah`.
const NEXT_98: u16 = 0x2F26;
const NEXT_F0: u16 = 0x2F28;
const NEXT_9A: u16 = 0x2F2A;
/// `[2F2Eh]`, `[2F2Ch]`, `[2F30h]`: the animation code 9 last gave each,
/// `FFFFh` none.
const GIVEN_98: u16 = 0x2F2E;
const GIVEN_F0: u16 = 0x2F2C;
const GIVEN_9A: u16 = 0x2F30;
/// `[2F36h]`, `[2F34h]`, `[2F32h]`: bits 1, 2 and 3 of the hook's code 0
/// argument, whether objects `98h`, `F0h` and `9Ah` speak.
const SPEAKS_98: u16 = 0x2F36;
const SPEAKS_F0: u16 = 0x2F34;
const SPEAKS_9A: u16 = 0x2F32;
/// `[2F38h]`/`[2F3Ah]`: the walking record `98h` (`0x15B2A`, `0x15B26`).
const MOVER_98: u16 = 0x2F38;
/// `[2F3Ch]`/`[2F3Eh]`: object `9Ah`'s record (`0x15C10`, `0x15C0C`).
const REC_9A: u16 = 0x2F3C;
/// `[2F40h]`/`[2F42h]`: object `98h`'s record, null from course 3 on
/// (`0x15B3D`, `0x15B43`).
const REC_98: u16 = 0x2F40;
/// `[2F44h]`/`[2F46h]`: object `F0h`'s record (`0x15B0E`, `0x15B0A`).
const REC_F0: u16 = 0x2F44;

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

/// `push 0 ; push 98h ; call far 0F48h:0784h ; pop cx ; mov [2F42h], dx ;
/// mov [2F40h], ax ; push 0 ; push dx ; push ax ; call far 03F7h:1CF9h`:
/// object `98h` shown, kept and set on animation 0.
fn show_98(api: &mut dyn Api) {
    let r = api.obj_show(OBJECT_98);
    keep(api, REC_98, r);
    api.obj_anim(r, 0);
}

/// Everything before the frame loop, `0x15AE3` to `0x15C1D`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x15AE3
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x15AEF
    api.visit_enter(loc, entry); // 0x15AF3
    api.set_word(WALKS_OUT, 0); // 0x15AFB
    let r = api.obj_find(OBJECT_F0); // 0x15B04
    keep(api, REC_F0, r); // 0x15B0A, 0x15B0E
    api.obj_anim(r, 0); // 0x15B15
    let m = api.mover_find(OBJECT_98); // 0x15B20, 06E3h:04EFh
    keep(api, MOVER_98, m); // 0x15B26, 0x15B2A
    if api.byte(COURSE) >= 3 {
        // 0x15B2D (`jc`, unsigned): object 98h hidden, its record null.
        api.obj_hide(OBJECT_98); // 0x15B37
        keep(api, REC_98, Far::NULL); // 0x15B3D, 0x15B43
    } else if api.byte(BYTE_2BC7) == 1 {
        // 0x15B4B: object 98h shown, the player out of the draw chain,
        // scene 17h, [2BC7h] 2 and the course 3.
        show_98(api); // 0x15B55, 0x15B5B, 0x15B66
        api.input_lock(); // 0x15B6E
        let player = api.ds(WALKER);
        api.walker_detach(player); // 0x15B77, 05DCh:0E45h
        api.scene_play(0x17); // 0x15B81
        api.set_byte(BYTE_2BC7, 2); // 0x15B87
        api.set_byte(COURSE, 3); // 0x15B8C
    } else {
        show_98(api); // 0x15B96, 0x15B9C, 0x15BA7
    }
    // 0x15BAF: object F1h shown on a roll of 1 or 2 out of 0 to 2 (`jng`,
    // signed), else hidden.
    if num::signed(api.random(0, 2)) > 0 {
        api.obj_show(OBJECT_F1); // 0x15BC2
    } else {
        api.obj_hide(OBJECT_F1); // 0x15BCC
    }
    // 0x15BD2: object F2h shown on animation 0 on a roll of 3 out of 0 to
    // 3 (`cmp ax, 2 ; jng`), else hidden.
    if num::signed(api.random(0, 3)) > 2 {
        // `push 0 ; push 0F2h ; call far 0F48h:0784h ; pop cx`: the 0
        // stays on the stack as 03F7h:1CF9h's animation.
        let r = api.obj_show(OBJECT_F2); // 0x15BE8
        api.obj_anim(r, 0); // 0x15BF0
    } else {
        api.obj_hide(OBJECT_F2); // 0x15BFD
    }
    let r = api.obj_find(OBJECT_9A); // 0x15C06
    keep(api, REC_9A, r); // 0x15C0C, 0x15C10
    api.visit_finish_entry(); // 0x15C13
    api.set_word(WORDS.entry, 0); // 0x15C18
}

/// One turn of the frame loop, `0x15C1E` to `0x15C84`: while the walking
/// record `98h` walks out, by its node (`+28h`): on `0BB9h` door `6Fh`
/// takes animation 1 and the record walks on to `0BBAh` facing 1; on
/// `0BBAh` the visit goes on to Schlönstadt's Büro with entry `9Ah`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x15C1E
    api.dispatch(); // 0x15C23
    if api.word(WALKS_OUT) != 0 {
        // 0x15C28
        let m = rec(api, MOVER_98);
        let node = api.far_word(Far {
            off: m.off.wrapping_add(MOVER_NODE),
            seg: m.seg,
        }); // 0x15C2F, 0x15C33
        match node {
            NODE_BB9 => {
                // 0x15C43: `push 1 ; push 6Fh ; call far 06E3h:047Eh ; pop
                // cx ; push dx ; push ax ; call far 03F7h:1CF9h`.
                let door = api.obj_find(DOOR_6F); // 0x15C47
                api.obj_anim(door, 1); // 0x15C4F
                let m = rec(api, MOVER_98);
                api.walker_walk_to(m, NODE_BBA, 1); // 0x15C64, answer not read
            }
            NODE_BBA => {
                api.set_word(WORDS.location, BUERO); // 0x15C6E
                api.set_word(WORDS.entry, BUERO_ENTRY); // 0x15C74
            }
            _ => {}
        }
    }
    api.frame_pass(); // 0x15C7A
    api.word(WORDS.entry) == 0 // 0x15C7F
}

/// Everything after the frame loop, `0x15C86` to `0x15CA3`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x15C91
    api.visit_leave(); // 0x15C99
    api.slot_unload(); // 0x15C9E
}

/// The hook, `15A5h:0254h`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read. Codes above 9 do nothing (`cmp bx, 9 ; jna` over a
/// `jmp` to the `retf`, `0x15CAD`–`0x15CB2`); the others go through the table at `cs:045Bh` (image `0x15EAB`): 0
/// `0x15CBC`, 4 `0x15DFC`, 5 and 6 `0x15CE9`, 7 `0x15E22`, 8 `0x15D33`, 9
/// `0x15D87`, 1 to 3 the `retf` path `0x15E9F`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0x15CBC: a dialogue comes up; bits 1 to 3 say whether objects
        // 98h, F0h and 9Ah speak; no animation given yet.
        0 => {
            api.set_word(SPEAKS_98, arg & 2); // 0x15CC1
            api.set_word(SPEAKS_F0, arg & 4); // 0x15CC9
            api.set_word(SPEAKS_9A, arg & 8); // 0x15CD1
            api.set_word(GIVEN_9A, 0xFFFF); // 0x15CD4
            api.set_word(GIVEN_98, 0xFFFF); // 0x15CDA
            api.set_word(GIVEN_F0, 0xFFFF); // 0x15CE0
        }
        4 => items(api, arg),  // 0x15DFC
        5 | 6 => rest(api),    // 0x15CE9
        7 => events(api, arg), // 0x15E22
        8 => next(api, arg),   // 0x15D33
        9 => give(api),        // 0x15D87
        _ => {}
    }
}

/// Code 4, `0x15DFC`: argument `1Ah` gives the `Autogramm`, `1Bh` the
/// `kandierten Apfel`, `1Ch` the `Bi-Fi-Roll` (`0x15E1A`).
fn items(api: &mut dyn Api, arg: u16) {
    let item = match arg {
        0x1A => AUTOGRAMM,  // 0x15E10
        0x1B => APFEL,      // 0x15E14
        0x1C => BI_FI_ROLL, // 0x15E18
        _ => return,        // 0x15E0D
    };
    api.inv_give(item); // 0x15E1A
}

/// Codes 5 and 6, `0x15CE9`: whoever speaks back on animation 0 — object
/// `98h` only while its record is not null; the speaking is kept.
fn rest(api: &mut dyn Api) {
    if api.word(SPEAKS_98) != 0 {
        // 0x15CE9
        let r = rec(api, REC_98);
        if r != Far::NULL {
            // 0x15CF0–0x15CF7: `mov ax, [2F40h] ; or ax, [2F42h] ; jz`.
            api.obj_anim(r, 0); // 0x15D03
        }
    }
    if api.word(SPEAKS_F0) != 0 {
        // 0x15D0B
        let r = rec(api, REC_F0);
        api.obj_anim(r, 0); // 0x15D1C
    }
    if api.word(SPEAKS_9A) != 0 {
        // 0x15D24, then `push 0 ; jmp 0x15DE9`.
        let r = rec(api, REC_9A);
        api.obj_anim(r, 0); // 0x15DF1
    }
}

/// Code 8, `0x15D33`: the animations code 9 is to give, 1 for `98h` and
/// `F0h` and 0 for `9Ah`, then by the argument plus 1 up to 4 (`jna`,
/// unsigned) through the table at `cs:0451h` (image `0x15EA1`): `FFFFh`
/// both 0, 0 nothing more, 1 object `98h` on 3, 2 object `F0h` on 2, 3
/// object `98h` on 2 and `9Ah` on 1.
fn next(api: &mut dyn Api, arg: u16) {
    api.set_word(NEXT_9A, 0); // 0x15D33
    api.set_word(NEXT_F0, 1); // 0x15D39
    api.set_word(NEXT_98, 1); // 0x15D3F
    match arg.wrapping_add(1) {
        0 => {
            api.set_word(NEXT_98, 0); // 0x15D57
            api.set_word(NEXT_F0, 0); // 0x15D5D
        }
        2 => api.set_word(NEXT_98, 3), // 0x15D66
        3 => api.set_word(NEXT_F0, 2), // 0x15D6F
        4 => {
            api.set_word(NEXT_98, 2); // 0x15D78
            api.set_word(NEXT_9A, 1); // 0x15D7E
        }
        _ => {} // 1: 0x15E9F; above 4: 0x15D4D
    }
}

/// `0x15D8E`–`0x15DA3` and its two twins: a speaking object's record on
/// the animation `[next]` when that is not the one `[given]` keeps.
fn give_one(api: &mut dyn Api, speaks: u16, next: u16, given: u16, at: u16) {
    if api.word(speaks) == 0 {
        return;
    }
    let k = api.word(next);
    if k == api.word(given) {
        return;
    }
    api.set_word(given, k);
    let r = rec(api, at);
    api.obj_anim(r, num::signed(k));
}

/// Code 9, `0x15D87`: each speaker on its next animation, `98h` (with no
/// test for a null record), `F0h`, then `9Ah`.
fn give(api: &mut dyn Api) {
    give_one(api, SPEAKS_98, NEXT_98, GIVEN_98, REC_98); // 0x15D87–0x15DA3
    give_one(api, SPEAKS_F0, NEXT_F0, GIVEN_F0, REC_F0); // 0x15DAB–0x15DC7
    give_one(api, SPEAKS_9A, NEXT_9A, GIVEN_9A, REC_9A); // 0x15DCF–0x15DF1
}

/// Code 7, `0x15E22`: events `0Ah` and `0Bh` set object `98h`'s next
/// animation to 4 and 5; `0Ch` sends it out.
fn events(api: &mut dyn Api, arg: u16) {
    match arg {
        0x0A => api.set_word(NEXT_98, 4), // 0x15E35
        0x0B => api.set_word(NEXT_98, 5), // 0x15E3D
        // 0x15E45: object 98h no longer speaks and is hidden; its walking
        // record names subject slot 1, is bound to graph 1 on node 0BB8h
        // facing 1 and walks to node 0BB9h facing 1.
        0x0C => {
            api.set_word(SPEAKS_98, 0); // 0x15E45
            api.set_word(WALKS_OUT, 1); // 0x15E4B
            api.input_lock(); // 0x15E51
            api.obj_hide(OBJECT_98); // 0x15E59
            let m = rec(api, MOVER_98);
            api.slot_set_figure(1, 0, m); // 0x15E6B, 080Ch:19B4h
            let m = rec(api, MOVER_98);
            api.walker_bind_graph(m, 1, NODE_BB8, 1); // 0x15E82, 05DCh:0DB4h
            let m = rec(api, MOVER_98);
            api.walker_walk_to(m, NODE_BB9, 1); // 0x15E97, answer not read
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

/// The Kantine's rule handlers by their offset in segment `15A5h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 15A5:0006 (0x15A56), `Sprich mit` object 98h, by the course: 0
        // scene 0 and course 1; 1 with [2BBEh] at 2 or more (`jc`,
        // unsigned) scene 5 and course 2, else one of scenes 1 to 4; 2 one
        // of scenes 6 to 9; otherwise none.
        0x0006 => {
            match api.byte(COURSE) {
                0 => {
                    api.set_byte(COURSE, 1); // 0x15A6E
                    api.scene_play(0); // 0x15A73, 0x15A9A
                }
                1 => {
                    if api.byte(BYTE_2BBE) >= 2 {
                        // 0x15A77
                        api.set_byte(COURSE, 2); // 0x15A7E
                        api.scene_play(5); // 0x15A83, 0x15A9A
                    } else {
                        scene_between(api, 1, 4); // 0x15A87, 0x15A91, 0x15A9A
                    }
                }
                2 => scene_between(api, 6, 9), // 0x15A8D, 0x15A91, 0x15A9A
                _ => {}                        // 0x15A6C
            }
            0x0A // 0x15AA0
        }
        // 15A5:0055 (0x15AA5), `Sprich mit` object F0h: scene 0Ah while
        // [2BA1h] is 0 and neither the `kandierten Apfel` nor the
        // `Bi-Fi-Roll` is carried, else one of scenes 11h and 12h.
        0x0055 => {
            if api.byte(BYTE_2BA1) == 0 && api.inv_has(APFEL) == 0 && api.inv_has(BI_FI_ROLL) == 0 {
                // 0x15AA8, 0x15AB1, 0x15ABD
                api.scene_play(0x0A); // 0x15AC7, 0x15AD8
            } else {
                scene_between(api, 0x11, 0x12); // 0x15ACB, 0x15ACF, 0x15AD8
            }
            0x0A // 0x15ADE
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
