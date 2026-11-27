//! Location 36, the Lift, a row of kind 2 in room slot `25h`, which it
//! shares with the corridors 18, 19 and 30 to 35. The module is named after
//! the location as the game itself names it in its location table (`Lift`,
//! the record's `+37h`, `250Ah:020Dh`).
//!
//! Re-expressed from the handler at `1715h:0095h` (image `0x171E5` to its
//! `retf` at `0x172B3`), its animation hook `1715h:0164h` (`0x172B4`, the
//! record's `+43h`) and its three rule handlers, against [`Api`] alone. The
//! record's hook `+3Fh` is `0000h:0000h`; the Lift has none. The segment
//! holds nothing else: it opens at `0x17157` and the next segment, `172Bh`
//! (location 39's handler `172Bh:0009h`), begins at `0x172B9`.
//!
//! The location's rules are the three records at `2446h:0B30h` (image
//! `0x24F90`, the record's `+3Bh`), fourteen bytes each:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | `0Dh` | 4 | class 8 (`FFF7h`) | — | `0007h` |
//! | 4 | 2 | class 8 (`FFF7h`) | — | `0079h` |
//! | 0 | 1 | class 8 (`FFF7h`) | — | `0087h` |
//!
//! The eight exits of the record (`+9`, image `0x21DA9`) are `64h` to `6Bh`,
//! and their destinations (`+19h`, `0x21DB9`) the corridors 18, 19, 31, 32,
//! 33, 34, 35 and 30. Each corridor's second exit (`+0Ah`) is the one of
//! these ids that leads to it, its destination `24h`, the Lift.
//!
//! Two words outside the saved block carry the lift between the Lift and
//! the corridors: `[2F8Eh]`, the floor the visit was entered from (written
//! here at `0x171F9`, by the corridor at `0x150C6`, stepped by the
//! corridor at `0x151E3`/`0x151F5`), and `[2F8Ch]`, the floor left for
//! (written here at `0x172AB`, by the corridor at `0x150C3`). A floor is an
//! exit id less `64h`.
//!
//! No far pointer of segment `1715h` is written anywhere in the image (the
//! immediate `1715h` occurs only in the location record, `0x221D5`/
//! `0x221E5`, and in the rule table, `0x24F9C`–`0x24FB8`): the Lift has no
//! gate of its own.

