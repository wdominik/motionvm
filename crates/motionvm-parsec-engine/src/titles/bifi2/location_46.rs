//! Location 46, a row of kind 4 with no name (the record's `+37h` is
//! `0000h:0000h`): the cell of the maze before the Eisengitter to
//! Schlönstadt's Gefängnis (location 41), whose Schloß the combination on
//! the `Labyrinth-Karte` (item `1Bh`) opens. Its exits (`+9`/`+19h`) are
//! `6Eh` to location 45, `6Fh` to 41, `70h` to 56 and `73h` to 41.
//!
//! A row of kind 4 loads slot `26h` for its picture (`cmp ax, 4 ; jz` →
//! `mov si, 26h`, `0x02EE7`/`0x02F01`), the maze's shared picture, and binds
//! its own slot's object map onto it; slot 46 holds no dialogue bank of its
//! own, so the scenes below are places of slot `26h`'s bank. There the
//! picture names hotspot `C173h` `Eisengitter` (object `73h`) and hotspot
//! `C0F9h` `Schloß` (object `F9h`, „Eines dieser tollen
//! Computerschlösser“), and hotspot `C16Fh` `Ausgang` (exit `6Fh`) covers
//! the same rectangle as the Eisengitter. The places the rules play read:
//! `0Ah` „Da könnte ich tagelang versuchen, die richtige Kombination zu
//! finden...“, `0Bh` „Auf der Karte stand doch irgendwie eine
//! Zahlenkombination... 2 - 5 - 6 - 3 ... Das Tor müßte jetzt eigentlich
//! aufgehen!“, `0Ch` „Die Tür ist doch schon aufgegangen!“.
//!
//! Re-expressed from the handler at `17A9h:008Fh` (image `0x17B1F` to its
//! `retf` at `0x17C21`), its hook `17A9h:0192h` (`0x17C22` to its `retf` at
//! `0x17C64`, the record's `+3Fh`) and its two rule handlers `17A9h:0005h`
//! and `004Ah` (`0x17A95`–`0x17B1E`), against [`Api`] alone. The record's
//! animation hook `+43h` is `0000h:0000h`, the library's empty one. The
//! location's rules are the two records at `2446h:0C10h` (image
//! `0x25070`, the record's `+3Bh`), fourteen bytes each:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 6 | 4 | `73h` | — | `0005h` |
//! | 4 | 4 | `F9h` | — | `004Ah` |
//!
//! The verbs are, by the verb text row at `DS:116Ah`: 4 `Benutze`, 6
//! `Öffne`. One byte of the logic's saved state block (`DS:2B98h`, `4Eh`
//! bytes) holds the location's progress: `[2BD6h]`, the Schloß opened; no
//! other instruction of the program names it. The `Labyrinth-Karte` is
//! the `Stadtplan` (item `1Ah`) after the `Eimer` `2Eh` is used on it at
//! Rambowsky's (location 7, rule `11B0h:0172h`, `0x11C72`); its look text (place 26 of the global bank
//! 2) tells of the „Zahlenkombination“ on its back.
//!
//! While the location is shown, object `73h`, the Eisengitter, is a door:
//! the handler shows it on frame 1, lets its hotspot follow its live bit
//! (`0F48h:06B9h`, whose table at `cs:0743h` skips kinds 1 to 4, before the
//! kind is written) and writes its kind `+0` as 4 (`0x17B7E`); it writes
//! the kind back to 0 after the frame loop (`0x17BFF`), before the object
//! states are saved. The visit's entry (`09B6h:0212h`, its entry argument
//! in `si`, `0x09D78`) makes an entry `6Eh`–`77h` the entry door only for
//! an object of kind 4 (`cmp byte es:[bx], 4`, `0x09EE6`), and it is
//! called before the kind is written here. For entry `73h` the handler
//! writes the same itself: frame −1 and `[1F3Ah]` (`0x17B82`–`0x17B95`).
//! With the kind 4, `0F48h:074Bh` answers the record's `+2` bit 1 for the
//! object instead of 3 (`0x0FBEB`–`0x0FBFF`).
//!
//! The hook's script events `1Eh` and `1Fh` play samples; `1Eh` also owes
//! ten more of samples 5 to 7, which the frame loop plays one at a time
//! whenever the hundredths `[1CF6h]` are past `32h`, each time setting
//! them back to a roll of 0 to `1Eh`. The count `[2FA6h]` lies outside the
//! saved block; the handler sets it to 0 on every entry (`0x17B9B`).

