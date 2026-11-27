//! Location 6, a row of kind 7 with no name (`+37h` is `0000h:0000h`) and
//! one exit, `6Eh`, to location 81. Row 41 (`Schlönstadt's Gefängnis`)
//! leads here by its exits `6Eh` and `72h`; no other row names it.
//!
//! Re-expressed from the handler at `1187h:0005h` (image `0x11875` to its
//! `retf` at `0x119E9`) and its animation hook `1187h:017Ah` (`0x119EA` to
//! its `retf` at `0x11AFA`, the record's `+43h`, image `0x219EB`), against
//! [`Api`] alone. The record's hook `+3Fh` and rule table `+3Bh` are
//! `0000h:0000h`: the location has no hook and no rules of its own. Segment
//! `1187h` is named nowhere in the image but in this record (`0x219DD`,
//! `0x219ED`): the location has no gate either.
//!
//! The visit is a scene the player watches: the input is taken over on
//! entry (`0x11903`) and nothing in the segment gives it back. The player
//! walks to node `7Fh`; objects `F0h` and `F1h` are shown and set on their
//! animations, and the animation hook's codes `64h` to `6Dh` drive the
//! sound and the story from there. Code `6Ah` marks `[183Eh]`; from then on
//! the player is hidden once it stands on node `7Fh`, and `F0h` goes on
//! animation 3. Code `6Ch` leaves for location 39 (`27h`) with entry `7Fh`
//! — location 39 goes on to location 13 (`mov word [200Ch], 0Dh`,
//! `0x17409`). The row's exit `6Eh` to location 81 is not used by
//! anything in the segment.
//!
//! The sound: in state 1 (code `64h`) and state 2 (code `66h`) the frame
//! loop plays sample `10h` whenever the hundredths `[1CF6h]` pass 50, then
//! sets them to a draw from `[2DB0h]` to `[2DB0h] + 0Ah`; `[2DB0h]` climbs
//! by one a sample up to 30 in state 1 and falls to 0 in state 2, so the
//! samples come faster and slower. Code `6Dh` counts and plays sample
//! `0Fh` on every odd count, and in state 4 on the counts that are not a
//! multiple of 4.
//!
//! The words the location keeps, each read or written in segment `1187h`
//! alone: `[183Eh]` (outside every block the save copies), and `[2DAAh]`
//! to `[2DB2h]` (outside the `38h` bytes from `DS:2D38h` the visit clears;
//! `[2DB2h]` is read without an entry write).

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x119D2`).
const OBJECT_STATES: u16 = 0x2018;

/// The objects the logic names (`push 0F0h`, `0x118AA`/`0x118CE`; `push
/// 0F1h`, `0x118EE`).
const OBJECT_F0: u16 = 0xF0;
const OBJECT_F1: u16 = 0xF1;

/// The node the player walks to on entry (`mov ax, 7Fh`, `0x1188C`).
const NODE_7F: u16 = 0x7F;

/// `[183Eh]`: set by the animation hook's code `6Ah` (`0x11A9D`), never
/// cleared; read on entry (`0x1189F`) and by the frame loop (`0x11928`).
const ARRIVED: u16 = 0x183E;
/// `[2DAAh]`/`[2DACh]`: object `F0h`'s record, kept on entry (`0x118B5`,
/// `0x118B9`; `0x118D9`, `0x118DD`).
const REC_F0: u16 = 0x2DAA;
/// `[2DAEh]`: the count code `6Dh` steps (`0x11AD9`).
const COUNT: u16 = 0x2DAE;
/// `[2DB0h]`: the low bound of the draw for the hundredths.
const PACE: u16 = 0x2DB0;
/// `[2DB2h]`: the state, 0 to 4, set by the animation hook.
const STATE: u16 = 0x2DB2;

/// `[1CF6h]`: the hundredths.
const HUNDREDTHS: u16 = 0x1CF6;
/// `cmp word [1CF6h], 32h ; jna` (`0x11975`, `0x119A7`).
const SAMPLE_DELAY: u16 = 0x32;
/// `cmp word [2DB0h], 1Eh ; jnl` (`0x1197C`).
const PACE_TOP: i16 = 0x1E;

/// The player record's flag byte `+2` (`[1770h]`); bit 1 is its hidden
/// bit.
const PLAYER_FLAGS: u16 = WALKER + 2;
const HIDDEN: u8 = 0x02;
/// `[1796h]`: the player record's `+28h`, its node (`0x1192F`).
const PLAYER_NODE: u16 = WALKER + 0x28;

