//! Location 8, Jones 1, a row of kind 1 with two exits: `6Eh` to Jones 2
//! (location 9) and `6Fh` to the Flur 6. Stock (location 35). The module
//! is named after the location as the game itself names it in its
//! location table (`Jones 1`, the record's `+37h`, `250Ah:00CFh`).
//!
//! Re-expressed from the handler at `1258h:0005h` (image `0x12585` to its
//! `retf` at `0x127D1`), its hook `1258h:035Eh` (`0x128DE` to `0x12BE6`,
//! the record's `+3Fh`) and its animation hook `1258h:0252h` (`0x127D2` to
//! `0x128DD`, the record's `+43h`), against [`Api`] alone. The record's
//! rule table `+3Bh` is null: the location has no rules of its own. The
//! handlers that follow the hook's jump tables (`0x12C17` on) are
//! `12C1h:0007h` on, Jones 2's, named by that location's rule table at
//! `2446h:03BAh`.
//!
//! The location's figures are objects `F1h` to `F4h`, their records kept
//! at `DS:2DEAh` on, a far pointer each. Each figure has a row of six
//! bytes in segment `2446h`, its far pointer kept at `DS:2DFAh` on (null
//! for a figure without one), with the frame it was last given at
//! `DS:2DD8h` on and the frame wanted at `DS:2DE0h` on, a word each:
//!
//! | row | bytes | figure |
//! |---|---|---|
//! | `0360h` | `00 01 02 01 01 01` | 0, the handler |
//! | `0366h` | `06 07 08 07 07 07` | 0, the animation hook's code `65h` |
//! | `036Ch` | `00 01 01 03 04 01` | 1, the player's `+28h` not `F0h` |
//! | `0372h` | `00 04 01 03 04 01` | 1, the player's `+28h` `F0h` |
//! | `0378h` | `00 01 01 01 04 01` | 2 |
//! | `037Eh` | `00 01 01 01 01 05` | 3 |
//!
//! A scene claiming its speakers (the hook's code 0, the argument their
//! bits) gives figure 0 its row's byte 0 for bit 1 and sets figures 1 to 3
//! on their rows for any of bits 2 to 4; a page's speaker (code 8, the
//! argument) names byte `1 + arg` of each row as the frame wanted, and the
//! page's end (code 9) gives it. The scene's end (codes 5 and 6) gives
//! each figure byte 0 of its row and lets the rows go.
//!
//! Five bytes of the logic's saved state block (`DS:2B98h`, `4Eh` bytes)
//! hold the location's progress. `[2BA8h]`, which the VIP-Lounge sets to 1
//! and Bi-Fi and the Maske carry on to 3 (`0x15A45`), below 3 plays one of
//! scenes 0 to 2 in turn (counter `[2BA9h]`) with figure `F1h` alone. From
//! 3 on `[2BACh]` leads: 0 plays scenes 6 to 8 in turn (counter `[2BAAh]`)
//! and sets `[2BABh]`; 1, which the global rule `0FE3h:0112h` sets once
//! the `Bedienungsanleitung` (item `1Ch`) is carried (`0x0FF60`), plays
//! scene 9 or scenes 9 to `0Ch` (counter `[2BABh]`) until the `Pyrotool`
//! (item `1Dh`) is carried, then scene `0Dh` or `0Eh` and sets `[2BACh]` to
//! 2; 2 hides the figures. The items are named by the item row at
//! `DS:10AAh`.
//!
//! The hook's code 7 events walk the player (`0Ch`–`0Eh`), arm the task
//! the event came from and mark `[2DE8h]`, after which the frame loop
//! starts task 0 once the player record's `+28h` is above zero
//! (`0Fh`–`11h`), take the `Pyrotool` into
//! the room as object `1Dh` (`12h`), leave through exit `6Fh` (`0Ah`) or
//! for the Kaffee (location 1) with entry 2 (`0Bh`), whose row at
//! `DS:182Ch` leads back to location 8.

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x1258;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x127B8`).
const OBJECT_STATES: u16 = 0x2018;

/// Object `F1h`, the first of the four figures `F1h` to `F4h`
/// (`add ax, 0F1h`, `0x12620`).
const FIGURE_F1: u16 = 0xF1;
/// The figures (`mov si, 4`, `0x12617`).
const FIGURES: u16 = 4;
/// Object `F5h`, hidden with the `Pyrotool` carried and set on its
/// animation by the animation hook.
const OBJECT_F5: u16 = 0xF5;
/// Object `F6h`, shown by the animation hook's code `65h` (`0x12893`).
const OBJECT_F6: u16 = 0xF6;
/// Objects `C8h` to `CEh` (`mov si, 7 ; … add ax, 0C8h`, `0x126CF`).
const OBJECT_C8: u16 = 0xC8;
const SEVEN: u16 = 7;
/// Object `1Dh`, the `Pyrotool` laid down (`0x12BAB`).
const OBJECT_1D: u16 = 0x1D;
/// Item `1Dh`, the `Pyrotool`.
const PYROTOOL: u16 = 0x1D;
/// An object record's two cel words, `+0Ah` and `+0Ch` (`0x126C8`,
/// `0x126F5`, `0x126F9`).
const CEL: u16 = 0x0A;
const CEL_BASE: u16 = 0x0C;

/// The segment of the figures' rows (`mov word [2DFCh], 2446h`,
/// `0x125AE`).
const ROWS_SEG: u16 = 0x2446;
const ROW_0360: u16 = 0x0360;
const ROW_0366: u16 = 0x0366;
const ROW_036C: u16 = 0x036C;
const ROW_0372: u16 = 0x0372;
const ROW_0378: u16 = 0x0378;
const ROW_037E: u16 = 0x037E;

/// Exit `6Fh`, to the Flur 6. Stock (`push 6Fh`, `0x12B41`).
const EXIT_6F: u16 = 0x6F;
/// The Kaffee, location 1, and the entry event `0Bh` sends it with
/// (`0x12B4A`, `0x12B50`).
const KAFFEE: u16 = 1;
const KAFFEE_ENTRY: u16 = 2;

/// The nodes events `0Ch` to `0Eh` send the player to, and the facing each
/// asks for.
const NODE_F0: u16 = 0x00F0;
const NODE_F1: u16 = 0x00F1;
const NODE_FA0: u16 = 0x0FA0;

/// `[1796h]`: the player record's `+28h` (`DS:176Eh + 28h`).
const PLAYER_28: u16 = WALKER + 0x28;
/// `[2D4Ah]`: the mode block's walk target.
const WALK_TARGET: u16 = 0x2D4A;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BA8h]`: the story's stage, carried on by locations 25, 12 and 22.
const BYTE_2BA8: u16 = 0x2BA8;
/// `[2BA9h]`: the counter of scenes 0 to 2.
const BYTE_2BA9: u16 = 0x2BA9;
/// `[2BAAh]`: the counter of scenes 6 to 8.
const BYTE_2BAA: u16 = 0x2BAA;
/// `[2BABh]`: 0, 1 after scenes 6 to 8, 2 after scene 9; the counter of
/// scenes 9 to `0Ch`.
const BYTE_2BAB: u16 = 0x2BAB;
/// `[2BACh]`: the location's stage, 0 to 2.
const BYTE_2BAC: u16 = 0x2BAC;