use super::logic::{Logic, WORDS};
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x17A9;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x17C09`).
const OBJECT_STATES: u16 = 0x2018;

/// Object `73h`, the Eisengitter; exit `73h` leads to location 41.
const GITTER_73: u16 = 0x73;
/// Object `F9h`, the Schloß.
const SCHLOSS_F9: u16 = 0xF9;
/// Item `1Bh`, the `Labyrinth-Karte`.
const LABYRINTH_KARTE: u16 = 0x1B;

/// The kind written into the Eisengitter's `+0` for the visit
/// (`mov byte es:[bx], 4`, `0x17B7E`): a door.
const KIND_DOOR: u8 = 4;
/// The kind written back after the frame loop (`0x17BFF`).
const KIND_PLAIN: u8 = 0;

/// `[2BD6h]` (inside the saved block at `DS:2B98h`): the Schloß opened, 1.
const BYTE_2BD6: u16 = 0x2BD6;

/// `[2FA6h]`: the samples the frame loop still owes, written in segment
/// `17A9h` alone (`0x17B9B`, `0x17BCA`, `0x17C3C`).
const OWED: u16 = 0x2FA6;
/// `[1CF6h]`: the hundredths (`super::build`'s `hundredths`).
const HUNDREDTHS: u16 = 0x1CF6;
/// `cmp word [1CF6h], 32h ; jna` (`0x17BC3`).
const OWED_DELAY: u16 = 0x32;

/// The mode block's object class `[2D3Eh]`, its wait for an animation
/// `[2D6Ch]`, the gate `[2D66h]`/`[2D68h]` and the entry door `[1F3Ah]`.
const OBJECT_CLASS: u16 = 0x2D3E;
const ANIM_WAIT: u16 = 0x2D6C;
const GATE_OFF: u16 = 0x2D66;
const GATE_SEG: u16 = 0x2D68;
const ENTRY_DOOR: u16 = 0x1F3A;
/// The mode block's wait bits (`09B6h:1820h`, `or word [2D46h], 8`).
const BITS: u16 = 0x2D46;

/// The handler's own stack frame: `[bp-4]`/`[bp-2]`, the far pointer
/// `0F48h:096Eh` answered for object `73h` (`0x17B6B`, `0x17B6E`).
const REC_OFF: usize = 0;
const REC_SEG: usize = 1;

fn kept(api: &dyn Api) -> Far {
    Far {
        off: api.local(REC_OFF),
        seg: api.local(REC_SEG),
    }
}

/// `push k ; push id ; call far 06E3h:047Eh ; pop cx ; push dx ; push ax ;
/// call far 03F7h:1CF9h`: the object found and set on animation `k`.
fn anim(api: &mut dyn Api, id: u16, k: i16) {
    let r = api.obj_find(id);
    api.obj_anim(r, k);
}

/// Everything before the frame loop, `0x17B1F` to `0x17BAC`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x17B23
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.visit_enter(loc, entry); // 0x17B30
    // 0x17B38: `09B6h:1820h` is `or word [2D46h], 8 ; retf` (0x0B380).
    let bits = api.word(BITS);
    api.set_word(BITS, bits | 8);
    // 0x17B3D: the Schloß on animation 1 once it is opened, else 0.
    let k = i16::from(api.byte(BYTE_2BD6) == 1);
    anim(api, SCHLOSS_F9, k); // 0x17B4F, 0x17B57
    // 0x17B63: the Eisengitter on frame 1, its record kept.
    let rec = api.obj_show_frame(GITTER_73, 1);
    api.set_local(REC_SEG, rec.seg); // 0x17B6B
    api.set_local(REC_OFF, rec.off); // 0x17B6E
    api.hotspot_follow(rec); // 0x17B73, 0F48h:06B9h
    api.set_far_byte(rec, KIND_DOOR); // 0x17B7E
    // 0x17B82: entered through the Eisengitter, it is the entry door; the
    // answer of the second 0F48h:096Eh is not kept.
    if api.word(WORDS.entry) == GITTER_73 {
        api.obj_show_frame(GITTER_73, -1); // 0x17B8D
        api.set_word(ENTRY_DOOR, GITTER_73); // 0x17B95
    }
    api.set_word(OWED, 0); // 0x17B9B
    api.visit_finish_entry(); // 0x17BA1
    let loc = api.word(WORDS.location); // 0x17BA6
    api.set_word(WORDS.here, loc); // 0x17BA9
    api.set_word(WORDS.entry, 0); // 0x17BAC
}

