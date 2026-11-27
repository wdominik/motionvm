//! Location 7, Rambowsky, a row of kind 1 with two exits: `6Eh` to the
//! Flur 4. Stock (location 33) and `6Fh` to the Dino Zoo (location 14),
//! the row's `+9h` and `+19h`. The module is named after the location as
//! the game itself names it in its location table (`Rambowsky`, the
//! record's `+37h`, `250Ah:00C5h`).
//!
//! Re-expressed from the handler at `11B0h:0223h` (image `0x11D23` to its
//! `retf` at `0x12141`), its hook `11B0h:06B4h` (`0x121B4` to its `retf`
//! at `0x12510`, the record's `+3Fh`), its animation hook `11B0h:0662h`
//! (`0x12162` to `0x121B3`, the record's `+43h`) and its fourteen rule
//! handlers `11B0h:000Fh` to `0208h` (`0x11B0F`–`0x11D22`), against
//! [`Api`] alone. The segment installs no gate. The location's rules are
//! the twenty-seven records at `2446h:01D0h` (image `0x24630`, the
//! record's `+3Bh`), fourteen bytes each, all of verb 5 (`Benutze … mit`,
//! by the verb text row at `DS:116Ah`) and phase `20h`; a second of 0 is
//! nothing:
//!
//! | object | second | handler |
//! |---|---|---|
//! | `14h` | `87h` | `000Fh` |
//! | `2Ch`, `2Dh`, `2Eh` | `87h` | `0043h` |
//! | `2Ch`, `2Dh`, `2Eh` | `F0h`, and the other way round | `007Dh` |
//! | `2Eh` | `1Ah`, and the other way round | `0172h` |
//! | `2Eh` | `1Bh`, and the other way round | `0192h` |
//! | `1Ah`, `1Bh` | 0 | `0208h` |
//! | `29h` | `87h` | `00FAh` |
//! | `2Ah` | `87h` | `012Ah` |
//! | `2Ah` | `F0h`, and the other way round | `0152h` |
//! | `F0h` | 0 | `01A2h` |
//! | `2Ah` | 0 | `01BDh` |
//! | `29h` | 0 | `01CDh` |
//! | `2Ch`, `2Dh`, `2Eh` | 0 | `01DDh` |
//! | `14h` | 0 | `01EDh` |
//!
//! The items the logic names are, by the item row at `DS:10AAh`: `14h`,
//! `2Ch`, `2Dh` and `2Eh` `Eimer`, `17h` and `18h` `Theaterkostüm`, `1Ah`
//! `Stadtplan`, `1Bh` `Labyrinth-Karte`, `29h` and `2Ah` `Wasserpistole`.
//! With object `87h` the `Eimer` `14h` becomes `2Ch` and `2Ch` to `2Eh`
//! become `14h`, the `Wasserpistole` `29h` becomes `2Ah` and `2Ah` becomes
//! `29h`; with object `F0h` `2Ah` becomes `29h` and the `Eimer` `2Ch` to
//! `2Eh` become `2Eh`, or `14h` with the `Labyrinth-Karte`.
//!
//! The location's course is `[2BA6h]` in the logic's saved state block
//! (`DS:2B98h`, `4Eh` bytes), which the Kantine (location 23) moves from 0
//! to 1 and 2 (`0x15A6E`, `0x15A7E`) and to 3 (`0x15B8C`). Below 3, and
//! above 6, the entry opens with the script's interaction with node
//! `0FA1h` (scenes 0 and 1 in turn, counter `[2BA3h]`), object `F3h` shown
//! and `98h`, `94h`, `F6h` hidden, and the Denkfabrik's `[2BBEh]` moved
//! from 0 to 1. In state 3 the four are shown: carrying a `Theaterkostüm`
//! the player walks to node `0FA0h`, where the walking record `F2h` takes
//! it (`18h` before `17h`, kept at `[2DD0h]`) and walks off to node
//! `0BB9h`; without one scene 2 plays. The script's events then lead
//! through the phases the handler keeps at `[2DD2h]`; event `0Ch` gives
//! item `18h`, moves `[2BA6h]` to 4 and goes to the Kaffee (location 1)
//! with entry 1. In state 4 the entry hides the four, plays scene `0Bh`
//! and moves to 5; states 5 and 6 show object `F1h` hidden on animation 1,
//! and in 5 object `F0h` on animation 0, until event `1Eh` hides it and
//! moves to 6. The phase-3 choice between phases 6 and 8 reads `[2BA7h]`,
//! which the Denkfabrik (location 17) sets (`0x14DFE`). `[2BA4h]` and
//! `[2BA5h]` count two scene sequences; the Maske reads `[2BA4h]`
//! (`0x1588E`) and `[2BA6h]` (`0x15887`), the Denkfabrik `[2BA6h]`
//! (`0x14E58`, `0x14E93`).
//!
//! The record that walks by id is `06E3h:04EFh` ([`Api::mover_find`]),
//! its graph `05DCh:0DB4h` ([`Api::walker_bind_graph`]); `05DCh:0D95h`
//! (`0x06B55`) is Dunkle Schatten's `0A9Fh:0E7Fh` (`0x0B86F`) without its
//! null test ([`Api::walker_unlink`]); `03F7h:1DAEh` (`0x05D1E`) is Dunkle
//! Schatten's `0883h:20D9h` (`0x0A909`) without its arm for a record of
//! state `0Bh` and its test for a null entry ([`Api::figure_anim`]).

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x11B0;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x12129`).
const OBJECT_STATES: u16 = 0x2018;

/// The objects the logic shows and hides.
const OBJECT_F0: u16 = 0xF0;
const OBJECT_F1: u16 = 0xF1;
const OBJECT_F3: u16 = 0xF3;
const OBJECT_F6: u16 = 0xF6;
const OBJECT_94: u16 = 0x94;
const OBJECT_98: u16 = 0x98;
/// The walking record `F2h` (`0x11F1A`).
const MOVER_F2: u16 = 0xF2;
/// Exit `6Eh`, to the Flur 4. Stock.
const EXIT_6E: u16 = 0x6E;

/// The node of the script's interaction (`push 0FA1h`, `0x11D72`).
const NODE_FA1: u16 = 0x0FA1;
/// The node the player walks to with a `Theaterkostüm` (`0x11E26`).
const NODE_FA0: u16 = 0x0FA0;
/// The walking record's nodes: bound on `0BB8h`, sent to `0BB9h`, then to
/// `0BBAh` and back to `0BB8h`.
const NODE_BB8: u16 = 0x0BB8;
const NODE_BB9: u16 = 0x0BB9;
const NODE_BBA: u16 = 0x0BBA;
/// The player's node, `[1796h]` (`0x11F0F`).
const PLAYER_NODE: u16 = WALKER + 0x28;
/// A walking record's node, `+28h` (`0x11FBC`).
const MOVER_NODE: u16 = 0x28;

/// The items the logic moves (the item row at `DS:10AAh`).
const EIMER: u16 = 0x14;
const KOSTUEM_17: u16 = 0x17;
const KOSTUEM_18: u16 = 0x18;
const STADTPLAN: u16 = 0x1A;
const LABYRINTH_KARTE: u16 = 0x1B;
const PISTOLE_29: u16 = 0x29;
const PISTOLE_2A: u16 = 0x2A;
const EIMER_2C: u16 = 0x2C;
const EIMER_2D: u16 = 0x2D;
const EIMER_2E: u16 = 0x2E;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BA3h]`: the counter of the entry's interaction.
const BYTE_2BA3: u16 = 0x2BA3;
/// `[2BA4h]`: the counter of phase 4's scene sequence.
const BYTE_2BA4: u16 = 0x2BA4;
/// `[2BA5h]`: the counter of phase 6's scene sequence.
const BYTE_2BA5: u16 = 0x2BA5;
/// `[2BA6h]`: the location's course, 0 to 6.
const COURSE: u16 = 0x2BA6;
/// `[2BA7h]`: set by the Denkfabrik (`0x14DFE`).
const BYTE_2BA7: u16 = 0x2BA7;
/// `[2BBEh]`: the Denkfabrik's state, moved from 0 to 1 here.
const BYTE_2BBE: u16 = 0x2BBE;