/// The handler's and the hook's words, read and written in segment
/// `1258h` alone; the image holds 0 for each.
/// `[2DD4h]`: the figures' rest owed at the scene's end (event `13h`).
const REST_OWED: u16 = 0x2DD4;
/// `[2DD6h]`: the countdown of the animation hook's codes `65h` and `66h`.
const COUNTDOWN: u16 = 0x2DD6;
/// `DS:2DD8h`: four words, the frame each figure was last given.
const GIVEN: u16 = 0x2DD8;
/// `DS:2DE0h`: four words, the frame each figure is wanted on.
const WANTED: u16 = 0x2DE0;
/// `[2DE8h]`: non-zero while task 0 is to be started by the frame loop
/// (`0x1278A`); events `0Fh` to `11h` write a node into it, which nothing
/// compares.
const AWAITED: u16 = 0x2DE8;
/// `DS:2DEAh`: four far pointers, the figures' records.
const RECORDS: u16 = 0x2DEA;
/// `DS:2DFAh`: four far pointers, the figures' rows.
const ROWS: u16 = 0x2DFA;

fn far_at(api: &dyn Api, at: u16) -> Far {
    Far {
        off: api.word(at),
        seg: api.word(at.wrapping_add(2)),
    }
}

fn keep(api: &mut dyn Api, at: u16, r: Far) {
    api.set_word(at.wrapping_add(2), r.seg);
    api.set_word(at, r.off);
}

