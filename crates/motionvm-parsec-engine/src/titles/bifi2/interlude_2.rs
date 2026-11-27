//! Location 2, a row of kind 7 with no name (`+37h` is `0000h:0000h`) and
//! two exits: `7Ah` to the Vorführraum (location 26) and `7Fh` to location
//! 3, the other row of kind 7 it pairs with. The game gives the row no
//! name; the module is named after its number.
//!
//! Re-expressed from the handler at `109Ah:0001h` (image `0x109A1` to its
//! `retf` at `0x10A36`) and its animation hook `109Ah:0097h` (`0x10A37` to
//! `0x10A71`, the record's `+43h`), against [`Api`] alone. The record's
//! hook `+3Fh` is `0000h:0000h`, and the visit's entry puts the library's
//! empty routine `09B6h:1BD2h` (`retf`, `0x0B732`) in its place
//! (`0x09E51`). The rule table `+3Bh` is `0000h:0000h` too; the entry hands
//! it to `0E40h:0031h` as it is (`0x09DAB`–`0x09DB3`), which marks a null
//! table absent (`or ax, si ; jz`, `0x0E444`–`0x0E44C`). The location has
//! no rules and no gate: the immediate `109Ah` occurs in the image only in
//! the location record (`0x218CD`, `0x218DD`), and the segment holds
//! nothing past the animation hook's table (`cs:00D2h`, `0x10A72`); the
//! next segment, `10A7h`, is location 3's (`10A7h:000Ah`, `0x10A7A`).
//!
//! The visit is a passage without input: the input is taken over on entry
//! (`0x109E8`) and never given back here. Object `F0h` is shown and set on
//! animation 1 when the visit comes from the Vorführraum and on animation 0
//! otherwise; its animation codes `64h` and `65h` end the visit, to the
//! Vorführraum by entry `7Ah` and to location 3 by entry `7Fh`, and codes
//! `66h` and `67h` play sample `2Eh`.
//!
//! The word the main loop compares the running handler with, `[200Eh]`,
//! is written only after the entry is finished (`0x109FA`), not before the
//! shared entry as the rooms of kind 1 do: until then it names the location the
//! visit came from. The handler reads it so (`0x109B6`), and so do the
//! loader (`02D5h:010Ah`) and the entry's end (`09B6h:00BBh`) through the
//! pair routine `0F48h:082Bh`, which answers 7 for a visit between this
//! row and location 3.
//!
//! The one byte of the logic's saved state block (`DS:2B98h`, `4Eh` bytes)
//! the location touches is `[2BCEh]`, the Requisite's state of object
//! `93h`, raised to 2 when it is below.

use super::logic::{Logic, WORDS};
use crate::titles::parsec1994::{Api, Far};

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x10A1F`).
const OBJECT_STATES: u16 = 0x2018;

/// The object the visit plays (`push 0F0h`, `0x109C7`).
const OBJECT_F0: u16 = 0xF0;

/// `[2BCEh]`: the Requisite's state of object `93h` (inside the saved
/// block at `DS:2B98h`).
const BYTE_2BCE: u16 = 0x2BCE;

/// The Vorführraum, and the entry into it (`0x10A4C`, `0x10A52`).
const VORFUEHRRAUM: u16 = 26;
const ENTRY_VORFUEHRRAUM: u16 = 0x7A;
/// Location 3, and the entry into it (`0x10A5A`, `0x10A60`).
const LOCATION_3: u16 = 3;
const ENTRY_LOCATION_3: u16 = 0x7F;

/// The sample codes `66h` and `67h` play (`push 2Eh`, `0x10A68`).
const SAMPLE: u16 = 0x2E;

/// Everything before the frame loop, `0x109A1` to `0x109FD`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x109A1
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.visit_enter(loc, entry); // 0x109AE
    // 0x109B6: the location the visit came from, `[200Eh]` not yet
    // written.
    let k = i16::from(api.word(WORDS.here) == VORFUEHRRAUM);
    let r = api.obj_show_frame(OBJECT_F0, -1); // 0x109CA, the record not kept
    api.obj_anim(r, k); // 0x109D4
    // 0x109DC: `cmp byte [2BCEh], 2 ; jnc`, unsigned.
    if api.byte(BYTE_2BCE) < 2 {
        api.set_byte(BYTE_2BCE, 2); // 0x109E3
    }
    api.input_lock(); // 0x109E8
    api.hover_off(); // 0x109ED
    api.visit_finish_entry(); // 0x109F2
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x109F7, 0x109FA
    api.set_word(WORDS.entry, 0); // 0x109FD
}

/// One turn of the frame loop, `0x10A03` to `0x10A17`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x10A03
    api.dispatch(); // 0x10A08
    api.frame_pass(); // 0x10A0D
    api.word(WORDS.entry) == 0 // 0x10A12
}

/// Everything after the frame loop, `0x10A19` to `0x10A36`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x10A24
    api.visit_leave(); // 0x10A2C
    api.slot_unload(); // 0x10A31
}

/// The record's `+3Fh` is null: the library's empty hook `09B6h:1BD2h`.
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// The animation hook, `109Ah:0097h`: the code in `[bp+0Ah]`; codes `64h`
/// to `67h` have arms (`sub bx, 64h ; cmp bx, 3 ; ja`, `0x10A3D`–`0x10A43`,
/// the table at `cs:00D2h`, `0x10A72`: `00ACh`, `00BAh`, `00C8h`, `00C8h`).
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        // 0x10A4C: on to the Vorführraum by entry 7Ah.
        0x64 => {
            api.set_word(WORDS.location, VORFUEHRRAUM); // 0x10A4C
            api.set_word(WORDS.entry, ENTRY_VORFUEHRRAUM); // 0x10A52
        }
        // 0x10A5A: on to location 3 by entry 7Fh.
        0x65 => {
            api.set_word(WORDS.location, LOCATION_3); // 0x10A5A
            api.set_word(WORDS.entry, ENTRY_LOCATION_3); // 0x10A60
        }
        // 0x10A68: sample 2Eh.
        0x66 | 0x67 => api.play_sample(SAMPLE), // 0x10A6A
        _ => {}
    }
}

/// The record's `+3Bh` is null: the location has no rules.
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
