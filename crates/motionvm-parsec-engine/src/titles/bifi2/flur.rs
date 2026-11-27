//! Locations 18, 19 and 30 to 35, the Flure: the corridors of the
//! building, one row a floor, all of kind 2 and all naming the one handler
//! `14F9h:00EBh`. The module is named after the locations as the game
//! itself names them in its location table (`Flur Erdgeschoß`, `Flur 1.
//! Stock` … `Flur 7. Stock`, the records' `+37h`).
//!
//! Re-expressed from the handler at `14F9h:00EBh` (image `0x1507B` to its
//! `retf` at `0x15143`), the gate `14F9h:01B4h` (`0x15144`, installed by
//! the animation hook at `0x151AC`), the animation hook `14F9h:01E8h`
//! (`0x15178` to its `retf` at `0x15238`, every row's `+43h`) and the seven
//! rule handlers `14F9h:0003h` to `00D0h` (`0x14F93`–`0x1507A`), against
//! [`Api`] alone. No row has a hook of its own (`+3Fh` is null in all
//! eight), so the entry installs the library's empty one, `09B6h:1BD2h`
//! (`0x09E51`–`0x09E57`, a lone `retf` at `0x0B732`).
//!
//! A corridor's floor is its row's second exit, `64h` plus the floor
//! (`mov al, es:[bx+0Ah] ; add ax, 0FF9Ch`, `0x150B9`–`0x150C0`): the
//! door of the Lift, location 36, whose destination every row gives as
//! `24h`. Two words carry the lift between the corridors and the Lift:
//! `[2F8Ch]`, the floor it goes to, and `[2F8Eh]`, the floor it is on. The
//! Lift's own handler (segment `1715h`) sets `[2F8Eh]` from the exit it was
//! entered by (`0x171F9`) and `[2F8Ch]` from the exit it is left by
//! (`0x172AB`); a corridor entered from anywhere else sets both to its own
//! floor.
//!
//! Entered from the Lift (`[200Eh]` still `24h`, `0x15080`, read before
//! the handler writes its own number there at `0x1510D`), the entry is
//! handed on `0BB8h` higher (`0x1508A`), the input is held and the lift
//! runs through `[2F0Eh]`:
//!
//! - 3: object `A4h` steps one animation towards `[2F8Ch]` each time its
//!   animation reports, with sample `16h` (`0x151D0`–`0x15218`); when it
//!   is there, the lift door `64h + [2F8Eh]` takes animation 1
//!   (`0x1521A`–`0x15234`) and the state is 2;
//! - 2: on an animation code 2 the figure walks to the door's node, and
//!   the mode machine waits behind this segment's gate (`0x1518B`–
//!   `0x151CE`); the state is 1;
//! - 1: the gate, asked with 0, closes the door (animation 0), gives the
//!   input back and lets go; the state is 0.
//!
//! The rows differ only in their exits and names; the code reads nothing
//! of a row but its second exit. Only row 18, the ground floor, has rules
//! of its own (`+3Bh`, `2446h:0660h`, image `0x24AC0`): door `78h` to the
//! VIP-Lounge (location 25), which item 3 (`VIP-Karte`) opens. Fourteen
//! bytes a rule as in the global table; a class is stored complemented
//! (`FFFBh` class 4, `FFFCh` class 3):
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 6 | 4 | `78h` | — | `0003h` |
//! | 5 | `20h` | 3 | `78h` | `0035h` |
//! | 5 | `20h` | 1 | `78h` | `0095h` |
//! | 5 | `20h` | class 4 | `78h` | `007Ah` |
//! | 5 | `20h` | class 3 | `78h` | `00A5h` |
//! | 5 | `20h` | `0Dh` | `78h` | `00B5h` |
//! | 5 | `20h` | `0Dh`, class 3, class 4 | 0 | `00D0h` |
//!
//! Items by the row at `DS:10AAh`: 1 `Infrarotclip`, 3 `VIP-Karte`, `0Dh`
//! `Haarnadel`. Scenes are pushed as the program pushes them; `push 0FFDBh`
//! is `!0x24`.
//!
//! An object shown on a frame is `0F48h:096Eh(id, k)` (`0xFDEE`), Dunkle
//! Schatten's `14DDh:0A06h` instruction for instruction
//! ([`Api::obj_show_frame`]).

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers and the gate.
pub(super) const SEGMENT: u16 = 0x14F9;
/// The gate's offset (`mov word [2D66h], 1B4h`, `0x151B2`).
const GATE: u16 = 0x01B4;