/// Figure `i`'s record, `[2DEAh + 4i]`.
fn record(api: &dyn Api, i: u16) -> Far {
    far_at(api, RECORDS.wrapping_add(i << 2))
}

/// Figure `i`'s row, `[2DFAh + 4i]`.
fn row(api: &dyn Api, i: u16) -> Far {
    far_at(api, ROWS.wrapping_add(i << 2))
}

fn set_row(api: &mut dyn Api, i: u16, off: u16) {
    let r = if off == 0 {
        Far::NULL
    } else {
        Far { off, seg: ROWS_SEG }
    };
    keep(api, ROWS.wrapping_add(i << 2), r);
}

fn given_at(i: u16) -> u16 {
    GIVEN.wrapping_add(i << 1)
}

fn wanted_at(i: u16) -> u16 {
    WANTED.wrapping_add(i << 1)
}

/// Byte `k` of a figure's row (`les bx, [2DFAh + 4i] ; mov al, es:[bx+k]`).
fn row_byte(api: &mut dyn Api, r: Far, k: u16) -> u16 {
    u16::from(api.far_byte(Far {
        off: r.off.wrapping_add(k),
        seg: r.seg,
    }))
}

fn field(r: Far, k: u16) -> Far {
    Far {
        off: r.off.wrapping_add(k),
        seg: r.seg,
    }
}

/// `push k ; push id ; call far 06E3h:047Eh ; pop cx ; push dx ; push ax ;
/// call far 03F7h:1CF9h`: the object found and set on animation `k`.
fn anim(api: &mut dyn Api, id: u16, k: i16) {
    let r = api.obj_find(id);
    api.obj_anim(r, k);
}

/// The figures `F4h` down to `F1h` shown, kept and set on animation 0
/// (`0x1261C`–`0x12646`, `0x1267A`–`0x126A4`).
fn show_figures(api: &mut dyn Api) {
    for i in (0..FIGURES).rev() {
        let r = api.obj_show(FIGURE_F1 + i); // 0x12624, 0x12682
        keep(api, RECORDS.wrapping_add(i << 2), r); // 0x1262F, 0x1268D
        api.obj_anim(r, 0); // 0x12639, 0x12697
    }
}

