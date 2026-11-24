//! The shared far hook, `0A01h:18F6h` (image `0x0B906` to its `retf` at
//! `0x0BCC7`).
//!
//! The scene machine calls it with a code, an argument and a task. It does
//! what every location does alike, then hands the call on to the
//! location's own hook, the far pointer in `[18CEh]:[18D0h]` (`0x0BCBD`).
//!
//! The code is bounded at 9 (`0x0B915`, unsigned) and dispatched through
//! the ten arms of the jump table at `cs:1D34h` (`0x0B91F`):
//!
//! | code | arm |
//! |---|---|
//! | 0 | `0x0B924`: the six words cleared |
//! | 1, 2, 3 | `0x0BCB8`: nothing of its own |
//! | 4 | `0x0B93F`: nothing of its own |
//! | 5, 6 | `0x0B942` |
//! | 7 | `0x0BA94`: a script event, through the search table at `cs:1CC0h` |
//! | 8 | `0x0B99E` |
//! | 9 | `0x0B9FA` |
//!
//! Every path ends in the hand-on at `0x0BCB8`; no arm leaves early. The
//! original answers with the location hook's `AX` (`leave ; retf` keep it).
//!
//! The words at `DS:18C2h`…`DS:18CCh` are read and written only as this
//! listing does; their names say no more than their address.
//!
//! Four instructions of the range are never reached: each is a `jmp` that
//! follows an unconditional `jmp`, and no branch and no entry of the three
//! tables (`cs:1CB8h`, `cs:1CFAh`, `cs:1D34h`) names it: `0x0B9F7`,
//! `0x0BBFD`, `0x0BC5A`, `0x0BC7A`.

use super::logic::{WALKER, WALKER_28};
use crate::num;
use crate::titles::parsec1994::Api;

/// The first of the hook's words, the row
/// [`crate::titles::parsec1994::DsMap::script`] starts at: the code-0 arm
/// puts `[18C6h]` and `[18C4h]` to `FFFFh` and clears `[18C8h]` and
/// `[18CCh]` (`0x0B924`–`0x0B936`).
pub(crate) const W18C2: u16 = 0x18C2;
const W18C4: u16 = 0x18C4;
const W18C6: u16 = 0x18C6;
const W18C8: u16 = 0x18C8;
const W18CA: u16 = 0x18CA;
const W18CC: u16 = 0x18CC;
/// `[1CE8h]`: the object events 200 to 202 animate (`0x0BC43`).
const OBJECT_1CE8: u16 = 0x1CE8;
/// Byte `[11EEh]`, the walker record's `+12h`, compared with 0 and 0Ah
/// (`0x0B94D`, `0x0BB0E`).
const WALKER_12: u16 = 0x11EE;
/// Byte `[11F1h]`, the walker record's `+15h`, the divisor less one
/// (`0x0BB6E`).
const WALKER_15: u16 = 0x11F1;
/// `[1208h]`, the walker record's `+2Ch`, the dividend (`0x0BB75`).
const WALKER_2C: u16 = 0x1208;
/// `[120Ah]`, the walker record's `+2Eh`, written by events 90–93 when
/// `[1204h]` is −1 (`0x0BAF5`).
const WALKER_2E: u16 = 0x120A;

/// `0A01h:18F6h(code, arg, task)`.
pub(crate) fn answer(api: &mut dyn Api, code: u16, arg: u16, task: u16) {
    // 0x0B90C: si = arg; 0x0B90F: di = task; 0x0B915: `cmp bx, 9 ; jbe`.
    match code {
        0 => clear(api),                    // 0x0B924
        1..=3 => {}                         // 0x0BCB8
        4 => {}                             // 0x0B93F
        5 | 6 => codes_5_6(api, arg, task), // 0x0B942
        7 => script_event(api, arg),        // 0x0BA94
        8 => code_8(api, arg, task),        // 0x0B99E
        9 => code_9(api, arg),              // 0x0B9FA
        _ => {}                             // 0x0B91A
    }
    forward(api, code, arg, task);
}

/// `0x0BCB8`: `push di ; push si ; push [bp+6] ; call far [18CEh]`.
fn forward(api: &mut dyn Api, code: u16, arg: u16, task: u16) {
    api.location_hook(code, arg, task); // 0x0BCBD
}

