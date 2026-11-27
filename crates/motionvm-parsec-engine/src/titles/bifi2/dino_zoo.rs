//! Location 14, the Dino Zoo, a row of kind 1 with one exit, `6Fh`, to
//! the Rambowsky (location 7). The module is named after the location as
//! the game itself names it in its location table (`Dino Zoo`, the
//! record's `+37h`, `250Ah:0114h`).
//!
//! Re-expressed from the handler at `1407h:00E5h` (image `0x14155` to its
//! `retf` at `0x1426A`), its hook `1407h:02C0h` (`0x14330` to its `retf`
//! at `0x14574`, the record's `+3Fh`), its animation hook `1407h:0205h`
//! (`0x14275` to `0x14327`, the record's `+43h`), its gate `1407h:0067h`
//! (`0x140D7` to `0x1411D`, installed by rule `00AEh`) and its five rule
//! handlers `1407h:000Fh` to `00D7h` (`0x1407F`–`0x14154`), against
//! [`Api`] alone. The location's rules are the five records at
//! `2446h:04E0h` (image `0x24940`, the record's `+3Bh`), fourteen bytes
//! each:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 3 | 4 | `8Fh` | — | `000Fh` |
//! | 3 | 4 | `90h` | — | `002Dh` |
//! | 2 | 4 | `15h` | — | `005Bh` |
//! | `0Bh` | 4 | `1Eh` | — | `00AEh` |
//! | 4 | 2 | `1Eh` | — | `00D7h` |
//!
//! The verbs are, by the verb text row at `DS:116Ah`: 2 `Nimm`, 3 `Sprich
//! mit`, 4 `Benutze`, `0Bh` `Wirf`. The items the logic names are, by the
//! item row at `DS:10AAh`: `1Eh` `Ball`, `1Fh` `Wagenheber`. `Benutze` on
//! the `Ball` becomes `Wirf` (rule `00D7h`); throwing it at object `1Eh`
//! sends the player to node `0FA0h` and installs the gate, which, asked
//! with 0, stops a speaking task 1, plays scene 9, takes the input over and
//! moves the state on to 4. The animation hook's codes `64h` to `67h` give
//! the input back, bring objects `42h` and `15h` in, and swap the player
//! for object `F0h` and back.
//!
//! The bytes of the logic's saved state block (`DS:2B98h`, `4Eh` bytes)
//! the location reads or writes, each referenced in segment `1407h` alone:
//! `[2BB5h]`, the location's state (0 to 5), and `[2BD9h]`, set by the
//! hook's item event `28h`. On entry state 0 plays scene 0 and becomes 1;
//! states 1 and 2 hand their own number to task 1 as its one scene and
//! step on; state 3 hands scene 3 to task 1; state 4 sets object `15h` on
//! animation 0 and shows object `42h`; states above 4 show `42h` alone.
//! Rule `005Bh` (`Nimm` object `15h`) sets state 5.
//!
//! A scene handed to task 1 alone is `080Ch:185Ch(n, flags)` (`0x0991C`),
//! Dunkle Schatten's `0CFDh:194Bh` (`0x0E91B`) instruction for
//! instruction ([`Api::ambient_play_one`]). A task stopped is
//! `080Ch:191Fh` (`0x099DF`), Dunkle Schatten's `0CFDh:1A0Eh`
//! ([`Api::task_stop`]). Whether task 1 speaks is `080Ch:1A39h`
//! (`0x09AF9`), Dunkle Schatten's `0CFDh:1B55h` (`0x0EB25`,
//! [`Api::ambient_speakers`]).
//!
//! The hook's event `11h` takes the player out of the picture — its
//! record's live bit cleared (`0x14532`) — and object `F0h` stands in for
//! it; the animation code `67h` puts it back (`0x14315`).

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers and the gate.
pub(super) const SEGMENT: u16 = 0x1407;
/// The gate's offset (`mov word [2D66h], 67h`, `0x1413C`).
const GATE: u16 = 0x0067;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x14253`).
const OBJECT_STATES: u16 = 0x2018;

/// The objects the logic names.
const OBJECT_15: u16 = 0x15;
const OBJECT_1E: u16 = 0x1E;
const OBJECT_42: u16 = 0x42;
const OBJECT_8F: u16 = 0x8F;
const OBJECT_90: u16 = 0x90;
const OBJECT_F0: u16 = 0xF0;

/// The items the logic moves (the item row at `DS:10AAh`).
const BALL: u16 = 0x1E;
const WAGENHEBER: u16 = 0x1F;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BB5h]`: the location's state, 0 to 5.
const STATE: u16 = 0x2BB5;
/// `[2BD9h]`: set by the hook's item event `28h` (`0x144F9`).
const BYTE_2BD9: u16 = 0x2BD9;