/// The location's own words, read and written in segment `11B0h` alone
/// and outside the `38h` bytes from `DS:2D38h` the visit clears.
/// `[2DB4h]`: how many of the kept records from `[2DC2h]` the hook
/// animates, 0, 1 or 3.
const RECORDS: u16 = 0x2DB4;
/// `[2DB6h]`: the animation last started on the walking record.
const MOVER_ANIM: u16 = 0x2DB6;
/// `[2DB8h]`: the animation last given to the kept records.
const RECORDS_ANIM: u16 = 0x2DB8;
/// `[2DBAh]`/`[2DBCh]`: the walking record `F2h` (`0x11F27`, `0x11F23`).
const REC_F2: u16 = 0x2DBA;
/// `[2DBEh]`/`[2DC0h]`: object `F1h`'s record (`0x11E86`, `0x11E82`).
const REC_F1: u16 = 0x2DBE;
/// `[2DC2h]`, `[2DC6h]`, `[2DCAh]`: the records of objects `F3h`, `98h`
/// and `94h`, four bytes apart (`[bx+2DC2h]`, `0x1230E`).
const REC_F3: u16 = 0x2DC2;
const REC_98: u16 = 0x2DC6;
const REC_94: u16 = 0x2DCA;
/// `[2DCEh]`: the samples the frame loop plays, `10h` or `1Fh`; 0 none.
const AMBIENT: u16 = 0x2DCE;
/// `[2DD0h]`: 1 when the walking record took item `18h`, 0 for `17h`.
const TOOK_18: u16 = 0x2DD0;
/// `[2DD2h]`: the handler's phase, 0 to `0Fh`.
const PHASE: u16 = 0x2DD2;

