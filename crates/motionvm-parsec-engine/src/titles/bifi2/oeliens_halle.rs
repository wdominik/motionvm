//! Location 28, the Öliens-Halle, a row of kind 1 with two exits: `7Bh` to
//! Öliens (location 27) and `72h`, whose destination is 0 (the row's `+9h`
//! and `+19h`). The module is named after the location as the game itself
//! names it in its location table (`Öliens-Halle`, the record's `+37h`,
//! `250Ah:01A6h`).
//!
//! Re-expressed from the handler at `16D7h:00AAh` (image `0x16E1A` to its
//! `retf` at `0x16E9F`), its hook `16D7h:0130h` (`0x16EA0` to its `retf`
//! at `0x16EBC`, the record's `+3Fh`) and its five rule handlers
//! `16D7h:0009h` to `0082h` (`0x16D79`–`0x16E19`), against [`Api`] alone.
//! The row's `+43h` is null, so the animation hook is the library's empty
//! one. The segment holds nothing else: the next one, `16EBh` (location
//! 29's handler `16EBh:0058h`), follows the hook. The location's rules are
//! the six records at `2446h:0A70h` (image `0x24ED0`, the record's
//! `+3Bh`), fourteen bytes each, ended by the zero phase at `0x24F24`; a
//! class is stored complemented (`FFFBh` class 4):
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 5 | `20h` | 5 | `A0h` | `0009h` |
//! | 5 | `20h` | class 4 | `A0h` | `0033h` |
//! | 5 | `20h` | `0Dh` | `A0h` | `0033h` |
//! | 4 | 4 | `A0h` | — | `0082h` |
//! | 1 | 4 | `72h` | — | `0059h` |
//! | 0 | 1 | `72h` | — | `0074h` |
//!
//! The verbs are, by the verb text row at `DS:116Ah`: 1 `Betrachte`, 4
//! `Benutze`, 5 `Benutze … mit`. The items the logic names are, by the item
//! row at `DS:10AAh`: 5 `Schlüssel`, `0Dh` `Haarnadel`.
//!
//! Two bytes of the logic's saved state block (`DS:2B98h`, `4Eh` bytes)
//! carry the location. `[2BCCh]` is Öliens's state: its handler goes
//! through the table at `16A0h:0254h` by it (`0x16B32`) and writes 1
//! (`0x16B3C`), 3 (`0x16B9E`) and 4 (`0x16BFB`); here the `Schlüssel` used
//! on object `A0h` writes 2 (rule `0009h`), and rules `0033h` and `0082h`
//! read whether it is at least 2. `[2BD2h]` is this location's alone: set
//! on an entry by `72h`, it chooses the scene `Betrachte` on `72h` plays.
//!
//! The entry `72h` is the one location 13 (kind 7) leaves by: its
//! animation hook `13FAh:008Fh` writes `[200Ch]` = `1Ch` and `[200Ah]` =
//! `72h` for code `65h` (`0x1405F`, `0x14065`). Entered so, the figure
//! walks to node `0FA0h`; otherwise object `72h` is shown on frame 1.
//! Rule `0074h`, verb 0 and phase 1 on `72h`, writes verb 1 (`Betrachte`)
//! and answers 1.

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::Api;

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x16D7;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x16E88`).
const OBJECT_STATES: u16 = 0x2018;

/// Object `72h`, the exit whose destination is 0 and the entry from
/// location 13.
const OBJECT_72: u16 = 0x72;
/// Object `F0h`, shown on frame 0 by rule `0009h` (`0x16D86`).
const OBJECT_F0: u16 = 0xF0;

/// Item 5, the `Schlüssel` (`push 5 ; call far 0751h:099Dh`, `0x16D90`).
const SCHLUESSEL: u16 = 0x05;

/// The node the figure walks to on an entry by `72h` (`mov ax, 0FA0h`,
/// `0x16E40`).
const NODE_0FA0: u16 = 0x0FA0;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BCCh]`: Öliens's state, 2 once the `Schlüssel` is used on `A0h`.
const BYTE_2BCC: u16 = 0x2BCC;
/// `[2BD2h]`: the location entered by `72h`.
const BYTE_2BD2: u16 = 0x2BD2;

