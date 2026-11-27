//! Location 12, the Bi-Fi, a row of kind 1 with one exit, `6Eh`, to the
//! Flur 5. Stock (location 34). The module is named after the location as
//! the game itself names it in its location table (`Bi-Fi`, the record's
//! `+37h`, `250Ah:010Eh`).
//!
//! Re-expressed from the handler at `13CDh:0009h` (image `0x13CD9` to its
//! `retf` at `0x13DCE`), its animation hook `13CDh:00FFh` (`0x13DCF` to
//! `0x13E35`, the record's `+43h`) and its hook `13CDh:0172h` (`0x13E42`
//! to `0x13F89`, the record's `+3Fh`), against [`Api`] alone. The record's
//! rule table `+3Bh` is null: the location has no rules of its own, and
//! the segment installs no gate (no write to `[2D66h]`/`[2D68h]` between
//! `0x13CD9` and the hook's table's end at `0x13FA7`, just before location
//! 13's handler `13FAh:0008h` at `0x13FA8`).
//!
//! Three objects have a part, `8Dh`, `F0h` and `8Eh` (also the row's bytes
//! `+29h` on), their records kept at `DS:2EB8h`, `DS:2EB4h` and
//! `DS:2EB0h`, a far pointer each, and object `12h`, the id of item `12h`
//! (`Perücke`, the item row at `DS:10AAh`), which the animation hook
//! shows. Two bytes of the logic's saved state block (`DS:2B98h`, `4Eh`
//! bytes) bring them in. `[2BA8h]` at 1 (set by the VIP-Lounge, location
//! 25, `0x163D6`) shows object `8Dh` on animation 0 and opens with the
//! location's own interaction with node `0FA0h` (facing 2, scene 4); the
//! byte becomes 2, which the Maske (location 22) compares with
//! (`0x1596D`). Any other value hides object `8Dh`. `[2BD0h]` at 1 (set
//! by Schlönstadt's Gefängnis, location 41, `0x177FA`) shows objects `F0h`
//! and `8Eh` and plays scene 5. Each of the two takes the input over
//! (`09B6h:17D4h`); the segment gives it back nowhere.
//!
//! The scene's hook drives objects `8Eh` and `F0h`. Code 0, a scene
//! claiming its speakers (the argument their bits), forgets both
//! animations and sets the base `[2EAEh]`: `FFFFh` for bit 1, else 0 for
//! bit 2, else `FFFEh`. Code 8 keeps its argument, at least 0, as the
//! animation wanted while the base is not negative; code 9, at the page's
//! end, gives a new animation wanted to object `8Eh` and the base plus it
//! to object `F0h`. Code 7's events: `0Ah` sets object `8Dh` on animation
//! 1; `0Bh` and `0Ch` hold the task until the frame loop sees the player
//! on node `0FA0h` or `0FA1h`; `0Dh` makes the base and the animation
//! wanted 4, plays samples `0Ch` and `0Ah` and takes item `27h`
//! (`Drehbuch`); `0Eh` sends the player to node `0FA1h` facing 0. At the
//! scene's end (codes 5 and 6) a base of `FFFEh` stays, `FFFFh` leads on
//! to the Kaffee (location 1) with entry 5, any other with entry 6; the
//! Kaffee's row at `DS:182Ch` names location 12 for entry 5 and `FFFFh`,
//! the game's end, for entry 6.
//!
//! The location's own interaction is `09B6h:16C4h(0FA0h, 2, 4)`
//! (`0x13D26`, [`Api::interaction`]): an id of `100h` and above skips the
//! choice of an object (`cmp si, 100h ; jl`, `0x0B22F`, to `0x0B30F`), so
//! the player walks to node `0FA0h` facing 2 (`05DCh:05C8h`, `0x0B317`)
//! and scene 4 plays.

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x13DB7`).
const OBJECT_STATES: u16 = 0x2018;

/// The objects the logic names.
const OBJECT_8D: u16 = 0x8D;
const OBJECT_8E: u16 = 0x8E;
const OBJECT_F0: u16 = 0xF0;
/// Object `12h`, the id of item `12h` (`Perücke`).
const OBJECT_12: u16 = 0x12;
/// `[2EB8h]`/`[2EBAh]`: object `8Dh`'s record (`0x13D10`, `0x13D0C`).
const REC_8D: u16 = 0x2EB8;
/// `[2EB4h]`/`[2EB6h]`: object `F0h`'s record (`0x13D57`, `0x13D53`).
const REC_F0: u16 = 0x2EB4;
/// `[2EB0h]`/`[2EB2h]`: object `8Eh`'s record (`0x13D67`, `0x13D63`).
const REC_8E: u16 = 0x2EB0;

/// The Drehbuch, item `27h` (`0x13F80`).
const DREHBUCH: u16 = 0x27;

/// The node of the location's own interaction (`push 0FA0h`, `0x13D23`),
/// also awaited by event `0Bh`; event `0Ch` awaits node `0FA1h`, where
/// event `0Eh` sends the player.
const NODE_INTERACTION: u16 = 0x0FA0;
const NODE_0FA1: u16 = 0x0FA1;
/// `[1796h]`: the player record's `+28h`, its node (`0x13D91`).
const PLAYER_NODE: u16 = WALKER + 0x28;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BA8h]`: 1 when the VIP-Lounge has been visited, 2 after this
/// location's interaction.
const BYTE_2BA8: u16 = 0x2BA8;
/// `[2BD0h]`: 1 brings objects `F0h` and `8Eh` in.
const BYTE_2BD0: u16 = 0x2BD0;

