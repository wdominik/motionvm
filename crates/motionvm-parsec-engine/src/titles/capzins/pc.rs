//! Location 5, the PC (`PC` in the game's location table).
//!
//! Re-expressed from the handler at `11F2:0040` (image `0x11F60`), its
//! hook at `11F2:00DD` (`0x11FFD`) and its one rule handler at `11F2:0007`
//! (`0x11F27`), against [`Api`] alone. The location has no animation hook.
//! Addresses are those of *Captain Zins*' `XP01.EXE`, `DGROUP` `1FC4h`.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::Api;

/// Object 200, the one the dialogue animates.
const OBJECT_200: u16 = 0xC8;

/// `[1CF1h]`: this location's state, stepped by the rule and by script
/// event 10.
const STATE: u16 = 0x1CF1;
/// `[1CEFh]`: the Telefon's state (its rule switches on it).
const TELEFON_STATE: u16 = 0x1CEF;
/// `[1EB4h]`: whether object 200 speaks in the dialogue that is up.
const IT_SPEAKS: u16 = 0x1EB4;
/// `[1EB2h]`: the animation it is to play at the next page's end.
const NEXT_ANIM: u16 = 0x1EB2;
/// `[1EB0h]`: the animation it is playing.
const PLAYING_ANIM: u16 = 0x1EB0;
/// `[1EB6h]`: set by hook code 8 with a negative argument; while it is
/// set the frame loop plays its samples.
const NOISE: u16 = 0x1EB6;
/// `[16F4h]`: a counter the frame loop compares against 100 and reloads
/// (Dunkle Schatten: `[1C82h]`).
const COUNTER: u16 = 0x16F4;

/// Everything before the frame loop, `0x11F60` to `0x11F90`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x11F60
    let entry = api.word(ENTRY); // 0x11F65
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x11F6C
    api.visit_enter(loc, entry); // 0x11F70
    api.obj_anim_id(OBJECT_200, 0); // 0x11F7D
    api.visit_finish_entry(); // 0x11F85
    api.set_word(NOISE, 0); // 0x11F8A
    api.set_word(ENTRY, 0); // 0x11F90
}

/// One turn of the frame loop, `0x11F96` to `0x11FDD`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x11F96
    api.dispatch(); // 0x11F9B
    // 0x11FA0: `cmp [1EB6h], 0 ; je` then `cmp [16F4h], 64h ; jbe`.
    if api.word(NOISE) != 0 && api.word(COUNTER) > 0x64 {
        let n = api.random(0, 2); // 0x11FB2
        api.play_sample(n.wrapping_add(4)); // 0x11FBE
        let next = api.random(0, 0x32); // 0x11FC8
        api.set_word(COUNTER, next); // 0x11FD0
    }
    api.frame_pass(); // 0x11FD3
    api.word(ENTRY) == 0 // 0x11FD8
}

/// Everything after the frame loop, `0x11FDF` to `0x11FFC`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5)); // 0x11FE2
    api.objs_save_state(out); // 0x11FEA
    api.visit_leave(); // 0x11FF2
    api.slot_unload(); // 0x11FF7
}

/// The hook, `11F2:00DD`: object 200's animations follow the dialogue,
/// and one script event. Codes 0 to 9 through the table at `0x120AD`
/// (0 `00F5h`; 1 to 4 `018Bh`, the return; 5 and 6 `011Ah`; 7 `017Dh`;
/// 8 `0144h`; 9 `0163h`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x12015: `test dx, 2`.
            if arg & 2 != 0 {
                api.set_word(IT_SPEAKS, 1); // 0x1201B
                api.set_word(PLAYING_ANIM, 0xFFFF); // 0x12021
            } else {
                api.set_word(IT_SPEAKS, 0); // 0x1202A
                api.set_word(PLAYING_ANIM, 0); // 0x12032
                api.set_word(NEXT_ANIM, 0); // 0x12035
            }
        }
        5 | 6 => {
            // 0x1203A
            if api.word(IT_SPEAKS) != 0 {
                api.obj_anim_id(OBJECT_200, 0); // 0x12046
                api.set_word(IT_SPEAKS, 0); // 0x1204E
                api.set_word(PLAYING_ANIM, 0); // 0x12056
                api.set_word(NEXT_ANIM, 0); // 0x12059
                api.set_word(NOISE, 0); // 0x1205C
            }
        }
        8 => {
            // 0x12064
            if api.word(IT_SPEAKS) != 0 {
                let speaks = api.word(IT_SPEAKS);
                api.set_word(NEXT_ANIM, speaks.wrapping_add(arg)); // 0x12070
                // 0x12073: `or dx, dx ; jge` — 1 for a negative argument.
                api.set_word(NOISE, u16::from(num::signed(arg) < 0)); // 0x1207E
            }
        }
        9 => {
            // 0x12083
            let next = api.word(NEXT_ANIM);
            if next != api.word(PLAYING_ANIM) {
                api.set_word(PLAYING_ANIM, next); // 0x1208C
                api.obj_anim_id(OBJECT_200, num::signed(next)); // 0x12093
            }
        }
        7 => {
            // 0x1209D: event 10.
            if arg == 0x0A {
                api.set_byte(STATE, 2); // 0x120A6
            }
        }
        _ => {} // 1 to 4 and above 9: 0x120AB
    }
}

/// The PC's rule handlers by their offset in segment `11F2h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 11F2:0007 (the rule with word +2 = 2, mask 4, word +6 = C8h), by
        // the location's state.
        0x0007 => {
            match api.byte(STATE) {
                0 => {
                    api.set_byte(STATE, 1); // 0x11F3F
                    api.scene_play(1); // 0x11F44
                }
                1 => {
                    // 0x11F48: `cmp [1CEFh], 2 ; jne`.
                    if api.byte(TELEFON_STATE) == 2 {
                        api.scene_play(0); // 0x11F4F
                    } else {
                        api.scene_play(2); // 0x11F53
                    }
                }
                2 => {
                    api.scene_play(2); // 0x11F53
                }
                _ => {} // 0x11F3D
            }
            0x0A // 0x11F5B
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
