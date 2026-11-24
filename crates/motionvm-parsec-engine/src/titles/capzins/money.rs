//! Location 18, `Money`, as the game names it in its location table
//! (record at image `0x1EEC8`).
//!
//! Re-expressed from the handler at `143E:0004` (image `0x143E4`), its hook
//! at `143E:0361` (`0x14741`) and its animation hook at `143E:00CA`
//! (`0x144AA`), against [`Api`] alone. The location has no rules of its
//! own (rules pointer `0000:0000`). Addresses in comments are offsets in
//! segment `143Eh`.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 17.
const OBJECT_11: u16 = 0x11;
/// Object 201.
const OBJECT_C9: u16 = 0xC9;
/// Object 203.
const OBJECT_CB: u16 = 0xCB;
/// Object 204.
const OBJECT_CC: u16 = 0xCC;

/// `[11DEh]`: the walker record's byte `+2` (`DS:11DCh + 2`); the
/// animation hook sets its bit 0 and script event 0Ch clears it.
const WALKER_02: u16 = 0x11DE;
/// `DS:11DCh`, the player's walker record, handed to `03E8:1FB9` as
/// `push ds ; push 11DCh`.
const WALKER_REC: u16 = 0x11DC;
/// `[1204h]`: the walker record's word `+28h` (DS1 `[16BBh]`).
const WALKER_28: u16 = 0x1204;
/// `[1CECh]`: this location's state byte.
const STATE: u16 = 0x1CEC;
/// `[1F1Ah]`: the base animation of object 17 while pages are spoken, or
/// 0.
const BASE_11: u16 = 0x1F1A;
/// `[1F16h]`: the animation object 17 is asked to take.
const WANT_11: u16 = 0x1F16;
/// `[1F18h]`: the animation object 17 was last given.
const ANIM_11: u16 = 0x1F18;
/// `[1F24h]`: the base animation of object 201, or 0.
const BASE_C9: u16 = 0x1F24;
/// `[1F20h]`: the animation object 201 is asked to take.
const WANT_C9: u16 = 0x1F20;
/// `[1F22h]`: the animation object 201 was last given.
const ANIM_C9: u16 = 0x1F22;
/// `[1F1Ch]`: the animation object 203 is asked to take.
const WANT_CB: u16 = 0x1F1C;
/// `[1F1Eh]`: the animation object 203 was last given.
const ANIM_CB: u16 = 0x1F1E;
/// `[1F26h]`: 64h while the frame loop waits for `[1204h]` to be 64h.
const WAIT_NODE: u16 = 0x1F26;

/// Everything before the frame loop, `0004` to `0079`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0004
    let entry = api.word(ENTRY); // 0009
    let loc = api.word(LOCATION); // 000D
    api.set_word(HERE, loc); // 0010
    api.visit_enter(loc, entry); // 0014
    let rec = api.obj_show(OBJECT_C9); // 001C
    api.obj_anim(rec, 4); // 0027

    api.hover_off(); // 0031
    api.set_word(BASE_C9, 1); // 0036

    api.input_lock(); // 003C
    match api.byte(STATE) {
        0 => {
            api.set_byte(STATE, 1); // 0051
            api.scene_play(2); // 0056
        }
        2 => {
            api.set_byte(STATE, 3); // 0060
            api.set_word(BASE_C9, 1); // 0065
            api.scene_play(3); // 006B -> 0058
        }
        _ => {}
    }
    api.visit_finish_entry(); // 006F
    api.set_word(ENTRY, 0); // 0074
}

/// One turn of the frame loop, `007A` to `00AA`. Answers whether the loop
/// goes on.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 007A
    api.dispatch(); // 007F
    // 0084: both `[1F26h]` and `[1204h]` at 64h start task 0.
    if api.word(WAIT_NODE) == 0x64 && api.word(WALKER_28) == 0x64 {
        api.set_word(WAIT_NODE, 0); // 0092
        api.task_start(0); // 0098
    }
    api.frame_pass(); // 00A0
    api.word(ENTRY) == 0 // 00A5
}

/// Everything after the frame loop, `00AC` to `00C9`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE); // 00AC
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 00B7
    api.visit_leave(); // 00BF
    api.slot_unload(); // 00C4
}

/// A sample drawn by `random(lo, hi)` and played.
fn sample_in(api: &mut dyn Api, lo: u16, hi: u16) {
    let k = api.random(lo, hi);
    api.play_sample(k);
}