/// Everything before the frame loop, `0x12585` to `0x1277A`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x1258A
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x12596
    api.visit_enter(loc, entry); // 0x1259A
    if api.byte(BYTE_2BA8) < 3 {
        // 0x125A2: figure F1h alone, one of scenes 0 to 2 in turn.
        api.input_lock(); // 0x125A9
        set_row(api, 0, ROW_0360); // 0x125AE, 0x125B4
        let counter = api.ds(BYTE_2BA9);
        api.scene_sequence(counter, &[0, 1, 2]); // 0x125C6
        let r = api.obj_show(FIGURE_F1); // 0x125D3
        keep(api, RECORDS, r); // 0x125D9, 0x125DD
        api.obj_anim(r, 0); // 0x125E2
    } else {
        match api.byte(BYTE_2BAC) {
            // 0x12606: the four figures, one of scenes 6 to 8 in turn.
            0 => {
                api.input_lock(); // 0x12606
                set_row(api, 0, ROW_0360); // 0x1260B, 0x12611
                show_figures(api);
                let counter = api.ds(BYTE_2BAA);
                api.scene_sequence(counter, &[6, 7, 8]); // 0x12654
                api.set_byte(BYTE_2BAB, 1); // 0x1265C
            }
            // 0x12664: the four figures, and the Pyrotool asked for.
            1 => {
                api.input_lock(); // 0x12664
                set_row(api, 0, ROW_0360); // 0x12669, 0x1266F
                show_figures(api);
                if api.inv_has(PYROTOOL) != 0 {
                    // 0x126A8: object F5h hidden, its cel base given to
                    // objects CEh down to C8h; scene 0Dh or 0Eh.
                    let f5 = api.obj_show_frame(OBJECT_F5, -1); // 0x126B7
                    let cel = api.far_word(field(f5, CEL_BASE)); // 0x126C8
                    for i in (0..SEVEN).rev() {
                        let r = api.obj_show_frame(OBJECT_C8 + i, -1); // 0x126DC
                        if r != Far::NULL {
                            // 0x126EA
                            api.set_far_word(field(r, CEL), cel); // 0x126F5
                            api.set_far_word(field(r, CEL_BASE), cel); // 0x126F9
                        }
                    }
                    let n = if api.byte(BYTE_2BAB) == 0 { 0x0D } else { 0x0E };
                    api.scene_play(n); // 0x12704, 0x12711
                    api.set_byte(BYTE_2BAC, 2); // 0x12717
                } else if api.byte(BYTE_2BAB) == 0 {
                    // 0x1271E
                    api.scene_play(9); // 0x12727
                    api.set_byte(BYTE_2BAB, 2); // 0x1272D
                } else {
                    let counter = api.ds(BYTE_2BAB);
                    api.scene_sequence(counter, &[9, 0x0A, 0x0B, 0x0C]); // 0x12742
                }
            }
            // 0x1274C: the figures' rows let go and the figures hidden.
            2 => {
                for i in (0..FIGURES).rev() {
                    set_row(api, i, 0); // 0x12756, 0x1275C
                    api.obj_hide(FIGURE_F1 + i); // 0x12768
                }
            }
            _ => {}
        }
    }
    api.visit_finish_entry(); // 0x12775
    api.set_word(WORDS.entry, 0); // 0x1277A
}

/// One turn of the frame loop, `0x12780` to `0x127B0`: an armed task 0
/// starts once the player record's `+28h` is above zero.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x12780
    api.dispatch(); // 0x12785
    if api.word(AWAITED) != 0 && num::signed(api.word(PLAYER_28)) > 0 {
        // 0x1278A, 0x12791
        api.task_start(0); // 0x1279A
        api.set_word(AWAITED, 0); // 0x127A0
    }
    api.frame_pass(); // 0x127A6
    api.word(WORDS.entry) == 0 // 0x127AB
}

/// Everything after the frame loop, `0x127B2` to `0x127D1`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x127BD
    api.visit_leave(); // 0x127C5
    api.slot_unload(); // 0x127CA
}

/// The hook, `1258h:035Eh`: code in `[bp+6]`, argument in `[bp+8]`, task
/// in `[bp+0Ah]`. Codes above 9 and codes 1 to 4 do nothing (`0x128EC`,
/// the table at `cs:0683h`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, task: u16) {
    match code {
        0 => speakers(api, arg),     // 0x128FB
        5 | 6 => scene_end(api),     // 0x129AE
        7 => events(api, arg, task), // 0x12B2D
        // 0x12AA2: each figure with a row wants its byte 1 + arg.
        8 => {
            for i in (0..FIGURES).rev() {
                let r = row(api, i);
                if r != Far::NULL {
                    // 0x12AAC
                    let k = row_byte(api, r, arg.wrapping_add(1)); // 0x12ABF
                    api.set_word(wanted_at(i), k); // 0x12AC9
                }
            }
        }
        // 0x12AD7: each figure with a row and a new frame wanted is given it.
        9 => {
            for i in (0..FIGURES).rev() {
                if row(api, i) == Far::NULL {
                    continue; // 0x12AE9
                }
                let k = api.word(wanted_at(i));
                if api.word(given_at(i)) == k {
                    continue; // 0x12AFB
                }
                api.set_word(given_at(i), k); // 0x12B09
                let r = record(api, i);
                api.obj_anim(r, num::signed(k)); // 0x12B1B
            }
        }
        _ => {}
    }
}

