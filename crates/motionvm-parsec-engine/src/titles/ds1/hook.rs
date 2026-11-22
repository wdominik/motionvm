//! The far hook a location installs, `0EB9h:1968h`.
//!
//! The entry function installs it at `[1E32h]` (`0x0EEC3`, `0CFDh:1B77h`),
//! and the scene machine calls it with a code, an argument and a task:
//! the code is bounded at 0Ah (`0x10506`) and dispatched through eleven arms
//! of the jump table at `cs:1D81h` (`0x10510`).
//!
//! | code | raised by | arm |
//! |---|---|---|
//! | 0 | a dialogue's speakers claimed (`0x0D16C`) | `0x10515`: the script globals cleared |
//! | 1, 2, 3, 4 | a text shown, a speaker's text shown, a menu up, a record's id | `0x10859`: nothing of its own |
//! | 5, 6 | a dialogue over, a task reset for another | `0x10533`: the figure faces where the script said |
//! | 7 | a `03 n` control | `0x10691`: the script events, 50 to 129 |
//! | 8 | a page about to be scanned (its slot, `0x0D1B7`), a menu about to go up (−1) | `0x1058F` |
//! | 9, 10 | a page's end, a task started | `0x105F2`: face, animate |
//!
//! Every arm but a few tails ends at `0x10859`, which hands the call on to
//! the location's own hook — the far pointer its record keeps at `+2Fh`,
//! held in `[1E5Ch]` (`0x0EE6C`):
//! `0x10859` pushes the task, the argument and the code again and calls far
//! through `[1E5Ch]`.
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

use crate::num;
use crate::titles::parsec1994::Adventure;

/// `0EB9h:1968h(code, arg, task)`.
pub(crate) fn answer(adv: &mut Adventure, code: u16, arg: i16, task: usize) {
    match code {
        0 => {
            // `0x10515`.
            adv.script.g1e54 = -1;
            adv.script.g1e52 = -1;
            adv.script.g1e56 = 0;
            adv.script.g1e5a = 0;
        }
        5 | 6 => {
            // `0x10533`: after a dialogue, the facing the text asked for.
            //
            // With a turn pending (`[1E5Ah]`, `0x10537`) and its animation
            // running (`[16A5h]` = 0Ah, `0x1053E`) the figure faces `[1E58h]`
            // through `0A9Fh:0C0Ch` (`0x1054D`); else, with an animation set
            // in `[1E56h]` (`0x1055D`), it does the same (`0x1056C`).
            if task == 0 && adv.script.g1e5a != 0 && adv.player_animating() {
                let dir = adv.script.g1e58;
                adv.face_player(dir);
                adv.script.g1e5a = 0;
            } else if adv.script.g1e56 != 0 {
                let dir = adv.script.g1e58;
                adv.face_player(dir);
            } else {
                // `0x10586`: without either only `[1E50h]` is cleared.
                adv.script.g1e50 = -1;
                return forward(adv, code, arg, task);
            }
            adv.script.g1e54 = -1;
            adv.script.g1e52 = -1;
            adv.script.g1e56 = 0;
            adv.script.g1e50 = -1;
        }
        7 => {
            script_event(adv, arg, task);
            return forward(adv, code, arg, task);
        }
        8 => {
            // `0x1058F`: a menu is going up.
            if task == 0 && adv.script.g1e5a == 2 && adv.player_animating() {
                if adv.player_linked() {
                    let dir = adv.script.g1e58;
                    adv.face_player(dir);
                }
                adv.script.g1e5a = 0;
            }
            if adv.script.g1e56 != 0 {
                let v = adv.script.g1e56 + i16::from(arg == 0);
                adv.script.g1e52 = v;
                adv.script.g1e50 = v;
            } else {
                adv.script.g1e52 = -1;
            }
            // `0x105E7`: an argument of 9 or more leaves without the
            // location hearing of it.
            if arg >= 9 {
                return;
            }
        }
        9 | 10 => {
            // `0x105F2`.
            //
            // With the figure linked (`[16BBh]` not −1, `0x105F2`), a
            // direction set by events 100–103 in `[1E54h]` (`0x105FC`) is
            // faced through `0A9Fh:0C0Ch` (`0x10615`), and a −2 in `[1E52h]`
            // becomes the animation (`0x10629`).
            if adv.player_linked() {
                if adv.script.g1e54 >= 0 {
                    if adv.script.g1e56 != 0 {
                        adv.script.g1e58 = adv.script.g1e54;
                        let dir = adv.script.g1e58;
                        adv.face_player(dir);
                    }
                    adv.script.g1e54 = -1;
                    adv.script.g1e5a = 0;
                }
                if adv.script.g1e52 == -2 && adv.script.g1e56 != 0 {
                    adv.script.g1e52 = adv.script.g1e56 + i16::from(arg == 0);
                }
                if adv.script.g1e50 == -2 && adv.script.g1e56 != 0 {
                    adv.script.g1e50 = adv.script.g1e56 + i16::from(arg == 0);
                }
                if adv.script.g1e52 >= 0 {
                    let k = adv.script.g1e52;
                    adv.anim_player(k);
                }
            }
            if arg >= 9 {
                return;
            }
        }
        _ => {}
    }
    forward(adv, code, arg, task);
}