/// `0x0B924`: code 0.
fn clear(api: &mut dyn Api) {
    api.set_word(W18C6, 0xFFFF); // 0x0B924
    tail_191a(api);
}

/// `0x0B92A`: the tail of arm 0, also the end of events 90–93.
fn tail_191a(api: &mut dyn Api) {
    api.set_word(W18C4, 0xFFFF); // 0x0B92A
    api.set_word(W18C8, 0); // 0x0B930
    api.set_word(W18CC, 0); // 0x0B936
}

/// `05F2h:0BC0h(DS:11DCh, [18CAh])`, as `push [18CAh] ; push ds ; push 11DCh`
/// builds it.
fn face_18ca(api: &mut dyn Api) {
    let rec = api.ds(WALKER);
    let dir = api.word(W18CA);
    api.figure_face(rec, num::signed(dir));
}

/// `[18C8h] + (arg == 0)`: `or si, si ; jne` then `mov ax, 1` or
/// `xor ax, ax`, `add dx, ax` (`0x0B9CA`, `0x0BA3F`, `0x0BA62`).
fn plus_first(api: &dyn Api, arg: u16) -> u16 {
    api.word(W18C8).wrapping_add(u16::from(arg == 0))
}

/// `0x0BB6E`: `[1208h] / ([11F1h] + 1)`, `cwd ; idiv bx`, into `[18CAh]`.
fn facing(api: &mut dyn Api) -> u16 {
    let divisor = i16::from(api.byte(WALKER_15)) + 1; // 0x0BB6E..0x0BB73
    let q = num::signed(api.word(WALKER_2C)) / divisor; // 0x0BB75..0x0BB7A
    let q = num::unsigned(q);
    api.set_word(W18CA, q); // 0x0BB7C
    q
}

/// `0x0B942`: codes 5 and 6.
fn codes_5_6(api: &mut dyn Api, _arg: u16, task: u16) {
    // 0x0B942 task 0, 0x0B946 [18CCh] set, 0x0B94D byte [11EEh] = 0Ah.
    if task == 0 && api.word(W18CC) != 0 && api.byte(WALKER_12) == 0x0A {
        face_18ca(api); // 0x0B95C
        api.set_word(W18CC, 0); // 0x0B964
    } else if api.word(W18C8) != 0 {
        // 0x0B96C
        face_18ca(api); // 0x0B97B
        api.set_word(W18C6, 0xFFFF); // 0x0B983
        api.set_word(W18C4, 0xFFFF); // 0x0B989
        api.set_word(W18C8, 0); // 0x0B98F
    }
    api.set_word(W18C2, 0xFFFF); // 0x0B995
}

/// `0x0B99E`: code 8.
fn code_8(api: &mut dyn Api, arg: u16, task: u16) {
    if task != 0 {
        return; // 0x0B9A0
    }
    // 0x0B9A5 [18CCh] = 2, 0x0B9AC byte [11EEh] = 0Ah.
    if api.word(W18CC) == 2 && api.byte(WALKER_12) == 0x0A {
        if api.word(WALKER_28) != 0xFFFF {
            face_18ca(api); // 0x0B9B3, 0x0B9C2
        }
        api.set_word(W18CC, 0); // 0x0B9CA
    }
    if api.word(W18C8) == 0 {
        api.set_word(W18C4, 0xFFFF); // 0x0B9D5 -> 0x0BB7F
        return;
    }
    let v = plus_first(api, arg); // 0x0B9DA..0x0B9E9
    api.set_word(W18C4, v); // 0x0B9EB
    api.set_word(W18C2, v); // 0x0B9F1
}