/// One turn of the frame loop, `0x17BB2` to `0x17BFA`: between the
/// dispatch and the frame pass, one owed sample once the hundredths are
/// past `32h`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x17BB2
    api.dispatch(); // 0x17BB7
    let owed = api.word(OWED);
    if owed != 0 && api.word(HUNDREDTHS) > OWED_DELAY {
        // 0x17BBC, 0x17BC3
        api.set_word(OWED, owed.wrapping_sub(1)); // 0x17BCA
        let h = api.random(0, 0x1E); // 0x17BD2
        api.set_word(HUNDREDTHS, h); // 0x17BDA
        let k = api.random(5, 7); // 0x17BE1
        api.play_sample(k); // 0x17BEA
    }
    api.frame_pass(); // 0x17BF0
    api.word(WORDS.entry) == 0 // 0x17BF5
}

/// Everything after the frame loop, `0x17BFC` to `0x17C21`.
pub(crate) fn leave(api: &mut dyn Api) {
    let rec = kept(api);
    api.set_far_byte(rec, KIND_PLAIN); // 0x17BFF
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x17C0E
    api.visit_leave(); // 0x17C16
    api.slot_unload(); // 0x17C1B
}

/// The hook, `17A9h:0192h`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read. Only code 7, a script event, has arms (`0x17C28`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    if code != 7 {
        return;
    }
    match arg {
        // 0x17C3C: ten samples owed, sample 5, the hundredths from a roll
        // of 0 to 1Eh.
        0x1E => {
            api.set_word(OWED, 0x0A); // 0x17C3C
            api.play_sample(5); // 0x17C44
            let h = api.random(0, 0x1E); // 0x17C4E
            api.set_word(HUNDREDTHS, h); // 0x17C56
        }
        // 0x17C5B: sample 19h.
        0x1F => api.play_sample(0x19), // 0x17C5D
        _ => {}
    }
}

/// The gate cleared (`mov word [2D68h], 0 ; mov word [2D66h], 0`).
fn clear_gate(api: &mut dyn Api) {
    api.set_word(GATE_SEG, 0);
    api.set_word(GATE_OFF, 0);
}

/// The location's rule handlers by their offset in segment `17A9h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 17A9:0005 (0x17A95), Öffne on the Eisengitter: with the Schloß
        // opened it opens (animation 1) and is the entry door, the mode
        // machine waits for the animation; answer 0Ch. Else the class is
        // cleared, the gate cleared and the search goes on (0x17AC4).
        0x0005 => {
            if api.byte(BYTE_2BD6) == 1 {
                // 0x17A98
                anim(api, GITTER_73, 1); // 0x17AA3, 0x17AAB
                api.set_word(ENTRY_DOOR, GITTER_73); // 0x17AB3
                api.set_word(ANIM_WAIT, 2); // 0x17AB9
                0x0C
            } else {
                api.set_word(OBJECT_CLASS, 0); // 0x17AC4
                clear_gate(api); // 0x17ACA, 0x17AD0
                0
            }
        }
        // 17A9:004A (0x17ADA), Benutze on the Schloß: with the
        // Labyrinth-Karte the first time the Schloß opens (animation 1) and
        // scene 0Bh, after that scene 0Ch; without it scene 0Ah.
        0x004A => {
            let n = if api.inv_has(LABYRINTH_KARTE) != 0 {
                // 0x17ADF, 0x17AE5
                if api.byte(BYTE_2BD6) == 0 {
                    // 0x17AE9
                    api.set_byte(BYTE_2BD6, 1); // 0x17AF0
                    anim(api, SCHLOSS_F9, 1); // 0x17AFA, 0x17B02
                    0x0B
                } else {
                    0x0C // 0x17B0E
                }
            } else {
                0x0A // 0x17B12
            };
            api.scene_play(n); // 0x17B14
            0x0A
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
