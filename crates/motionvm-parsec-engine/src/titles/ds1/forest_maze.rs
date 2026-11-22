//! Locations 30 to 59, the forest maze: thirty rows of kind 3 that share
//! one handler, one rules table and both hooks. The location table gives
//! them no name, so the module has an English one.
//!
//! Re-expressed from the handler at `04B0:0223` (image `0x4D23`), its
//! helpers at `04B0:0059`, `00AB`, `013D` and `01CD`, the rule handler at
//! `04B0:0004` (both rules of the table at `1FFC:04C0` name it), the hook
//! at `04B0:0324` and the animation hook at `04B0:031F`, against [`Api`]
//! alone.
//!
//! The maze keeps a byte per location at `1FFC:0482h + n`, outside
//! `DGROUP`. On the first visit (row 30's byte still the image's 58h) the
//! handler fills it with each maze row's distance from the three ways
//! out (rows 57, 30 and 44), then overwrites it, row by row, with the
//! exit that leads one step closer. The rule walks the player through
//! that exit.

use super::logic::Logic;
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// `1FFCh`: the segment of the maze's per-location bytes.
const MAZE_SEG: u16 = 0x1FFC;
/// `1FFC:0482h + n`: location `n`'s byte.
const BY_LOCATION: u16 = 0x0482;
/// `1FFC:04A0h`: the bytes of the maze rows, 30 onwards.
const ROWS: u16 = 0x04A0;
/// `204Bh`: the location table, `34h` bytes a row.
const TABLE_SEG: u16 = 0x204B;
const STRIDE: u16 = 0x34;
/// A row's exit numbers from `+9`, its neighbors from `+11h`.
const EXITS: u16 = 0x09;
const NEIGHBORS: u16 = 0x11;
/// The first maze row and the row after the last (`cmp si, 1Eh / 3Ch`).
const FIRST: u16 = 0x1E;
const END: u16 = 0x3C;

/// `[2732h]`: visits since the last scene; `[2733h]`: how many visits
/// the next scene waits.
const VISITS: u16 = 0x2732;
const PATIENCE: u16 = 0x2733;
/// `[195Ah]`: the last neighbor `04B0:0059` answered.
const LAST_STEP: u16 = 0x195A;
/// `[1C82h]`: the hundredths the scene machine counts.
const HUNDREDTHS: u16 = 0x1C82;
/// `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`, `DS:1F56h`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
const HERE: u16 = 0x1F4C;
const OBJECT_STATES: u16 = 0x1F56;

/// Handler local 0: `si`, the hundredths until the next forest sound.
const SOUND_DUE: usize = 0;

fn maze(off: u16) -> Far {
    Far { off, seg: MAZE_SEG }
}

fn row_byte(api: &mut dyn Api, at: u16) -> u8 {
    api.far_byte(Far {
        off: at,
        seg: TABLE_SEG,
    })
}

/// `04B0:0059(n, k, d)`: neighbor `k` of location `n`, if it is a maze
/// row whose byte is above `d + 1`, gets `d + 1` and is answered; else 0.
fn step(api: &mut dyn Api, n: u16, k: u16, d: u16) -> u16 {
    let at = n.wrapping_mul(STRIDE).wrapping_add(k);
    let m = u16::from(row_byte(api, at.wrapping_add(NEIGHBORS))); // 0x4B6E
    if !(FIRST..END).contains(&m) {
        return 0; // 0x4BA6
    }
    let here = u16::from(api.far_byte(maze(BY_LOCATION.wrapping_add(m)))); // 0x4B86
    if here <= d.wrapping_add(1) {
        return 0; // 0x4B92
    }
    api.set_far_byte(
        maze(BY_LOCATION.wrapping_add(m)),
        num::low8(d).wrapping_add(1),
    ); // 0x4B9D
    m
}

/// `04B0:00AB(n)`: the distances spread from location `n` through its
/// neighbors 1 to 4, depth first.
fn spread(api: &mut dyn Api, n: u16) {
    for k in 1..=4 {
        let d = u16::from(api.far_byte(maze(BY_LOCATION.wrapping_add(n)))); // 0x4BB7
        let m = step(api, n, k, d);
        api.set_word(LAST_STEP, m); // 0x4BC8
        if m != 0 {
            spread(api, m); // 0x4BD0
        }
    }
}

