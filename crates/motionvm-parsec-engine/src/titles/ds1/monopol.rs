//! Location 10, the Monopol. The module is named after the location as the
//! game itself names it in its location table (`Monopol`).
//!
//! Re-expressed from the handler at `025E:0175` (image `0x2755`), its hook
//! at `025E:0371` (`0x2951`) and its seven rule handlers at `025E:000A`,
//! `0033`, `0070`, `00A5`, `010E`, `012E` and `0165`, against [`Api`] alone.
//!
//! The location's own record in the location table (`204Bh:0208h`, record
//! 10) holds ten object ids at `+19h` to `+22h`, the people the hook
//! animates. The handler rewrites the second of them (`204Bh:0222h`) by
//! the location's state and reads it back for subject slot 2.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// `204Bh`, the location table's segment, as the program loads it.
const TABLE_SEG: u16 = 0x204B;
/// `204Bh:0221h + i`, i = 0..9: the ids at `+19h` of record 10 (the hook
/// indexes from `221h`, so `+1Ah` is the one at `222h`).
const PEOPLE: u16 = 0x0221;
/// `204Bh:0222h`, the person the handler names for subject slot 2.
const PERSON: u16 = 0x0222;

/// `[272Dh]`: location state; `[272Eh]`: its scene counter.
const STATE: u16 = 0x272D;
const COUNTER: u16 = 0x272E;
/// `DS:18F4h + i` whether person i speaks, `DS:18EAh + i` the animation
/// he is to play, `DS:18E0h + i` the one he plays.
const SPEAKS: u16 = 0x18F4;
const NEXT: u16 = 0x18EA;
const PLAYING: u16 = 0x18E0;

fn table(off: u16) -> Far {
    Far {
        seg: TABLE_SEG,
        off,
    }
}

/// `mov es, 204Bh ; mov es:[222h], dl ; push dx ; call 0883:20B3`
/// (`0x278B`): the person named and woken.
fn name_and_wake(api: &mut dyn Api, id: u8) {
    api.set_far_byte(table(PERSON), id);
    api.obj_wake(u16::from(id));
}

/// Everything before the frame loop, `0x2755` to `0x28FD`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup();
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x2761
    api.visit_enter(loc, entry); // 0x2765
    // 0x276D: by [272Dh] - 2 through the table at cs:0363h.
    let state = api.byte(STATE);
    let mut reset = false;
    match state.wrapping_sub(2) {
        0 => name_and_wake(api, 0xC8), // 0x2786
        1 => {
            // 0x27A0
            api.set_far_byte(table(PERSON), 0x82);
            api.obj_show_frame(0x82, 7);
        }
        2 => {
            // 0x27BE
            api.walk_to_node(0x0BB8);
            name_and_wake(api, 0x83);
            for id in [0xCD, 0xCF, 0xD1, 0xD3, 0xCA, 0xD5, 0xD9] {
                api.obj_wake(id); // 0x27DE..0x2817
            }
            api.obj_show(0xCC); // 0x2820
            let counter = api.ds(COUNTER);
            api.scene_sequence(counter, &[5, 6]); // 0x2830
        }
        3..=6 => {
            // 0x283A: once item 11h is carried, the state resets.
            if api.inv_has(0x11) != 0 {
                reset = true;
            } else {
                name_and_wake(api, 0x83); // 0x2846
            }
        }
        _ => reset = true,
    }
    if reset {
        // 0x284E
        api.set_byte(COUNTER, 0);
        api.set_byte(STATE, 1);
    }
    // 0x2858: subject slot 2 names the person at 204Bh:0222h.
    let person = api.far_byte(table(PERSON));
    let rec = api.obj_find(u16::from(person));
    api.slot_set_object(2, 0, rec); // 0x2870
    if api.byte(STATE) != 4 {
        // 0x287F: one of six arrangements of the guests, through the
        // table at cs:0357h.
        match api.random(0, 5) {
            0 => {
                // 0x2899
                api.obj_show(0xCC);
                api.obj_wake(0xD5);
                api.obj_wake(0xD7);
                api.obj_wake(0xCA);
            }
            1 => {
                // 0x28B6
                api.obj_wake(0xD5);
                api.obj_wake(0xD7);
                api.obj_wake(0xD9);
            }
            2 => {
                // 0x28CD, into 0x28C7
                api.obj_show(0xCC);
                api.obj_wake(0xD9);
            }
            3 => {
                // 0x28D7, into 0x28C2
                api.obj_wake(0xCA);
                api.obj_wake(0xD9);
            }
            4 => {
                // 0x28DC, into 0x28B3
                api.obj_show(0xCC);
                api.obj_wake(0xCA);
            }
            5 => {
                api.obj_wake(0xCA); // 0x28E6
            }
            _ => {}
        }
    }
    api.obj_wake(0xC9); // 0x28EF
    api.visit_finish_entry(); // 0x28F8
    api.set_word(ENTRY, 0); // 0x28FD
}

/// One turn of the frame loop, `0x2903` to `0x2917`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x2919` to `0x2936`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The people the hook loops over: `mov [bp-2], 0Ah` and a test that
/// decrements after it, so the body sees 9 down to 0.
const PEOPLE_DOWN: [u16; 10] = [9, 8, 7, 6, 5, 4, 3, 2, 1, 0];