/// The mode block's object `[2D3Ch]` and walk target `[2D4Ah]`.
const OBJECT: u16 = 0x2D3C;
const WALK_TARGET: u16 = 0x2D4A;
/// `[1CF6h]`: the library's hundredths.
const HUNDREDTHS: u16 = 0x1CF6;

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

/// `les bx, [2DBAh] ; cmp word es:[bx+28h], node`.
fn mover_at(api: &mut dyn Api, node: u16) -> bool {
    let m = rec(api, REC_F2);
    api.far_word(Far {
        off: m.off.wrapping_add(MOVER_NODE),
        seg: m.seg,
    }) == node
}

/// `push hi ; push lo ; call far 203Dh:0024h ; push ax ; call far
/// 080Ch:164Ch`: the roll is the scene.
fn roll(api: &mut dyn Api, lo: u16, hi: u16) {
    let n = api.random(lo, hi);
    api.scene_play(num::signed(n));
}

/// `0751h:0A43h(a, b)` (`0x07F53`): `0751h:099Dh(a)` ([`Api::inv_take`])
/// and then `0751h:090Eh(b)` ([`Api::inv_give`]).
fn swap(api: &mut dyn Api, a: u16, b: u16) {
    api.inv_take(a);
    api.inv_give(b);
}

/// `push 1 ; push -1 ; push 0F1h ; call far 0F48h:096Eh ; push dx ; push
/// ax ; call far 03F7h:1CF9h`: object `F1h` hidden and its record on
/// animation 1.
fn f1_on_1(api: &mut dyn Api) {
    let r = api.obj_show_frame(OBJECT_F1, -1);
    api.obj_anim(r, 1);
}

/// Everything before the frame loop, `0x11D23` to `0x11EE9`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.set_word(PHASE, 0); // 0x11D27
    api.visit_setup(); // 0x11D2D
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x11D39
    api.visit_enter(loc, entry); // 0x11D3D
    // 0x11D45: by the course less 3 through the table at cs:065Ah
    // (0x1215A); below 3 and above 6 to 0x11D5B.
    match api.byte(COURSE) {
        3 => enter_3(api),
        // 0x11E4C: the four hidden, F1h hidden and kept, course 5,
        // scene 0Bh.
        4 => {
            api.input_lock(); // 0x11E4C
            api.obj_hide(OBJECT_F3); // 0x11E54
            api.obj_hide(OBJECT_98); // 0x11E5D
            api.obj_hide(OBJECT_94); // 0x11E66
            api.obj_hide(OBJECT_F6); // 0x11E6F
            let r = api.obj_show_frame(OBJECT_F1, -1); // 0x11E7A
            keep(api, REC_F1, r); // 0x11E82, 0x11E86
            api.set_byte(COURSE, 5); // 0x11E89
            api.scene_play(0x0B); // 0x11E44, by 0x11E90
        }
        // 0x11E92: F1h on animation 1, F0h shown on animation 0.
        5 => {
            f1_on_1(api); // 0x11E99, 0x11EA3
            let r = api.obj_show(OBJECT_F0); // 0x11EB0
            api.obj_anim(r, 0); // 0x11EB8
        }
        // 0x11EC2: F1h on animation 1, F0h hidden.
        6 => {
            f1_on_1(api); // 0x11EC9, 0x11ED3
            api.obj_hide(OBJECT_F0); // 0x11EDE
        }
        _ => enter_first(api),
    }
    api.visit_finish_entry(); // 0x11EE4
    api.set_word(WORDS.entry, 0); // 0x11EE9
}

/// `0x11D5B` to `0x11DC5`: phase 1, the script's interaction with node
/// `0FA1h`, `F3h` shown, `98h`, `94h` and `F6h` hidden, the Denkfabrik's
/// state 0 moved to 1.
fn enter_first(api: &mut dyn Api) {
    api.input_lock(); // 0x11D5B
    api.set_word(PHASE, 1); // 0x11D60
    let counter = api.ds(BYTE_2BA3);
    // 09B6h:164Dh, Dunkle Schatten's 0EB9h:1703h.
    api.interaction_sequence(NODE_FA1, 1, counter, &[0, 1]); // 0x11D75
    let r = api.obj_show(OBJECT_F3); // 0x11D80
    keep(api, REC_F3, r); // 0x11D86, 0x11D8A
    let r = api.obj_hide(OBJECT_98); // 0x11D90
    keep(api, REC_98, r); // 0x11D96, 0x11D9A
    let r = api.obj_hide(OBJECT_94); // 0x11DA0
    keep(api, REC_94, r); // 0x11DA6, 0x11DAA
    api.obj_hide(OBJECT_F6); // 0x11DB0
    if api.byte(BYTE_2BBE) == 0 {
        // 0x11DB6
        api.set_byte(BYTE_2BBE, 1); // 0x11DC0
    }
}