/// `0307`: the object named by the record's byte `+13h` is hidden.
fn hide_record_object(api: &mut dyn Api, rec: Far) {
    let id = api.far_byte(Far {
        off: rec.off.wrapping_add(0x13),
        seg: rec.seg,
    });
    api.obj_hide(u16::from(id)); // 0311
}

/// The hook, `143E:0361`: `code` at `[bp+6]`, `arg` at `[bp+8]`; codes 7,
/// 8 and 9 by compare (`036B`), anything else returns.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0380: each object with a base is asked for `base + arg`.
        8 => {
            let base = api.word(BASE_C9);
            if base != 0 {
                api.set_word(WANT_C9, base.wrapping_add(arg)); // 0387
            }
            let base = api.word(BASE_11); // 038F
            if base != 0 {
                api.set_word(WANT_11, base.wrapping_add(arg)); // 0399
            }
        }
        // 03A4: the three objects follow what they were asked for.
        9 => {
            let want = api.word(WANT_11);
            if api.word(ANIM_11) != want {
                api.set_word(ANIM_11, want); // 03AD
                api.obj_anim_id(OBJECT_11, num::signed(want)); // 03B3
            }
            let want = api.word(WANT_C9); // 03BE
            if api.word(ANIM_C9) != want {
                api.set_word(ANIM_C9, want); // 03C7
                api.obj_anim_id(OBJECT_C9, num::signed(want)); // 03CD
            }
            let want = api.word(WANT_CB); // 03D9
            if api.word(ANIM_CB) != want {
                api.set_word(ANIM_CB, want); // 03E5
                api.obj_anim_id(OBJECT_CB, num::signed(want)); // 03EB
            }
        }
        7 => script_event(api, arg), // 03FA
        _ => {}                      // 037D
    }
}

/// 03FA: script events, a sparse switch on `arg`: thirteen values
/// at `cs:05A3` (0Ah to 15h, 6Eh), targets at `cs:05BD`.
fn script_event(api: &mut dyn Api, arg: u16) {
    match arg {
        0x0A => {
            api.set_word(WAIT_NODE, 0x64); // 0417
            api.task_arm(0); // 041D
        }
        0x0B => {
            api.set_word(LOCATION, 2); // 0427
            api.set_word(ENTRY, 0x6D); // 042D

            api.task_stop(0); // 04EE
        }
        0x0C => {
            let b = api.byte(WALKER_02); // 0436
            api.set_byte(WALKER_02, b & 0xFE);
            api.play_sample(7); // 043B
            api.play_sample(8); // 0443
            api.inv_take(0x11); // 044B
            let rec = api.obj_show(OBJECT_11); // 0453
            api.obj_anim(rec, 4); // 045D
            api.task_arm(0); // 0467 -> 041D
        }
        0x0D => {
            api.set_word(BASE_C9, 0); // 0469
            api.obj_hide(OBJECT_C9); // 046F
            let rec = api.obj_show(OBJECT_CB); // 0478
            api.slot_set_object(1, 0, rec); // 0481
            api.set_word(ANIM_CB, 0xFFFF); // 048F
            api.set_word(WANT_CB, 0); // 0495
        }
        0x0E => api.set_word(WANT_CB, 1), // 049E
        0x0F => api.set_word(WANT_CB, 2), // 04A7
        0x10 => {
            api.play_sample(0x16); // 04B0
            api.play_sample(0x16); // 04B8
            api.set_word(WANT_CB, 3); // 04C0
        }
        0x11 => {
            api.play_sample(7); // 04C9
            api.play_sample(8); // 04D1
            api.set_word(BASE_11, 6); // 04D9
        }
        0x12 => {
            api.set_word(LOCATION, 0xFFFF); // 04E2
            api.set_word(ENTRY, 0xFFFF); // 04E8

            api.task_stop(0); // 04EE
        }
        0x13 => {
            api.figure_anim(api.ds(WALKER_REC), 0x0C); // 04F9
        }
        0x14 => {
            api.set_word(BASE_C9, 0); // 050A
            api.set_word(WANT_C9, 0);
            api.set_word(ANIM_C9, 0);
            api.obj_hide(OBJECT_C9); // 0515
            let rec = api.obj_show(OBJECT_CB); // 051E
            api.slot_set_object(1, 0, rec); // 0527
            api.play_sample(0x1D); // 0535
            api.set_word(ANIM_CB, 0xFFFF); // 053D
            api.set_word(WANT_CB, 4); // 0543
        }
        0x15 => {
            api.set_word(BASE_C9, 1); // 054B
            api.obj_hide(OBJECT_CB); // 0551
            let rec = api.obj_show(OBJECT_C9); // 055A
            api.slot_set_object(1, 0, rec); // 0563
            api.set_word(WANT_CB, 0xFFFF); // 0571
            api.set_word(ANIM_CB, 0xFFFF);
            api.set_word(ANIM_C9, 0xFFFF);
            api.set_word(WANT_C9, 2); // 057D
        }
        0x6E => {
            // 0585: each object with a base is asked for `base - 1`.
            let base = api.word(BASE_11);
            if base != 0 {
                api.set_word(WANT_11, base.wrapping_sub(1)); // 058C
            }
            let base = api.word(BASE_C9); // 0593
            if base != 0 {
                api.set_word(WANT_C9, base.wrapping_sub(1)); // 059A
            }
        }
        _ => {} // 0410
    }
}