/// `0x10859`: the call handed on to the location's own hook at `[1E5Ch]`.
fn forward(adv: &mut Adventure, code: u16, arg: i16, task: usize) {
    adv.events
        .push(crate::titles::parsec1994::Event::LocationHook { code, arg, task });
    // `0x10859`: the call handed on to the location's own hook, at once.
    let hook = adv.location_hook;
    hook(adv, code, arg, task);
}

/// `0x10691`: the arm for code 7, a `03 n` control in a text.
///
/// Events start at 50 (`0x10693`), end at 129 (`0x10696`), and go through the
/// eighty entries of the jump table at `cs:1CE1h` (`0x106A0`).
///
/// The table, entry by entry, as the image has it at `0x10871`:
///
/// | event | arm | what |
/// |---|---|---|
/// | 50, 51, 52, 56, 57, 59, 60–62 | `0x106A5` … | play sample 5, 6, 7, 10, 9, 11, 18 (`0EB9h:1949h`) |
/// | 55 | `0x106B6` | sample 8 |
/// | 90–93 | `0x106C9` | face direction `n − 90` |
/// | 94–97, 98, 99 | `0x106F6`, `0x10732` | turn animation (`0A9Fh:10F7h`), pending 1 or 2 |
/// | 100–103 | `0x10780` | after the text, face `n − 100`, animate `2(n − 100) + 2` |
/// | 104 | `0x10793` | the same, with the direction the figure faces |
/// | 105, 106, 107 | `0x107A6`, `0x107F9`, `0x10807` | animations 1, `0Ah`, `0Bh`, then sample 8 |
/// | 110 | `0x1081B` | animate `[1E56h]` |
/// | 111 | `0x1082A` | `[1ED0h] = 1`, arm task 0 |
/// | 112 | `0x10839` | after the text, face 0, animate 12 |
/// | 120–129 | `0x1084D` | leave through exit `n − 20` (`1586h:05E1h`) |
/// | the rest | `0x10859` | nothing here |
fn script_event(adv: &mut Adventure, n: i16, task: usize) {
    let _ = task;
    match n {
        50 => sample(adv, 5),
        51 => sample(adv, 6),
        52 => sample(adv, 7),
        55 => sample(adv, 8),
        56 => sample(adv, 10),
        57 => sample(adv, 9),
        59 => sample(adv, 11),
        60..=62 => sample(adv, 18),
        90..=93 => {
            // `0x106D2`: the direction is `n − 90`; it is faced through
            // `0A9Fh:0C0Ch` when the figure is linked (`0x106DD`) and kept in
            // the record's `+2Eh`, `[16C1h]`, when it is not (`0x106ED`).
            let dir = n - 90;
            adv.script.g1e58 = dir;
            if adv.player_linked() {
                adv.face_player(dir);
            } else if let Some(w) = &mut adv.walker {
                w.final_facing = dir;
            }
            // `0x106E5`: on to the tail of arm 0.
            adv.script.g1e52 = -1;
            adv.script.g1e56 = 0;
            adv.script.g1e5a = 0;
        }
        94..=99 => {
            // Only with the figure linked (`0x106F6`): `[1E50h]` takes
            // `[1E52h]` (`0x10700`), and a standing or animating figure
            // (`[16A5h]`, `0x10706`) is turned through `0A9Fh:10F7h` by
            // `n − 94` for 94–97 (`0x10719`, `0x1071D`) with `[1E5Ah]` = 1
            // (`0x1072A`), or by `n − 98` for 98 and 99 (`0x1074F`) with
            // `[1E5Ah]` = 2 (`0x10760`); `[1E58h]` then takes the facing out
            // of `[16BFh]` (`0x10766`).
            if !adv.player_linked() {
                return;
            }
            adv.script.g1e50 = adv.script.g1e52;
            let (k, pending) = if n <= 97 { (n - 94, 1) } else { (n - 98, 2) };
            if adv.turn_player(k) == 0 {
                return;
            }
            adv.script.g1e5a = pending;
            adv.script.g1e58 = adv.player_facing();
            adv.script.g1e52 = -1;
        }
        100..=103 => {
            let dir = n - 100;
            adv.script.g1e54 = dir;
            adv.script.g1e56 = 2 * dir + 2;
            adv.script.g1e52 = -2;
        }
        104 => {
            // `0x10793`: `[1E58h] = [16BFh] / ([16A8h] + 1)`, the facing.
            let dir = adv.player_facing();
            adv.script.g1e58 = dir;
            adv.script.g1e56 = 2 * dir + 2;
            adv.script.g1e52 = -2;
        }
        105 => {
            if !adv.player_linked() {
                return;
            }
            adv.script.g1e58 = adv.player_facing();
            if adv.script.g1e54 >= 0 {
                adv.script.g1e58 = adv.script.g1e54;
            }
            adv.script.g1e50 = adv.script.g1e52;
            adv.script.g1e52 = match adv.script.g1e58 {
                0 => 1,
                1 => 0xA,
                _ => 0xB,
            };
            sample(adv, 8);
        }
        106 => {
            adv.script.g1e50 = adv.script.g1e52;
            adv.script.g1e54 = 2;
            adv.script.g1e52 = 0xB;
            sample(adv, 8);
        }
        107 => {
            adv.script.g1e50 = adv.script.g1e52;
            adv.script.g1e54 = 1;
            adv.script.g1e52 = 0xA;
            sample(adv, 8);
        }
        110 => {
            if adv.script.g1e56 != 0 {
                adv.script.g1e52 = adv.script.g1e56;
            }
        }
        111 => {
            adv.script.g1ed0 = 1;
            adv.task_arm(0);
        }
        112 => {
            adv.script.g1e54 = 0;
            adv.script.g1e56 = 0xC;
            adv.script.g1e52 = -2;
        }
        120..=129 => {
            // `1586h:05E1h(n − 20)`: the door chosen, the machine into
            // mode 7 with the script exit's gate.
            adv.script_exit(num::low8(num::unsigned(n - 20)));
        }
        _ => {}
    }
}

/// `0EB9h:1949h(k)`: sample `k` of the ones the room preloaded, through
/// `0590h:091Ch`, which plays only with a digital driver installed
/// (`[210h]`, `0x6226`). The engine runs with the Ad Lib card alone, so
/// the sample plays nothing; the call is kept as an event.
pub(super) fn sample(adv: &mut Adventure, k: u8) {
    adv.events.push(crate::titles::parsec1994::Event::Sample(k));
}