/// `04B0:013D`: for each maze row, the first of its five ways (0 to 4)
/// that leads one step closer, or 5; copied over the rows' bytes.
fn routes(api: &mut dyn Api) {
    // `[bp-22h]`: 20h bytes copied; the handler writes 1Eh of them, and the
    // last two are whatever its stack held.
    let mut buf = [0u8; 0x20];
    for si in (0..FIRST).rev() {
        // 0x4C4D: the distance one step closer.
        let closer = i16::from(api.far_byte(maze(ROWS.wrapping_add(si)))) - 1;
        let mut dx: u16 = 0;
        while dx <= 4 {
            let at = si.wrapping_mul(STRIDE).wrapping_add(dx);
            // 0x4C68: `+629h` is row si + 30's neighbors.
            let di = u16::from(row_byte(api, at.wrapping_add(FIRST * STRIDE + NEIGHBORS)));
            if si < 0x0A && di == 0x12 {
                break; // 0x4C76
            }
            if di == 0x17 {
                break; // 0x4C7B
            }
            if dx == 0 && di == 0x32 {
                break; // 0x4C84
            }
            if (FIRST..END).contains(&di) {
                let d = i16::from(api.far_byte(maze(BY_LOCATION.wrapping_add(di)))); // 0x4C98
                if d == closer {
                    break;
                }
            }
            dx += 1;
        }
        buf[usize::from(si)] = num::low8(dx); // 0x4CAA
    }
    api.movmem_from_local(&buf, maze(ROWS)); // 0x4CC1
}

/// `04B0:01CD`: the maze laid out, the first time.
fn lay_out(api: &mut dyn Api) {
    if api.byte(PATIENCE) == 0 {
        api.set_byte(PATIENCE, 0x0A); // 0x4CD4
    }
    api.setmem(maze(ROWS), 0xFE, 0x1E); // 0x4CE4
    api.set_far_byte(maze(0x04BB), 1); // 0x4CF1: row 57
    spread(api, 0x39);
    api.set_far_byte(maze(ROWS), 0x14); // 0x4D02: row 30
    spread(api, 0x1E);
    api.set_far_byte(maze(0x04AE), 0x0A); // 0x4D13: row 44
    spread(api, 0x2C);
    routes(api); // 0x4D1F
}

/// Everything before the frame loop, `0x4D23` to `0x4DB5`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x4D24
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.visit_enter(loc, entry); // 0x4D31
    if api.far_byte(maze(ROWS)) == 0x58 {
        lay_out(api); // 0x4D46
    }
    api.obj_anim_id(0xD7, 0); // 0x4D4E
    // 0x4D56: every so many visits, a scene — 3 with item 17, else 2 —
    // and the wait grows by two up to 30.
    let visits = api.byte(VISITS);
    api.set_byte(VISITS, visits.wrapping_add(1));
    if visits > api.byte(PATIENCE) {
        let patience = api.byte(PATIENCE);
        if patience < 0x1E {
            api.set_byte(PATIENCE, patience.wrapping_add(2)); // 0x4D6F
        }
        api.set_byte(VISITS, 0); // 0x4D72
        let scene = if api.inv_has(0x11) != 0 { 3 } else { 2 }; // 0x4D79
        api.scene_play(scene);
    }
    api.visit_finish_entry(); // 0x4D8F
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x4D97
    api.set_word(ENTRY, 0);
    api.set_word(HUNDREDTHS, 0); // 0x4DA0
    let due = api.random(0x64, 0xC8); // 0x4DAB
    api.set_local(SOUND_DUE, due);
}

/// One turn of the frame loop, `0x4DB5` to `0x4DFE`: now and then a
/// forest sound.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    if api.word(HUNDREDTHS) > api.local(SOUND_DUE) {
        let k = api.random(0x0F, 0x14); // 0x4DC9
        if num::signed(k) <= 0x11 {
            api.play_sample(k); // 0x4DD9
        }
        api.set_word(HUNDREDTHS, 0); // 0x4DDF
        let due = api.random(0x64, 0x012C); // 0x4DEA
        api.set_local(SOUND_DUE, due);
    }
    api.frame_pass(); // 0x4DF4
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x4E00` to `0x4E1E`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out);
    api.visit_leave();
    api.slot_unload();
}

/// The hook, `04B0:0324`: it compares the code with 7 and does nothing
/// either way.
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// The animation hook, `04B0:031F`: empty.
pub(crate) fn anim_hook(_api: &mut dyn Api, _rec: Far, _code: u16) {}

/// The rule handler `04B0:0004`: the player walks out through the exit
/// that leads closer; answers 0 when this row has none (its byte above 4).
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    if off != 0x0004 {
        return None;
    }
    api.set_byte(VISITS, 0); // 0x4B08
    let here = api.word(HERE);
    let way = u16::from(api.far_byte(maze(BY_LOCATION.wrapping_add(here)))); // 0x4B16
    let at = here.wrapping_mul(STRIDE).wrapping_add(way);
    let exit = u16::from(row_byte(api, at.wrapping_add(EXITS))); // 0x4B29
    if way > 4 {
        return Some(0); // 0x4B37
    }
    if (0x64..=0x68).contains(&exit) {
        api.walk_to_node(exit); // 0x4B46
        api.script_exit(exit); // 0x4B4D
    }
    Some(0x0A)
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