/// Code 0, `0x128FB`: for any of bits 2 to 4 figures 1 to 3 are set on
/// their rows, else their rows let go; bit 1 gives figure 0 byte 0 of its
/// row, else its row lets go.
fn speakers(api: &mut dyn Api, arg: u16) {
    if arg & 0x1C != 0 {
        let first = if api.word(PLAYER_28) == NODE_F0 {
            ROW_0372 // 0x12909, 0x1290F
        } else {
            ROW_036C // 0x12917, 0x1291D
        };
        set_row(api, 1, first);
        set_row(api, 2, ROW_0378); // 0x12923, 0x12929
        set_row(api, 3, ROW_037E); // 0x1292F, 0x12935
        for i in (1..FIGURES).rev() {
            api.set_word(wanted_at(i), 0); // 0x12946
            api.set_word(given_at(i), 0); // 0x1294E
            let r = record(api, i);
            api.obj_anim(r, 0); // 0x12961
        }
    } else {
        for i in (1..FIGURES).rev() {
            set_row(api, i, 0); // 0x12978, 0x1297E
        }
    }
    if arg & 2 == 0 {
        set_row(api, 0, 0); // 0x1298D, 0x12993
        return;
    }
    // 0x1299C: figure 0's row read without a test for null.
    let r = row(api, 0);
    let k = row_byte(api, r, 0); // 0x129A0
    api.set_word(wanted_at(0), k); // 0x129A5
    api.set_word(given_at(0), k); // 0x129A8
}

/// Codes 5 and 6, `0x129AE`: each figure with a row is given its byte 0
/// and the row lets go; then, with the rest owed, the pointer's hover off,
/// the player faces 3 and the figures, `F5h` and `CEh` to `C8h` rest.
fn scene_end(api: &mut dyn Api) {
    for i in (0..FIGURES).rev() {
        let r = row(api, i);
        if r != Far::NULL {
            // 0x129C0
            let k = row_byte(api, r, 0); // 0x129CB
            let rec = record(api, i);
            api.obj_anim(rec, num::signed(k)); // 0x129DE
            set_row(api, i, 0); // 0x129EB, 0x129F1
        }
        api.set_word(wanted_at(i), 0xFFFF); // 0x129FE
        api.set_word(given_at(i), 0xFFFF); // 0x12A06
    }
    api.set_word(wanted_at(0), 0xFFFF); // 0x12A14
    api.set_word(given_at(0), 0xFFFF); // 0x12A17
    if api.word(REST_OWED) == 0 {
        return; // 0x12A1A
    }
    api.hover_off(); // 0x12A24
    api.set_word(COUNTDOWN, 7); // 0x12A29
    let player = api.ds(WALKER);
    api.figure_face(player, 3); // 0x12A35
    for i in (0..FIGURES).rev() {
        let r = record(api, i);
        api.obj_anim(r, 0); // 0x12A51
    }
    anim(api, OBJECT_F5, 0); // 0x12A65, 0x12A6D
    for i in (0..SEVEN).rev() {
        anim(api, OBJECT_C8 + i, 0); // 0x12A82, 0x12A8A
    }
    api.set_word(REST_OWED, 0); // 0x12A99
}

/// Code 7, `0x12B2D`: the argument less `0Ah`, `0` to `0Dh`, through the
/// table at `cs:0667h`.
fn events(api: &mut dyn Api, arg: u16, task: u16) {
    match arg {
        0x0A => api.script_exit(EXIT_6F), // 0x12B43, 0FE3h:088Ah
        0x0B => {
            api.set_word(WORDS.entry, KAFFEE_ENTRY); // 0x12B4A
            api.set_word(WORDS.location, KAFFEE); // 0x12B50
        }
        0x0C => walk(api, NODE_F0, 1),           // 0x12B59
        0x0D => walk(api, NODE_F1, 3),           // 0x12B60
        0x0E => walk(api, NODE_FA0, 1),          // 0x12B67
        0x0F => await_node(api, task, NODE_F0),  // 0x12B7F, 0x12B85
        0x10 => await_node(api, task, NODE_F1),  // 0x12B8E, 0x12B94
        0x11 => await_node(api, task, NODE_FA0), // 0x12B9D, 0x12BA3
        0x12 => {
            api.obj_show(OBJECT_1D); // 0x12BAD
            api.inv_take(PYROTOOL); // 0x12BB5
        }
        0x13 => api.set_word(REST_OWED, 1),       // 0x12BBD
        0x14 => api.set_word(wanted_at(1), 7),    // 0x12BC5
        0x15 => api.set_word(wanted_at(1), 6),    // 0x12BCD
        0x16 => api.set_word(wanted_at(0), 0x0C), // 0x12BD5
        0x17 => api.set_word(wanted_at(2), 6),    // 0x12BDD
        _ => {}
    }
}