/// `2181h`, the location table, `44h` bytes a row (`0x150B1`–`0x150B7`).
const LOCATIONS: u16 = 0x2181;
const STRIDE: u16 = 0x44;
/// The row's second exit byte (`es:[bx+0Ah]`, `0x150B9`).
const SECOND_EXIT: u16 = 0x0A;
/// `add ax, 0FF9Ch` (`0x150C0`): exit `64h` is floor 0.
const FIRST_LIFT_DOOR: u16 = 0x64;

/// The Lift, location 36 (`cmp word [200Eh], 24h`, `0x15080`).
const LIFT: u16 = 0x24;
/// `add ax, 0BB8h` (`0x1508A`): the entry handed on after the Lift.
const FROM_LIFT: u16 = 0x0BB8;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x1512C`).
const OBJECT_STATES: u16 = 0x2018;

/// `[2F0Eh]`: the lift's state, 3 to 0 (`0x1509A`, `0x150A7`).
const STATE: u16 = 0x2F0E;
/// `[2F8Ch]`: the floor the lift goes to (`0x150C3`).
const TARGET: u16 = 0x2F8C;
/// `[2F8Eh]`: the floor the lift is on (`0x150C6`).
const CURRENT: u16 = 0x2F8E;

/// Object `A4h`, on the animation of the floor the lift is on
/// (`0x150EC`–`0x150F7`) and stepped by the animation hook.
const OBJECT_A4: u16 = 0xA4;
/// Object `A5h`, shown on the frame of the floor the lift goes to
/// (`0x150D9`–`0x150E0`).
const OBJECT_A5: u16 = 0xA5;
/// Door `78h`, row 18's rules' object.
const OBJECT_78: u16 = 0x78;
/// Sample `16h`, a step of the lift (`0x15210`).
const SAMPLE_STEP: u16 = 0x16;

/// The story byte rule `00B5h` reads, inside the saved block `DS:2B98h`.
const BYTE_2BBF: u16 = 0x2BBF;

/// The mode block: the mode `[2D3Ah]`, the wait bits `[2D46h]`, the walk
/// target `[2D4Ah]`, the gate `[2D66h]`/`[2D68h]`, the animation code a
/// door waits for `[2D6Ch]`, and the entry door `[1F3Ah]`.
const MODE: u16 = 0x2D3A;
const BITS: u16 = 0x2D46;
const WALK_TARGET: u16 = 0x2D4A;
const GATE_OFF: u16 = 0x2D66;
const GATE_SEG: u16 = 0x2D68;
const ANIM_WAIT: u16 = 0x2D6C;
const ENTRY_DOOR: u16 = 0x1F3A;
/// `[2D6Eh]`: the status line's refresh age; 1 asks for the line to be
/// composed again a few frames on (the frame pass's `0x0B0EF`–`0x0B10A`,
/// cleared by the composer at `0x0B0CD`). Dunkle Schatten's `[28C3h]`.
const STATUS_AGE: u16 = 0x2D6E;

/// The lift door of `floor`: `[2F8Eh] + 64h` (`0x15153`–`0x15156`,
/// `0x15220`–`0x15223`).
fn door_of(floor: u16) -> u16 {
    floor.wrapping_add(FIRST_LIFT_DOOR)
}

/// `push k ; push id ; call far 06E3h:047Eh ; pop cx ; push dx ; push ax ;
/// call far 03F7h:1CF9h`: the object found and set on animation `k`, the
/// record not tested for null.
fn anim(api: &mut dyn Api, id: u16, k: u16) {
    let rec = api.obj_find(id);
    api.obj_anim(rec, num::signed(k));
}