/// The location's own words, read and written in segment `1407h` alone
/// and outside the `38h` bytes from `DS:2D38h` the visit clears.
/// `[2EC0h]`: the task the script's event `50h` named, `FFFFh` none.
const MARKED_TASK: u16 = 0x2EC0;
/// `[2EC2h]`/`[2EC4h]`: the animation last given to and wanted for `8Fh`.
const GIVEN_8F: u16 = 0x2EC2;
const WANTED_8F: u16 = 0x2EC4;
/// `[2EC6h]`: `8Fh`'s mode, 0 or `0Ah` to `0Dh`.
const MODE_8F: u16 = 0x2EC6;
/// `[2EC8h]`/`[2ECAh]`: the animation last given to and wanted for `90h`.
const GIVEN_90: u16 = 0x2EC8;
const WANTED_90: u16 = 0x2ECA;
/// `[2ECCh]`: `90h`'s mode, 0 or `0Eh` to `10h`.
const MODE_90: u16 = 0x2ECC;
/// The records kept on entry, a far pointer each: `[2ECEh]` object `8Fh`,
/// `[2ED2h]` `90h`, `[2ED6h]` `1Eh`, `[2EDAh]` `F0h`, `[2EDEh]` `15h`.
const REC_8F: u16 = 0x2ECE;
const REC_90: u16 = 0x2ED2;
const REC_1E: u16 = 0x2ED6;
const REC_F0: u16 = 0x2EDA;
const REC_15: u16 = 0x2EDE;

/// The player record's flag byte `+2` (`[1770h]`); bit 0 is its live bit,
/// which the chain's draw tests (`and ax, 3 ; cmp ax, 1`, `0x07243`).
const PLAYER_FLAGS: u16 = WALKER + 2;
const LIVE: u8 = 0x01;

/// The node rule `00AEh` sends the player to (`mov ax, 0FA0h`, `0x14123`).
const NODE_THROW: u16 = 0x0FA0;

/// The mode block's verb, wait bits, walk target and gate.
const VERB: u16 = 0x2D38;
const BITS: u16 = 0x2D46;
const WALK_TARGET: u16 = 0x2D4A;
const GATE_OFF: u16 = 0x2D66;
const GATE_SEG: u16 = 0x2D68;

/// The verb `Wirf`.
const VERB_WIRF: u16 = 0x0B;

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

/// `push k ; push word [at+2] ; push word [at] ; call far 03F7h:1CF9h`: a
/// kept record on animation `k`.
fn anim(api: &mut dyn Api, at: u16, k: u16) {
    let r = rec(api, at);
    api.obj_anim(r, num::signed(k));
}

/// `push 0 ; push n ; call far 080Ch:185Ch` (`0x141F4`–`0x14206`): scene
/// `n` on task 1 alone, not repeated; the answer is not read.
fn ambient_one(api: &mut dyn Api, n: u16) {
    api.ambient_play_one(num::signed(n), 0);
}