/// `mov [2D4Ah], ax ; push ax ; push ds ; push 176Eh ; call far
/// 05DCh:05C8h` (`0x12B6C`–`0x12B74`); the answer is not read.
fn walk(api: &mut dyn Api, node: u16, facing: i16) {
    api.set_word(WALK_TARGET, node); // 0x12B6C
    let player = api.ds(WALKER);
    api.walker_walk_to(player, node, facing); // 0x12B74
}

/// `push ax ; call far 080Ch:193Fh` with the hook's task, and `[2DE8h]`
/// set to `node`.
fn await_node(api: &mut dyn Api, task: u16, node: u16) {
    api.task_arm(task);
    api.set_word(AWAITED, node);
}

/// The animation hook, `1258h:0252h`: the record in `[bp+6]`, the code in
/// `[bp+0Ah]`; codes `64h` to `66h` have arms (`0x127D6`–`0x127E8`).
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        // 0x127EE: one of samples 20h to 22h.
        0x64 => {
            let n = api.random(0, 2); // 0x127F2
            api.play_sample(n.wrapping_add(0x20)); // 0x127FE
        }
        // 0x12807: sample 23h, or one of 20h to 23h; with the countdown run
        // out, the figures' second play and the countdown from 7 again.
        0x65 => {
            let roll = api.random(0, 4); // 0x1280B
            let n = if roll == 0 || roll == 1 {
                0x23 // 0x1281C
            } else {
                api.random(0, 3).wrapping_add(0x20) // 0x12824, 0x1282C
            };
            api.play_sample(n); // 0x12830
            if counted_down(api) {
                second_play(api);
            }
        }
        // 0x128C1: with the countdown run out (it is not set again here),
        // the pointer shown and scene 5.
        0x66 => {
            let run_out = counted_down(api);
            if run_out {
                api.pointer_show(); // 0x128CC, 1819h:0AB0h
                api.scene_play(5); // 0x128D3
            }
        }
        _ => {}
    }
}

/// `mov ax, [2DD6h] ; dec word [2DD6h] ; or ax, ax ; jg`: whether the
/// countdown was at zero or below before it stepped down (`0x12836`,
/// `0x128C1`).
fn counted_down(api: &mut dyn Api) -> bool {
    let before = api.word(COUNTDOWN);
    api.set_word(COUNTDOWN, before.wrapping_sub(1)); // 0x12839, 0x128C4
    num::signed(before) <= 0 // 0x1283F, 0x128CA
}

/// `0x12844`: the countdown from 7, `F5h` and `CEh` to `C8h` on animation
/// 1, sample 15h twice, object `F6h` shown, figure 0 on animation 6 with
/// row `0366h`.
fn second_play(api: &mut dyn Api) {
    api.set_word(COUNTDOWN, 7); // 0x12844
    anim(api, OBJECT_F5, 1); // 0x1284F, 0x12857
    for i in (0..SEVEN).rev() {
        anim(api, OBJECT_C8 + i, 1); // 0x1286C, 0x12874
    }
    api.play_sample(0x15); // 0x12885
    api.play_sample(0x15); // 0x1288D
    api.obj_show(OBJECT_F6); // 0x12896
    api.set_word(given_at(0), 6); // 0x1289F
    let r = record(api, 0);
    api.obj_anim(r, 6); // 0x128AB
    set_row(api, 0, ROW_0366); // 0x128B3, 0x128B9
}

/// The location has no rules of its own (the record's `+3Bh` is null).
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
