//! Location 29, the 08-15, a row of kind 1 with one exit, `6Fh`, to the
//! Flur 7. Stock (location 30, see [`super::flur`]). The module is named
//! after the location as the game itself names it in its location table
//! (`08-15`, the record's `+37h`, `250Ah:01B3h`), spelled out.
//!
//! Re-expressed from the handler at `16EBh:0058h` (image `0x16F08` to its
//! `retf` at `0x16FBA`), its hook `16EBh:014Bh` (`0x16FFB` to `0x1713C`,
//! the record's `+3Fh`), its animation hook `16EBh:010Bh` (`0x16FBB` to
//! `0x16FFA`, the record's `+43h`) and its three rule handlers
//! `16EBh:000Dh` to `003Dh` (`0x16EBD`–`0x16F07`), against [`Api`] alone.
//! The segment installs no gate of its own. The location's rules are the
//! four records at `2446h:0AE0h` (image `0x24F40`, the record's `+3Bh`),
//! fourteen bytes each, ended by the zero phase at `0x24F78`:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 1 | 4 | `A2h` | — | `000Dh` |
//! | 5 | `20h` | `26h` | `A2h` | `0028h` |
//! | 5 | `20h` | `26h` | 0 | `003Dh` |
//! | 4 | 4 | `A2h` | — | `000Dh` |
//!
//! The verbs are, by the verb text row at `DS:116Ah` (indexed by `[2D38h]`
//! × 4 at `0x0B02D`): 1 `Betrachte`, 4 `Benutze`, 5 `Benutze … mit`. The
//! items the logic names are, by the item row at `DS:10AAh` (× 4,
//! `0x0A5A5`): `0Eh` `Fotostreifen`, `26h` `Geld`.
//!
//! Object `A2h` is the one the `Geld` is used on: `Betrachte` or
//! `Benutze` on it plays scene 4 until the `Geld` has been used on it,
//! then scene 5; the `Geld` on it sets `[2BDAh]` and plays scene 6. One
//! byte of the logic's saved state block (`DS:2B98h`, `4Eh` bytes) is the
//! location's: `[2BDAh]`, read and written in segment `16EBh` alone.
//!
//! Three figures, objects `F2h`, `F1h` and `F0h` (also the row's bytes
//! `+29h` on, with `A2h`), are shown on entry and their records kept at
//! `DS:2F80h` on, a far pointer each; scenes 0 and 1 run behind them on
//! task 1 (`080Ch:18B5h(0, 1, 1)`, `0x16F56`, [`Api::ambient_play`]).
//! The hook's code 8, for a task other than 0, gives the figure whose
//! subject slot the page names animation 1 and the others animation 0.
//!
//! The hook's code 7 events work on object `A2h`: `0Ah` shows it on
//! frame −1, sets it on animation 0 and marks task 0 to be held
//! (`[2F7Eh]` `FFFFh`); `0Bh` arms task 0 and keeps it held (`FFFEh`)
//! when so marked, and takes the `Geld`; `0Ch` and `0Dh` set `A2h` on
//! animations 1 and 2, `0Dh` also arming task 0 until the player stands
//! on node `0FA0h`, which the frame loop watches. The animation hook's
//! code `64h` starts a task 0 held by `0Bh`, code `65h` walks the player
//! to node `0FA0h`. Item event `27h` gives the `Fotostreifen`.

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x16EB;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x16FA3`).
const OBJECT_STATES: u16 = 0x2018;

/// The three figures, in the order of their kept records (`0x16F20`–
/// `0x16F4D`).
const FIGURES: [u16; 3] = [0xF2, 0xF1, 0xF0];
/// Object `A2h`, the one the `Geld` is used on.
const OBJECT_A2: u16 = 0xA2;

/// `DS:2F80h`: three far pointers, the figures' records (`0x16F2D`,
/// `0x16F3D`, `0x16F4D`; `[bx+2F80h]`, `bx = i × 4`, `0x17055`).
const RECORDS: u16 = 0x2F80;
/// `[2F7Ch]`: the subject slot the figures' animations were last given
/// for, `FFFFh` none (`0x1701F`, `0x17068`).
const SPEAKER: u16 = 0x2F7C;
/// `[2F7Eh]`: task 0's hold, signed: `FFFFh` to be held, `FFFEh` held for
/// the animation hook's code `64h`, a node (`0FA0h`) held until the
/// player stands on it, 0 none.
const HOLD: u16 = 0x2F7E;
/// The marks `[2F7Eh]` takes besides a node (`0x17097`, `0x170C9`).
const HOLD_MARKED: u16 = 0xFFFF;
const HOLD_ARMED: u16 = 0xFFFE;

/// The node task 0 waits for and the animation hook walks the player to
/// (`0x16FE6`, `0x1712B`).
const NODE_FA0: u16 = 0x0FA0;
/// `[1796h]`: the player record's `+28h`, its node (`0x16F7A`).
const PLAYER_NODE: u16 = WALKER + 0x28;
/// `[2D4Ah]`: the mode block's walk target (`0x16FE9`).
const WALK_TARGET: u16 = 0x2D4A;

/// The logic's state byte (inside the saved block at `DS:2B98h`).
/// `[2BDAh]`: the `Geld` used on object `A2h`.
const BYTE_2BDA: u16 = 0x2BDA;

/// The `Fotostreifen`, item `0Eh` (`0x17079`), and the `Geld`, item
/// `26h` (`0x170E7`).
const FOTOSTREIFEN: u16 = 0x0E;
const GELD: u16 = 0x26;
/// The item event that gives the `Fotostreifen` (`cmp ax, 27h`,
/// `0x17071`).
const EVENT_FOTOSTREIFEN: u16 = 0x27;

fn keep(api: &mut dyn Api, at: u16, r: Far) {
    api.set_word(at.wrapping_add(2), r.seg);
    api.set_word(at, r.off);
}

/// Figure `i`'s kept record (`[bx+2F80h]`, `[bx+2F82h]`, `bx = i × 4`).
fn record(api: &dyn Api, i: u16) -> Far {
    let at = RECORDS.wrapping_add(i << 2);
    Far {
        off: api.word(at),
        seg: api.word(at.wrapping_add(2)),
    }
}

/// `push k ; push id ; call far 06E3h:047Eh ; pop cx ; push dx ; push ax ;
/// call far 03F7h:1CF9h`: the object found and set on animation `k`.
fn anim(api: &mut dyn Api, id: u16, k: i16) {
    let r = api.obj_find(id);
    api.obj_anim(r, k);
}