/// The mode block's verb `[2D38h]` and walk target `[2D4Ah]`.
const VERB: u16 = 0x2D38;
const WALK_TARGET: u16 = 0x2D4A;

/// Everything before the frame loop, `0x16E1A` to `0x16E66`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x16E1A
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x16E26
    api.visit_enter(loc, entry); // 0x16E2A
    if api.word(WORDS.entry) == OBJECT_72 {
        // 0x16E32
        api.set_byte(BYTE_2BD2, 1); // 0x16E39
        api.set_word(WALK_TARGET, NODE_0FA0); // 0x16E43
        let player = api.ds(WALKER);
        api.walker_walk_to(player, NODE_0FA0, 1); // 0x16E4B, answer not read
    } else {
        api.obj_show_frame(OBJECT_72, 1); // 0x16E59
    }
    api.visit_finish_entry(); // 0x16E61
    api.set_word(WORDS.entry, 0); // 0x16E66
}

/// One turn of the frame loop, `0x16E6C` to `0x16E80`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x16E6C
    api.dispatch(); // 0x16E71
    api.frame_pass(); // 0x16E76
    api.word(WORDS.entry) == 0 // 0x16E7B
}

/// Everything after the frame loop, `0x16E82` to `0x16E9F`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x16E8D
    api.visit_leave(); // 0x16E95
    api.slot_unload(); // 0x16E9A
}

/// The hook, `16D7h:0130h`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read. Only the script event `1Eh` (code 7) has an arm:
/// sample `1Ch`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    if code == 7 && arg == 0x1E {
        // 0x16EA6, 0x16EAE
        api.play_sample(0x1C); // 0x16EB5
    }
}

/// `push hi ; push lo ; call far 203Dh:0024h ; push ax ; call far
/// 080Ch:164Ch`: the roll is the scene.
fn roll(api: &mut dyn Api, lo: u16, hi: u16) {
    let n = api.random(lo, hi);
    api.scene_play(num::signed(n));
}

/// Whether Öliens's state is at least 2 (`cmp byte [2BCCh], 2`, unsigned).
fn state_2(api: &dyn Api) -> bool {
    api.byte(BYTE_2BCC) >= 2
}

/// The Öliens-Halle's rule handlers by their offset in segment `16D7h`.
/// None reads the phase it is called with.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 16D7:0009 (0x16D79), the Schlüssel on object A0h: Öliens's state
        // 2, object F0h on frame 0, the Schlüssel taken, scene 2.
        0x0009 => {
            api.set_byte(BYTE_2BCC, 2); // 0x16D7C
            api.obj_show_frame(OBJECT_F0, 0); // 0x16D86
            api.inv_take(SCHLUESSEL); // 0x16D90
            api.scene_play(2); // 0x16D98
            0x0A // 0x16D9E
        }
        // 16D7:0033 (0x16DA3), class 4 or the Haarnadel on object A0h:
        // below state 2 scene 3, else one of scenes !0x1F to !0x1D.
        0x0033 => {
            if state_2(api) {
                roll(api, !0x1F, !0x1D); // 0x16DB1, 0x16DBE
            } else {
                api.scene_play(3); // 0x16DBE
            }
            0x0A // 0x16DC4
        }
        // 16D7:0059 (0x16DC9), Betrachte object 72h: scene 0 before an
        // entry by 72h, scene 1 after.
        0x0059 => {
            let n = if api.byte(BYTE_2BD2) == 0 { 0 } else { 1 }; // 0x16DCC
            api.scene_play(n); // 0x16DD9
            0x0A // 0x16DDF
        }
        // 16D7:0074 (0x16DE4), phase 1 on object 72h: the verb becomes
        // Betrachte, answer 1.
        0x0074 => {
            api.set_word(VERB, 1); // 0x16DE7
            1 // 0x16DED
        }
        // 16D7:0082 (0x16DF2), Benutze object A0h: below state 2 one of
        // scenes !0x21 to !0x20, else one of scenes !0x1F to !0x1D.
        0x0082 => {
            if state_2(api) {
                roll(api, !0x1F, !0x1D); // 0x16E06, 0x16E0F
            } else {
                roll(api, !0x21, !0x20); // 0x16E06, 0x16E0F
            }
            0x0A // 0x16E15
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