/// Everything before the frame loop, `0x14155` to `0x14231`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x14155
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x14161
    api.visit_enter(loc, entry); // 0x14165
    let r = api.obj_hide(OBJECT_15); // 0x1416F
    keep(api, REC_15, r); // 0x14175, 0x14179
    let r = api.obj_hide(OBJECT_F0); // 0x1417F
    keep(api, REC_F0, r); // 0x14185, 0x14189
    let r = api.obj_hide(OBJECT_1E); // 0x1418E
    keep(api, REC_1E, r); // 0x14194, 0x14198
    let r = api.obj_find(OBJECT_90); // 0x1419E
    keep(api, REC_90, r); // 0x141A4, 0x141A8
    api.obj_anim(r, 0); // 0x141AF
    let r = api.obj_find(OBJECT_8F); // 0x141BA
    keep(api, REC_8F, r); // 0x141C0, 0x141C4
    api.obj_anim(r, 0); // 0x141CB
    // 0x141D3: by the state through the table at cs:01FBh (0x1426B);
    // states above 4 show 42h alone.
    match api.byte(STATE) {
        0 => {
            api.scene_play(0); // 0x141E8
            api.set_byte(STATE, 1); // 0x141EE
        }
        s @ (1 | 2) => {
            api.set_byte(STATE, s.wrapping_add(1)); // 0x141F9
            ambient_one(api, u16::from(s)); // 0x14206, the state before
        }
        3 => ambient_one(api, 3), // 0x14204, 0x14206
        4 => {
            anim(api, REC_15, 0); // 0x1421A
            api.obj_show(OBJECT_42); // 0x14226
        }
        _ => {
            api.obj_show(OBJECT_42); // 0x14226
        }
    }
    api.visit_finish_entry(); // 0x1422C
    api.set_word(WORDS.entry, 0); // 0x14231
}

/// One turn of the frame loop, `0x14237` to `0x1424B`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x14237
    api.dispatch(); // 0x1423C
    api.frame_pass(); // 0x14241
    api.word(WORDS.entry) == 0 // 0x14246
}

/// Everything after the frame loop, `0x1424D` to `0x1426A`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x14258
    api.visit_leave(); // 0x14260
    api.slot_unload(); // 0x14265
}

/// The hook, `1407h:02C0h`: code in `[bp+6]`, argument in `[bp+8]`, task
/// in `[bp+0Ah]`. Codes above 9 and codes 1 to 3 and 8 do nothing
/// (`0x1433E`, the table at `cs:0539h`, `0x145A9`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, task: u16) {
    match code {
        // 0x1434D: a dialogue comes up; the marked task forgotten if it is
        // this one, both objects' animations forgotten; bit 1 of the
        // argument puts 8Fh in mode 0Ah, bit 2 puts 90h in mode 0Eh.
        0 => {
            unmark(api, task); // 0x1434D
            api.set_word(GIVEN_8F, 0xFFFF); // 0x1435C
            api.set_word(WANTED_8F, 0xFFFF); // 0x1435F
            let m = if arg & 2 != 0 { 0x0A } else { 0 };
            api.set_word(MODE_8F, m); // 0x14368, 0x14370
            api.set_word(GIVEN_90, 0xFFFF); // 0x14379
            api.set_word(WANTED_90, 0xFFFF); // 0x1437C
            if arg & 4 != 0 {
                api.set_word(MODE_90, 0x0E); // 0x14385
                api.set_word(GIVEN_90, 0xFFFF); // 0x1438B
            } else {
                api.set_word(MODE_90, 0); // 0x14394
            }
        }
        4 => items(api, arg), // 0x144DC
        // 0x1439D: the dialogue over; 90h, then 8Fh, back on animation 0
        // while it has a mode. The modes are left as they are.
        5 | 6 => {
            unmark(api, task); // 0x1439D
            if api.word(MODE_90) != 0 {
                anim(api, REC_90, 0); // 0x143BA
            }
            if api.word(MODE_8F) != 0 {
                anim(api, REC_8F, 0); // 0x143D6, 0x14552
            }
        }
        7 => events(api, arg, task), // 0x1450A
        9 => page(api, arg, task),   // 0x143D9
        _ => {}
    }
}

/// `cmp [2EC0h], dx ; jnz ; mov word [2EC0h], 0FFFFh`: the marked task
/// forgotten when it is this one. Answers whether it was.
fn unmark(api: &mut dyn Api, task: u16) -> bool {
    if api.word(MARKED_TASK) != task {
        return false;
    }
    api.set_word(MARKED_TASK, 0xFFFF);
    true
}