/// `0x0B9FA`: code 9.
fn code_9(api: &mut dyn Api, arg: u16) {
    if api.word(WALKER_28) == 0xFFFF {
        return; // 0x0B9FA
    }
    // 0x0BA04: `cmp [18C6h], 0 ; jl`.
    if num::signed(api.word(W18C6)) >= 0 {
        if api.word(W18C8) != 0 {
            // 0x0BA0B
            let v = api.word(W18C6);
            api.set_word(W18CA, v); // 0x0BA15
            face_18ca(api); // 0x0BA1D
        }
        api.set_word(W18C6, 0xFFFF); // 0x0BA25
        api.set_word(W18CC, 0); // 0x0BA2B
    }
    // 0x0BA31, 0x0BA38.
    if api.word(W18C4) == 0xFFFE && api.word(W18C8) != 0 {
        let v = plus_first(api, arg);
        api.set_word(W18C4, v); // 0x0BA50
    }
    // 0x0BA54, 0x0BA5B.
    if api.word(W18C2) == 0xFFFE && api.word(W18C8) != 0 {
        let v = plus_first(api, arg);
        api.set_word(W18C2, v); // 0x0BA73
    }
    // 0x0BA77: `cmp [18C4h], 0 ; jge`.
    if num::signed(api.word(W18C4)) >= 0 {
        let rec = api.ds(WALKER);
        let k = api.word(W18C4);
        api.figure_anim(rec, num::signed(k)); // 0x0BA89
    }
}

/// `0x0BA94`: code 7. The argument is looked for among the 1Dh words at
/// `cs:1CC0h` (`0x0BA97`..`0x0BAA8`) and its arm taken from the word 3Ah
/// further on (`0x0BAAD`); a word not there is no event here.
///
/// | event | arm |
/// |---|---|
/// | 50, 51, 52, 56, 57, 59 | `0x0BAB1` … : sample 4, 5, 6, 9, 8, 0Ah |
/// | 55 | `0x0BAC2`: sample 7 |
/// | 90–93 | `0x0BAD1` |
/// | 94–97 | `0x0BAFE` |
/// | 98, 99 | `0x0BB3A` |
/// | 100–103 | `0x0BB88` |
/// | 104 | `0x0BB92` |
/// | 105 | `0x0BBB4` |
/// | 106, 107 | `0x0BBFF`, `0x0BC13` |
/// | 110 | `0x0BC2A` |
/// | 200, 201, 202 | `0x0BC3C`, `0x0BC5C`, `0x0BC7C` |
fn script_event(api: &mut dyn Api, n: u16) {
    match n {
        50 => api.play_sample(4),   // 0x0BAB1
        51 => api.play_sample(5),   // 0x0BABA
        52 => api.play_sample(6),   // 0x0BABE
        55 => api.play_sample(7),   // 0x0BAC2 -> 0x0BC25
        56 => api.play_sample(9),   // 0x0BAC5
        57 => api.play_sample(8),   // 0x0BAC9
        59 => api.play_sample(0xA), // 0x0BACD
        90..=93 => {
            let dir = n.wrapping_sub(0x5A); // 0x0BAD8 / 0x0BAF0: `add ax, FFA6h`
            if api.word(WALKER_28) != 0xFFFF {
                // 0x0BAD1
                api.set_word(W18CA, dir); // 0x0BADD
                let rec = api.ds(WALKER);
                api.figure_face(rec, num::signed(dir)); // 0x0BAE5
            } else {
                api.set_word(WALKER_2E, dir); // 0x0BAF5
                api.set_word(W18CA, dir); // 0x0BAF8
            }
            tail_191a(api); // 0x0BAED / 0x0BAFB
        }
        94..=97 => {
            if api.word(WALKER_28) == 0xFFFF {
                return; // 0x0BAFE
            }
            let v = api.word(W18C4);
            api.set_word(W18C2, v); // 0x0BB0B
            turn(api, n.wrapping_sub(0x5E), 1); // 0x0BB0E
        }
        98 | 99 => {
            if api.word(WALKER_28) == 0xFFFF {
                return; // 0x0BB3A
            }
            turn(api, n.wrapping_sub(0x62), 2); // 0x0BB44
        }
        100..=103 => {
            let dir = n.wrapping_sub(100); // 0x0BB88: `add ax, FF9Ch`
            api.set_word(W18C6, dir); // 0x0BB8D
            owe_anim(api, dir); // 0x0BB90 -> 0x0BBA3
        }
        104 => {
            let dir = facing(api); // 0x0BB92
            owe_anim(api, dir); // 0x0BBA3
        }
        105 => {
            if api.word(WALKER_28) == 0xFFFF {
                return; // 0x0BBB4
            }
            let dir = facing(api); // 0x0BBBE
            let v = api.word(W18C4);
            api.set_word(W18C2, v); // 0x0BBD2
            // 0x0BBD9: `cmp bx, 3 ; ja`, then the table at `cs:1CB8h`.
            match dir {
                0 => api.set_word(W18C4, 1),       // 0x0BBE5
                1 => api.set_word(W18C4, 0xA),     // 0x0BBED
                2 | 3 => api.set_word(W18C4, 0xB), // 0x0BBF5
                _ => {}
            }
            api.play_sample(7); // 0x0BC25
        }
        106 => {
            let v = api.word(W18C4);
            api.set_word(W18C2, v); // 0x0BC02
            api.set_word(W18C6, 2); // 0x0BC05
            api.set_word(W18C4, 0xB); // 0x0BC0B
            api.play_sample(7); // 0x0BC25
        }
        107 => {
            let v = api.word(W18C4);
            api.set_word(W18C2, v); // 0x0BC16
            api.set_word(W18C6, 1); // 0x0BC19
            api.set_word(W18C4, 0xA); // 0x0BC1F
            api.play_sample(7); // 0x0BC25
        }
        110 => {
            let v = api.word(W18C8);
            if v != 0 {
                api.set_word(W18C4, v); // 0x0BC2A, 0x0BC37
            }
        }
        200..=202 => object_events(api, n),
        _ => {} // 0x0BAAA
    }
}

