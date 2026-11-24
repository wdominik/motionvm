//! Location 14, `Sharky`, as the game names it in its location table
//! (record at image `0x1EE18`).
//!
//! Re-expressed from the handler at `1380:000E` (image `0x1380E`), its hook
//! at `1380:0119` (`0x13919`) and its animation hook at `1380:00F4`
//! (`0x138F4`), against [`Api`] alone. The location has no rules of its
//! own (rules pointer `0000:0000`). Addresses in comments are offsets in
//! segment `1380h`.
//!
//! The handler's prologue keeps a node in `si` across its frame loop; here
//! it is local 0.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 200.
const OBJECT_C8: u16 = 0xC8;
/// Object 203.
const OBJECT_CB: u16 = 0xCB;

/// `si` in the handler: the node the figure walks to on entry, or 0.
const WALK_NODE: usize = 0;

/// `[1CFAh]`: this location's state byte.
const STATE: u16 = 0x1CFA;
/// `[1CF0h]`: a state byte the handler's arm 2 switches on.
const STATE_1CF0: u16 = 0x1CF0;
/// `[1EEEh]`: non-zero while a spoken page drives objects 203 and 200.
const TALKING: u16 = 0x1EEE;
/// `[1EEAh]`: the animation the next page end asks for.
const TALK_WANT: u16 = 0x1EEA;
/// `[1EECh]`: the animation the two objects were last given.
const TALK_ANIM: u16 = 0x1EEC;
/// `[1204h]`: the walker record's word `+28h` (`DS:11DCh + 28h`; DS1
/// `[16BBh]`).
const WALKER_28: u16 = 0x1204;

/// Everything before the frame loop, `000E` to `00AA`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 000F
    let entry = api.word(ENTRY); // 0014
    let loc = api.word(LOCATION); // 0018
    api.set_word(HERE, loc); // 001B
    api.visit_enter(loc, entry); // 001F
    api.set_local(WALK_NODE, 0); // 0027

    // 0029: by `[1CFAh]`: 0, 1, 2; anything else goes to 008B with `si`
    // still 0. Every arm that plays a scene does so at 0045 and then sets
    // `si = C8h` (004B).
    let scene = match api.byte(STATE) {
        0 => {
            api.set_byte(STATE, 1); // 003E
            Some(1) // 0043
        }
        1 => Some(2), // 0050
        // 0054: by `[1CF0h]`.
        2 => match api.byte(STATE_1CF0) {
            3 => Some(4), // 006C
            4 => {
                api.set_byte(STATE, 3); // 0070
                api.obj_show(OBJECT_CB); // 0075
                api.obj_show(OBJECT_C8); // 007E
                Some(5) // 0087
            }
            _ => {
                api.set_byte(STATE_1CF0, 3); // 0063
                Some(3) // 0068
            }
        },
        _ => None,
    };
    if let Some(n) = scene {
        api.scene_play(n); // 0045
        api.set_local(WALK_NODE, OBJECT_C8); // 004B
    }
    // 008B: with a node set, task 0 armed and the figure sent to node 200.
    if api.local(WALK_NODE) != 0 {
        api.task_arm(0); // 0091
        api.walk_to_node(0xC8); // 0097
    }
    api.visit_finish_entry(); // 00A0
    api.set_word(ENTRY, 0); // 00A5
}

/// One turn of the frame loop, `00AB` to `00D3`. Answers whether the loop
/// goes on.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 00AB
    api.dispatch(); // 00B0
    // 00B5: once `[1204h]` equals the node, task 0 starts and the node is
    // forgotten.
    let node = api.local(WALK_NODE);
    if node != 0 && api.word(WALKER_28) == node {
        api.task_start(0); // 00C1
        api.set_local(WALK_NODE, 0); // 00C7
    }
    api.frame_pass(); // 00C9
    api.word(ENTRY) == 0 // 00CE
}

/// Everything after the frame loop, `00D5` to `00F3`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE); // 00D5
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 00E0
    api.visit_leave(); // 00E8
    api.slot_unload(); // 00ED
}

/// The hook, `1380:0119`: `code` at `[bp+6]`, `arg` at `[bp+8]`. Ten arms
/// through the table at `cs:0242` (0: `0132`, 5 and 6: `0168`, 7: `01B3`,
/// 8: `017C`, 9: `0185`, the rest and anything above 9: the return at
/// `0228`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0132: `[1EEEh] = arg & 6`; when set, both counters cleared and
        // both objects on animation 0.
        0 => {
            api.set_word(TALKING, arg & 6);
            if api.word(TALKING) != 0 {
                api.set_word(TALK_WANT, 0); // 0146
                api.set_word(TALK_ANIM, 0); // 0149
                api.obj_anim_id(OBJECT_CB, 0); // 014D
                api.obj_anim_id(OBJECT_C8, 0); // 0158
            }
        }
        // 0168: the flag cleared and both objects on animation 0; the
        // jump to 014D leaves `[1EEAh]` and `[1EECh]` as they are.
        5 | 6 => {
            if api.word(TALKING) != 0 {
                api.set_word(TALKING, 0); // 0172
                api.obj_anim_id(OBJECT_CB, 0); // 0178 -> 014D
                api.obj_anim_id(OBJECT_C8, 0); // 0158
            }
        }
        // 017C: `[1EEAh] = arg + 1`.
        8 => api.set_word(TALK_WANT, arg.wrapping_add(1)),
        // 0185: while the flag is set, both objects follow `[1EEAh]`.
        9 => {
            if api.word(TALKING) != 0 {
                let want = api.word(TALK_WANT); // 019B
                if api.word(TALK_ANIM) != want {
                    api.set_word(TALK_ANIM, want); // 019E
                    api.obj_anim_id(OBJECT_CB, num::signed(want)); // 01A1
                    api.obj_anim_id(OBJECT_C8, num::signed(want)); // 01AD -> 015A
                }
            }
        }
        // 01B3: script events, a sparse switch on `arg`: six values at
        // `cs:022A` (0Ah, 0Bh, 0Ch, 0Dh, 0Eh, 6Eh), targets at `cs:0236`.
        7 => match arg {
            0x0A => api.script_exit(0x64), // 01CF
            0x0B => {
                api.set_byte(STATE, 2); // 01D8
                api.play_sample(7); // 01DD
                api.inv_give(0x0A); // 01E5
            }
            0x0C => {
                api.set_word(TALKING, 0); // 01EE
                api.play_sample(0x16); // 01F4
                api.obj_anim_id(OBJECT_CB, 4); // 01FC -> 015D
            }
            0x0D => {
                api.set_word(TALKING, 0); // 0204
                api.play_sample(0x16); // 020A
                api.obj_anim_id(OBJECT_C8, 4); // 0212 -> 015A
            }
            0x0E => {
                api.walk_to_node(0x0BB8); // 0217
            }
            0x6E => api.set_word(TALK_WANT, 0), // 0222
            _ => {}                             // 01C9
        },
        _ => {} // 0228
    }
}

/// The animation hook, `1380:00F4`: on code 64h the object named by the
/// record's byte `+13h` is hidden and item 10 taken away.
pub(crate) fn anim_hook(api: &mut dyn Api, rec: Far, code: u16) {
    if code == 0x64 {
        // 00FF
        let id = api.far_byte(Far {
            off: rec.off.wrapping_add(0x13),
            seg: rec.seg,
        });
        api.obj_hide(u16::from(id)); // 0109
        api.inv_take(0x0A); // 0111
    }
}

/// Sharky has no rule handlers of its own.
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
