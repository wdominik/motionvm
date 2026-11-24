//! Location 2, the Zimmer: Karsten's room, where a fresh game opens.
//! The module is named after the location as the game itself names it in
//! its location table (`Zimmer`, the record's `+27h` name).
//!
//! Re-expressed from the handler at `0089:0079` (image `0x909`), its hook
//! at `0089:018B` (`0xA1B`) and its three rule handlers at `0089:0002`,
//! `0089:0036` and `0089:0069`, against [`Api`] alone. Every branch carries
//! the address it was read from.
//!
//! The original handler owns its frame loop; here it is cut into
//! [`enter`] (everything before the loop), [`frame`] (one turn of it) and
//! [`leave`] (everything after), which the engine calls in that order.

use super::logic::Logic;
use super::logic::WALKER;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::titles::parsec1994::Api;

/// The bed with Karsten in it, object 200.
const BED_OBJECT: u16 = 0xC8;
/// The drawer, object 203.
const DRAWER: u16 = 0xCB;

/// `[2725h]`: the location state every handler switches on.
const STATE: u16 = 0x2725;
/// `[270Fh]`: 2 while Karsten sleeps in the bed.
const ASLEEP: u16 = 0x270F;
/// `[2727h]`: the day.
const DAY: u16 = 0x2727;
/// `[2726h]`: a sub-state the day arms reset.
const SUBSTATE: u16 = 0x2726;
/// `[2711h]`: the drawer has been looked into.
const DRAWER_SEEN: u16 = 0x2711;
/// `[1892h]`: the opening is armed and waits for its second.
const OPENING: u16 = 0x1892;
/// `[1C82h]`: hundredths since the opening's task was armed.
const OPENING_CLOCK: u16 = 0x1C82;
/// `[1C76h]`: the ticks taken so far.
const TICKS_TAKEN: u16 = 0x1C76;
/// `[16BBh]`: the walker record's flag word (`DS:1693h + 28h`).
const WALKER_FLAGS: u16 = 0x16BB;

/// Everything before the frame loop, `0x909` to `0x9C2`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x909
    let entry = api.word(ENTRY); // 0x90E
    let loc = api.word(LOCATION); // 0x912
    api.set_word(HERE, loc); // 0x915
    api.visit_enter(loc, entry); // 0x919
    // 0x921: by the state, seven arms through the table at 0x0A0D.
    match api.byte(STATE) {
        0 => {
            // 0x934: the opening — Karsten in bed, scene 0, task 0 armed.
            api.set_byte(ASLEEP, 2);
            let bed = api.obj_find(BED_OBJECT);
            api.slot_set_object(0, 0, bed);
            api.scene_play(0);
            api.set_word(OPENING, 1);
            api.task_arm(0);
            api.set_word(OPENING_CLOCK, 0);
        }
        2 => {
            api.scene_play(4); // 0x96E
        }
        3 => {
            // 0x972: scene 7, once Karsten has been in bed.
            if api.byte(ASLEEP) != 0 {
                api.scene_play(7);
            }
        }
        5 => {
            api.scene_play(6); // 0x97D
        }
        _ => {} // 1, 4, 6 and above 6: 0x985
    }
    // 0x985: while he sleeps the bed animation is Karsten.
    if api.byte(ASLEEP) == 2 {
        let bed = api.obj_find(BED_OBJECT);
        api.slot_set_object(0, 0, bed);
        let walker = api.ds(WALKER);
        api.walker_detach(walker);
        api.obj_wake(BED_OBJECT);
    }
    api.visit_finish_entry(); // 0x9B8
    api.set_word(ENTRY, 0); // 0x9BD
}

/// One turn of the frame loop, `0x9C3` to `0x9ED`. Answers whether the
/// loop goes on.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x9C3
    api.dispatch(); // 0x9C8
    // 0x9CD: the opening's next page starts a second after it was armed.
    if api.word(OPENING) != 0 && api.word(OPENING_CLOCK) > 0x64 {
        api.task_start(0); // 0x9DD
    }
    api.frame_pass(); // 0x9E3
    api.word(ENTRY) == 0 // 0x9E8
}

/// Everything after the frame loop, `0x9EF` to `0xA0C`.
pub(crate) fn leave(api: &mut dyn Api) {
    // 0x9EF: this location's object states kept in its 32 bytes.
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave(); // 0xA02
    api.slot_unload(); // 0xA07
}

/// The hook, `0089:018B`: what the scene machine hands the location.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0xA2E: a page's end while the opening is on — task 0 armed again
        // and its clock from zero.
        9 => {
            if api.word(OPENING) != 0 {
                api.task_arm(0);
                api.set_word(OPENING_CLOCK, 0);
            }
        }
        // 0xA49: script events 10 to 15 through the table at 0xD18.
        7 => match arg {
            10 => wakes(api),
            11 => back_to_bed(api),
            12 => api.obj_anim_id(BED_OBJECT, 1), // 0xCD4
            13 => {
                api.walk_to_node(BED_OBJECT); // 0xCE3
            }
            14 => {
                api.inv_give(2); // 0xCED
                api.set_byte(DRAWER_SEEN, 1);
            }
            15 => {
                api.obj_hide(1); // 0xCFC
                api.inv_give(1);
            }
            _ => {}
        },
        _ => {}
    }
}