/// `0x11DC8` to `0x11E49`: the four shown; with a `Theaterkostüm` phase 3
/// and the player sent to node `0FA0h`, else phase 2 and scene 2.
fn enter_3(api: &mut dyn Api) {
    api.input_lock(); // 0x11DC8
    let r = api.obj_show(OBJECT_F3); // 0x11DD0
    keep(api, REC_F3, r); // 0x11DD6, 0x11DDA
    let r = api.obj_show(OBJECT_98); // 0x11DE0
    keep(api, REC_98, r); // 0x11DE6, 0x11DEA
    let r = api.obj_show(OBJECT_94); // 0x11DF0
    keep(api, REC_94, r); // 0x11DF6, 0x11DFA
    api.obj_show(OBJECT_F6); // 0x11E00, the record not kept
    if api.inv_has(KOSTUEM_18) != 0 || api.inv_has(KOSTUEM_17) != 0 {
        // 0x11E08, 0x11E14
        api.set_word(PHASE, 3); // 0x11E1E
        api.set_word(WALK_TARGET, NODE_FA0); // 0x11E29
        let player = api.ds(WALKER);
        api.walker_walk_to(player, NODE_FA0, 0); // 0x11E31, answer not read
    } else {
        api.set_word(PHASE, 2); // 0x11E3C
        api.scene_play(2); // 0x11E44
    }
}

/// One turn of the frame loop, `0x11EEF` to `0x12104`: the phase's step
/// between the dispatch and the frame pass, then the samples of
/// `[2DCEh]`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x11EEF
    api.dispatch(); // 0x11EF4
    step(api); // 0x11EF9
    ambient(api); // 0x12092
    api.frame_pass(); // 0x120F8
    api.word(WORDS.entry) == 0 // 0x120FD
}

/// `0x11EF9`: by the phase less 3 through the table at `cs:0642h`
/// (`0x12142`); phases 5, 7, 9, 0Bh, 0Ch, 0Eh and those outside 3 to 0Eh
/// do nothing.
fn step(api: &mut dyn Api) {
    let phase = api.word(PHASE);
    // Each arm but phase 3's first asks the walking record's node.
    let node = match phase {
        3 => return take_costume(api),
        4 | 6 | 8 => NODE_BB9, // 0x11FBC, 0x11FE6, 0x12012
        0x0A => NODE_BBA,      // 0x1202D
        0x0D => NODE_BB8,      // 0x12049
        _ => return,
    };
    if !mover_at(api, node) {
        return;
    }
    match phase {
        // 0x11FC7: phase 5 and the next of scenes 3 and 8.
        4 => {
            api.set_word(PHASE, 5); // 0x11FC7
            let counter = api.ds(BYTE_2BA4);
            api.scene_sequence(counter, &[3, 8]); // 0x11FD7
        }
        // 0x11FF1: phase 7 and the next of scenes 4 to 6.
        6 => {
            api.set_word(PHASE, 7); // 0x11FF1
            let counter = api.ds(BYTE_2BA5);
            api.scene_sequence(counter, &[4, 5, 6]); // 0x12003
        }
        // 0x1201A: phase 9 and scene 7.
        8 => {
            api.set_word(PHASE, 9); // 0x1201A
            api.scene_play(7); // 0x12022
        }
        // 0x12035: phase 0Bh and task 0 started.
        0x0A => {
            api.set_word(PHASE, 0x0B); // 0x12035
            api.task_start(0); // 0x1203D
        }
        // 0x12051: the record back on 0BB8h: subject slot 0 the player
        // again, the record unlinked, the costume given back, out through
        // exit 6Eh, phase 0Eh.
        _ => {
            let player = api.ds(WALKER);
            api.slot_set_figure(0, 0, player); // 0x12059, 080Ch:19B4h
            let m = rec(api, REC_F2);
            api.walker_unlink(m); // 0x12069, 05DCh:0D95h
            let item = if api.word(TOOK_18) != 0 {
                KOSTUEM_18 // 0x12078
            } else {
                KOSTUEM_17 // 0x1207C
            };
            api.inv_give(item); // 0x1207E
            api.script_exit(EXIT_6E); // 0x12086, 0FE3h:088Ah
            api.set_word(PHASE, 0x0E); // 0x1208C
        }
    }
}

