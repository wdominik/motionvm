//! Location 5, Ralf. The module is named after the location as the game
//! itself names it in its location table (`Ralf`).
//!
//! Re-expressed from the handler at `0140:0057` (image `0x1457`), its hook
//! at `0140:00BB` (`0x14BB`) and its one rule handler at `0140:0009`,
//! against [`Api`] alone.

use super::logic::Logic;
use super::logic::WALKER;
use crate::num;
use crate::titles::parsec1994::Api;

/// Object 128, whose animations follow the dialogue.
const PERSON: u16 = 0x80;

/// `[2718h]`: the state the rule switches on.
const STATE: u16 = 0x2718;
/// `[18AAh]` whether object 128 speaks in the dialogue that is up,
/// `[18A8h]` its next animation, `[18A6h]` the one it is playing.
const SPEAKS: u16 = 0x18AA;
const NEXT_ANIM: u16 = 0x18A8;
const PLAYING_ANIM: u16 = 0x18A6;
/// `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`, `DS:1F56h`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
const HERE: u16 = 0x1F4C;
const OBJECT_STATES: u16 = 0x1F56;

/// Everything before the frame loop, `0x1457` to `0x1481`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x1457
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x1463
    api.visit_enter(loc, entry); // 0x1467
    api.obj_anim_id(PERSON, 0); // 0x1474
    api.visit_finish_entry(); // 0x147C
    api.set_word(ENTRY, 0); // 0x1481
}

/// One turn of the frame loop, `0x1487` to `0x149B`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x149D` to `0x14BA`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `0140:00BB`: codes 0 to 9 through the table at `0x157A`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x14D3: a dialogue comes up; speaker bit 1 is object 128.
            api.set_word(SPEAKS, arg & 2);
            api.set_word(PLAYING_ANIM, 0xFFFF);
            api.set_word(NEXT_ANIM, 0xFFFF);
        }
        5 | 6 => {
            // 0x14E7: if it spoke, back to animation 0.
            if api.word(SPEAKS) != 0 {
                api.set_word(SPEAKS, 0); // 0x14F3
                api.set_word(PLAYING_ANIM, 0);
                api.set_word(NEXT_ANIM, 0);
                api.obj_anim_id(PERSON, 0); // 0x1500
            }
        }
        8 => {
            // 0x150A: joins 0x14E1, which writes [18A8h] alone.
            if api.word(SPEAKS) != 0 {
                api.set_word(NEXT_ANIM, arg.wrapping_add(1));
            }
        }
        9 => {
            // 0x1516: a new animation to play; [18AAh] is not asked.
            if api.word(PLAYING_ANIM) != api.word(NEXT_ANIM) {
                let next = api.word(NEXT_ANIM);
                api.set_word(PLAYING_ANIM, next);
                api.obj_anim_id(PERSON, num::signed(next)); // 0x1529
            }
        }
        7 => match arg {
            // 0x1533: script events 0Ah, 0Bh, 6Eh.
            0x0A => {
                let walker = api.ds(WALKER);
                api.walker_walk_to(walker, PERSON, 3); // 0x154F
            }
            0x0B => {
                api.inv_give(0x0F); // 0x155B
                api.set_byte(0x26FB, 0x18);
                api.set_byte(0x2721, 0);
                api.set_byte(STATE, 2);
            }
            0x6E => api.set_word(NEXT_ANIM, 0), // 0x1572
            _ => {}
        },
        _ => {} // 1 to 4 and above 9: 0x1578
    }
}

/// Ralf's rule handler by its offset in segment `0140h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    match off {
        // 0140:0009, Sprich mit object 128, by [2718h].
        0x0009 => Some(match api.byte(STATE) {
            0 => {
                // 0x141C
                api.scene_play(0);
                api.set_byte(0x26FB, 0x18);
                api.set_byte(0x2704, 1);
                api.set_byte(0x2722, 0);
                api.set_byte(0x2719, 1);
                api.set_byte(0x2721, 0);
                api.set_byte(STATE, 2);
                0x0A
            }
            1 => {
                api.scene_play(1); // 0x1446
                0x0A
            }
            _ => 0, // 0x144E
        }),
        _ => None,
    }
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