/// `0x0BB0E` / `0x0BB44`: events 94–99 once the walker is linked. Byte
/// `[11EEh]` must be 0 or 0Ah; `05F2h:1078h(k)` must answer non-zero; then
/// `[18CCh]` takes `owed`, `[18CAh]` the facing, `[18C4h]` −1.
fn turn(api: &mut dyn Api, k: u16, owed: u16) {
    let s = api.byte(WALKER_12);
    if s != 0 && s != 0x0A {
        return; // 0x0BB0E, 0x0BB15
    }
    if api.figure_turn(num::signed(k)) == 0 {
        return; // 0x0BB25, 0x0BB2B
    }
    api.set_word(W18CC, owed); // 0x0BB32 / 0x0BB68
    facing(api); // 0x0BB6E
    api.set_word(W18C4, 0xFFFF); // 0x0BB7F
}

/// `0x0BBA3`: `[18C8h] = 2 dir + 2`, `[18C4h] = −2`.
fn owe_anim(api: &mut dyn Api, dir: u16) {
    api.set_word(W18C8, (dir << 1).wrapping_add(2)); // 0x0BBA3..0x0BBA8
    api.set_word(W18C4, 0xFFFE); // 0x0BBAB
}

/// `0x0BC3C`, `0x0BC5C`, `0x0BC7C`: events 200 to 202, a sample each and
/// the object `[1CE8h]` names on an animation, the band held for owner 8
/// (`0x0BCB0`) after all three.
fn object_events(api: &mut dyn Api, n: u16) {
    match n {
        200 => {
            api.play_sample(0xB); // 0x0BC3F
            let id = api.word(OBJECT_1CE8);
            if id != 0 {
                api.obj_anim_id(id, 0); // 0x0BC43, 0x0BC50
            }
            api.band_hold(8); // 0x0BCB0
        }
        201 => {
            api.play_sample(0xC); // 0x0BC5F
            let id = api.word(OBJECT_1CE8);
            if id != 0 {
                api.obj_anim_id(id, 1); // 0x0BC63, 0x0BC70
            }
            api.band_hold(8); // 0x0BCB0
        }
        202 => {
            api.play_sample(0xD); // 0x0BC7F
            let id = api.word(OBJECT_1CE8);
            if id != 0 {
                // 0x0BC83, 0x0BC8E: `call far [1980h]`, then `or ax, ax`,
                // `cmp ax, 1`.
                let r = api.callback(id);
                // 0x0BCA4: `push word [1CE8h]` read again after the call.
                let id = api.word(OBJECT_1CE8);
                match r {
                    0 => api.obj_anim_id(id, 2), // 0x0BC9E, 0x0BCA8
                    1 => api.obj_anim_id(id, 3), // 0x0BCA2, 0x0BCA8
                    _ => {}                      // 0x0BC9C
                }
            }
            api.band_hold(8); // 0x0BCB0
        }
        _ => {}
    }
}