/// The animation hook, `143E:00CA`: a sparse switch on `code`: eleven
/// values at `cs:0335` (`FF9Ch`, 64h to 6Dh), targets at `cs:034B`.
pub(crate) fn anim_hook(api: &mut dyn Api, rec: Far, code: u16) {
    match code {
        // 0165: `random(0, 4)`, four arms through the table at `cs:032D`;
        // above 3 nothing.
        0xFF9C => match api.random(0, 4) {
            0 => sample_in(api, 0x1A, 0x1C), // 0182
            1 => sample_in(api, 4, 0x0A),    // 0197
            2 => sample_in(api, 0x0F, 0x12), // 01AC
            3 => sample_in(api, 0x19, 0x1D), // 01C1
            _ => {}
        },
        0x64 => {
            let b = api.byte(WALKER_02); // 00EE
            api.set_byte(WALKER_02, b | 1);
            api.play_sample(0x0A); // 00F3
            api.play_sample(8); // 00FB
            api.play_sample(0x19); // 0103
            api.task_start(0); // 010B
        }
        0x65 => sample_in(api, 4, 7), // 0115
        0x66 => {
            api.slot_set_object(2, 0, rec); // 012A
            api.play_sample(4); // 013C
            api.play_sample(5); // 0144
            api.play_sample(6); // 014C
            api.play_sample(0x0A); // 0154
            api.set_word(BASE_11, 1); // 015C
        }
        // 01D6: `random(0, 4)`, four arms through the table at `cs:0325`
        // (arm 3 is arm 0's address); above 3 nothing.
        0x67 => match api.random(0, 4) {
            0 | 3 => sample_in(api, 0x1A, 0x1C), // 01F3
            1 => sample_in(api, 4, 0x0A),        // 0208
            2 => sample_in(api, 0x0F, 0x12),     // 021D
            _ => {}
        },
        // 0232: `random(0, 6)`, six arms through the table at `cs:0319`;
        // arms 0 to 3 fall through one another (`pop cx` and on), 4 and 5
        // play one sample; above 5 nothing.
        0x68 => {
            let r = api.random(0, 6);
            match r {
                0..=3 => {
                    if r <= 1 {
                        sample_in(api, 4, 0x0A); // 024F
                    }
                    if r <= 2 {
                        sample_in(api, 0x0F, 0x12); // 0262
                    }
                    sample_in(api, 0x19, 0x1D); // 0275
                }
                4 => sample_in(api, 4, 0x0A),    // 028A
                5 => sample_in(api, 0x0F, 0x12), // 029E
                _ => {}
            }
        }
        0x69 => api.play_sample(0x1C), // 02B2
        0x6A => api.play_sample(0x1D), // 02BB
        0x6B => {
            let shown = api.obj_show(OBJECT_CC); // 02C4
            api.obj_anim(shown, 0); // 02CF
        }
        0x6C => {
            api.play_sample(3); // 02DB
            api.play_sample(0x0A); // 02E3 -> 0301
            hide_record_object(api, rec); // 0307
        }
        0x6D => {
            api.play_sample(3); // 02E7
            api.play_sample(8); // 02EF
            api.play_sample(0x0A); // 02F7
            api.play_sample(0x1B); // 02FF
            hide_record_object(api, rec); // 0307
        }
        _ => {} // 00E7
    }
}

/// Money has no rule handlers of its own.
pub(crate) fn rule(_api: &mut dyn Api, _off: u16, _flags: u16) -> Option<i16> {
    None
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