/// The mode block's walk target.
const WALK_TARGET: u16 = 0x2D4A;

/// The location code `6Ch` leaves for (`mov word [200Ch], 27h`, `0x11ACB`)
/// and the entry it is left with (`mov word [200Ah], 7Fh`, `0x11AD1`).
const NEXT_LOCATION: u16 = 0x27;
const NEXT_ENTRY: u16 = 0x7F;

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

/// `push k ; push word [2DACh] ; push word [2DAAh] ; call far
/// 03F7h:1CF9h`: object `F0h` on animation `k`.
fn anim_f0(api: &mut dyn Api, k: i16) {
    let r = rec(api, REC_F0);
    api.obj_anim(r, k);
}

/// `push hi ; push lo ; call far 203Dh:0024h ; mov [1CF6h], ax`: the
/// hundredths set to a draw from `lo` to `hi`.
fn draw_hundredths(api: &mut dyn Api, lo: u16, hi: u16) {
    let n = api.random(lo, hi);
    api.set_word(HUNDREDTHS, n);
}

/// `cmp word [2B0h], 0 ; jz ; push 0 ; call far 1975h:0436h`
/// (`0x11A2C`–`0x11A35`, `0x11A80`–`0x11A89`): with a digital driver,
/// its `StopAll` without the wait.
fn stop_effects(api: &mut dyn Api) {
    api.digital_stop(0);
}

/// Everything before the frame loop, `0x11875` to `0x1191D`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x11875
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.visit_enter(loc, entry); // 0x11882
    api.set_word(WALK_TARGET, NODE_7F); // 0x1188F
    let player = api.ds(WALKER);
    api.walker_walk_to(player, NODE_7F, 0); // 0x11897, answer not read
    // 0x1189F: with [183Eh] set, F0h on animation 2 and F1h on 1; else
    // both on 0. Only F0h's record is kept.
    let (k_f0, k_f1) = if api.word(ARRIVED) != 0 {
        (2, 1)
    } else {
        (0, 0)
    };
    let r = api.obj_show_frame(OBJECT_F0, -1); // 0x118AD, 0x118D1
    keep(api, REC_F0, r); // 0x118B5, 0x118B9; 0x118D9, 0x118DD
    api.obj_anim(r, k_f0); // 0x118BE, 0x118E2
    let r = api.obj_show_frame(OBJECT_F1, -1); // 0x118F1
    api.obj_anim(r, k_f1); // 0x118FB
    api.input_lock(); // 0x11903
    api.hover_off(); // 0x11908
    api.visit_finish_entry(); // 0x1190D
    // 0x11912: here is written after the entry's end, not before it: the
    // entry's end and the visit's leave ask `0F48h:082Bh` (`0x0FCAB`),
    // which compares the kinds of the rows of [200Eh] and [200Ch].
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x11915
    api.set_word(WORDS.entry, 0); // 0x11918
}

/// One turn of the frame loop, `0x1191E` to `0x119C9`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x1191E
    api.dispatch(); // 0x11923
    // 0x11928–0x11936: with [183Eh] set and the player on the walk
    // target, the target cleared, the player hidden and F0h on
    // animation 3.
    if api.word(ARRIVED) != 0 && api.word(PLAYER_NODE) == api.word(WALK_TARGET) {
        api.set_word(WALK_TARGET, 0); // 0x11938
        let flags = api.byte(PLAYER_FLAGS) | HIDDEN;
        api.set_byte(PLAYER_FLAGS, flags); // 0x1193E
        let player = api.ds(WALKER);
        api.obj_invalidate(player); // 0x11947
        anim_f0(api, 3); // 0x11959
    }
    // 0x11961: by the state; states 0, 3 and 4 do nothing here.
    match api.word(STATE) {
        1 if api.word(HUNDREDTHS) > SAMPLE_DELAY => {
            // 0x1197C: `jnl`, signed.
            if num::signed(api.word(PACE)) < PACE_TOP {
                let p = api.word(PACE).wrapping_add(1);
                api.set_word(PACE, p); // 0x11983
            }
            paced_sample(api); // 0x11987
        }
        2 if api.word(HUNDREDTHS) > SAMPLE_DELAY => {
            // 0x119AE: `jng`, signed.
            if num::signed(api.word(PACE)) > 0 {
                let p = api.word(PACE).wrapping_sub(1);
                api.set_word(PACE, p); // 0x119B5
            }
            paced_sample(api); // 0x11987
        }
        _ => {}
    }
    api.frame_pass(); // 0x119BD
    api.word(WORDS.entry) == 0 // 0x119C2
}