/// Code 9, `0x143D9`: the argument (the speaking slot) taken as 0 for the
/// marked task, which is forgotten. Argument 1 picks `8Fh`'s talking
/// animation by its mode, any other argument its resting one; argument 2
/// does the same for `90h`. A new animation goes to its object.
fn page(api: &mut dyn Api, arg: u16, task: u16) {
    let arg = if unmark(api, task) { 0 } else { arg }; // 0x143E5
    let mode = api.word(MODE_8F);
    if arg == 1 {
        // 0x143EC: by the mode less 0Ah through the table at cs:0531h.
        match mode {
            0x0A => api.set_word(WANTED_8F, 1), // 0x143FF
            0x0B => api.set_word(WANTED_8F, 4), // 0x14407
            0x0C => {
                api.set_word(GIVEN_8F, 0xFFFF); // 0x1440F
                api.set_word(WANTED_8F, 3); // 0x14415
            }
            0x0D => api.set_word(WANTED_8F, 2), // 0x1441D
            _ => {}
        }
    } else if (0x0A..=0x0D).contains(&mode) {
        api.set_word(WANTED_8F, 0); // 0x14438, by the table at cs:0529h
    }
    let k = api.word(WANTED_8F);
    if k != api.word(GIVEN_8F) {
        // 0x14441
        api.set_word(GIVEN_8F, k); // 0x14447
        anim(api, REC_8F, k); // 0x14453
    }
    let mode = api.word(MODE_90);
    let (on_0e, on_0f) = if arg == 2 { (1, 5) } else { (0, 4) }; // 0x1445B
    match mode {
        0x0E => api.set_word(WANTED_90, on_0e), // 0x14474, 0x144A3
        0x0F => {
            if api.word(GIVEN_90) == api.word(WANTED_90) {
                api.set_word(WANTED_90, on_0f); // 0x14485, 0x144B4
            }
        }
        0x10 => api.set_word(WANTED_90, 2), // 0x144BC
        _ => {}
    }
    let k = api.word(WANTED_90);
    if k != api.word(GIVEN_90) {
        // 0x144C5
        api.set_word(GIVEN_90, k); // 0x144CE
        anim(api, REC_90, k); // 0x144DA, 0x14552
    }
}

/// Code 4, `0x144DC`: the script's item events `0Fh`, `28h` and `29h`.
fn items(api: &mut dyn Api, arg: u16) {
    match arg {
        0x0F => {
            api.inv_give(WAGENHEBER); // 0x144F2
        }
        0x28 => api.set_byte(BYTE_2BD9, 1), // 0x144F9
        0x29 => {
            api.inv_take(WAGENHEBER); // 0x14502
        }
        _ => {}
    }
}

/// Code 7, `0x1450A`: the argument looked up in the row at `cs:0505h`
/// (`0Ah` to `11h`, `50h`), its arm in the row after it (`cs:0517h`).
fn events(api: &mut dyn Api, arg: u16, task: u16) {
    match arg {
        // 0x1455C: 8Fh's mode.
        0x0A..=0x0D => api.set_word(MODE_8F, arg),
        // 0x14568: 90h's mode.
        0x0E | 0x10 => api.set_word(MODE_90, arg),
        // 0x14562: 90h wants animation 3, mode 0Fh.
        0x0F => {
            api.set_word(WANTED_90, 3); // 0x14562
            api.set_word(MODE_90, arg); // 0x14568
        }
        // 0x14526: the player taken out, the Ball taken, object F0h shown
        // in its place on animation 0.
        0x11 => {
            let player = api.ds(WALKER);
            api.obj_invalidate(player); // 0x1452A
            let flags = api.byte(PLAYER_FLAGS) & !LIVE;
            api.set_byte(PLAYER_FLAGS, flags); // 0x14532
            api.inv_take(BALL); // 0x14539
            api.obj_show(OBJECT_F0); // 0x14542, the record not kept
            anim(api, REC_F0, 0); // 0x14552
        }
        // 0x1456E: the task marked.
        0x50 => api.set_word(MARKED_TASK, task),
        _ => {}
    }
}

