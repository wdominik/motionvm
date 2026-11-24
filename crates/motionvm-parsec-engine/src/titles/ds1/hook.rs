//! The far hook a location installs, `0EB9h:1968h` (image `0x104F8` to
//! its `retf` at `0x10868`).
//!
//! The entry function installs it at `[1E32h]` (`0x0EEC3`, `0CFDh:1B77h`),
//! and the scene machine calls it with a code, an argument and a task:
//! the code is bounded at 0Ah (`0x10506`, unsigned) and dispatched through
//! the eleven arms of the jump table at `cs:1D81h` (`0x10510`).
//!
//! | code | raised by | arm |
//! |---|---|---|
//! | 0 | a dialogue's speakers claimed (`0x0D16C`) | `0x10515`: the script words cleared |
//! | 1, 2, 3 | a text shown, a speaker's text shown, a menu up | `0x10859`: nothing of its own |
//! | 4 | a record's id | `0x10530`: nothing of its own |
//! | 5, 6 | a dialogue over, a task reset for another | `0x10533`: the figure faces where the script said |
//! | 7 | a `03 n` control | `0x10691`: the script events, 50 to 129 |
//! | 8 | a page about to be scanned (its slot, `0x0D1B7`), a menu about to go up (−1) | `0x1058F` |
//! | 9, 10 | a page's end, a task started | `0x105F2`: face, animate |
//!
//! A code past 0Ah goes straight to the hand-on (`0x1050B`). Every arm
//! ends at `0x10859`, which hands the call on to the location's own hook
//! — the far pointer its record keeps at `+2Fh`, held in `[1E5Ch]`
//! (`0x0EE6C`): it pushes the task, the argument and the code again and
//! calls far through `[1E5Ch]` (`0x1085E`). Codes 8, 9 and 10 with an
//! argument of 9 or more leave without it (`0x105E7`, `0x10686`, signed).
//! The original answers with the location hook's `AX`, which no caller
//! reads.
//!
//! So a script event is answered twice: first here, for what every
//! location does alike — facing, turning, leaving, a sample — and then by
//! the location, for what only it knows. The location's half is
//! [`super::logic`]'s, which hands it to the location's module.
//!
//! The figure is driven through three library routines. `0A9Fh:0C0Ch`
//! turns it to face a direction — the arms call its argument a node, and
//! it is a direction: `0x0B641` hands `dir + 3` to `02CFh`, which on the
//! figure's record is `01B9h`, the cel block by direction. `0A9Fh:10F7h`
//! starts one of two turn animations, and `0883h:20D9h` any animation.
//!
//! The figure's record is `DS:1693h` ([`WALKER`]); the hook reads its
//! state byte `+12h`, its phases byte `+15h`, its node id `+28h` and its
//! block `+2Ch`, and writes its kept facing `+2Eh`.

use super::logic::WALKER;
use crate::num;
use crate::titles::parsec1994::Api;

/// `[1E50h]`: the animation owed once a running one signals 11 or 12.
pub(crate) const W1E50: u16 = 0x1E50;
/// `[1E52h]`: an animation for the figure, −1 none, −2 pending.
const W1E52: u16 = 0x1E52;
/// `[1E54h]`: a direction to face once a text ends, −1 none.
const W1E54: u16 = 0x1E54;
/// `[1E56h]`: the animation events 100–104 and 112 owe.
const W1E56: u16 = 0x1E56;
/// `[1E58h]`: the direction the figure was last told to face.
const W1E58: u16 = 0x1E58;
/// `[1E5Ah]`: a turn pending, 1 or 2.
const W1E5A: u16 = 0x1E5A;
/// `[1ED0h]`: event 111's word, which asks the dispatcher to start task 0.
pub(crate) const W1ED0: u16 = 0x1ED0;
/// Byte `[16A5h]`, the record's `+12h`, the state: compared with 0 and
/// 0Ah (`0x1053E`, `0x10706`).
const STATE: u16 = 0x16A5;
/// Byte `[16A8h]`, the record's `+15h`, the phases: the divisor less one
/// (`0x10766`).
const PHASES: u16 = 0x16A8;
/// `[16BBh]`, the record's `+28h`, the node id: −1 while the figure is
/// not linked to a node (`0x105A1`).
const NODE: u16 = 0x16BB;
/// `[16BFh]`, the record's `+2Ch`, the block: the dividend (`0x1076D`).
const BLOCK: u16 = 0x16BF;
/// `[16C1h]`, the record's `+2Eh`, the facing kept: written by events
/// 90–93 when the figure is not linked (`0x106ED`).
const KEPT_FACING: u16 = 0x16C1;