/// The Kaffee, location 1, and the entries the scene's end sends it with
/// (`0x13E9F`, `0x13EAE`).
const KAFFEE: u16 = 1;
const KAFFEE_ENTRY_ON: u16 = 5;
const KAFFEE_ENTRY_END: u16 = 6;

/// The handler's and the hook's words, read and written in segment
/// `13CDh` alone.
/// `[2EAAh]`: the animation wanted.
const WANTED: u16 = 0x2EAA;
/// `[2EACh]`: the animation last given.
const GIVEN: u16 = 0x2EAC;
/// `[2EAEh]`: the base, added to the animation for object `F0h`
/// (`0x13EF7`); `FFFEh` leaves the scene's end without an exit.
const BASE: u16 = 0x2EAE;
/// `[2EBCh]`: the node the frame loop waits for the player on, 0 none.
const AWAITED: u16 = 0x2EBC;

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

/// `push k ; push word [at+2] ; push word [at] ; call far 03F7h:1CF9h`.
fn anim(api: &mut dyn Api, at: u16, k: i16) {
    let r = rec(api, at);
    api.obj_anim(r, k);
}

/// Everything before the frame loop, `0x13CD9` to `0x13D77`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x13CD9
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x13CE5
    api.visit_enter(loc, entry); // 0x13CE9
    api.set_word(AWAITED, 0); // 0x13CF1
    if api.byte(BYTE_2BA8) == 1 {
        // 0x13CF7
        api.input_lock(); // 0x13CFE
        let r = api.obj_show(OBJECT_8D); // 0x13D06
        keep(api, REC_8D, r); // 0x13D0C, 0x13D10
        api.obj_anim(r, 0); // 0x13D17
        api.interaction(NODE_INTERACTION, 2, 4); // 0x13D26, 09B6h:16C4h
        api.set_byte(BYTE_2BA8, 2); // 0x13D2E
    } else {
        api.obj_hide(OBJECT_8D); // 0x13D38
    }
    if api.byte(BYTE_2BD0) == 1 {
        // 0x13D3E
        api.input_lock(); // 0x13D45
        let r = api.obj_show(OBJECT_F0); // 0x13D4D
        keep(api, REC_F0, r); // 0x13D53, 0x13D57
        let r = api.obj_show(OBJECT_8E); // 0x13D5D
        keep(api, REC_8E, r); // 0x13D63, 0x13D67
        api.scene_play(5); // 0x13D6C
    }
    api.visit_finish_entry(); // 0x13D72
    api.set_word(WORDS.entry, 0); // 0x13D77
}

/// One turn of the frame loop, `0x13D7D` to `0x13DAF`: once the player
/// stands on the awaited node, task 0 starts and nothing is awaited.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x13D7D
    api.dispatch(); // 0x13D82
    let awaited = api.word(AWAITED);
    if awaited != 0 && awaited == api.word(PLAYER_NODE) {
        // 0x13D87, 0x13D91
        api.task_start(0); // 0x13D99
        api.set_word(AWAITED, 0); // 0x13D9F
    }
    api.frame_pass(); // 0x13DA5
    api.word(WORDS.entry) == 0 // 0x13DAA
}

/// Everything after the frame loop, `0x13DB1` to `0x13DCE`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x13DBC
    api.visit_leave(); // 0x13DC4
    api.slot_unload(); // 0x13DC9
}

/// The animation hook, `13CDh:00FFh`: the code in `[bp+0Ah]`; codes `64h`
/// and `65h` have arms (`0x13DD2`–`0x13DDF`).
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        // 0x13DE1: sample 0Eh, then one roll of 0 to 5 through the table
        // at cs:0166h (0x13E36): 0 and 1 sample 13h, 2 and 3 sample 14h,
        // 4 sample 13h twice, 5 sample 14h twice.
        0x64 => {
            api.play_sample(0x0E); // 0x13DE3
            match api.random(0, 5) {
                // 0x13DED
                0 | 1 => api.play_sample(0x13), // 0x13E03, 0x13E05
                2 | 3 => api.play_sample(0x14), // 0x13E0C, 0x13E20, 0x13E05
                4 => {
                    api.play_sample(0x13); // 0x13E10
                    api.play_sample(0x13); // 0x13E03, 0x13E05
                }
                5 => {
                    api.play_sample(0x14); // 0x13E1A
                    api.play_sample(0x14); // 0x13E20, 0x13E05
                }
                _ => {} // 0x13DFA
            }
        }
        // 0x13E24: sample 0Ah, object 12h shown.
        0x65 => {
            api.play_sample(0x0A); // 0x13E26
            api.obj_show(OBJECT_12); // 0x13E2E
        }
        _ => {}
    }
}

