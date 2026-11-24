//! Location 6, the Bits (`Bits` in the game's location table).
//!
//! Re-expressed from the handler at `120C:0161` (image `0x12221`), its
//! hook at `120C:0303` (`0x123C3`), its animation hook at `120C:02A9`
//! (`0x12369`), its six rule handlers at `120C:0098`, `00A8`, `00C5`,
//! `00D5`, `012A` and `0151`, the far routine `120C:0001` the handler
//! installs at `[1980h]`, and the near routine `120C:0040`, against
//! [`Api`] alone. Addresses are those of *Captain Zins*' `XP01.EXE`,
//! `DGROUP` `1FC4h`.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Objects 128, 129 and 101.
const OBJECT_128: u16 = 0x80;
const OBJECT_129: u16 = 0x81;
const OBJECT_101: u16 = 0x65;
/// Objects 18 to 25: the eight the pattern is read from.
const FIRST_BIT: u16 = 0x12;
const LAST_BIT: u16 = 0x19;

/// `[1CEFh]`: the Telefon's state; 3 and 4 decide the entry.
const TELEFON_STATE: u16 = 0x1CEF;
/// `[1CF0h]`, `[1CF3h]`, `[1CF5h]` (the Formel's state).
const BYTE_1CF0: u16 = 0x1CF0;
const BYTE_1CF3: u16 = 0x1CF3;
const FORMEL_STATE: u16 = 0x1CF5;
/// `[1CF4h]`: this location's state: 1 once entered, 2 once the pattern
/// stands.
const STATE: u16 = 0x1CF4;
/// `[1EBCh]`: whether object 129 speaks in the dialogue that is up.
const IT_SPEAKS: u16 = 0x1EBC;
/// `[1EB8h]`: the animation it is to play at the next page's end.
const NEXT_ANIM: u16 = 0x1EB8;
/// `[1EBAh]`: the animation it is playing.
const PLAYING_ANIM: u16 = 0x1EBA;
/// `[1EBEh]`: the value `[1204h]` has to reach before the frame loop
/// starts task 0 again; zero, nothing is waited for.
const WAIT_FOR: u16 = 0x1EBE;
/// `[1204h]`: word `+28h` of the record at `DS:11DCh` (Dunkle Schatten:
/// `[16BBh]`).
const RECORD_11DC_28: u16 = 0x1204;
/// `DS:11DCh`: the record the walker routines are handed.
const RECORD_11DC: u16 = 0x11DC;
/// `[1E5Ch]`: the mode block's first object (Dunkle Schatten: `[2891h]`).
const MODE_OBJECT: u16 = 0x1E5C;
/// `[1E86h]`, `[1E88h]`: the far pointer rule `012Ah` clears (Dunkle
/// Schatten: `[28BBh]`, `[28BDh]`, the gate the mode machine asks).
const GATE_OFF: u16 = 0x1E86;
const GATE_SEG: u16 = 0x1E88;
/// `[1980h]`, `[1982h]`: a far pointer to a routine `(id)`; the visit's
/// setup (`0A01:003B`) writes `0A01:0001` there, this handler its own.
const CALLBACK_OFF: u16 = 0x1980;
const CALLBACK_SEG: u16 = 0x1982;

/// A field of a far record.
fn at(rec: Far, field: u16) -> Far {
    Far {
        off: rec.off.wrapping_add(field),
        seg: rec.seg,
    }
}

/// `120C:0001(id)`, the routine installed at `[1980h]`: −2 for no such
/// object, −1 when bit 0 of its byte `+2` is clear, 1 when its byte `+9`
/// is below 2, else 0.
pub(crate) fn object_state(api: &mut dyn Api, id: u16) -> i16 {
    let rec = api.obj_find(id); // 0x120C8
    if rec == Far::NULL {
        return -2; // 0x120D9
    }
    if api.far_byte(at(rec, 2)) & 1 == 0 {
        return -1; // 0x120E8
    }
    // 0x120F0: `cmp es:[bx+9], 2 ; jae`.
    i16::from(api.far_byte(at(rec, 9)) < 2)
}