/// The state byte's value while an animation runs on the figure.
const ANIMATING: u8 = 0x0A;

/// `0EB9h:1968h(code, arg, task)`.
pub(crate) fn answer(api: &mut dyn Api, code: u16, arg: u16, task: u16) {
    // 0x104FD: si = arg; 0x10500: di = task; 0x10506: `cmp bx, 0Ah ; jbe`.
    match code {
        0 => clear(api),               // 0x10515
        5 | 6 => codes_5_6(api, task), // 0x10533
        7 => script_event(api, arg),   // 0x10691
        8 => {
            code_8(api, arg, task); // 0x1058F
            if num::signed(arg) >= 9 {
                return; // 0x105E7: `cmp si, 9 ; jge` to 0x10865
            }
        }
        9 | 10 => {
            codes_9_10(api, arg); // 0x105F2
            if num::signed(arg) >= 9 {
                return; // 0x10686: `cmp si, 9 ; jge` to 0x10865
            }
        }
        _ => {} // 1–3 0x10859, 4 0x10530, past 0Ah 0x1050B
    }
    // 0x10859: `push di ; push si ; push [bp+6] ; call far [1E5Ch]`.
    api.location_hook(code, arg, task);
}

/// `0x10515`: code 0.
fn clear(api: &mut dyn Api) {
    api.set_word(W1E54, 0xFFFF); // 0x10515
    tail_051b(api);
}

/// `0x1051B`: the tail of arm 0, also the end of events 90–93.
fn tail_051b(api: &mut dyn Api) {
    api.set_word(W1E52, 0xFFFF); // 0x1051B
    api.set_word(W1E56, 0); // 0x10521
    api.set_word(W1E5A, 0); // 0x10527
}

/// `0A9Fh:0C0Ch(DS:1693h, dir)`, as `push dir ; push ds ; push 1693h`
/// builds it.
fn face(api: &mut dyn Api, dir: u16) {
    let rec = api.ds(WALKER);
    api.figure_face(rec, num::signed(dir));
}

/// `0x10545` and its like: the figure faces `[1E58h]`.
fn face_1e58(api: &mut dyn Api) {
    let dir = api.word(W1E58);
    face(api, dir);
}

/// `[1E56h] + (arg == 0)`: `or si, si ; jne` then `mov ax, 1` or
/// `xor ax, ax`, `add dx, ax` (`0x105C5`, `0x10637`, `0x1065A`).
fn plus_first(api: &dyn Api, arg: u16) -> u16 {
    api.word(W1E56).wrapping_add(u16::from(arg == 0))
}

/// `[16BFh] / ([16A8h] + 1)`: `mov al, [16A8h] ; mov ah, 0 ; inc ax`,
/// `cwd ; idiv bx` (`0x10766`, `0x10793`, `0x107B0`). The divisor is 1 to
/// 100h, so the quotient always fits.
fn facing(api: &dyn Api) -> u16 {
    let divisor = i16::from(api.byte(PHASES)) + 1;
    num::unsigned(num::signed(api.word(BLOCK)).wrapping_div(divisor))
}

/// `0x10533`: codes 5 and 6, after a dialogue the facing the text asked
/// for.
fn codes_5_6(api: &mut dyn Api, task: u16) {
    // 0x10533 task 0, 0x10537 a turn pending, 0x1053E an animation running.
    if task == 0 && api.word(W1E5A) != 0 && api.byte(STATE) == ANIMATING {
        face_1e58(api); // 0x1054D
        api.set_word(W1E5A, 0); // 0x10555
    } else if api.word(W1E56) != 0 {
        // 0x1055D
        face_1e58(api); // 0x1056C
    } else {
        api.set_word(W1E50, 0xFFFF); // 0x10586
        return;
    }
    api.set_word(W1E54, 0xFFFF); // 0x10574
    api.set_word(W1E52, 0xFFFF); // 0x1057A
    api.set_word(W1E56, 0); // 0x10580
    api.set_word(W1E50, 0xFFFF); // 0x10586
}

/// `0x1058F`: code 8, a page about to be scanned or a menu about to go
/// up. With a task other than 0 nothing is done (`0x10591`, to
/// `0x105E7`).
fn code_8(api: &mut dyn Api, arg: u16, task: u16) {
    if task != 0 {
        return; // 0x10591
    }
    // 0x10593 the second turn pending, 0x1059A an animation running.
    if api.word(W1E5A) == 2 && api.byte(STATE) == ANIMATING {
        if api.word(NODE) != 0xFFFF {
            face_1e58(api); // 0x105A1, 0x105B0
        }
        api.set_word(W1E5A, 0); // 0x105B8
    }
    if api.word(W1E56) == 0 {
        api.set_word(W1E52, 0xFFFF); // 0x105BE, 0x105E1
        return;
    }
    let v = plus_first(api, arg); // 0x105C5..0x105D4
    api.set_word(W1E52, v); // 0x105D6
    api.set_word(W1E50, v); // 0x105DC
}