/// Phase 3, `0x11F0F` to `0x11FB5`: once the player stands on node
/// `0FA0h`, the walking record `F2h` is kept, named by subject slot 0,
/// bound to graph 1 on node `0BB8h`, takes a `Theaterkostüm` (`18h`
/// first: phase 6, or 8 once `[2BA7h]` is set; `17h`: phase 4) and walks
/// to node `0BB9h` facing 2. Without either the phase stays.
fn take_costume(api: &mut dyn Api) {
    if api.word(PLAYER_NODE) != NODE_FA0 {
        return; // 0x11F15
    }
    let m = api.mover_find(MOVER_F2); // 0x11F1D, 06E3h:04EFh
    keep(api, REC_F2, m); // 0x11F23, 0x11F27
    api.slot_set_figure(0, 0, m); // 0x11F30, 080Ch:19B4h
    let m = rec(api, REC_F2);
    api.walker_bind_graph(m, 1, NODE_BB8, 0); // 0x11F47, 05DCh:0DB4h
    if api.inv_has(KOSTUEM_18) != 0 {
        // 0x11F51
        api.set_word(TOOK_18, 1); // 0x11F5B
        api.inv_take(KOSTUEM_18); // 0x11F63
        let phase = if api.byte(BYTE_2BA7) == 0 { 6 } else { 8 }; // 0x11F69
        api.set_word(PHASE, phase); // 0x11F70, 0x11F78
    } else if api.inv_has(KOSTUEM_17) != 0 {
        // 0x11F82
        api.set_word(TOOK_18, 0); // 0x11F8C
        api.inv_take(KOSTUEM_17); // 0x11F94
        api.set_word(PHASE, 4); // 0x11F9A
    }
    api.walker_walk_to(m, NODE_BB9, 2); // 0x11FAD, answer not read
}

/// `0x12092` to `0x120F5`: with `[2DCEh]` at `10h`, past 14h hundredths
/// sample 22h; at `1Fh`, past 1Eh hundredths sample 25h and one of
/// samples 26h to 28h on a roll of 0 to 2 out of 0 to 5. Either way the
/// hundredths then start again from a roll of 0 to 0Ah.
fn ambient(api: &mut dyn Api) {
    match api.word(AMBIENT) {
        0x10 => {
            if api.word(HUNDREDTHS) <= 0x14 {
                return; // 0x120F2
            }
            api.play_sample(0x22); // 0x120F4, 0x120D6
        }
        0x1F => {
            if api.word(HUNDREDTHS) <= 0x1E {
                return; // 0x120A6
            }
            api.play_sample(0x25); // 0x120AA
            let n = api.random(0, 5); // 0x120B4, kept at [bp-2]
            if n <= 2 {
                // 0x120BF–0x120CB
                api.play_sample(n.wrapping_add(0x26)); // 0x120D6
            }
        }
        _ => return, // 0x1209F
    }
    let n = api.random(0, 0x0A); // 0x120E0
    api.set_word(HUNDREDTHS, n); // 0x120E8
}

/// Everything after the frame loop, `0x12107` to `0x12141`: an `Eimer`
/// `2Eh` becomes `2Dh`.
pub(crate) fn leave(api: &mut dyn Api) {
    if api.inv_has(EIMER_2E) != 0 {
        // 0x12109
        api.inv_take(EIMER_2E); // 0x12115
        api.inv_give(EIMER_2D); // 0x1211D
    }
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x1212E
    api.visit_leave(); // 0x12136
    api.slot_unload(); // 0x1213B
}

/// The hook, `11B0h:06B4h`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read. Codes 1 to 4, 8 and those above 9 do nothing
/// (`0x121BF`, the table at `cs:0A71h`, `0x12571`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => dialogue(api), // 0x121CE
        // 0x1229E: no record animated any more; the samples of event 1Fh
        // stop.
        5 | 6 => {
            api.set_word(RECORDS, 0); // 0x1229E
            if api.word(AMBIENT) == 0x1F {
                // 0x122A7
                api.set_word(AMBIENT, 0); // 0x122AF
            }
        }
        7 => events(api, arg),  // 0x1240F
        9 => animate(api, arg), // 0x122B8
        _ => {}
    }
}