/// `120C:0040`, near: whether the pattern stands. Objects 18 to 25 answer
/// one bit each through [`object_state`]; any answer but 0 and 1 ends it
/// with 0; the pattern `8Dh` sets `[1CF4h]` to 2.
fn pattern_stands(api: &mut dyn Api) -> bool {
    // 0x12105: `cmp [1CF4h], 2 ; jb`.
    if api.byte(STATE) >= 2 {
        return true; // 0x1210C
    }
    let mut bits: u16 = 0;
    for i in 0..8u16 {
        // 0x1211A
        match object_state(api, i.wrapping_add(FIRST_BIT)) {
            0 => {}
            1 => bits |= 1 << i, // 0x1212F
            _ => return false,   // 0x1212D
        }
    }
    // 0x12146: `cmp si, 8Dh`.
    if bits == 0x8D {
        api.set_byte(STATE, 2); // 0x1214C
        true
    } else {
        false // 0x12153
    }
}

/// Everything before the frame loop, `0x12221` to `0x1230E`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x12227

    // 0x1222E: object 25's word +0Ch, and objects 24 down to 18 given the
    // same word, their word +0Ah that plus their own byte +9.
    let rec = api.obj_find(LAST_BIT);
    let base = api.far_word(at(rec, 0x0C)); // 0x12238
    for id in (FIRST_BIT..LAST_BIT).rev() {
        let rec = api.obj_find(id); // 0x12242
        api.set_far_word(at(rec, 0x0C), base); // 0x12251
        let b = u16::from(api.far_byte(at(rec, 9))); // 0x12255
        api.set_far_word(at(rec, 0x0A), base.wrapping_add(b)); // 0x1225F
    }
    let entry = api.word(ENTRY); // 0x1226B
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x12272
    api.visit_enter(loc, entry); // 0x12276
    api.set_word(CALLBACK_SEG, 0x120C); // 0x1227E
    api.set_word(CALLBACK_OFF, 0x0001); // 0x12284
    // 0x1228A: a compare chain on the Telefon's state.
    let hide = match api.byte(TELEFON_STATE) {
        3 => {
            api.scene_play(1); // 0x1229D
            api.obj_show(OBJECT_129); // 0x122A6
            api.obj_hide(OBJECT_101); // 0x122AE
            api.obj_anim_id(OBJECT_128, 0); // 0x122B9
            api.walk_to_node(OBJECT_129); // 0x122C4
            api.set_byte(BYTE_1CF3, 1); // 0x122CA
            api.set_byte(TELEFON_STATE, 4); // 0x122CF
            false
        }
        // 0x122D6: `cmp [1CF0h], 2 ; jae`.
        4 if api.byte(BYTE_1CF0) < 2 => {
            let rec = api.obj_show(OBJECT_129); // 0x122E2
            api.obj_anim(rec, 0); // 0x122EA
            false
        }
        _ => true,
    };
    if hide {
        api.obj_hide(OBJECT_129); // 0x122F7
    }
    // 0x122FD: `cmp [1CF4h], 1 ; jae`.
    if api.byte(STATE) < 1 {
        api.set_byte(STATE, 1); // 0x12304
    }
    api.visit_finish_entry(); // 0x12309
    api.set_word(ENTRY, 0); // 0x1230E
}

/// One turn of the frame loop, `0x12314` to `0x12346`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x12314
    api.dispatch(); // 0x12319
    // 0x1231E: the record's word +28h has reached what is waited for.
    let wait = api.word(WAIT_FOR);
    if wait != 0 && api.word(RECORD_11DC_28) == wait {
        api.set_word(WAIT_FOR, 0); // 0x1232E
        api.task_start(0); // 0x12336
    }
    api.frame_pass(); // 0x1233C
    api.word(ENTRY) == 0 // 0x12341
}

/// Everything after the frame loop, `0x12348` to `0x12368`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5)); // 0x1234B
    api.objs_save_state(out); // 0x12353
    api.visit_leave(); // 0x1235B
    api.slot_unload(); // 0x12360
}

