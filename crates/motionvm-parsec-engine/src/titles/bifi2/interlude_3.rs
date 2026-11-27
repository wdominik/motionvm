//! Location 3, a row of kind 7 with no name (the record's `+37h` is
//! `0000h:0000h`) and two exits: `71h` to location 69 and `7Fh` to
//! location 2, the two rows whose exits lead here (location 2's `7Fh`,
//! location 69's `71h`). The player does not act here: the input is taken
//! over, object `F0h` stands in for the figure on an animation, and the
//! animation hook's codes take the visit on.
//!
//! Re-expressed from the handler at `10A7h:000Ah` (image `0x10A7A` to its
//! `retf` at `0x10B75`) and its animation hook `10A7h:0106h` (`0x10B76`
//! to its `retf` at `0x10BC5`, the record's `+43h`; its table at
//! `cs:0156h`, `0x10BC6`), against [`Api`] alone. The record's hook `+3Fh`
//! and its rule table `+3Bh` are `0000h:0000h`: the location has no hook
//! and no rules of its own (the installer `0E40h:0031h`, handed the null
//! table at `0x09DB3`, keeps no count for it, `0x0E444`–`0x0E451`). The
//! segment holds nothing else of this location: `10A7h:0000h`–`0009h` is
//! the tail of location 2's animation hook and its table, and the code from
//! `0x10BCE` on is segment `10BCh`'s (the Pforte's rule `000Eh`). The
//! immediate `10A7h` occurs only in the location record (`0x21911`,
//! `0x21921`): the location has no gate.
//!
//! Which way the visit goes depends on where it came from. The handler
//! reads `[200Eh]` (`0x10A94`) before it writes it (`0x10B04`, after the
//! visit's entry is finished, as the corridors do at `0x1510D`), so it
//! reads the location the main loop ran last. From location 2 the figure is hidden at once and `F0h` plays
//! animation 1. From anywhere else the figure walks to node `7Fh`, and when
//! it stands there (`[1796h]`, its record's `+28h`, is the walk target
//! `[2D4Ah]`, `0x10B1B`) it is hidden and `F0h` plays animation 0. The
//! input is taken over for the whole visit (`0x10AF2`) and never given
//! back here.
//!
//! The animation hook's code `64h` sends the visit back to location 2 by
//! entry `7Fh` directly; code `65h` hides `F0h`, shows the figure again and
//! leaves through exit `71h` (location 69) by the script exit; codes `66h`
//! and `67h` play sample `2Eh`.

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::titles::parsec1994::{Api, Far};

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x10B5C`).
const OBJECT_STATES: u16 = 0x2018;

/// The object that stands in for the figure (`push 0F0h`, `0x10AA6`,
/// `0x10B99`).
const OBJECT_F0: u16 = 0xF0;

/// The player record's flag byte `+2` (`[1770h]`); bit 1 is its hidden
/// bit, which the chain's draw tests (`and ax, 3 ; cmp ax, 1`,
/// `0x07243`).
const PLAYER_FLAGS: u16 = WALKER + 2;
const HIDDEN: u8 = 0x02;
/// `[1796h]`: the player record's `+28h`, the node it stands on
/// (`0x10B1B`).
const PLAYER_NODE: u16 = WALKER + 0x28;

/// The mode block's walk target (`0x10AE2`, `0x10B1E`, `0x10B24`).
const WALK_TARGET: u16 = 0x2D4A;

/// The location the hidden figure stays for (`cmp word [200Eh], 2`,
/// `0x10A94`), and the one code `64h` returns to (`0x10B8B`).
const FROM: u16 = 2;
/// The node the figure walks to from anywhere else (`mov ax, 7Fh`,
/// `0x10ADF`), and the entry code `64h` returns by (`0x10B91`).
const NODE_7F: u16 = 0x7F;
/// The exit code `65h` leaves through (`push 71h`, `0x10BB3`).
const EXIT_71: u16 = 0x71;
/// The sample codes `66h` and `67h` play (`push 2Eh`, `0x10BBC`).
const SAMPLE_2E: u16 = 0x2E;

/// The handler's own stack frame: `si`, whether the visit came from
/// location 2 (`0x10AA2`), and `[bp-4]`/`[bp-2]`, the far pointer
/// `0F48h:096Eh` answered for `F0h` (`0x10AB1`, `0x10AB4`).
const CAME_FROM_2: usize = 0;
const REC_OFF: usize = 1;
const REC_SEG: usize = 2;

fn kept(api: &dyn Api) -> Far {
    Far {
        off: api.local(REC_OFF),
        seg: api.local(REC_SEG),
    }
}

/// `or byte [1770h], 2 ; push ds ; push 176Eh ; call far 03F7h:0001h ;
/// push si ; push word [bp-2] ; push word [bp-4] ; call far 03F7h:1CF9h`:
/// the figure hidden and `F0h` on animation `si`.
fn stand_in(api: &mut dyn Api) {
    let flags = api.byte(PLAYER_FLAGS) | HIDDEN;
    api.set_byte(PLAYER_FLAGS, flags); // 0x10ABB, 0x10B2A
    let player = api.ds(WALKER);
    api.obj_invalidate(player); // 0x10AC4, 0x10B33
    let k = api.local(CAME_FROM_2);
    let rec = kept(api);
    api.obj_anim(rec, crate::num::signed(k)); // 0x10AD3, 0x10B42
}