/// Code 0, `0x121CE` to `0x1227C`: a dialogue comes up. Nothing given yet;
/// by the phase less 1 through the table at `cs:0A57h` (`0x12557`) a set
/// of bits says who joins: bit 0 the walking record faces 3, else bit 1
/// it faces 2; bit 2 object `F3h` on animation 0, else bit 3 on 4, and
/// one record animated; bit 4 objects `98h` and `94h` on animation 0,
/// else bit 5 on 4, and three records animated.
fn dialogue(api: &mut dyn Api) {
    api.set_word(RECORDS_ANIM, 0xFFFF); // 0x121D1
    api.set_word(MOVER_ANIM, 0xFFFF); // 0x121D4
    api.set_word(RECORDS, 0); // 0x121D7
    let bits: u16 = match api.word(PHASE) {
        1 => 0x04,                       // 0x121FA
        4 | 6 | 8 | 0x0A | 0x0D => 0x14, // 0x121FF
        5 | 7 | 9 => 0x16,               // 0x121F0
        0x0B | 0x0C => 0x29,             // 0x121F5
        _ => 0,                          // 0x121DD, 0x12202
    };
    let mover = rec(api, REC_F2);
    if bits & 1 != 0 {
        api.figure_face(mover, 3); // 0x12208, 0x1221C
    } else if bits & 2 != 0 {
        api.figure_face(mover, 2); // 0x12212, 0x1221C
    }
    if bits & 0x0C != 0 {
        let k = if bits & 4 != 0 { 0 } else { 4 }; // 0x1222A, 0x12234
        let r = rec(api, REC_F3);
        api.obj_anim(r, k); // 0x1223E
        api.set_word(RECORDS, 1); // 0x12246
    }
    if bits & 0x30 != 0 {
        let k = if bits & 0x10 != 0 { 0 } else { 4 };
        let r = rec(api, REC_98);
        api.obj_anim(r, k); // 0x1225C, 0x12292
        let r = rec(api, REC_94);
        api.obj_anim(r, k); // 0x1226E
        api.set_word(RECORDS, 3); // 0x12276
    }
}

/// `0x122F1` to `0x12321`: a new animation `k` given to the kept records,
/// the last first (`[bx+2DC2h]`, `bx` four times the index).
fn give(api: &mut dyn Api, k: u16) {
    if api.word(RECORDS_ANIM) == k {
        return; // 0x122F5
    }
    api.set_word(RECORDS_ANIM, k); // 0x122FA
    let mut i = api.word(RECORDS); // 0x122FE
    while i != 0 {
        // 0x1231A
        i -= 1;
        let r = rec(api, REC_F3.wrapping_add(i << 2));
        api.obj_anim(r, num::signed(k)); // 0x12312
    }
}

/// `03F7h:1DAEh(rec, k)` on the walking record when `[2DB6h]` is not
/// `kept`; its answer, non-zero, keeps `kept`.
fn mover_anim(api: &mut dyn Api, kept: u16, k: u16) {
    if api.word(MOVER_ANIM) == kept {
        return;
    }
    let mover = rec(api, REC_F2);
    if api.figure_anim(mover, num::signed(k)) != 0 {
        api.set_word(MOVER_ANIM, kept);
    }
}

/// Code 9, `0x122B8`: the argument, a negative one taken as 0, is the
/// animation wanted; by the phase less 1 through the table at `cs:0A3Dh`
/// (`0x1253D`). Phase 3 and phases outside 1 to 0Dh do nothing.
fn animate(api: &mut dyn Api, arg: u16) {
    let k = if num::signed(arg) < 0 { 0 } else { arg }; // 0x122B8
    match api.word(PHASE) {
        // 0x122D2: the walking record and the kept records on k.
        9 => {
            mover_anim(api, k, k); // 0x122D2–0x122ED
            give(api, k); // 0x122F1
        }
        // 0x12360: the kept records on 4.
        0x0A => give(api, 4),
        // 0x12324: the walking record on 4 for 0, else 5; the kept
        // records on 4.
        0x0B => {
            let a = if k == 0 { 4 } else { 5 }; // 0x1232A
            mover_anim(api, k, a); // 0x12324–0x1235C
            give(api, 4); // 0x12360
        }
        // 0x12397: 0 taken as 4, the walking record on 4; anything else
        // starts animation 5 on it and is kept as given; the kept records
        // on the argument.
        0x0C => {
            if k == 0 {
                mover_anim(api, 4, 4); // 0x1239B–0x123B9
                give(api, 4); // 0x123DC
            } else {
                if api.word(MOVER_ANIM) != 5 {
                    // 0x123BB
                    let mover = rec(api, REC_F2);
                    if api.figure_anim(mover, 5) != 0 {
                        // 0x123CC
                        api.set_word(MOVER_ANIM, k); // 0x123D8
                    }
                }
                give(api, k); // 0x123DC
            }
        }
        1 | 2 | 4..=8 | 0x0D => give(api, k), // 0x122F1
        _ => {}
    }
}

