//! Location 9, the Sparplan. The module is named after the location as
//! the game itself names it in its location table (`Sparplan`,
//! `1FC4:00E9`).
//!
//! Re-expressed from the handler at `1292:005E` (image `0x1297E`), its
//! hook at `1292:00F0` (`0x12A10`), its animation hook at `1292:00D3`
//! (`0x129F3`) and its one rule handler at `1292:0006` (`0x12926`),
//! against [`Api`] alone. DGROUP is `1FC4h`.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 200, the one that speaks here.
const OBJECT_200: u16 = 0xC8;

/// `[1CF0h]`: the location state.
const STATE: u16 = 0x1CF0;
/// `[1CEFh]`: a state byte the rule compares with 4.
const BYTE_1CEF: u16 = 0x1CEF;
/// `[1ED4h]`: whether object 200 speaks in the dialogue that is up.
const SPEAKS: u16 = 0x1ED4;
/// `[1ED2h]`: the animation it is to play at the next page's end.
const NEXT_ANIM: u16 = 0x1ED2;
/// `[1ED0h]`: the animation it is playing.
const PLAYING_ANIM: u16 = 0x1ED0;

/// Everything before the frame loop, `0x1297E` to `0x129B9`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x1297E
    let entry = api.word(ENTRY); // 0x12983
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x1298A
    api.visit_enter(loc, entry); // 0x1298E
    // 0x12996: `push 32h ; push 118h ; push 10h ; push 1`.
    api.slot_set_raw(1, 0x10, 0x118, 0x32); // 0x1299F
    api.obj_anim_id(OBJECT_200, 0); // 0x129AC
    api.visit_finish_entry(); // 0x129B4
    api.set_word(ENTRY, 0); // 0x129B9
}

/// One turn of the frame loop, `0x129BF` to `0x129D3`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x129BF
    api.dispatch(); // 0x129C4
    api.frame_pass(); // 0x129C9
    api.word(ENTRY) == 0 // 0x129CE
}

/// Everything after the frame loop, `0x129D5` to `0x129F2`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE); // 0x129D5
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x129E0
    api.visit_leave(); // 0x129E8
    api.slot_unload(); // 0x129ED
}

/// The hook, `1292:00F0`: object 200's animations follow the dialogue,
/// and two script events. Codes 0 to 9 through the table at `0x12AB8`;
/// codes 1 to 4 and above 9 do nothing (`0x12AB6`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x12A28: a dialogue comes up; bit 1 of its speakers.
            api.set_word(SPEAKS, arg & 2); // 0x12A2D
            api.set_word(PLAYING_ANIM, 0); // 0x12A32
            api.set_word(NEXT_ANIM, 0); // 0x12A35
        }
        5 | 6 => {
            // 0x12A3A: if it spoke, back to animation 0.
            if api.word(SPEAKS) != 0 {
                api.set_word(SPEAKS, 0); // 0x12A41
                api.set_word(PLAYING_ANIM, 0); // 0x12A49
                api.set_word(NEXT_ANIM, 0); // 0x12A4C
                api.obj_anim_id(OBJECT_200, 0); // 0x12A53
            }
        }
        8 => {
            // 0x12A5D: only while it speaks; the jump lands on the write
            // of `[1ED2h]` alone (`0x12A35`).
            if api.word(SPEAKS) != 0 {
                api.set_word(NEXT_ANIM, arg.wrapping_add(1));
            }
        }
        9 => {
            // 0x12A69: a new animation to play; no test of `[1ED4h]`.
            let next = api.word(NEXT_ANIM);
            if next != api.word(PLAYING_ANIM) {
                api.set_word(PLAYING_ANIM, next); // 0x12A72
                api.obj_anim_id(OBJECT_200, num::signed(next)); // 0x12A79
            }
        }
        7 => match arg {
            // 0x12A83: events 10 and 11.
            0x0A => {
                api.set_byte(STATE, 2); // 0x12A91
                api.play_sample(4); // 0x12A98
                api.play_sample(5); // 0x12AA0
                api.inv_give(7); // 0x12AA8
            }
            0x0B => api.set_word(NEXT_ANIM, 3), // 0x12AB0
            _ => {}
        },
        _ => {}
    }
}

/// The animation hook, `1292:00D3`: code 64h plays sample 9 and gives
/// item 9.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    // 0x129F9: `cmp ax, 64h ; jne`.
    if code == 0x64 {
        api.play_sample(9); // 0x12A00
        api.inv_give(9); // 0x12A08
    }
}

/// The Sparplan's rule handler by its offset in segment `1292h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 1292:0006, verb 2 on object 200 (rule row `1F73:0230`), by the
        // location state.
        0x0006 => {
            match api.byte(STATE) {
                0 => {
                    // 0x1293E
                    api.set_byte(STATE, 1);
                    api.scene_play(1); // 0x12945
                    // 0x1294B: `cmp [1CEFh], 4 ; jne`.
                    if api.byte(BYTE_1CEF) == 4 {
                        api.scene_enqueue(2); // 0x12954
                    }
                }
                1 => {
                    // 0x1295B
                    if api.byte(BYTE_1CEF) == 4 {
                        api.scene_play(2); // 0x12962, 0x12973
                    } else {
                        api.scene_play(3); // 0x12971
                    }
                }
                3 => {
                    // 0x12968
                    api.set_byte(STATE, 4);
                    api.scene_play(4); // 0x1296D, 0x12973
                }
                _ => {
                    api.scene_play(3); // 0x12971
                }
            }
            0x0A // 0x12979
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