/// `0x105F2`: codes 9 and 10, a page's end or a task started. Nothing
/// without the figure linked (`0x105F2`).
fn codes_9_10(api: &mut dyn Api, arg: u16) {
    if api.word(NODE) == 0xFFFF {
        return; // 0x105F9
    }
    // 0x105FC: `cmp [1E54h], 0 ; jl`.
    if num::signed(api.word(W1E54)) >= 0 {
        if api.word(W1E56) != 0 {
            // 0x10603
            let dir = api.word(W1E54);
            api.set_word(W1E58, dir); // 0x1060D
            face(api, dir); // 0x10615
        }
        api.set_word(W1E54, 0xFFFF); // 0x1061D
        api.set_word(W1E5A, 0); // 0x10623
    }
    // 0x10629, 0x10630.
    if api.word(W1E52) == 0xFFFE && api.word(W1E56) != 0 {
        let v = plus_first(api, arg);
        api.set_word(W1E52, v); // 0x10648
    }
    // 0x1064C, 0x10653.
    if api.word(W1E50) == 0xFFFE && api.word(W1E56) != 0 {
        let v = plus_first(api, arg);
        api.set_word(W1E50, v); // 0x1066B
    }
    // 0x1066F: `cmp [1E52h], 0 ; jl`.
    let k = api.word(W1E52);
    if num::signed(k) >= 0 {
        let rec = api.ds(WALKER);
        api.figure_anim(rec, num::signed(k)); // 0x1067E
    }
}