/// Everything before the frame loop, `0x16F08` to `0x16F63`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x16F08
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x16F14
    api.visit_enter(loc, entry); // 0x16F18
    for (i, id) in (0u16..).zip(FIGURES) {
        let r = api.obj_show(id); // 0x16F23, 0x16F33, 0x16F43
        keep(api, RECORDS.wrapping_add(i << 2), r); // 0x16F29 … 0x16F4D
    }
    api.ambient_play(0, 1, 1); // 0x16F56, the answer not read
    api.visit_finish_entry(); // 0x16F5E
    api.set_word(WORDS.entry, 0); // 0x16F63
}

/// One turn of the frame loop, `0x16F69` to `0x16F9B`: once the player
/// stands on the node task 0 is held for, task 0 starts.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x16F69
    api.dispatch(); // 0x16F6E
    let hold = api.word(HOLD);
    if num::signed(hold) > 0 && api.word(PLAYER_NODE) == hold {
        // 0x16F73 (`jng`), 0x16F7A
        api.task_start(0); // 0x16F85
        api.set_word(HOLD, 0); // 0x16F8B
    }
    api.frame_pass(); // 0x16F91
    api.word(WORDS.entry) == 0 // 0x16F96
}

/// Everything after the frame loop, `0x16F9D` to `0x16FBA`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x16FA8
    api.visit_leave(); // 0x16FB0
    api.slot_unload(); // 0x16FB5
}

/// The animation hook, `16EBh:010Bh`: the code in `[bp+0Ah]`; codes `64h`
/// and `65h` have arms (`0x16FBE`–`0x16FCB`).
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        // 0x16FCD: a task 0 held by event 0Bh started; the hold over.
        0x64 => {
            if num::signed(api.word(HOLD)) < -1 {
                // 0x16FCD (`cmp …, -1 ; jnl`)
                api.task_start(0); // 0x16FD6
            }
            api.set_word(HOLD, 0); // 0x16FDC
        }
        // 0x16FE4: the walk target and the player's walk to node 0FA0h,
        // facing 0; the answer not read.
        0x65 => {
            api.set_word(WALK_TARGET, NODE_FA0); // 0x16FE9
            let player = api.ds(WALKER);
            api.walker_walk_to(player, NODE_FA0, 0); // 0x16FF1
        }
        _ => {}
    }
}