/// The hook, `13CDh:0172h`: code in `[bp+6]`, argument in `[bp+8]`, task
/// in `[bp+0Ah]`. Codes above 9 and codes 1 to 4 do nothing (`0x13E4E`,
/// the table at `cs:02C4h`, `0x13F94`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, task: u16) {
    match code {
        // 0x13E5D: a scene claims its speakers; both animations forgotten,
        // the base by bit 1 of the argument before bit 2.
        0 => {
            api.set_word(WANTED, 0xFFFF); // 0x13E60
            api.set_word(GIVEN, 0xFFFF); // 0x13E63
            let base = if arg & 2 != 0 {
                0xFFFF // 0x13E6C
            } else if arg & 4 != 0 {
                0 // 0x13E7B
            } else {
                0xFFFE // 0x13E84
            };
            api.set_word(BASE, base);
        }
        // 0x13E8D: the scene's end.
        5 | 6 => match api.word(BASE) {
            0xFFFE => {} // 0x13E93
            0xFFFF => {
                api.set_word(WORDS.entry, KAFFEE_ENTRY_ON); // 0x13E9F
                api.set_word(WORDS.location, KAFFEE); // 0x13EA5
            }
            _ => {
                api.set_word(WORDS.entry, KAFFEE_ENTRY_END); // 0x13EAE
                api.set_word(WORDS.location, KAFFEE); // 0x13EB4
            }
        },
        7 => events(api, arg, task), // 0x13F06
        // 0x13EBD: while the base is not negative (`cmp word [2EAEh], -1
        // ; jg`), the argument, a negative one as 0, is the animation
        // wanted.
        8 => {
            if num::signed(api.word(BASE)) > -1 {
                let k = if num::signed(arg) < 0 { 0 } else { arg }; // 0x13EC7
                api.set_word(WANTED, k); // 0x13ECD
            }
        }
        // 0x13ED4: a new animation wanted goes to object 8Eh, and the base
        // plus it to object F0h.
        9 => {
            let k = api.word(WANTED);
            if k == api.word(GIVEN) {
                return; // 0x13EDB
            }
            api.set_word(GIVEN, k); // 0x13EE0
            anim(api, REC_8E, num::signed(k)); // 0x13EEC
            let k = api.word(BASE).wrapping_add(api.word(GIVEN)); // 0x13EF4, 0x13EF7
            anim(api, REC_F0, num::signed(k)); // 0x13F21, by 0x13F04
        }
        _ => {}
    }
}

/// Code 7, `0x13F06`: the argument less `0Ah`, 0 to 4, through the table
/// at `cs:02BAh` (`0x13F8A`).
fn events(api: &mut dyn Api, arg: u16, task: u16) {
    match arg {
        // 0x13F17.
        0x0A => anim(api, REC_8D, 1), // 0x13F21
        // 0x13F2B: unless the player stands on node 0FA0h, the node awaited
        // and the task armed.
        0x0B => await_node(api, NODE_INTERACTION, task),
        // 0x13F54: the same for node 0FA1h.
        0x0C => await_node(api, NODE_0FA1, task),
        // 0x13F64: the base and the animation wanted 4, samples 0Ch and
        // 0Ah, the Drehbuch taken.
        0x0D => {
            api.set_word(BASE, 4); // 0x13F64
            api.set_word(WANTED, 4); // 0x13F6A
            api.play_sample(0x0C); // 0x13F72
            api.play_sample(0x0A); // 0x13F7A
            api.inv_take(DREHBUCH); // 0x13F82
        }
        // 0x13F41: the player walks to node 0FA1h facing 0; answer not read.
        0x0E => {
            let player = api.ds(WALKER);
            api.walker_walk_to(player, NODE_0FA1, 0); // 0x13F4A
        }
        _ => {} // 0x13F0E
    }
}

/// `0x13F2B`/`0x13F54` to `0x13F3F`: `cmp word [1796h], node ; jz`, else
/// `mov word [2EBCh], node` and the task armed (`080Ch:193Fh`, `0x13F3A`).
fn await_node(api: &mut dyn Api, node: u16, task: u16) {
    if api.word(PLAYER_NODE) == node {
        return;
    }
    api.set_word(AWAITED, node); // 0x13F33, 0x13F5C
    api.task_arm(task); // 0x13F3A
}

/// The location has no rule table (the record's `+3Bh` is null).
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
