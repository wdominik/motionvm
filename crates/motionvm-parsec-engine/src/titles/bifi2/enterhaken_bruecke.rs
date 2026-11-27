//! Location 10, the Enterhaken - Brücke, a row of kind 1 with two exits:
//! `6Eh` to the Flur 3. Stock (location 32) and `6Fh` to the Enterhaken -
//! Maschinenraum (location 11, the row's `+9h` and `+19h`). The module is
//! named after the location as the game itself names it in its location
//! table (`Enterhaken - Brücke`, the record's `+37h`, `250Ah:00DFh`).
//!
//! Re-expressed from the handler at `1340h:0018h` (image `0x13418` to its
//! `retf` at `0x13529`), its hook `1340h:012Ah` (`0x1352A` to its `retf`
//! at `0x13768`, the record's `+3Fh`) and its one rule handler
//! `1340h:0008h` (`0x13408`–`0x13417`), against [`Api`] alone. The row's
//! `+43h` is null, so the animation hook is the library's empty one. The
//! location's rules are the one record at `2446h:04A0h` (image `0x24900`,
//! the record's `+3Bh`), fourteen bytes:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 0 | `80h` | 0 | — | `0008h` |
//!
//! Phase `80h` with verb 0 and object 0 is matched by the search a walk
//! stopped short of its target runs, for any verb and any object (see
//! [`super::sfx`]); its answer `0Ah` resets the mode block after it. Here
//! it plays place `0Ch` (`push 0FFF3h`, `!0x0C`).
//!
//! Four objects stand on the bridge, `F4h`, `F0h`, `F2h` and `F6h`; their
//! records are kept, in that order, as four far pointers from `DS:2E60h`.
//! While the Tontechnik's state `[2BC0h]` (location 20, see
//! [`super::tontechnik`]) is below 4 the entry writes subject slots 1 and 3
//! as they are (points `C8h`, `46h` and `118h`, `46h`), blocks nodes `3E8h` and `3E9h` of the player's graph and starts
//! scenes 0 to 4 rolled on task 1, repeated (`080Ch:18B5h`, Dunkle
//! Schatten's `0CFDh:19A4h`, [`Api::ambient_play`]); from 4 on all four are
//! hidden and nothing of that happens.
//!
//! The hook reacts to every task but task 0 (`cmp word [bp+0Ah], 0`,
//! `0x13531`) and moves the four figures with the scene: code 0 puts all
//! four on animation 0 and the hook's mode `[18A8h]` at 1; code 8 names an
//! animation in `[2E5Ah]`; code 9 hands it on by the mode — modes 1 and 4
//! to all four at once, mode 2 to figures 2 to 0 one step behind unless a
//! figure is held, mode 3 animation 5 to the figure the named number
//! counts from 1; code 7 carries the script's events `0Ah` to `11h`, which
//! switch the mode and hold or let go figures 1 and 2. `[2E5Ch]` is the
//! animation last handed on.
//!
//! The location writes nothing of the logic's saved state block
//! (`DS:2B98h`, `4Eh` bytes) and reads `[2BC0h]` alone. Its own words,
//! read and written in segment `1340h` alone and outside the `38h` bytes
//! from `DS:2D38h` the visit clears, keep their values from visit to
//! visit: `[18A8h]` (1 in the program's data), and `DS:2E42h` to
//! `DS:2E6Fh`.
//!
//! A node blocked is `05DCh:0EFDh`, Dunkle Schatten's `0A9Fh:1002h`
//! ([`Api::node_block`]).

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rule's far pointer.
pub(super) const SEGMENT: u16 = 0x1340;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x13512`).
const OBJECT_STATES: u16 = 0x2018;

/// The four objects on the bridge, in the order of their kept records
/// (`0x13430`, `0x13440`, `0x13450`, `0x13460`).
const FIGURES: [u16; 4] = [0xF4, 0xF0, 0xF2, 0xF6];

/// The player's graph nodes the entry blocks (`0x134BF`, `0x134CE`).
const NODE_3E8: u16 = 0x03E8;
const NODE_3E9: u16 = 0x03E9;

/// `[2BC0h]`: the Tontechnik's state, inside the saved block at
/// `DS:2B98h`; from 4 on the bridge is empty (`0x13470`).
const TONTECHNIK_STATE: u16 = 0x2BC0;

/// The hook's mode, 0 to 4 (`[18A8h]`).
const MODE: u16 = 0x18A8;
/// `DS:2E42h`: four words, a figure held on an animation of its own.
const HELD: u16 = 0x2E42;
/// `DS:2E4Ah`: four words, the animation a figure takes next.
const PENDING: u16 = 0x2E4A;
/// `DS:2E52h`: four words, the animation a figure was handed.
const PLAYING: u16 = 0x2E52;
/// `[2E5Ah]`: the animation code 8 names.
const NAMED: u16 = 0x2E5A;
/// `[2E5Ch]`: the animation last handed on.
const FOLLOWED: u16 = 0x2E5C;
/// `DS:2E60h`: four far pointers, the figures' records.
const RECORDS: u16 = 0x2E60;

/// Figure `i`'s word in one of the word rows.
fn word_of(row: u16, i: u16) -> u16 {
    row.wrapping_add(i << 1)
}

/// Figure `i`'s kept record, at `DS:2E60h + 4i`.
fn record_of(i: u16) -> u16 {
    RECORDS.wrapping_add(i << 2)
}

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

/// `push k ; push word [n+2] ; push word [n] ; call far 03F7h:1CF9h`: the
/// record kept at `n` on animation `k`.
fn anim_at(api: &mut dyn Api, at: u16, k: u16) {
    let r = rec(api, at);
    api.obj_anim(r, num::signed(k));
}

/// Everything before the frame loop, `0x13418` to `0x134F0`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x13418
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x13424
    api.visit_enter(loc, entry); // 0x13428
    for (i, id) in (0u16..).zip(FIGURES) {
        let r = api.obj_find(id); // 0x13433, 0x13443, 0x13453, 0x13463
        keep(api, record_of(i), r); // 0x13439, 0x1343D and after
    }
    if api.byte(TONTECHNIK_STATE) >= 4 {
        // 0x13470, 0x13475
        for id in FIGURES {
            api.obj_hide(id); // 0x1347A, 0x13483, 0x1348C, 0x13495
        }
    } else {
        api.slot_set_raw(1, 0, 0xC8, 0x46); // 0x134A6
        api.slot_set_raw(3, 0, 0x118, 0x46); // 0x134B7
        let player = api.ds(WALKER);
        api.node_block(player, NODE_3E8); // 0x134C6
        api.node_block(player, NODE_3E9); // 0x134D5
        api.ambient_play(0, 4, 1); // 0x134E3, the answer not read
    }
    api.visit_finish_entry(); // 0x134EB
    api.set_word(WORDS.entry, 0); // 0x134F0
}

/// One turn of the frame loop, `0x134F6` to `0x1350A`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x134F6
    api.dispatch(); // 0x134FB
    api.frame_pass(); // 0x13500
    api.word(WORDS.entry) == 0 // 0x13505
}

/// Everything after the frame loop, `0x1350C` to `0x13529`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x13517
    api.visit_leave(); // 0x1351F
    api.slot_unload(); // 0x13524
}

/// The hook, `1340h:012Ah`: code in `[bp+6]`, argument in `[bp+8]`, task
/// in `[bp+0Ah]`. Task 0 does nothing (`0x13531`); codes above 9 and codes
/// 1 to 6 do nothing (`0x1353D`, the table at `cs:0381h`, `0x13781`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, task: u16) {
    if task == 0 {
        return; // 0x13537
    }
    match code {
        // 0x1354C: mode 1, nothing handed on, and figures 3 to 0 let go,
        // handed 0 and on animation 0.
        0 => {
            api.set_word(MODE, 1); // 0x1354C
            api.set_word(FOLLOWED, 0); // 0x13552
            for i in (0..4u16).rev() {
                api.set_word(word_of(HELD, i), 0); // 0x13561
                api.set_word(word_of(PLAYING, i), 0); // 0x1356B
                anim_at(api, record_of(i), 0); // 0x13580
            }
        }
        7 => events(api, arg),         // 0x136CB
        8 => api.set_word(NAMED, arg), // 0x13592
        9 => hand_on(api),             // 0x13599
        _ => {}
    }
}

/// Code 9, `0x13599`: by the mode less 1 through the table at `cs:0379h`
/// (`0x13779`); mode 0 and modes above 4 nothing (`0x1359E`).
fn hand_on(api: &mut dyn Api) {
    match api.word(MODE) {
        // 0x135AD: a new named animation handed to figures 3 to 0.
        1 | 4 => {
            if api.word(FOLLOWED) == api.word(NAMED) {
                return; // 0x135B4
            }
            for i in (0..4u16).rev() {
                let named = api.word(NAMED);
                api.set_word(word_of(PLAYING, i), named); // 0x135C5
                anim_at(api, record_of(i), named); // 0x135DE
            }
            let named = api.word(NAMED);
            api.set_word(FOLLOWED, named); // 0x136C2
        }
        // 0x135F0: a new named animation becomes the next of figures 2 to
        // 0 that wait for none; then each figure 2 to 0 with one waiting,
        // unless held, plays it, and waits for the named one when that is
        // another, else for none.
        2 => {
            let named = api.word(NAMED);
            if api.word(FOLLOWED) != named {
                // 0x135F7
                for i in (0..3u16).rev() {
                    if api.word(word_of(PENDING, i)) == 0 {
                        api.set_word(word_of(PENDING, i), named); // 0x13610
                    }
                }
                api.set_word(FOLLOWED, named); // 0x1361E
            }
            for i in (0..3u16).rev() {
                let pending = api.word(word_of(PENDING, i));
                if pending == 0 {
                    continue; // 0x1362F
                }
                if api.word(word_of(HELD, i)) == 0 {
                    // 0x1363A
                    api.set_word(word_of(PLAYING, i), pending); // 0x13648
                    anim_at(api, record_of(i), pending); // 0x1365A
                }
                let named = api.word(NAMED);
                if api.word(word_of(PENDING, i)) != named {
                    api.set_word(word_of(PENDING, i), named); // 0x13677
                } else {
                    api.set_word(word_of(PENDING, i), 0); // 0x13681
                }
            }
        }
        // 0x13691: a new named animation n puts the word at `2E50h + 2n`
        // at 5 and the record kept at `2E5Ch + 4n` on animation 5 — figure
        // n − 1 for n from 1 to 4.
        3 => {
            let named = api.word(NAMED);
            if named == api.word(FOLLOWED) {
                return; // 0x13698
            }
            api.set_word(0x2E50u16.wrapping_add(named << 1), 5); // 0x136A6
            anim_at(api, 0x2E5Cu16.wrapping_add(named << 2), 5); // 0x136BA
            let named = api.word(NAMED);
            api.set_word(FOLLOWED, named); // 0x136C2
        }
        _ => {}
    }
}

/// Code 7, `0x136CB`: nothing in mode 0 (`0x136D0`); else the events `0Ah`
/// to `11h` through the table at `cs:0369h` (`0x13769`).
fn events(api: &mut dyn Api, arg: u16) {
    if api.word(MODE) == 0 {
        return; // 0x136D2
    }
    match arg {
        // 0x136E9: mode 2, figure 3 handed animation 5.
        0x0A => {
            api.set_word(MODE, 2); // 0x136E9
            api.set_word(word_of(PLAYING, 3), 5); // 0x136F2
            anim_at(api, record_of(3), 5); // 0x13750
        }
        // 0x13700: nothing handed on, mode 3.
        0x0B => {
            api.set_word(FOLLOWED, 0); // 0x13700
            api.set_word(MODE, 3); // 0x13706
        }
        0x0C => api.set_word(MODE, 4), // 0x1370E
        // 0x13716, 0x13720: figure 2 held, on animation 7 or 6.
        0x0D | 0x0E => {
            api.set_word(word_of(HELD, 2), 1); // 0x13716, 0x13720
            let k = if arg == 0x0D { 7 } else { 6 }; // 0x1371C, 0x13726
            anim_at(api, record_of(2), k); // 0x13750
        }
        // 0x13732: figure 2 let go, waiting for the named animation.
        0x0F => {
            api.set_word(word_of(HELD, 2), 0); // 0x13732
            let named = api.word(NAMED);
            api.set_word(word_of(PENDING, 2), named); // 0x1373B
        }
        // 0x13740: figure 1 held, on animation 6.
        0x10 => {
            api.set_word(word_of(HELD, 1), 1); // 0x13740
            anim_at(api, record_of(1), 6); // 0x13750
        }
        // 0x1375A: figure 1 let go, waiting for the named animation.
        0x11 => {
            api.set_word(word_of(HELD, 1), 0); // 0x1375A
            let named = api.word(NAMED);
            api.set_word(word_of(PENDING, 1), named); // 0x13763
        }
        _ => {}
    }
}

/// The Enterhaken - Brücke's rule handler by its offset in segment `1340h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 1340:0008 (0x13408), a walk stopped short: place 0Ch.
        0x0008 => {
            api.scene_play(!0x0C); // 0x1340B, 0x1340D
            0x0A // 0x13413
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