/// The hook, `16EBh:014Bh`: code in `[bp+6]`, argument in `[bp+8]`, task
/// in `[bp+0Ah]`. Codes above 8 and codes 1 to 3, 5 and 6 do nothing
/// (`0x17009`, the table at `cs:0295h`, `0x17145`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, task: u16) {
    match code {
        // 0x17018: for a task other than 0 the figures' speaker forgotten.
        0 => {
            if task != 0 {
                api.set_word(SPEAKER, 0xFFFF); // 0x1701F
            }
        }
        // 0x1706F: item event 27h gives the Fotostreifen; the answer not
        // read.
        4 => {
            if arg == EVENT_FOTOSTREIFEN {
                api.inv_give(FOTOSTREIFEN); // 0x1707B
            }
        }
        7 => events(api, arg), // 0x17083
        // 0x17028: a page of a task other than 0 naming a new speaker:
        // figures 2, 1 and 0 on animation 1 for the speaker (slot i + 1),
        // else 0.
        8 => {
            if task == 0 || arg == api.word(SPEAKER) {
                return; // 0x1702A, 0x17033
            }
            for i in (0..3u16).rev() {
                // 0x17038: di from 3, decremented before each turn.
                let k = i16::from(i + 1 == arg); // 0x1703D–0x1704B
                let r = record(api, i);
                api.obj_anim(r, k); // 0x17059
            }
            api.set_word(SPEAKER, arg); // 0x17068
        }
        _ => {}
    }
}

/// Code 7, `0x17083`: events `0Ah` to `0Dh` through the table at
/// `cs:028Dh` (`0x1713D`); others do nothing.
fn events(api: &mut dyn Api, arg: u16) {
    match arg {
        // 0x17097: task 0 marked to be held, object A2h shown on frame -1
        // and set on animation 0, sample 1Ch.
        0x0A => {
            api.set_word(HOLD, HOLD_MARKED); // 0x17097
            let r = api.obj_show_frame(OBJECT_A2, -1); // 0x170A4
            api.obj_anim(r, 0); // 0x170AE
            api.play_sample(0x1C); // 0x17107, by 0x170B8
        }
        // 0x170BA: when marked, task 0 armed and held for code 64h, else
        // no hold; samples 2Bh, 2Bh; the Geld taken, the answer not read.
        0x0B => {
            if num::signed(api.word(HOLD)) < 0 {
                // 0x170BA (`jnl`)
                api.task_arm(0); // 0x170C3
                api.set_word(HOLD, HOLD_ARMED); // 0x170C9
            } else {
                api.set_word(HOLD, 0); // 0x170D1
            }
            api.play_sample(0x2B); // 0x170D9
            api.play_sample(0x2B); // 0x170E1
            api.inv_take(GELD); // 0x170E9
        }
        // 0x170F0: A2h on animation 1, sample 2Ch.
        0x0C => {
            anim(api, OBJECT_A2, 1); // 0x170F5, 0x170FD
            api.play_sample(0x2C); // 0x17107
        }
        // 0x1710E: A2h on animation 2, sample 1Ch, task 0 armed and held
        // until the player stands on node 0FA0h.
        0x0D => {
            anim(api, OBJECT_A2, 2); // 0x17113, 0x1711B
            api.play_sample(0x1C); // 0x17125
            api.set_word(HOLD, NODE_FA0); // 0x1712B
            api.task_arm(0); // 0x17133
        }
        _ => {}
    }
}

/// `push hi ; push lo ; call far 203Dh:0024h ; push ax ; call far
/// 080Ch:164Ch`: the roll is the scene.
fn roll(api: &mut dyn Api, lo: u16, hi: u16) {
    let n = api.random(lo, hi);
    api.scene_play(num::signed(n));
}

/// The 08-15's rule handlers by their offset in segment `16EBh`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 16EB:000D (0x16EBD), Betrachte or Benutze on object A2h: scene 4
        // until the Geld has been used on it, scene 5 after.
        0x000D => {
            let n = if api.byte(BYTE_2BDA) == 0 { 4 } else { 5 }; // 0x16EC0
            api.scene_play(n); // 0x16ECD
            0x0A // 0x16ED3
        }
        // 16EB:0028 (0x16ED8), the Geld on object A2h: [2BDAh] set, scene 6.
        0x0028 => {
            api.set_byte(BYTE_2BDA, 1); // 0x16EDB
            api.scene_play(6); // 0x16EE2
            0x0A // 0x16EE8
        }
        // 16EB:003D (0x16EED), the Geld on nothing: one of scenes !0x24 to
        // !0x22.
        0x003D => {
            roll(api, !0x24, !0x22); // 0x16EF4, 0x16EFD
            0x0A // 0x16F03
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