/// Code 7, `0x1240F`: the argument less 0Ah through the table at
/// `cs:0A11h` (`0x12511`); `0Ah` to `10h`, `1Eh` and `1Fh` have arms.
fn events(api: &mut dyn Api, arg: u16) {
    match arg {
        // 0x12423: out through exit 6Eh.
        0x0A => api.script_exit(EXIT_6E), // 0x12425, 0FE3h:088Ah
        // 0x1242D: the walking record back to node 0BB8h, phase 0Dh.
        0x0B => {
            let m = rec(api, REC_F2);
            api.walker_walk_to(m, NODE_BB8, 0); // 0x1243A
            api.set_word(PHASE, 0x0D); // 0x12442
        }
        // 0x1244B: item 18h given, course 4, on to the Kaffee with
        // entry 1.
        0x0C => {
            api.inv_give(KOSTUEM_18); // 0x1244D
            api.set_byte(COURSE, 4); // 0x12453
            api.set_word(WORDS.location, 1); // 0x12458
            api.set_word(WORDS.entry, 1); // 0x1245E
        }
        // 0x12467: the walking record to node 0BBAh facing 3, task 0
        // armed, phase 0Ah.
        0x0D => {
            let m = rec(api, REC_F2);
            api.walker_walk_to(m, NODE_BBA, 3); // 0x12474
            api.task_arm(0); // 0x1247E
            api.set_word(PHASE, 0x0A); // 0x12484
        }
        // 0x1248D: the kept records' animation forgotten, phase 0Bh.
        0x0E => {
            api.set_word(RECORDS_ANIM, 0xFFFF); // 0x1248D
            api.set_word(PHASE, 0x0B); // 0x12493
        }
        // 0x1249B.
        0x0F => api.set_word(PHASE, 0x0C),
        // 0x124A3: phase 0Fh, the samples of 10h from now, F1h on
        // animation 0, sample 22h.
        0x10 => {
            api.set_word(PHASE, 0x0F); // 0x124A3
            api.set_word(AMBIENT, 0x10); // 0x124A9
            api.set_word(HUNDREDTHS, 0); // 0x124AF
            let r = rec(api, REC_F1);
            api.obj_anim(r, 0); // 0x124BF
            api.play_sample(0x22); // 0x124C9
        }
        // 0x124D1: samples 25h, 23h, 23h, object F0h hidden, course 6.
        0x1E => {
            api.play_sample(0x25); // 0x124D3
            api.play_sample(0x23); // 0x124DB
            api.play_sample(0x23); // 0x124E3
            api.obj_hide(OBJECT_F0); // 0x124EC
            api.set_byte(COURSE, 6); // 0x124F2
        }
        // 0x124F9: sample 25h, the samples of 1Fh from now.
        0x1F => {
            api.play_sample(0x25); // 0x124FB
            api.set_word(AMBIENT, 0x1F); // 0x12501
            api.set_word(HUNDREDTHS, 0); // 0x12507
        }
        _ => {}
    }
}

/// The animation hook, `11B0h:0662h`: the code in `[bp+0Ah]`; codes `64h`
/// and `66h` have arms (`0x12165`–`0x12172`).
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        // 0x12174: samples 22h and 23h, object F0h (`push 0F0h`,
        // 0x12188) hidden and its record on animation 1, the samples of
        // [2DCEh] stopped.
        0x64 => {
            api.play_sample(0x22); // 0x12176
            api.play_sample(0x23); // 0x1217E
            let r = api.obj_show_frame(OBJECT_F0, -1); // 0x1218B
            api.obj_anim(r, 1); // 0x12195
            api.set_word(AMBIENT, 0); // 0x1219D
        }
        // 0x121A5: sample 25h, the input given back.
        0x66 => {
            api.play_sample(0x25); // 0x121A7
            api.input_unlock(); // 0x121AD
        }
        _ => {}
    }
}

/// Samples 25h, 26h and 27h (`0x11B26`–`0x11B3D`, `0x11B58`–`0x11B6F`,
/// `0x11BB4`–`0x11BCB`).
fn samples_25_27(api: &mut dyn Api) {
    api.play_sample(0x25);
    api.play_sample(0x26);
    api.play_sample(0x27);
}

/// `11B0h:007Dh` (`0x11B7D`), an `Eimer` `2Ch` to `2Eh` with object
/// `F0h`: with the `Labyrinth-Karte` scene 12h, the three taken, item
/// `14h` given and samples 25h to 28h; else scene 0Eh, `2Ch` and `2Dh`
/// taken and `2Eh` given.
fn rule_007d(api: &mut dyn Api) -> i16 {
    if api.inv_has(LABYRINTH_KARTE) != 0 {
        // 0x11B82
        api.scene_play(0x12); // 0x11B8E
        api.inv_take(EIMER_2C); // 0x11B96
        api.inv_take(EIMER_2D); // 0x11B9E
        api.inv_take(EIMER_2E); // 0x11BA6
        api.inv_give(EIMER); // 0x11BAE
        samples_25_27(api); // 0x11BB6, 0x11BBE, 0x11BC6
        api.play_sample(0x28); // 0x11BCE
    } else {
        api.scene_play(0x0E); // 0x11BD7
        api.inv_take(EIMER_2C); // 0x11BDF
        api.inv_take(EIMER_2D); // 0x11BE7
        api.inv_give(EIMER_2E); // 0x11BEF
    }
    0x0A // 0x11BF5
}