/// Everything before the frame loop, `0x1507B` to `0x1510D`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x1507B
    if api.word(WORDS.here) == LIFT {
        // 0x15080
        let entry = api.word(WORDS.entry).wrapping_add(FROM_LIFT); // 0x1508A
        let loc = api.word(WORDS.location);
        api.visit_enter(loc, entry); // 0x15092
        api.set_word(STATE, 3); // 0x1509A
        api.input_lock(); // 0x150A0
    } else {
        api.set_word(STATE, 0); // 0x150A7
        let loc = api.word(WORDS.location);
        let exit = api.far_byte(Far {
            seg: LOCATIONS,
            off: loc.wrapping_mul(STRIDE).wrapping_add(SECOND_EXIT),
        }); // 0x150B9
        let floor = u16::from(exit).wrapping_sub(FIRST_LIFT_DOOR); // 0x150C0
        api.set_word(TARGET, floor); // 0x150C3
        api.set_word(CURRENT, floor); // 0x150C6
        let entry = api.word(WORDS.entry);
        api.visit_enter(loc, entry); // 0x150D1
    }
    let target = api.word(TARGET);
    api.obj_show_frame(OBJECT_A5, num::signed(target)); // 0x150E0, 0F48h:096Eh
    let current = api.word(CURRENT);
    anim(api, OBJECT_A4, current); // 0x150EF, 0x150F7
    api.visit_finish_entry(); // 0x150FF
    api.set_word(WORDS.entry, 0); // 0x15104
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x1510D
}

/// One turn of the frame loop, `0x15110` to `0x15124`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x15110
    api.dispatch(); // 0x15115
    api.frame_pass(); // 0x1511A
    api.word(WORDS.entry) == 0 // 0x1511F
}

/// Everything after the frame loop, `0x15126` to `0x15143`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x15131
    api.visit_leave(); // 0x15139
    api.slot_unload(); // 0x1513E
}

/// No row has a hook of its own: `+3Fh` is null, and the entry puts the
/// library's `09B6h:1BD2h` (`0x0B732`, `retf`) in its place (`0x09E51`).
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// The animation hook, `14F9h:01E8h`: the record in `[bp+6]`, the code in
/// `[bp+0Ah]`, by the lift's state (`0x1517B`–`0x15188`); in any state
/// other than 2 and 3 nothing.
pub(crate) fn anim_hook(api: &mut dyn Api, rec: Far, code: u16) {
    match api.word(STATE) {
        // 0x1518B: on code 2 alone, whatever the record.
        2 if code == 2 => {
            let node = door_of(api.word(TARGET)); // 0x15196–0x15199
            api.set_word(WALK_TARGET, node); // 0x1519C
            let player = api.ds(WALKER);
            api.walker_walk_to(player, node, -1); // 0x151A4, answer not read
            api.set_word(GATE_SEG, SEGMENT); // 0x151AC
            api.set_word(GATE_OFF, GATE); // 0x151B2
            api.set_word(MODE, 7); // 0x151B8
            let bits = api.word(BITS) & 0xFE0F; // 0x151BE
            api.set_word(BITS, bits | 0x80); // 0x151C4
            let s = api.word(STATE).wrapping_sub(1);
            api.set_word(STATE, s); // 0x151CA
        }
        // 0x151D0: only for object A4h's record (`cmp byte es:[bx+13h],
        // 0A4h`), whatever the code.
        3 => {
            let id = api.far_byte(Far {
                off: rec.off.wrapping_add(0x13),
                seg: rec.seg,
            }); // 0x151D3
            if u16::from(id) != OBJECT_A4 {
                return; // 0x151D8
            }
            let target = num::signed(api.word(TARGET));
            let current = num::signed(api.word(CURRENT));
            // 0x151E1 `jnl`, 0x151F3 `jng`: signed.
            let step = if target < current {
                Some(api.word(CURRENT).wrapping_sub(1)) // 0x151E3
            } else if target > current {
                Some(api.word(CURRENT).wrapping_add(1)) // 0x151F5
            } else {
                None
            };
            if let Some(now) = step {
                api.set_word(CURRENT, now);
                anim(api, OBJECT_A4, now); // 0x15200, 0x15208
                api.play_sample(SAMPLE_STEP); // 0x15212
            } else {
                // 0x1521A: there; the door opens.
                let s = api.word(STATE).wrapping_sub(1);
                api.set_word(STATE, s); // 0x1521A
                let door = door_of(api.word(CURRENT));
                anim(api, door, 1); // 0x15227, 0x1522F
            }
        }
        _ => {}
    }
}

