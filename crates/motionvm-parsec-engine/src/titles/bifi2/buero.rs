//! Location 24, Schlönstadt's Büro: a room of kind 1 with one exit, `6Eh`,
//! to the Vorzimmer (location 5). The module is named after the location
//! as the game itself names it in its location table (`Schlönstadt's
//! Büro`, the record's `+37h`, `250Ah:0175h`).
//!
//! Re-expressed from the handler at `15EBh:00B8h` (image `0x15F68` to its
//! `retf` at `0x160D5`), its hook `15EBh:0243h` (`0x160F3`, the record's
//! `+3Fh`), its animation hook `15EBh:0226h` (`0x160D6`, the record's
//! `+43h`) and its three rule handlers, against [`Api`] alone. The
//! location's rules are the four records at `2446h:0870h` (image
//! `0x24CD0`, the record's `+3Bh`), fourteen bytes each:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 3 | 4 | `9Ah` | — | `000Fh` |
//! | 4 | 4 | `9Ah` | — | `000Fh` |
//! | 1 | 4 | `9Bh` | — | `0064h` |
//! | 1 | 4 | `9Dh` | — | `008Ah` |
//!
//! Object `F2h` is found on every visit and kept at `[2F54h]` (`0x15F89`).
//! Object `F0h` is hidden on every visit (`0x15FE7`) but the first one
//! after `[2BA6h]` reached 2. Then it is shown and kept at `[2F58h]`
//! (`0x15FAC`), the walking record of the same id is kept at `[2F50h]`
//! (`06E3h:04EFh`, `0x15FBC`), and the player is sent to node `0FA0h`
//! facing 3 with scene 0 (`09B6h:16C4h`, `0x15FD0`). Hook code 7 with
//! argument `0Bh` then hides the object, binds the record to graph 1 on
//! node `0BB8h` and sends it to node `0BB9h` (`0x161C9`–`0x1622C`); the
//! frame loop watches the record's node (`+28h`): on `0BB9h` the exit
//! object `6Eh` takes animation 1 and the record walks on to `0BBAh`, on
//! `0BBAh` the exit object takes animation 2 and the input is given back
//! (`0x16036`–`0x160A3`).
//!
//! The bytes of the logic's saved state block (`DS:2B98h`, `4Eh` bytes) the
//! location reads or writes: `[2BA6h]` (written by other locations),
//! `[2BC6h]` (set when `F0h` is shown), `[2BC7h]` (1 from rule `000Fh`, 2
//! from the Kantine, 3 from the hook's codes 5 and 6), `[2BC8h]` (rule
//! `0064h`), `[2B9Eh]` (rule `008Ah`, the Zettel given) and `[2BBFh]`
//! (cleared when `F0h` is shown). The items it moves are, by the
//! item row at `DS:10AAh` (`0x0A5A8`): `25h` `Memo`, `28h` `Zettel`.

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x15EB;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x160BE`).
const OBJECT_STATES: u16 = 0x2018;

/// Object `F2h`, found on every visit.
const OBJECT_F2: u16 = 0xF2;
/// Object `F0h`, and the walking record of the same id.
const OBJECT_F0: u16 = 0xF0;
/// Object `6Eh`, the row's one exit id, to the Vorzimmer.
const EXIT_6E: u16 = 0x6E;

/// `[2F54h]`/`[2F56h]`: object `F2h`'s record (`0x15F8F`, `0x15F93`).
const REC_F2: u16 = 0x2F54;
/// `[2F58h]`/`[2F5Ah]`: object `F0h`'s record (`0x15FB2`, `0x15FB6`).
const REC_F0: u16 = 0x2F58;
/// `[2F50h]`/`[2F52h]`: the walking record `F0h` (`0x15FC2`, `0x15FC6`).
const MOVER_F0: u16 = 0x2F50;

/// The nodes of the record's walk: bound on `0BB8h`, sent to `0BB9h`, then
/// to `0BBAh`.
const NODE_BOUND: u16 = 0x0BB8;
const NODE_MID: u16 = 0x0BB9;
const NODE_OUT: u16 = 0x0BBA;
/// The walking record's node, `+28h` (`0x16041`).
const MOVER_NODE: u16 = 0x28;

/// The node the player is sent to when `F0h` is shown (`0x15FCD`).
const NODE_MEETING: u16 = 0x0FA0;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
const BYTE_2B9E: u16 = 0x2B9E;
const BYTE_2BA6: u16 = 0x2BA6;
const BYTE_2BBF: u16 = 0x2BBF;
const BYTE_2BC6: u16 = 0x2BC6;
const BYTE_2BC7: u16 = 0x2BC7;
const BYTE_2BC8: u16 = 0x2BC8;

/// The location's own words, read and written in segment `15EBh` alone.
/// `[2F48h]`: the record's walk out is watched (`0x161CF`).
const WALKING_OUT: u16 = 0x2F48;
/// `[2F4Ah]`: the animation object `F0h` is to take.
const ANIM_WANTED: u16 = 0x2F4A;
/// `[2F4Ch]`: the animation last given.
const ANIM_GIVEN: u16 = 0x2F4C;
/// `[2F4Eh]`: bit 1 of code 0's argument (`0x1610D`).
const TRACK_F0: u16 = 0x2F4E;

/// The items rule `008Ah` and the hook's code 4 give.
const ITEM_ZETTEL: u16 = 0x28;
const ITEM_MEMO: u16 = 0x25;

/// The Kantine, location 23, and the entry hook code 4 sends it with
/// (`0x1619A`, `0x161A0`).
const KANTINE: u16 = 0x17;
const KANTINE_ENTRY: u16 = 0x0FA0;

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

/// Everything before the frame loop, `0x15F68` to `0x1602B`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x15F68
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x15F74
    api.visit_enter(loc, entry); // 0x15F78
    api.set_word(WALKING_OUT, 0); // 0x15F80
    let rf2 = api.obj_find(OBJECT_F2); // 0x15F89
    keep(api, REC_F2, rf2); // 0x15F8F
    if api.byte(BYTE_2BA6) == 2 && api.byte(BYTE_2BC6) == 0 {
        // 0x15F96, 0x15F9D: F0h shown, the player sent to node 0FA0h.
        api.input_lock(); // 0x15FA4
        let rf0 = api.obj_show(OBJECT_F0); // 0x15FAC
        keep(api, REC_F0, rf0); // 0x15FB2
        let mover = api.mover_find(OBJECT_F0); // 0x15FBC, 06E3h:04EFh
        keep(api, MOVER_F0, mover); // 0x15FC2
        api.interaction(NODE_MEETING, 3, 0); // 0x15FD0, 09B6h:16C4h
        api.set_byte(BYTE_2BC6, 1); // 0x15FD8
        api.set_byte(BYTE_2BBF, 0); // 0x15FDD
    } else {
        api.obj_hide(OBJECT_F0); // 0x15FE7
        if api.byte(BYTE_2BC7) == 2 {
            // 0x15FED: [2BC7h] 2, as the Kantine leaves it (0x15B87).
            api.input_lock(); // 0x15FF4
            api.scene_play(2); // 0x15FFB
            let r = rec(api, REC_F2);
            api.obj_anim(r, 1); // 0x1600B
            let player = api.ds(WALKER);
            api.figure_face(player, 2); // 0x16019
        }
    }
    api.visit_finish_entry(); // 0x16021
    api.set_word(WORDS.entry, 0); // 0x16026
}

/// One turn of the frame loop, `0x1602C` to `0x160B5`: while `[2F48h]` is
/// set, the walking record's node is watched between the dispatch and the
/// frame pass.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x1602C
    api.dispatch(); // 0x16031
    if api.word(WALKING_OUT) != 0 {
        // 0x16036
        let mover = rec(api, MOVER_F0);
        let node = api.far_word(Far {
            off: mover.off.wrapping_add(MOVER_NODE),
            seg: mover.seg,
        }); // 0x1603D
        match node {
            // 0x16051: 6Eh on animation 1, the record walks on, the player
            // faces 1.
            NODE_MID => {
                let exit = api.obj_find(EXIT_6E); // 0x16055
                api.obj_anim(exit, 1); // 0x1605D
                api.walker_walk_to(mover, NODE_OUT, 1); // 0x16072
                let player = api.ds(WALKER);
                api.figure_face(player, 1); // 0x16080
            }
            // 0x1608A: 6Eh on animation 2, the input given back.
            NODE_OUT => {
                let exit = api.obj_find(EXIT_6E); // 0x1608E
                api.obj_anim(exit, 2); // 0x16096
                api.input_unlock(); // 0x1609E
                api.set_word(WALKING_OUT, 0); // 0x160A3
            }
            _ => {}
        }
    }
    api.frame_pass(); // 0x160A9
    api.word(WORDS.entry) == 0 // 0x160AE
}

/// Everything after the frame loop, `0x160B8` to `0x160D5`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x160C3
    api.visit_leave(); // 0x160CB
    api.slot_unload(); // 0x160D0
}

/// The hook, `15EBh:0243h`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read. Codes above 9 and codes 1 to 3 do nothing
/// (`0x160FC`, the table at `cs:0383h`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0x1610B: bit 1 of the argument kept, the animation given
        // forgotten.
        0 => {
            api.set_word(TRACK_F0, arg & 2); // 0x16110
            api.set_word(ANIM_GIVEN, 0xFFFF); // 0x16113
        }
        // 0x1618B.
        4 => match arg {
            // 0x1619A: on to the Kantine.
            0x1F => {
                api.set_word(WORDS.location, KANTINE); // 0x1619A
                api.set_word(WORDS.entry, KANTINE_ENTRY); // 0x161A0
            }
            // 0x161A9: the Memo given.
            0x21 => {
                api.inv_give(ITEM_MEMO); // 0x161AB
            }
            _ => {}
        },
        // 0x1611C: [2BC7h] 2 becomes 3 and the input is given back; F2h
        // on animation 0.
        5 | 6 => {
            if api.byte(BYTE_2BC7) == 2 {
                api.set_byte(BYTE_2BC7, 3); // 0x16123
                api.input_unlock(); // 0x16128
            }
            let r = rec(api, REC_F2);
            api.obj_anim(r, 0); // 0x16180
        }
        // 0x161B3.
        7 => match arg {
            0x0A => api.set_word(ANIM_WANTED, 1), // 0x161C1
            0x0B => walk_out(api),                // 0x161C9
            _ => {}
        },
        // 0x16139.
        8 => match arg {
            0xFFFF | 0 => api.set_word(ANIM_WANTED, 0), // 0x1614C
            1 => api.set_word(ANIM_WANTED, 2),          // 0x16155
            _ => {}
        },
        // 0x1615E: F0h on the animation asked for, once each.
        9 => {
            if api.word(TRACK_F0) == 0 {
                return; // 0x16165
            }
            let k = api.word(ANIM_WANTED);
            if k == api.word(ANIM_GIVEN) {
                return; // 0x16171
            }
            api.set_word(ANIM_GIVEN, k); // 0x16174
            let r = rec(api, REC_F0);
            api.obj_anim(r, num::signed(k)); // 0x16180
        }
        _ => {}
    }
}

/// Code 7 with argument `0Bh`, `0x161C9` to `0x16231`: the object hidden,
/// the record bound to graph 1 on node `0BB8h` and sent to `0BB9h`,
/// subject slot 1 on it, the player facing 0 and the input taken over.
fn walk_out(api: &mut dyn Api) {
    api.set_word(TRACK_F0, 0); // 0x161C9
    api.set_word(WALKING_OUT, 1); // 0x161CF
    api.obj_hide(OBJECT_F0); // 0x161D8
    let mover = rec(api, MOVER_F0);
    api.walker_bind_graph(mover, 1, NODE_BOUND, 1); // 0x161ED, 05DCh:0DB4h
    api.walker_walk_to(mover, NODE_MID, 1); // 0x16202
    api.slot_set_figure(1, 0, mover); // 0x16216, 080Ch:19B4h
    let player = api.ds(WALKER);
    api.figure_face(player, 0); // 0x16224
    api.input_lock(); // 0x1622C
}

/// The animation hook, `15EBh:0226h`: the record in `[bp+6]`, the code in
/// `[bp+0Ah]`; code `64h` plays sample `0Eh` twice.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    if code == 0x64 {
        api.play_sample(0x0E); // 0x160E3
        api.play_sample(0x0E); // 0x160EB
    }
}

/// The Büro's rule handlers by their offset in segment `15EBh`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 15EB:000F (0x15EBF): at [2BA6h] 2 scene 1 and [2BC7h] 1; else one
        // of scenes 7 to 0Ah below 2, 0Bh to 0Eh above.
        0x000F => {
            let state = api.byte(BYTE_2BA6);
            if state == 2 {
                api.input_lock(); // 0x15EC9
                api.scene_play(1); // 0x15ED0
                api.set_byte(BYTE_2BC7, 1); // 0x15ED6
                let r = rec(api, REC_F2);
                api.obj_anim(r, 1); // 0x15EE5
            } else {
                let n = if state < 2 {
                    api.random(7, 0x0A) // 0x15F00
                } else {
                    api.random(0x0B, 0x0E) // 0x15F00
                };
                api.scene_play(num::signed(n)); // 0x15F09
            }
            0x0A // 0x15F0F
        }
        // 15EB:0064 (0x15F14): scene 3 the first time, else scene 4.
        0x0064 => {
            if api.byte(BYTE_2BC8) == 0 {
                api.scene_play(3); // 0x15F20
                api.set_byte(BYTE_2BC8, 1); // 0x15F26
            } else {
                api.scene_play(4); // 0x15F2F
            }
            0x0A // 0x15F35
        }
        // 15EB:008A (0x15F3A): scene 5 and the Zettel the first time, else
        // scene 6.
        0x008A => {
            if api.byte(BYTE_2B9E) == 0 {
                api.scene_play(5); // 0x15F46
                api.inv_give(ITEM_ZETTEL); // 0x15F4E
                api.set_byte(BYTE_2B9E, 1); // 0x15F54
            } else {
                api.scene_play(6); // 0x15F5D
            }
            0x0A // 0x15F63
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