/// `0x10691`: code 7, a `03 n` control in a text.
///
/// Events start at 50 (`0x10693`), end at 129 (`0x10696`, unsigned), and
/// go through the eighty entries of the jump table at `cs:1CE1h`
/// (`0x106A0`; in the image at `0x10871`). A sample is played through
/// `0EB9h:1949h` (`push k ; push cs ; call 1949h`, `0x106A5` …).
///
/// | event | arm | what |
/// |---|---|---|
/// | 50, 51, 52, 56, 57, 59, 60–62 | `0x106A5` … | sample 5, 6, 7, 0Ah, 9, 0Bh, 12h |
/// | 55 | `0x106B6` | sample 8 |
/// | 90–93 | `0x106C9` | face direction `n − 90` |
/// | 94–97 | `0x106F6` | `[1E50h]` from `[1E52h]`, turn animation `n − 94`, pending 1 |
/// | 98, 99 | `0x10732` | turn animation `n − 98`, pending 2 |
/// | 100–103 | `0x10780` | after the text, face `n − 100`, animate `2(n − 100) + 2` |
/// | 104 | `0x10793` | the same, with the direction the figure faces |
/// | 105, 106, 107 | `0x107A6`, `0x107F9`, `0x10807` | animations 1, `0Ah`, `0Bh`, then sample 8 |
/// | 110 | `0x1081B` | animate `[1E56h]` |
/// | 111 | `0x1082A` | `[1ED0h] = 1`, arm task 0 |
/// | 112 | `0x10839` | after the text, face 0, animate 12 |
/// | 120–129 | `0x1084D` | leave through exit `n − 20` (`1586h:05E1h`) |
/// | the rest | `0x10859` | nothing here |
fn script_event(api: &mut dyn Api, n: u16) {
    match n {
        50 => api.play_sample(5),         // 0x106A5
        51 => api.play_sample(6),         // 0x106AE
        52 => api.play_sample(7),         // 0x106B2
        55 => api.play_sample(8),         // 0x106B6 -> 0x107F4
        56 => api.play_sample(0xA),       // 0x106B9
        57 => api.play_sample(9),         // 0x106BD
        59 => api.play_sample(0xB),       // 0x106C1
        60..=62 => api.play_sample(0x12), // 0x106C5
        90..=93 => {
            // 0x106D2 / 0x106EA: `add ax, FFA6h`.
            let dir = n.wrapping_sub(90);
            if api.word(NODE) != 0xFFFF {
                // 0x106C9
                api.set_word(W1E58, dir); // 0x106D5
                face(api, dir); // 0x106DD
            } else {
                api.set_word(KEPT_FACING, dir); // 0x106ED
                api.set_word(W1E58, dir); // 0x106F0
            }
            tail_051b(api); // 0x106E5 / 0x106F3
        }
        94..=97 => {
            if api.word(NODE) == 0xFFFF {
                return; // 0x106F6
            }
            let v = api.word(W1E52);
            api.set_word(W1E50, v); // 0x10703
            turn(api, n.wrapping_sub(94), 1); // 0x10706
        }
        98 | 99 => {
            if api.word(NODE) == 0xFFFF {
                return; // 0x10732
            }
            turn(api, n.wrapping_sub(98), 2); // 0x1073C
        }
        100..=103 => {
            let dir = n.wrapping_sub(100); // 0x10782: `add ax, FF9Ch`
            api.set_word(W1E54, dir); // 0x10785
            owe_anim(api, dir); // 0x10788
        }
        104 => {
            let dir = facing(api); // 0x10793
            api.set_word(W1E58, dir); // 0x107A1
            owe_anim(api, dir); // 0x107A4 -> 0x10788
        }
        105 => {
            if api.word(NODE) == 0xFFFF {
                return; // 0x107A6
            }
            let dir = facing(api); // 0x107B0
            api.set_word(W1E58, dir); // 0x107BE
            // 0x107C1: `cmp [1E54h], 0 ; jl`.
            let pending = api.word(W1E54);
            if num::signed(pending) >= 0 {
                api.set_word(W1E58, pending); // 0x107CB
            }
            let v = api.word(W1E52);
            api.set_word(W1E50, v); // 0x107D1
            // 0x107D4: `cmp bx, 3 ; ja`, then the table at `cs:1CD9h`.
            match api.word(W1E58) {
                0 => api.set_word(W1E52, 1),       // 0x107E4
                1 => api.set_word(W1E52, 0xA),     // 0x107EC -> 0x10813
                2 | 3 => api.set_word(W1E52, 0xB), // 0x107EE
                _ => {}                            // 0x107DB
            }
            api.play_sample(8); // 0x107F4
        }
        106 => {
            let v = api.word(W1E52);
            api.set_word(W1E50, v); // 0x107FC
            api.set_word(W1E54, 2); // 0x107FF
            api.set_word(W1E52, 0xB); // 0x107EE
            api.play_sample(8); // 0x107F4
        }
        107 => {
            let v = api.word(W1E52);
            api.set_word(W1E50, v); // 0x1080A
            api.set_word(W1E54, 1); // 0x1080D
            api.set_word(W1E52, 0xA); // 0x10813
            api.play_sample(8); // 0x107F4
        }
        110 => {
            let v = api.word(W1E56);
            if v != 0 {
                api.set_word(W1E52, v); // 0x1081B, 0x10825
            }
        }
        111 => {
            api.set_word(W1ED0, 1); // 0x1082A
            api.task_arm(0); // 0x10832
        }
        112 => {
            api.set_word(W1E54, 0); // 0x10839
            api.set_word(W1E56, 0xC); // 0x1083F
            api.set_word(W1E52, 0xFFFE); // 0x10845
        }
        // 0x1084F: `add ax, FFECh`; `1586h:05E1h(n − 20)`, the door chosen,
        // the machine into mode 7 with the script exit's gate.
        120..=129 => api.script_exit(n.wrapping_sub(20)), // 0x10853
        _ => {} // 0x1069B, and the table's entries for 0x10859
    }
}

/// `0x10706` / `0x1073C`: events 94–99 once the figure is linked. Byte
/// `[16A5h]` must be 0 or 0Ah; `0A9Fh:10F7h(k)` must answer non-zero
/// (`0x10723`, `0x10759`); then `[1E5Ah]` takes `pending` (`0x1072A`,
/// `0x10760`), `[1E58h]` the facing (`0x10774`), `[1E52h]` −1
/// (`0x10777`).
fn turn(api: &mut dyn Api, k: u16, pending: u16) {
    let s = api.byte(STATE);
    if s != 0 && s != ANIMATING {
        return; // 0x10714 / 0x1074A
    }
    if api.figure_turn(num::signed(k)) == 0 {
        return; // 0x10727 / 0x1075D
    }
    api.set_word(W1E5A, pending);
    let dir = facing(api);
    api.set_word(W1E58, dir);
    api.set_word(W1E52, 0xFFFF);
}

/// `0x10788`: `[1E56h] = 2 dir + 2` (`shl ax, 1 ; add ax, 2`), then
/// `0x10845`: `[1E52h] = −2`.
fn owe_anim(api: &mut dyn Api, dir: u16) {
    api.set_word(W1E56, (dir << 1).wrapping_add(2)); // 0x1078D
    api.set_word(W1E52, 0xFFFE); // 0x10845
}