/// Everything before the frame loop, `0x10A7A` to `0x10B07`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x10A7F
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.visit_enter(loc, entry); // 0x10A8C
    // 0x10A94: `[200Eh]` is still the location the main loop ran last.
    let from_2 = u16::from(api.word(WORDS.here) == FROM);
    api.set_local(CAME_FROM_2, from_2); // 0x10AA2
    let rec = api.obj_show_frame(OBJECT_F0, -1); // 0x10AA9
    api.set_local(REC_SEG, rec.seg); // 0x10AB1
    api.set_local(REC_OFF, rec.off); // 0x10AB4
    if from_2 != 0 {
        stand_in(api); // 0x10ABB–0x10AD8
    } else {
        // 0x10ADD: the walk target set, the figure walks to node 7Fh
        // facing 0; the answer is not read.
        api.set_word(WALK_TARGET, NODE_7F); // 0x10AE2
        let player = api.ds(WALKER);
        api.walker_walk_to(player, NODE_7F, 0); // 0x10AEA
    }
    api.input_lock(); // 0x10AF2
    api.hover_off(); // 0x10AF7
    api.visit_finish_entry(); // 0x10AFC
    let loc = api.word(WORDS.location); // 0x10B01
    api.set_word(WORDS.here, loc); // 0x10B04
    api.set_word(WORDS.entry, 0); // 0x10B07
}

/// One turn of the frame loop, `0x10B0D` to `0x10B54`: after the
/// dispatch, a figure that walked to its target stands in for `F0h`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x10B0D
    api.dispatch(); // 0x10B12
    if api.local(CAME_FROM_2) == 0 && api.word(PLAYER_NODE) == api.word(WALK_TARGET) {
        // 0x10B17–0x10B22
        api.set_word(WALK_TARGET, 0); // 0x10B24
        stand_in(api); // 0x10B2A–0x10B47
    }
    api.frame_pass(); // 0x10B4A
    api.word(WORDS.entry) == 0 // 0x10B4F
}

/// Everything after the frame loop, `0x10B56` to `0x10B75`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x10B61
    api.visit_leave(); // 0x10B69
    api.slot_unload(); // 0x10B6E
}

/// The record names no hook (`+3Fh` is `0000h:0000h`).
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// The animation hook, `10A7h:0106h`: the code in `[bp+0Ah]` less `64h`
/// through the table at `cs:0156h` (`0x10B79`–`0x10B86`); other codes do
/// nothing.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        // 0x10B8B: back to location 2 by entry 7Fh.
        0x64 => {
            api.set_word(WORDS.location, FROM); // 0x10B8B
            api.set_word(WORDS.entry, NODE_7F); // 0x10B91
        }
        // 0x10B99: F0h hidden, the figure shown again, out through exit
        // 71h.
        0x65 => {
            api.obj_hide(OBJECT_F0); // 0x10B9C, the record not kept
            let flags = api.byte(PLAYER_FLAGS) & !HIDDEN;
            api.set_byte(PLAYER_FLAGS, flags); // 0x10BA2
            let player = api.ds(WALKER);
            api.obj_invalidate(player); // 0x10BAB
            api.script_exit(EXIT_71); // 0x10BB5
        }
        // 0x10BBC: sample 2Eh.
        0x66 | 0x67 => api.play_sample(SAMPLE_2E), // 0x10BBE
        _ => {}
    }
}

/// The record names no rule table (`+3Bh` is `0000h:0000h`).
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