use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x1715;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x1728A`).
const OBJECT_STATES: u16 = 0x2018;

/// The location table's segment and the Lift's own record in it
/// (`36 * 44h = 0990h`): its sixteen exit ids at `+9` (`es:[si+0999h]`,
/// `0x1716E`), their destinations at `+19h` (`es:[si+09A9h]`, `0x17180`),
/// and a record's kind word at `+7` (`0x171A7`).
const TABLE_SEG: u16 = 0x2181;
const EXIT_IDS: u16 = 0x0999;
const EXIT_TARGETS: u16 = 0x09A9;
const RECORD: u16 = 0x44;
const KIND: u16 = 0x07;
const EXITS: u16 = 0x10;

/// The objects of the visit: `64h` shown on frame 1 (`0x17224`), `A5h`
/// shown on the floor's frame (`0x17233`), `A3h` shown for the visit and
/// hidden after it (`0x1723E`, `0x17279`), `A4h` set on the floor's
/// animation (`0x1724B`, `0x17253`).
const OBJECT_64: u16 = 0x64;
const OBJECT_A3: u16 = 0xA3;
const OBJECT_A4: u16 = 0xA4;
const OBJECT_A5: u16 = 0xA5;

/// `[2F8Eh]`: the floor entered from; `[2F8Ch]`: the floor left for.
const FLOOR_FROM: u16 = 0x2F8E;
const FLOOR_TO: u16 = 0x2F8C;

/// The mode block: the verb `[2D38h]`, the object chosen `[2D3Ch]`, the
/// wait bits `[2D46h]`.
const VERB: u16 = 0x2D38;
const OBJECT: u16 = 0x2D3C;
const BITS: u16 = 0x2D46;

/// `cmp word [200Ah], 64h ; jl` and `cmp word [200Ah], 6Dh ; jg`
/// (`0x171E5`–`0x171F1`, again at `0x17297`–`0x172A3`), signed: the entry
/// word as a floor, `add ax, 0FF9Ch`. The test admits `6Ch` and `6Dh`,
/// which no exit of the record names.
fn floor(api: &dyn Api) -> Option<u16> {
    let entry = api.word(WORDS.entry);
    (0x64..=0x6D)
        .contains(&num::signed(entry))
        .then(|| entry.wrapping_add(0xFF9C))
}

/// Everything before the frame loop, `0x171E5` to `0x1725F`.
pub(crate) fn enter(api: &mut dyn Api) {
    // 0x171E5: before the visit's setup, the floor the entry names.
    if let Some(f) = floor(api) {
        api.set_word(FLOOR_FROM, f); // 0x171F9
    }
    api.visit_setup(); // 0x171FC
    // 0x17201–0x17207: `push 1 ; push [200Ch]` — entry 1, not `[200Ah]`.
    let loc = api.word(WORDS.location);
    api.visit_enter(loc, 1); // 0x17207
    // 0x1720F: `09B6h:1820h` is `or word [2D46h], 8 ; retf` (0x0B380),
    // Dunkle Schatten's `0EB9h:18EEh`, its one write made here: the bit
    // the options' gate holds the menu back by, and in the Lift ends
    // without it (`0x09FDA`, `0x09FE2`).
    let bits = api.word(BITS);
    api.set_word(BITS, bits | 8);
    let loc = api.word(WORDS.location); // 0x17214, read again
    api.set_word(WORDS.here, loc); // 0x17217
    api.set_word(WORDS.entry, 0); // 0x1721A
    api.obj_show_frame(OBJECT_64, 1); // 0x17224, 0F48h:096Eh
    let f = api.word(FLOOR_FROM);
    api.obj_show_frame(OBJECT_A5, num::signed(f)); // 0x17233, 0F48h:096Eh
    api.obj_show(OBJECT_A3); // 0x1723E
    let f = api.word(FLOOR_FROM); // 0x17244
    let rec = api.obj_find(OBJECT_A4); // 0x1724B
    api.obj_anim(rec, num::signed(f)); // 0x17253
    api.visit_finish_entry(); // 0x1725B
}

/// One turn of the frame loop, `0x17260` to `0x17274`: true while
/// `[200Ah]` is 0 (`0x1726F`), as the loop runs on.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x17260
    api.dispatch(); // 0x17265
    api.frame_pass(); // 0x1726A
    api.word(WORDS.entry) == 0 // 0x1726F
}

/// Everything after the frame loop, `0x17276` to `0x172B3`. The states are
/// saved after `visit_leave`, not before it as in the Pforte.
pub(crate) fn leave(api: &mut dyn Api) {
    api.obj_hide(OBJECT_A3); // 0x17279
    api.visit_leave(); // 0x1727F
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x1728F
    // 0x17297: the floor the exit names.
    if let Some(f) = floor(api) {
        api.set_word(FLOOR_TO, f); // 0x172AB
    }
    api.slot_unload(); // 0x172AE
}

/// The record names no hook (`+3Fh` is `0000h:0000h`).
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// The animation hook, `1715h:0164h`: `push bp ; mov bp, sp ; pop bp ;
/// retf` (`0x172B4`), nothing.
pub(crate) fn anim_hook(_api: &mut dyn Api, _rec: Far, _code: u16) {}

/// Rule `1715h:0007h` (`0x17157`): the object chosen is an exit id; the
/// Lift is left for its destination when that is another location of kind
/// 2. The phase is not read.
fn rule_0007(api: &mut dyn Api) -> i16 {
    let id = api.word(OBJECT);
    api.set_word(WORDS.entry, id); // 0x1715E
    let here = api.word(WORDS.here);
    api.set_word(WORDS.location, here); // 0x17164
    // 0x17167–0x17190: the first of the sixteen exit ids that is the
    // entry word, its destination the location.
    for i in 0..EXITS {
        let exit = api.far_byte(Far {
            off: EXIT_IDS.wrapping_add(i),
            seg: TABLE_SEG,
        }); // 0x1716E
        if u16::from(exit) == api.word(WORDS.entry) {
            // 0x17175
            let to = api.far_byte(Far {
                off: EXIT_TARGETS.wrapping_add(i),
                seg: TABLE_SEG,
            }); // 0x17180
            api.set_word(WORDS.location, u16::from(to)); // 0x17187
            break; // 0x1718A
        }
    }
    // 0x17192: here again, or a destination whose kind word is not 2
    // (0x171A7): stay, and no exit.
    let loc = api.word(WORDS.location);
    let stay = loc == api.word(WORDS.here)
        || api.far_word(Far {
            off: loc.wrapping_mul(RECORD).wrapping_add(KIND),
            seg: TABLE_SEG,
        }) != 2;
    if stay {
        let here = api.word(WORDS.here);
        api.set_word(WORDS.location, here); // 0x171B2
        api.set_word(WORDS.entry, 0); // 0x171B5
    }
    api.play_sample(5); // 0x171BD
    1 // 0x171C3
}

/// The Lift's rule handlers by their offset in segment `1715h`. None reads
/// the phase it is called with.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        0x0007 => rule_0007(api),
        // 1715:0079 (0x171C9): verb 0Dh, answer 5.
        0x0079 => {
            api.set_word(VERB, 0x0D); // 0x171CC
            5 // 0x171D2
        }
        // 1715:0087 (0x171D7): verb 0Dh, answer 1.
        0x0087 => {
            api.set_word(VERB, 0x0D); // 0x171DA
            1 // 0x171E0
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