/// The gate, `14F9h:01B4h` (`0x15144`): asked with anything but 0 it
/// answers 0 (`0x1514D`); asked with 0 the lift door of `[2F8Eh]` closes
/// (animation 0, `0x1515A`, `0x15162`), the state steps down
/// (`0x1516A`), the input is given back (`09B6h:17E2h`, `0x1516E`), and it
/// answers 1.
pub(crate) fn gate(api: &mut dyn Api, off: u16, arg: i16) -> Option<i16> {
    if off != GATE {
        return None;
    }
    if arg != 0 {
        return Some(0);
    }
    let door = door_of(api.word(CURRENT));
    anim(api, door, 0);
    let s = api.word(STATE).wrapping_sub(1);
    api.set_word(STATE, s);
    api.input_unlock();
    Some(1)
}

/// The gate cleared (`mov word [2D68h], 0 ; mov word [2D66h], 0`).
fn clear_gate(api: &mut dyn Api) {
    api.set_word(GATE_SEG, 0);
    api.set_word(GATE_OFF, 0);
}

/// `push hi ; push lo ; call far 203Dh:0024h ; push ax ; call far
/// 080Ch:164Ch`: the roll is the scene. Answer `0Ah`.
fn roll(api: &mut dyn Api, lo: u16, hi: u16) -> i16 {
    let n = api.random(lo, hi);
    api.scene_play(num::signed(n));
    0x0A
}

/// The corridors' rule handlers by their offset in segment `14F9h`; only
/// row 18's rule table names them.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 14F9:0003 (0x14F93), verb 6 on door 78h: with the VIP-Karte
        // carried the search goes on (0x14FA2); else the gate cleared, the
        // status line asked for, scene 13h.
        0x0003 => {
            if api.inv_has(3) != 0 {
                // 0x14F98
                0
            } else {
                clear_gate(api); // 0x14FA6, 0x14FAC
                api.set_word(STATUS_AGE, 1); // 0x14FB2
                api.scene_play(0x13); // 0x14FBA
                0x0A
            }
        }
        // 14F9:0035 (0x14FC5), the VIP-Karte on door 78h: a live door
        // answers 1 (0x14FD4); else the gate cleared, the door on
        // animation 1, the entry door, the wait for animation code 2, and
        // the answer 0Ch.
        0x0035 => {
            if api.obj_state(OBJECT_78) != 0 {
                // 0x14FCA
                1
            } else {
                clear_gate(api); // 0x14FD9, 0x14FDF
                anim(api, OBJECT_78, 1); // 0x14FE9, 0x14FF1
                api.set_word(ENTRY_DOOR, OBJECT_78); // 0x14FF9
                api.set_word(ANIM_WAIT, 2); // 0x14FFF
                0x0C
            }
        }
        // 14F9:007A (0x1500A), class 4 on door 78h: scene 14h or 15h.
        0x007A => roll(api, 0x14, 0x15), // 0x15011, 0x1501A
        // 14F9:0095 (0x15025), the Infrarotclip on door 78h: scene 16h.
        0x0095 => {
            api.scene_play(0x16); // 0x1502A
            0x0A
        }
        // 14F9:00A5 (0x15035), class 3 on door 78h: scene 17h.
        0x00A5 => {
            api.scene_play(0x17); // 0x1503A
            0x0A
        }
        // 14F9:00B5 (0x15045), the Haarnadel on door 78h: scene 18h while
        // [2BBFh] is 0, else 19h.
        0x00B5 => {
            let n = if api.byte(BYTE_2BBF) == 0 { 0x18 } else { 0x19 }; // 0x15048
            api.scene_play(n); // 0x15055
            0x0A
        }
        // 14F9:00D0 (0x15060), the Haarnadel, class 3 or class 4 on
        // nothing: one of scenes !0x24 to !0x22.
        0x00D0 => roll(api, !0x24, !0x22), // 0x15067, 0x15070
        _ => return None,
    })
}

/// The corridors' entry points, as [`super::logic`] calls them.
pub(super) const LOGIC: Logic = Logic {
    enter,
    frame,
    leave,
    hook,
    anim_hook: Some(anim_hook),
    rule,
};
