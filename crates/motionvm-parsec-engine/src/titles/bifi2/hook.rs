//! The shared far hook, `09B6h:186Ch` (image `0x0B3CC` to its `retf` at
//! `0x0B566`), registered by `push 9B6h ; push 186Ch ; call far
//! 080Ch:1A5Bh` (`0x09E87`).
//!
//! The scene machine calls it with a code, an argument and a task. Only
//! codes 4 to 8 have arms of their own (`sub bx, 4 ; cmp bx, 4 ; ja`,
//! `0x0B3DA`, through the table at `cs:1A6Bh`); every path, and every other
//! code, ends at the hand-on (`0x0B557`), which calls the location's own
//! hook through `[1ECEh]` with the code, the argument and the task again
//! (`0x0B55C`).
//!
//! | code | arm |
//! |---|---|
//! | 4 | `0x0B3EC`: an item taken or given by the argument |
//! | 5, 6 | `0x0B418`: the figure faces where a script asked |
//! | 7 | `0x0B47C`: the script events, 50 to 99 (`cs:1A07h`) |
//! | 8 | `0x0B446`: the same as 5, for a turn asked with 98 or 99 |
//!
//! The hook keeps two words: `[1ECAh]`, a direction the figure is to face,
//! and `[1ECCh]`, what it waits for (1 after a turn asked with 94 to 97, 2
//! after one asked with 98 or 99). It reads the figure's record `DS:176Eh`
//! ([`WALKER`]): its state byte `+12h`, its phases `+15h`, its node `+28h`
//! and its block `+2Ch`, and writes its kept facing `+2Eh`.
//!
//! The figure's facing is `05DCh:0B56h(rec, dir)` and its turn
//! `05DCh:0FF2h(k)`, Dunkle Schatten's `0A9Fh:0C0Ch` and `0A9Fh:10F7h`
//! ([`Api::figure_face`], [`Api::figure_turn`]); a sample is `09B6h:184Dh`,
//! Dunkle Schatten's `0EB9h:1949h` ([`Api::play_sample`]).

use super::build::WALKER;
use crate::num;
use crate::titles::parsec1994::Api;

/// `[1ECAh]`: the direction the figure is to face.
const FACING: u16 = 0x1ECA;
/// `[1ECCh]`: what the figure waits for, 0 nothing.
const WAITING: u16 = 0x1ECC;
/// The figure record's state byte `+12h`, phases `+15h`, node `+28h`,
/// block `+2Ch` and kept facing `+2Eh`.
const STATE: u16 = WALKER + 0x12;
const PHASES: u16 = WALKER + 0x15;
const NODE: u16 = WALKER + 0x28;
const BLOCK: u16 = WALKER + 0x2C;
const KEPT_FACING: u16 = WALKER + 0x2E;

/// The figure's state while an animation runs on it (`cmp byte [1780h],
/// 0Ah`).
const ANIMATING: u8 = 0x0A;

pub(crate) fn answer(api: &mut dyn Api, code: u16, arg: u16, task: u16) {
    match code {
        4 => items(api, arg),                    // 0x0B3EC
        5 | 6 => face_waiting(api, task, false), // 0x0B418
        7 => script_event(api, arg),             // 0x0B47C
        8 => face_waiting(api, task, true),      // 0x0B446
        _ => {}
    }
    api.location_hook(code, arg, task); // 0x0B557
}

/// Code 4: argument 0Ah takes item 9, 0Bh gives item 0Ah, 0Ch takes item
/// 0Ah (`0x0B3EE`–`0x0B416`).
fn items(api: &mut dyn Api, arg: u16) {
    match arg {
        0x0A => {
            api.inv_take(9);
        }
        0x0B => {
            api.inv_give(0x0A);
        }
        0x0C => {
            api.inv_take(0x0A);
        }
        _ => {}
    }
}

/// Codes 5, 6 and 8: for task 0, with the figure animating, the facing a
/// script asked for — code 5 and 6 for any wait (`cmp word [1ECCh], 0 ;
/// jne`, `0x0B41F`), code 8 for the wait after 98 or 99 alone (`cmp word
/// [1ECCh], 2`, `0x0B44D`) and only on a node (`0x0B461`) — and the wait
/// over (`0x0B4E6`).
fn face_waiting(api: &mut dyn Api, task: u16, code_8: bool) {
    let waiting = api.word(WAITING);
    let waits = if code_8 { waiting == 2 } else { waiting != 0 };
    if task != 0 || !waits || api.byte(STATE) != ANIMATING {
        return;
    }
    if !code_8 || api.word(NODE) != 0xFFFF {
        face(api); // 0x0B43B, 0x0B470
    }
    api.set_word(WAITING, 0); // 0x0B4E6
}

/// `05DCh:0B56h(176Eh, [1ECAh])` (`0x0B433`–`0x0B43B`).
fn face(api: &mut dyn Api) {
    let rec = api.ds(WALKER);
    let dir = api.word(FACING);
    api.figure_face(rec, num::signed(dir));
}

/// Code 7, `03 n`: through the table at `cs:1A07h` for `n − 32h` up to 31h
/// (`0x0B47E`–`0x0B48B`).
fn script_event(api: &mut dyn Api, n: u16) {
    match n {
        // 0x0B490–0x0B4B7: a sample each.
        50..=59 => {
            const SAMPLES: [u16; 10] = [5, 6, 7, 8, 9, 0x0A, 0x0C, 0x0B, 0x0D, 0x0E];
            api.play_sample(SAMPLES[usize::from(n - 50)]);
        }
        // 0x0B4BD: the figure faces `n − 5Ah`, or keeps it for when it is
        // on a node again.
        90..=93 => {
            let dir = n.wrapping_sub(0x5A);
            api.set_word(FACING, dir); // 0x0B4C9, 0x0B4E3
            if api.word(NODE) != 0xFFFF {
                face(api); // 0x0B4D1
            } else {
                api.set_word(KEPT_FACING, dir); // 0x0B4E0
            }
            api.set_word(WAITING, 0); // 0x0B4E6
        }
        // 0x0B4EE, 0x0B51B: a turn, `n − 5Eh` or `n − 62h`, on a figure on
        // a node and still or animating; when it starts, the wait it asks
        // for and the facing the block gives.
        94..=99 => {
            let state = api.byte(STATE);
            if api.word(NODE) == 0xFFFF || (state != 0 && state != ANIMATING) {
                return;
            }
            let (k, wait) = if n <= 97 {
                (n.wrapping_sub(0x5E), 1)
            } else {
                (n.wrapping_sub(0x62), 2)
            };
            if api.figure_turn(num::signed(k)) == 0 {
                return;
            }
            api.set_word(WAITING, wait); // 0x0B513, 0x0B540
            let divisor = i16::from(api.byte(PHASES)) + 1;
            let facing = num::signed(api.word(BLOCK)).wrapping_div(divisor);
            api.set_word(FACING, num::unsigned(facing)); // 0x0B554
        }
        _ => {}
    }
}
