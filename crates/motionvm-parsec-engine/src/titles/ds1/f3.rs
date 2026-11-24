//! Location 29, F3. The module is named after the location as the game
//! itself names it in its location table (`F3`).
//!
//! Re-expressed from the handler at `0485:000E` (image `0x485E`) and its
//! hook at `0485:01C6` (`0x4A16`), against [`Api`] alone. The location has
//! no rules of its own.
//!
//! The handler's prologue differs from the standard one: it hands
//! `[1F4Ah]` to the shared entry directly and writes `[1F4Ch]` only after
//! `0EB9:00B8`. It keeps three locals across its frame loop — `si`, a step
//! through the room's palette, and the far pointer to that palette in
//! `[bp-4]`/`[bp-2]` — and its loop has two checks of its own: task 0
//! started half a second after an event asks for it, as in F2, and DAC
//! entry 42h faded up or down through the palette's colors 2 to 15 while
//! `[1952h]` says so.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT_STATES};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Objects 200, 202 and 204, which the hook animates together.
const OBJECT_200: u16 = 0xC8;
const OBJECT_202: u16 = 0xCA;
const OBJECT_204: u16 = 0xCC;

/// Handler local 0: `si`, the color of the palette DAC entry 42h shows;
/// 2 at the entry (`0x4863`).
const STEP: usize = 0;
/// Handler locals 1 and 2: `[bp-2]` and `[bp-4]`, segment and offset of the
/// room's first palette's colors.
const PALETTE_SEG: usize = 1;
const PALETTE_OFF: usize = 2;

/// `[2851h]`: the loaded room's header, a far pointer; its `+1Ch` points
/// to the room's palette table.
const ROOM_HEADER: u16 = 0x2851;
/// `[1952h]`: 1 fades DAC entry 42h up, 2 down, 0 neither.
const FADE: u16 = 0x1952;
/// `[1954h]`: non-zero, task 0 is to be started; its value less one is the
/// animation to come back to.
const TASK_PENDING: u16 = 0x1954;
/// `[1956h]`: the animation the three objects play.
const PLAYING_ANIM: u16 = 0x1956;
/// `[1958h]`: the animation they are to play at the next page's end.
const NEXT_ANIM: u16 = 0x1958;
/// `[1C82h]`: the hundredths the scene machine counts.
const HUNDREDTHS: u16 = 0x1C82;

/// The DAC entry the fade writes.
const FADE_ENTRY: u16 = 0x42;

/// Everything before the frame loop, `0x485E` to `0x4939`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.set_local(STEP, 2); // 0x4863
    api.visit_setup(); // 0x4866
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.visit_enter(loc, entry); // 0x4873
    // 0x487B: the room header's palette table, its first entry, and past
    // that palette's first word, its colors.
    let header = Far {
        off: api.word(ROOM_HEADER),
        seg: api.word(ROOM_HEADER.wrapping_add(2)),
    };
    let table_at = Far {
        off: header.off.wrapping_add(0x1C),
        seg: header.seg,
    };
    let table = Far {
        off: api.far_word(table_at),
        seg: api.far_word(Far {
            off: table_at.off.wrapping_add(2),
            seg: table_at.seg,
        }),
    };
    let seg = api.far_word(Far {
        off: table.off.wrapping_add(2),
        seg: table.seg,
    }); // 0x4883
    let off = api.far_word(table).wrapping_add(2); // 0x4887
    api.set_local(PALETTE_SEG, seg); // 0x488D
    api.set_local(PALETTE_OFF, off);
    api.hover_off(); // 0x4893
    api.obj_wake(OBJECT_200); // 0x489B
    api.obj_wake(OBJECT_202); // 0x48A4
    let rec = api.obj_wake(OBJECT_204); // 0x48AD
    // 0x48B3: slot 0 names object 204 (the record left in DX:AX), slots 4
    // to 9 six points on the line y = 120.
    api.slot_set_object(0, 0, rec); // 0x48B9
    api.slot_set_raw(4, 3, 0x96, 0x78); // 0x48CA
    api.slot_set_raw(5, 3, 0xAA, 0x78); // 0x48DB
    api.slot_set_raw(6, 3, 0x82, 0x78); // 0x48EC
    api.slot_set_raw(7, 3, 0xBE, 0x78); // 0x48FD
    api.slot_set_raw(8, 3, 0x6E, 0x78); // 0x490D
    api.slot_set_raw(9, 3, 0xD2, 0x78); // 0x491E
    api.scene_play(0); // 0x4928
    api.visit_finish_entry(); // 0x492E
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x4936
    api.set_word(ENTRY, 0); // 0x4939
}