/// The hook, `120C:0303`: object 129's animations follow the dialogue,
/// and two script events. Codes 0 to 9 through the table at `0x1244D`
/// (0 `0318h`; 1 to 4 `038Bh`, the return; 5 and 6 `032Eh`; 7 `036Bh`;
/// 8 `0342h`; 9 `0351h`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x123D8
            api.set_word(IT_SPEAKS, arg & 2); // 0x123DD
            api.set_word(PLAYING_ANIM, 0); // 0x123E0
            api.set_word(NEXT_ANIM, 0); // 0x123E6
        }
        5 | 6 => {
            // 0x123EE
            if api.word(IT_SPEAKS) != 0 {
                api.set_word(PLAYING_ANIM, 0); // 0x123F7
                api.set_word(NEXT_ANIM, 0); // 0x123FA
                api.set_word(IT_SPEAKS, 0); // 0x123FD
                api.obj_anim_id(OBJECT_129, 0); // 0x12421
            }
        }
        8 => {
            // 0x12402
            if api.word(IT_SPEAKS) != 0 {
                api.set_word(NEXT_ANIM, arg.wrapping_add(1)); // 0x1240C
            }
        }
        9 => {
            // 0x12411
            let next = api.word(NEXT_ANIM);
            if next != api.word(PLAYING_ANIM) {
                api.set_word(PLAYING_ANIM, next); // 0x1241A
                api.obj_anim_id(OBJECT_129, num::signed(next)); // 0x12421
            }
        }
        7 => match arg {
            // 0x1242B: a compare chain, events 10 and 110.
            0x0A => {
                api.task_arm(0); // 0x1243B
                api.set_word(WAIT_FOR, OBJECT_129); // 0x12441
            }
            0x6E => api.set_word(NEXT_ANIM, 0), // 0x123E6
            _ => {}
        },
        _ => {} // 1 to 4 and above 9: 0x1244B
    }
}

/// The animation hook, `120C:02A9`: a compare chain on the code.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        0x0A => {
            // 0x12398: the pattern checked while `[1CF4h]` is below 2 and
            // `[1CF5h]` is 2 or more.
            if api.byte(STATE) < 2 && api.byte(FORMEL_STATE) >= 2 && pattern_stands(api) {
                api.play_sample(0x17); // 0x123AF
                api.obj_anim_id(OBJECT_101, 1); // 0x123B9
            }
        }
        0x64 => {
            api.obj_hide(OBJECT_128); // 0x1237C
            let rec = api.obj_show(OBJECT_101); // 0x12386
            api.obj_anim(rec, 2); // 0x1238E
        }
        _ => {}
    }
}

/// The Bits' rule handlers by their offset in segment `120Ch`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 120C:0098: scene 2.
        0x0098 => {
            api.scene_play(2); // 0x1215D
            0x0A
        }
        // 120C:00A8: scene 3 once the pattern stands, else nothing of
        // its own.
        0x00A8 => {
            if api.byte(STATE) == 2 {
                api.scene_play(3); // 0x12174
                0x0A
            } else {
                0 // 0x1217C
            }
        }
        // 120C:00C5: scene 4.
        0x00C5 => {
            api.scene_play(4); // 0x1218A
            0x0A
        }
        // 120C:00D5: the mode block's object, if it is carried, is taken
        // away, walked to and shown on animation 0; else scene 5.
        0x00D5 => {
            let id = api.word(MODE_OBJECT);
            if api.inv_has(id) != 0 {
                api.inv_take(id); // 0x121AA
                let player = api.ds(RECORD_11DC);
                api.walker_walk_to(player, id, 0); // 0x121BA
                let rec = api.obj_show(id); // 0x121C8
                api.obj_anim(rec, 0); // 0x121D0
                1 // 0x121D8
            } else {
                api.scene_play(5); // 0x121DF
                0x0A
            }
        }
        // 120C:012A: nothing of its own once the pattern stands; else
        // scene 6 and the far pointer at [1E86h] cleared.
        0x012A => {
            if pattern_stands(api) {
                0 // 0x121F4
            } else {
                api.scene_play(6); // 0x121FA
                api.set_word(GATE_SEG, 0); // 0x12200
                api.set_word(GATE_OFF, 0); // 0x12206
                0x0A
            }
        }
        // 120C:0151: scene 0.
        0x0151 => {
            api.scene_play(0); // 0x12216
            0x0A
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