/// The animation hook, `1407h:0205h`: the code in `[bp+0Ah]`; codes `64h`
/// to `67h` have arms (`0x14278`–`0x14288`, the table at `cs:02B8h`).
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        // 0x1428D: the input given back, samples 0Ah and 0Ch.
        0x64 => {
            api.input_unlock(); // 0x1428D
            api.play_sample(0x0A); // 0x14294
            api.play_sample(0x0C); // 0x1429C
        }
        // 0x142A5: objects 42h and 15h shown, samples 1Ch, 0Eh and 0Bh,
        // 15h on animation 1.
        0x65 => {
            api.obj_show(OBJECT_42); // 0x142A7
            api.obj_show(OBJECT_15); // 0x142AF
            api.play_sample(0x1C); // 0x142B7
            api.play_sample(0x0E); // 0x142BF
            api.play_sample(0x0B); // 0x142C7
            anim(api, REC_15, 1); // 0x142D7
        }
        // 0x142E1: object 1Eh hidden.
        0x66 => {
            api.obj_hide(OBJECT_1E); // 0x142E3
        }
        // 0x142EA: object 1Eh shown on animation 0, F0h hidden, sample
        // 0Ah, the player live again.
        0x67 => {
            api.obj_show(OBJECT_1E); // 0x142EC, the record not kept
            anim(api, REC_1E, 0); // 0x142FC
            api.obj_hide(OBJECT_F0); // 0x14307
            api.play_sample(0x0A); // 0x1430F
            let flags = api.byte(PLAYER_FLAGS) | LIVE;
            api.set_byte(PLAYER_FLAGS, flags); // 0x14315
            let player = api.ds(WALKER);
            api.obj_invalidate(player); // 0x1431E
        }
        _ => {}
    }
}

/// The gate, `1407h:0067h` (`0x140D7`): asked with 0 the wait bits keep
/// all but bits 4 to 8 and take bit 4, a speaking task 1 is stopped, scene
/// 9 plays, the input is taken over, the gate cleared and the state set
/// to 4; asked with anything else it does nothing. It answers 0 either
/// way (`0x1411A`).
pub(crate) fn gate(api: &mut dyn Api, off: u16, arg: i16) -> Option<i16> {
    if off != GATE {
        return None;
    }
    if arg != 0 {
        return Some(0); // 0x140DE
    }
    let bits = api.word(BITS) & 0xFE0F; // 0x140E0
    api.set_word(BITS, bits | 0x10); // 0x140E6
    if api.ambient_speakers() != 0 {
        // 0x140EB, 080Ch:1A39h; 0x140F0
        api.task_stop(1); // 0x140F6
    }
    api.scene_play(9); // 0x140FE
    api.input_lock(); // 0x14104
    api.set_word(GATE_SEG, 0); // 0x14109
    api.set_word(GATE_OFF, 0); // 0x1410F
    api.set_byte(STATE, 4); // 0x14115
    Some(0)
}

/// The Dino Zoo's rule handlers by their offset in segment `1407h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 1407:000F (0x1407F), Sprich mit 8Fh: scene 4 up to state 3
        // (`jna`), else 5.
        0x000F => {
            let n = if api.byte(STATE) > 3 { 5 } else { 4 }; // 0x14082
            api.scene_play(n); // 0x14092
            0x0A
        }
        // 1407:002D (0x1409D), Sprich mit 90h: scene 8 while [2BD9h] is 0,
        // else 7 with the Wagenheber carried and 6 without.
        0x002D => {
            let n = if api.byte(BYTE_2BD9) == 0 {
                8 // 0x140A7
            } else if api.inv_has(WAGENHEBER) != 0 {
                7 // 0x140B7, by 0x140AD
            } else {
                6 // 0x140BC
            };
            api.scene_play(n); // 0x140C0
            0x0A
        }
        // 1407:005B (0x140CB), Nimm 15h: state 5, and the search goes on.
        0x005B => {
            api.set_byte(STATE, 5); // 0x140CE
            0
        }
        // 1407:00AE (0x1411E), Wirf on 1Eh: the player walks to node 0FA0h
        // facing 0, the gate installed, answer 9.
        0x00AE => {
            api.set_word(WALK_TARGET, NODE_THROW); // 0x14126
            let player = api.ds(WALKER);
            api.walker_walk_to(player, NODE_THROW, 0); // 0x1412E, answer not read
            api.set_word(GATE_SEG, SEGMENT); // 0x14136
            api.set_word(GATE_OFF, GATE); // 0x1413C
            9
        }
        // 1407:00D7 (0x14147), Benutze on the Ball: the verb becomes Wirf
        // (the verb as chosen left alone), answer 5.
        0x00D7 => {
            api.set_word(VERB, VERB_WIRF); // 0x1414A
            5
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