/// `0x11987`–`0x119A4`: the hundredths set to a draw from `[2DB0h]` to
/// `[2DB0h] + 0Ah`, and sample `10h`.
fn paced_sample(api: &mut dyn Api) {
    let lo = api.word(PACE);
    draw_hundredths(api, lo, lo.wrapping_add(0x0A)); // 0x11992, 0x1199A
    api.play_sample(0x10); // 0x1199F
}

/// Everything after the frame loop, `0x119CC` to `0x119E9`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x119D7
    api.visit_leave(); // 0x119DF
    api.slot_unload(); // 0x119E4
}

/// The record names no hook (`+3Fh` is `0000h:0000h`).
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// The animation hook, `1187h:017Ah`: the code in `[bp+0Ah]`; codes `64h`
/// to `6Dh` have arms (`0x119ED`–`0x119FD`, the table at `cs:028Bh`,
/// `0x11AFB`), every other code does nothing.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        // 0x11A02: state 1, the pace from 0, sample 10h.
        0x64 => {
            api.set_word(STATE, 1); // 0x11A02
            api.set_word(PACE, 0); // 0x11A0C
            draw_hundredths(api, 0, 0x0A); // 0x11A10, 0x11A18
            api.play_sample(0x10); // 0x11A1D
        }
        // 0x11A26: state 0, the effects cut, sample 11h.
        0x65 => {
            api.set_word(STATE, 0); // 0x11A26
            stop_effects(api); // 0x11A2C, 0x11A35
            api.play_sample(0x11); // 0x11A3B, 0x11A1D
        }
        // 0x11A3F: sample 0Eh, state 2, the pace from 1Eh, sample 10h.
        0x66 => {
            api.play_sample(0x0E); // 0x11A41
            api.set_word(STATE, 2); // 0x11A47
            api.set_word(PACE, 0x1E); // 0x11A4F, 0x11A0C
            draw_hundredths(api, 0x1E, 0x28); // 0x11A10, 0x11A18
            api.play_sample(0x10); // 0x11A1D
        }
        // 0x11A54: state 0.
        0x67 => api.set_word(STATE, 0),
        // 0x11A5D: F0h on animation 1.
        0x68 => anim_f0(api, 1), // 0x11A67
        // 0x11A72: the count from 0, state 3, sample 0Fh.
        0x69 => {
            api.set_word(COUNT, 0); // 0x11A72
            api.set_word(STATE, 3); // 0x11A78
            api.play_sample(0x0F); // 0x11AF4, 0x11A1D
        }
        // 0x11A80: the effects cut, state 0, sample 0Eh, [183Eh] set.
        0x6A => {
            stop_effects(api); // 0x11A80, 0x11A89
            api.set_word(STATE, 0); // 0x11A8F
            api.play_sample(0x0E); // 0x11A97
            api.set_word(ARRIVED, 1); // 0x11A9D
        }
        // 0x11AA5: the count from 0, state 4, sample 0Eh.
        0x6B => {
            api.set_word(COUNT, 0); // 0x11AA5
            api.set_word(STATE, 4); // 0x11AAB
            draw_hundredths(api, 1, 0x0A); // 0x11AB5, 0x11ABD
            api.play_sample(0x0E); // 0x11AC0, 0x11A1D
        }
        // 0x11AC5: state 0, and on to location 39 with entry 7Fh.
        0x6C => {
            api.set_word(STATE, 0); // 0x11AC5
            api.set_word(WORDS.location, NEXT_LOCATION); // 0x11ACB
            api.set_word(WORDS.entry, NEXT_ENTRY); // 0x11AD1
        }
        // 0x11AD9: the count stepped; sample 0Fh on an odd count, and in
        // state 4 on an even one that is not a multiple of 4.
        0x6D => {
            let n = api.word(COUNT).wrapping_add(1);
            api.set_word(COUNT, n); // 0x11AD9
            let play = n & 1 != 0 // 0x11ADD
                || (api.word(STATE) == 4 && n & 3 != 0); // 0x11AE5, 0x11AEC
            if play {
                api.play_sample(0x0F); // 0x11AF4, 0x11A1D
            }
        }
        _ => {}
    }
}

/// The location has no rules of its own (`+3Bh` is `0000h:0000h`).
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
