//! Location 1, the Karte: the town map, where a place is chosen. The
//! module is named after the location as the game itself names it in its
//! location table (`Karte`).
//!
//! Re-expressed from the handler at `0060:008E` (image `0x68E`), its hook
//! at `0060:01C6` (`0x7C6`) and its four rule handlers at `0060:000A`,
//! `0060:0062`, `0060:0070` and `0060:007E`, against [`Api`] alone.
//!
//! The places on the map are objects 200 to 223, one for each location
//! 0 to 23: object `200 + n` stands for location `n`.

use super::logic::Logic;
use super::logic::WALKER;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The first place object; object `PLACES + n` is location `n`.
const PLACES: u16 = 0xC8;
/// Object 202, shown on the frame of the day.
const DAY_MARKER: u16 = 0xCA;
/// Object 130, whose point the hook's code 0 names.
const OBJECT_130: u16 = 0x82;

/// `[2891h]`: the object the click chose.
const OBJECT: u16 = 0x2891;
/// `[288Dh]`: the verb.
const VERB: u16 = 0x288D;
/// `DS:262Eh + object`: per place object, the scene it plays when it will
/// not be gone to (2 and above), with bit 0 set once played.
const PLACE_SCENES: u16 = 0x262E;
/// `DS:26F6h + n`: whether place `n` is shown on the map.
const PLACE_SHOWN: u16 = 0x26F6;
/// `[2727h]`: the day.
const DAY: u16 = 0x2727;
/// `[2730h]`: added to location 8 when the map goes there.
const BYTE_2730: u16 = 0x2730;
/// `[184Eh]`, `[1852h]`: the event record the poll fills — its kind and,
/// for a key, its code.
const INPUT_EVENT: u16 = 0x184E;
const INPUT_KEY: u16 = 0x1852;
/// `[1890h]`, `[188Eh]`, `[188Ch]`: whether the marker is in the
/// dialogue, the animation it is to play, the one it plays.
const MARKER_SPEAKS: u16 = 0x1890;
const MARKER_NEXT: u16 = 0x188E;
const MARKER_PLAYING: u16 = 0x188C;
/// `204Bh:003Bh`: the location table's row 1, its kind word (`+7`).
const MAP_KIND: Far = Far {
    off: 0x3B,
    seg: 0x204B,
};

/// Everything before the frame loop, `0x68E` to `0x747`.
pub(crate) fn enter(api: &mut dyn Api) {
    // 0x68F: arriving by exits 106 to 109, the entry is where the player
    // came from — object `200 + [1F4Ch]` (the location just left), or 208
    // from beyond the first 24.
    let entry = api.word(ENTRY);
    if (0x6A..=0x6D).contains(&entry) {
        let from = num::signed(api.word(HERE));
        let place = if from < 0x18 {
            num::unsigned(from).wrapping_add(PLACES)
        } else {
            0xD0
        };
        api.set_word(ENTRY, place);
    }
    api.visit_setup(); // 0x6BA
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc);
    api.visit_enter(loc, entry); // 0x6CA
    // 0x6D2: places 23 down to 0, shown or hidden by their byte.
    for n in (0..0x18u16).rev() {
        if api.byte(PLACE_SHOWN.wrapping_add(n)) != 0 {
            api.obj_show(n.wrapping_add(PLACES));
        } else {
            api.obj_hide(n.wrapping_add(PLACES));
        }
    }
    // 0x6FE: with [2719h] at 5, [2706h] says whether items 18 and 22 are
    // both carried.
    if api.byte(0x2719) == 5 {
        if api.inv_has(0x12) != 0 && api.inv_has(0x16) != 0 {
            api.set_byte(0x2706, 1);
        } else if api.byte(0x2706) == 1 {
            api.set_byte(0x2706, 0x26);
        }
    }
    // 0x730: the marker on the frame of the day.
    let day = u16::from(api.byte(DAY)).wrapping_sub(1);
    api.obj_show_frame(DAY_MARKER, num::signed(day));
    api.visit_finish_entry(); // 0x742
    api.set_word(ENTRY, 0); // 0x747
}

/// One turn of the frame loop, `0x74D` to `0x79D`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    // 0x757: three keys of the developers' — 501h toggles [270Eh], 502h
    // and 503h set the map's own kind in the location table to 2 or 1.
    if api.word(INPUT_EVENT) == 0xFFFF {
        match api.word(INPUT_KEY) {
            0x501 => {
                let v = api.byte(0x270E) ^ 1;
                api.set_byte(0x270E, v);
            }
            0x502 => api.set_far_word(MAP_KIND, 2),
            0x503 => api.set_far_word(MAP_KIND, 1),
            _ => {}
        }
    }
    api.frame_pass(); // 0x793
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x79F` to `0x7BD`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `0060:01C6`: codes 0 to 9 through the table at `0x87E`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x7DE: a dialogue comes up; bit 0 of its speakers is the
            // marker, which stands on animation 1, and slot 0 names the
            // point (130, 64).
            api.set_word(MARKER_SPEAKS, arg & 1);
            api.set_word(MARKER_NEXT, 1);
            api.set_word(MARKER_PLAYING, 1);
            let marker = api.obj_show(PLACES);
            api.obj_anim(marker, 1);
            api.slot_set_raw(0, 3, OBJECT_130, 0x40);
        }
        5 | 6 => {
            // 0x816: the dialogue over, the marker gone and slot 0 the
            // player again.
            api.obj_hide(PLACES);
            let walker = api.ds(WALKER);
            api.slot_set_figure(0, 0, walker);
            api.set_word(MARKER_SPEAKS, 0);
        }
        8 => {
            // 0x837: the next animation, at most 2.
            let next = arg.wrapping_add(1);
            api.set_word(MARKER_NEXT, if num::signed(next) > 2 { 2 } else { next });
        }
        9 => {
            // 0x84C
            if api.word(MARKER_SPEAKS) != 0 && api.word(MARKER_NEXT) != api.word(MARKER_PLAYING) {
                let next = api.word(MARKER_NEXT);
                api.set_word(MARKER_PLAYING, next);
                api.obj_anim_id(PLACES, num::signed(next));
            }
        }
        7 => {
            // 0x86D: event 110.
            if arg == 0x6E {
                api.set_word(MARKER_NEXT, 0);
            }
        }
        _ => {}
    }
}

/// The map's rule handlers by their offset in segment `0060h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 0060:000A, a place chosen: a scene instead while its byte says
        // so, else exit 109 to the place's location.
        0x000A => {
            let object = api.word(OBJECT);
            let at = PLACE_SCENES.wrapping_add(object);
            let scenes = api.byte(at);
            if scenes >= 2 {
                api.scene_play(i16::from(scenes) - 2); // 0x621
                api.set_byte(at, scenes | 1);
                0x0A
            } else {
                // 0x639
                api.input_lock();
                api.set_word(ENTRY, 0x6D);
                let mut dest = object.wrapping_sub(PLACES);
                api.set_word(LOCATION, dest);
                if dest == 8 {
                    dest = dest.wrapping_add(u16::from(api.byte(BYTE_2730)));
                    api.set_word(LOCATION, dest);
                }
                1
            }
        }
        // 0060:0062 and 0060:0070: the verb becomes 0Dh; answer 5 and 1.
        0x0062 => {
            api.set_word(VERB, 0x0D);
            5
        }
        0x0070 => {
            api.set_word(VERB, 0x0D);
            1
        }
        // 0060:007E: scene 32h.
        0x007E => {
            api.scene_play(0x32);
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