/// Code 9's loop, `0x2A09`: each person whose next animation is not the
/// one he plays starts it.
fn next_animations(api: &mut dyn Api) {
    for i in PEOPLE_DOWN {
        let next = api.byte(NEXT + i);
        if api.byte(PLAYING + i) != next {
            // 0x2A1D
            api.set_byte(PLAYING + i, next);
            let id = api.far_byte(table(PEOPLE + i));
            api.obj_anim_id(u16::from(id), i16::from(next));
        }
    }
}

/// The hook, `025E:0371`: codes 0 to 9 through the table at cs:04DDh.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x296A: a dialogue comes up; speaker bit i+1 is person i.
            for i in PEOPLE_DOWN {
                let speaks = arg & (2u16 << i) != 0;
                api.set_byte(SPEAKS + i, u8::from(speaks));
                api.set_byte(PLAYING + i, 0);
                api.set_byte(NEXT + i, 0);
            }
        }
        5 | 6 => {
            // 0x29A2: whoever spoke goes back to animation 0.
            for i in PEOPLE_DOWN {
                if api.byte(SPEAKS + i) != 0 {
                    api.set_byte(SPEAKS + i, 0); // 0x29B3
                    api.set_byte(NEXT + i, 0);
                    api.set_byte(PLAYING + i, 0);
                    let id = api.far_byte(table(PEOPLE + i));
                    api.obj_anim_id(u16::from(id), 0);
                }
            }
        }
        8 => {
            // 0x29E6: each speaker's next animation is arg + 1; then on as
            // code 9 (0x2A09).
            for i in PEOPLE_DOWN {
                if api.byte(SPEAKS + i) != 0 {
                    api.set_byte(NEXT + i, num::low8(arg).wrapping_add(1)); // 0x29F7
                }
            }
            next_animations(api);
        }
        9 => next_animations(api),
        7 => event(api, arg), // 0x2A49
        _ => {}
    }
}

/// Code 7, `0x2A49`: script events 0Ah to 0Fh through the table at
/// cs:04D1h.
fn event(api: &mut dyn Api, n: u16) {
    match n.wrapping_sub(0x0A) {
        0 => {
            // 0x2A5A
            api.set_byte(0x2700, 0x20);
            api.set_byte(0x2703, 1);
            api.set_byte(0x2720, 0);
            api.set_byte(0x2717, 7);
            api.set_byte(COUNTER, 0);
            api.set_byte(STATE, 1);
            api.walk_to_node(0xC9);
        }
        1 => {
            api.inv_give(0x15); // 0x2A82
        }
        2 => {
            // 0x2A86
            api.inv_give(0x11);
            api.set_byte(COUNTER, 0);
            api.set_byte(STATE, 1);
        }
        3 => {
            api.inv_take(0x10); // 0x2A9A
        }
        4 => {
            api.inv_take(0x15); // 0x2A9E
        }
        5 => {
            api.inv_give(0x0D); // 0x2AA7
        }
        _ => {}
    }
}

/// The Monopol's rule handlers by their offset in segment `025Eh`. Answers
/// what the original answers the rule search with, or `None` for an
/// offset that is not one of them.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 025E:000A (0x25EA)
        0x000A => {
            api.scene_play(0);
            api.set_byte(0x2714, 1);
            api.set_byte(COUNTER, 0);
            api.set_byte(STATE, 1);
            api.set_byte(0x2724, 0);
            api.set_byte(0x271B, 1);
            0x0A
        }
        // 025E:0033 (0x2613): by the state.
        0x0033 => {
            match api.byte(STATE) {
                1 => {
                    // 0x2634
                    let n = api.random(1, 4);
                    api.scene_play(num::signed(n));
                }
                6 => {
                    // 0x2625
                    api.scene_play(0x10);
                    api.set_byte(STATE, 7);
                }
                _ => {
                    api.scene_play(4); // 0x2643
                }
            }
            0x0A
        }
        // 025E:0070 (0x2650)
        0x0070 => {
            api.scene_play(8);
            api.set_byte(0x2700, 0x24);
            api.set_byte(COUNTER, 0);
            api.set_byte(STATE, 1);
            if api.inv_has(0x14) != 0 {
                // 0x2676
                api.set_byte(0x2724, 0);
                api.set_byte(0x271B, 6);
            }
            0x0A
        }
        // 025E:00A5 (0x2685): by the state; 0 for any other.
        0x00A5 => match api.byte(STATE) {
            5 => {
                // 0x269E
                api.scene_play(0x0A);
                api.set_byte(STATE, 6);
                0x0A
            }
            6 => {
                // 0x26AD
                let n = if api.inv_has(0x10) != 0 { 0x0B } else { 0x0C };
                api.scene_play(n);
                0x0A
            }
            7 => {
                // 0x26BF
                let n = if api.inv_has(0x15) == 0 {
                    0x0D
                } else if api.inv_has(0x10) != 0 {
                    0x0B
                } else {
                    0x0C
                };
                api.scene_play(n);
                0x0A
            }
            _ => 0, // 0x26E5
        },
        // 025E:010E (0x26EE)
        0x010E => {
            let n = if api.inv_has(0x10) != 0 { 0x0E } else { 0x11 };
            api.scene_play(n);
            0x0A
        }
        // 025E:012E (0x270E): by the state; 0 for any other.
        0x012E => match api.byte(STATE) {
            6 => {
                api.scene_play(0x0F); // 0x2722
                0x0A
            }
            7 => {
                // 0x272C
                let n = if api.inv_has(0x15) != 0 { 0x0F } else { 0x11 };
                api.scene_play(n);
                0x0A
            }
            _ => 0, // 0x273C
        },
        // 025E:0165 (0x2745)
        0x0165 => {
            api.scene_play(0x12);
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