/// Rambowsky's rule handlers by their offset in segment `11B0h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 11B0:000F (0x11B0F), the Eimer 14h with 87h: scene 0Ch, 14h for
        // 2Ch, samples 25h to 27h.
        0x000F => {
            api.scene_play(0x0C); // 0x11B14
            swap(api, EIMER, EIMER_2C); // 0x11B1E
            samples_25_27(api); // 0x11B28, 0x11B30, 0x11B38
            0x0A // 0x11B3E
        }
        // 11B0:0043 (0x11B43), an Eimer 2Ch to 2Eh with 87h: scene 0Dh,
        // the object taken, samples 25h to 27h, 14h given.
        0x0043 => {
            api.scene_play(0x0D); // 0x11B48
            let object = api.word(OBJECT);
            api.inv_take(object); // 0x11B52
            samples_25_27(api); // 0x11B5A, 0x11B62, 0x11B6A
            api.inv_give(EIMER); // 0x11B72
            0x0A // 0x11B78
        }
        0x007D => rule_007d(api),
        // 11B0:00FA (0x11BFA), the Wasserpistole 29h with 87h: scene 0Fh,
        // samples 28h and 27h, 29h for 2Ah.
        0x00FA => {
            api.scene_play(0x0F); // 0x11BFF
            api.play_sample(0x28); // 0x11C07
            api.play_sample(0x27); // 0x11C0F
            api.inv_take(PISTOLE_29); // 0x11C17
            api.inv_give(PISTOLE_2A); // 0x11C1F
            0x0A // 0x11C25
        }
        // 11B0:012A (0x11C2A), the Wasserpistole 2Ah with 87h: scene 10h,
        // sample 0Ah, 2Ah for 29h.
        0x012A => {
            api.scene_play(0x10); // 0x11C2F
            api.play_sample(0x0A); // 0x11C37
            api.inv_give(PISTOLE_29); // 0x11C3F
            api.inv_take(PISTOLE_2A); // 0x11C47
            0x0A // 0x11C4D
        }
        // 11B0:0152 (0x11C52), the Wasserpistole 2Ah with F0h: scene 11h,
        // 2Ah for 29h.
        0x0152 => {
            api.scene_play(0x11); // 0x11C57
            api.inv_give(PISTOLE_29); // 0x11C5F
            api.inv_take(PISTOLE_2A); // 0x11C67
            0x0A // 0x11C6D
        }
        // 11B0:0172 (0x11C72), the Eimer 2Eh with the Stadtplan: scene 9,
        // the Stadtplan for the Labyrinth-Karte.
        0x0172 => {
            api.scene_play(9); // 0x11C77
            api.inv_take(STADTPLAN); // 0x11C7F
            api.inv_give(LABYRINTH_KARTE); // 0x11C87
            0x0A // 0x11C8D
        }
        // 11B0:0192 (0x11C92), the Eimer 2Eh with the Labyrinth-Karte.
        0x0192 => {
            api.scene_play(0x0A); // 0x11C97
            0x0A // 0x11C9D
        }
        // 11B0:01A2 (0x11CA2), object F0h on nothing: one of scenes !0x32
        // and !0x31.
        0x01A2 => {
            roll(api, !0x32, !0x31); // 0x11CA9, 0x11CB2
            0x0A // 0x11CB8
        }
        // 11B0:01BD (0x11CBD), the Wasserpistole 2Ah on nothing.
        0x01BD => {
            api.scene_play(!0x17); // 0x11CC2
            0x0A // 0x11CC8
        }
        // 11B0:01CD (0x11CCD), the Wasserpistole 29h on nothing.
        0x01CD => {
            api.scene_play(!0x16); // 0x11CD2
            0x0A // 0x11CD8
        }
        // 11B0:01DD (0x11CDD), an Eimer 2Ch to 2Eh on nothing.
        0x01DD => {
            api.scene_play(!0x18); // 0x11CE2
            0x0A // 0x11CE8
        }
        // 11B0:01ED (0x11CED), the Eimer 14h on nothing: one of scenes
        // !0x1F to !0x1D.
        0x01ED => {
            roll(api, !0x1F, !0x1D); // 0x11CF4, 0x11CFD
            0x0A // 0x11D03
        }
        // 11B0:0208 (0x11D08), the Stadtplan or the Labyrinth-Karte on
        // nothing: one of scenes !0x24 to !0x22.
        0x0208 => {
            roll(api, !0x24, !0x22); // 0x11D0F, 0x11D18
            0x0A // 0x11D1E
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