/// Event 10, `0xA5E`: Karsten wakes up, and the new day sets the story.
fn wakes(api: &mut dyn Api) {
    api.set_word(OPENING, 0);
    api.set_byte(0x26FE, 6);
    api.set_byte(0x270E, 0);
    api.set_byte(ASLEEP, 0);
    let day = api.byte(DAY).wrapping_add(1); // 0xA73
    api.set_byte(DAY, day);
    api.set_byte(0x2714, 1);
    api.set_byte(SUBSTATE, 0);
    api.set_byte(STATE, 9);
    // 0xA86: `[2727h] - 1` as an unsigned word, five arms through the
    // table at 0xD0E; anything else goes straight to the tail.
    let writes: &[(u16, u8)] = match u16::from(day).wrapping_sub(1) {
        0 => &[(0x26FE, 1), (0x26F9, 1), (SUBSTATE, 0), (STATE, 0)], // 0xA9D
        1 => &[
            // 0xAB4
            (0x2720, 0),
            (0x2717, 1),
            (0x271F, 0),
            (0x2716, 0x80),
            (0x271E, 0),
            (0x2715, 0x80),
            (SUBSTATE, 0),
            (STATE, 4),
            (0x26FF, 0x16),
        ],
        2 => &[
            // 0xAE4
            (0x271F, 0),
            (0x2716, 0x80),
            (0x271E, 0),
            (0x2715, 0x80),
            (0x26FB, 1),
            (0x2718, 1),
            (0x2720, 0),
            (0x2717, 3),
            (0x26FF, 0x1E),
            (0x2729, 5),
            (0x2714, 2),
            (0x2722, 0),
            (0x2719, 3),
        ],
        3 => &[
            // 0xB28
            (0x271F, 0),
            (0x2716, 0x80),
            (0x271E, 0),
            (0x2715, 0x80),
            (0x26FB, 0x18),
            (0x2720, 0),
            (0x2717, 5),
            (0x2714, 3),
            (0x26FF, 1),
            (0x2700, 0x24),
            (0x2706, 0x26),
            (0x2722, 0),
            (0x2719, 5),
            (0x272F, 0),
            (0x2729, 6),
            (0x2731, 0),
            (0x2728, 4),
        ],
        4 => &[
            // 0xB7F
            (0x26FE, 1),
            (0x272A, 0),
            (0x271C, 7),
            (0x271F, 0),
            (0x2716, 0x80),
            (0x271E, 0),
            (0x2715, 5),
            (0x26FB, 0x18),
            (0x2720, 0),
            (0x2717, 4),
            (0x26FF, 0x1C),
            (0x2700, 0x24),
            (0x2706, 0x14),
            (0x2703, 1),
            (0x272C, 0),
            (0x272B, 1),
            (0x272E, 0),
            (0x272D, 1),
            (0x272F, 0),
            (0x2729, 5),
            (0x2724, 0),
            (0x271B, 2),
            (0x271D, 2),
            (0x2714, 4),
        ],
        _ => &[],
    };
    for &(addr, v) in writes {
        api.set_byte(addr, v);
    }
    // 0xBF7: the bed emptied, the figure standing on node 200.
    api.obj_hide(BED_OBJECT);
    let flags = api.word(WALKER_FLAGS);
    api.set_word(WALKER_FLAGS, flags | 1); // 0xC00
    let walker = api.ds(WALKER);
    api.walker_snap(walker, BED_OBJECT, 2); // 0xC0E
    api.slot_set_figure(0, 0, walker); // 0xC1E
    redraw_after_effect(api); // 0xC26
}

/// Event 11, `0xC64`: back to bed — the opening armed again.
fn back_to_bed(api: &mut dyn Api) {
    api.set_byte(ASLEEP, 2);
    api.set_word(OPENING, 1);
    api.task_arm(0); // 0xC71
    api.set_word(OPENING_CLOCK, 0);
    let flags = api.word(WALKER_FLAGS);
    api.set_word(WALKER_FLAGS, flags & 0xFFFE); // 0xC7D
    let walker = api.ds(WALKER);
    api.walker_detach(walker); // 0xC86
    api.obj_wake(BED_OBJECT); // 0xC91
    redraw_after_effect(api); // 0xC97
}

/// The tail both events share (`0xC26`, `0xC97`): what `0EB9h:00B8h` does
/// after its first picture, copied inline — the reveal effect between.
fn redraw_after_effect(api: &mut dyn Api) {
    api.pointer_hide();
    api.anim_pass(0);
    api.room_redraw();
    api.panel_refresh();
    api.fx_reveal();
    api.pointer_unfreeze();
    api.key_flush();
    api.input_poll();
    api.input_poll();
    api.hover_on();
    api.set_word(TICKS_TAKEN, 0); // 0xC5B, 0xCCC
}

/// The Zimmer's rule handlers by their offset in segment `0089h`. Answers
/// what the original answers the rule search with, or `None` for an
/// offset that is not one of them.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 0089:0002, Benutze the bed: `cmp [270Fh], 0 ; je` (0x895) — scene
        // 1 when [270Fh] is clear, else by the day (0x89C): 2: 7, 3: 5,
        // anything else: 6.
        0x0002 => {
            let n = if api.byte(ASLEEP) == 0 {
                1
            } else {
                match api.byte(DAY) {
                    2 => 7,
                    3 => 5,
                    _ => 6,
                }
            };
            api.scene_play(n);
            0x0A
        }
        // 0089:0036, Betrachte the drawer: only while object 203 is live;
        // scene 2 the first time, 9 after.
        0x0036 => {
            if api.obj_state(DRAWER) == 0 {
                0
            } else {
                if api.byte(DRAWER_SEEN) != 0 {
                    api.scene_play(9);
                } else {
                    api.set_byte(DRAWER_SEEN, 1);
                    api.scene_play(2);
                }
                0x0A
            }
        }
        // 0089:0069, Nimm the toolbox: scene 3.
        0x0069 => {
            api.scene_play(3);
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
    anim_hook: None,
    rule,
};