/// DAC entry 42h set to color `si` of the room's palette (`0x4990`,
/// `0x49C7`).
fn show_step(api: &mut dyn Api, step: u16) {
    let rgb = Far {
        off: api.local(PALETTE_OFF).wrapping_add(step.wrapping_mul(3)),
        seg: api.local(PALETTE_SEG),
    };
    api.pal_set(rgb, FADE_ENTRY, 1);
}

/// One turn of the frame loop, `0x493F` to `0x49F3`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    // 0x4949: task 0 started once more than 50 hundredths have passed
    // since the event asked for it (`jbe`: unsigned).
    let pending = api.word(TASK_PENDING);
    if pending != 0 && api.word(HUNDREDTHS) > 0x32 {
        api.set_word(NEXT_ANIM, pending.wrapping_sub(1)); // 0x495B
        api.set_word(TASK_PENDING, 0);
        api.task_start(0); // 0x4966
    }
    // 0x496C: the fade, one color per tenth of a second.
    match api.word(FADE) {
        1 if api.word(HUNDREDTHS) > 0x0A => {
            let step = api.local(STEP).wrapping_add(1); // 0x4982
            api.set_local(STEP, step);
            if num::signed(step) < 0x10 {
                show_step(api, step); // 0x499C
            } else {
                api.set_word(FADE, 0); // 0x49A6
            }
            api.set_word(HUNDREDTHS, 0); // 0x49AC
        }
        2 if api.word(HUNDREDTHS) > 0x0A => {
            let step = api.local(STEP).wrapping_sub(1); // 0x49BB
            api.set_local(STEP, step);
            if num::signed(step) >= 2 {
                show_step(api, step); // 0x49D5
            } else {
                api.set_word(FADE, 0); // 0x49DF
            }
            api.set_word(HUNDREDTHS, 0); // 0x49AC
        }
        _ => {}
    }
    api.frame_pass(); // 0x49E7
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x49F6` to `0x4A15`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `0485:01C6`: codes 7 to 10 through the table at `0x4AFC`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        8 => {
            // 0x4A31: the next animation, back to 0 above 3.
            api.set_word(NEXT_ANIM, arg.wrapping_add(1));
            if num::signed(api.word(NEXT_ANIM)) > 3 {
                api.set_word(NEXT_ANIM, 0); // 0x4A41
            }
        }
        9 | 10 => {
            // 0x4A4A: a page's end — the next animation when it changed,
            // or 0 while task 0 waits to start.
            if api.word(PLAYING_ANIM) != api.word(NEXT_ANIM) {
                if api.word(TASK_PENDING) != 0 {
                    api.set_word(PLAYING_ANIM, 0); // 0x4A5D
                } else {
                    let next = api.word(NEXT_ANIM);
                    api.set_word(PLAYING_ANIM, next); // 0x4A68
                }
                let anim = num::signed(api.word(PLAYING_ANIM));
                api.obj_anim_id(OBJECT_200, anim); // 0x4A72
                let anim = num::signed(api.word(PLAYING_ANIM));
                api.obj_anim_id(OBJECT_202, anim); // 0x4A81
                let anim = num::signed(api.word(PLAYING_ANIM));
                api.obj_anim_id(OBJECT_204, anim); // 0x4A90
            }
        }
        7 => event(api, arg), // 0x4A9A
        _ => {}
    }
}

/// Code 7, `0x4A9A`: script events 10 to 20 through the table at `0x4AE6`.
fn event(api: &mut dyn Api, n: u16) {
    match n {
        0x0A => {
            // 0x4AAB: task 0 armed, to start in the frame loop.
            api.task_arm(0);
            let next = api.word(NEXT_ANIM).wrapping_add(1); // 0x4AB3
            api.set_word(TASK_PENDING, next);
            api.set_word(NEXT_ANIM, 0);
            api.set_word(HUNDREDTHS, 0); // 0x4AD0
        }
        0x0B => {
            api.set_word(FADE, 1); // 0x4AC2: fade up
            api.set_word(HUNDREDTHS, 0);
        }
        0x0C => {
            api.set_word(FADE, 2); // 0x4ACA: fade down
            api.set_word(HUNDREDTHS, 0);
        }
        0x14 => {
            // 0x4AD8: to location 27, entry 102.
            api.set_word(LOCATION, 0x1B);
            api.set_word(ENTRY, 0x66);
        }
        _ => {}
    }
}

/// The location has no rules of its own (its record's `+2Bh` is null).
pub(crate) fn rule(_api: &mut dyn Api, _off: u16, _flags: u16) -> Option<i16> {
    None
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
