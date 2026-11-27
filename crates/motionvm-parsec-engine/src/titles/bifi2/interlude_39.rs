//! Location 39 (`27h`), a row of kind 7 with no name, no exits, no rule
//! table and no hook: an interlude between location 6 and location 13,
//! both of kind 7 as well. The module is named after its number, since the
//! location table gives the row no name (`+37h` is `0000h:0000h`).
//!
//! Re-expressed from the handler at `172Bh:0009h` (image `0x172B9` to its
//! `retf` at `0x173C7`) and its animation hook `172Bh:0118h` (`0x173C8` to
//! its `retf` at `0x1745D`, the record's `+43h`; its jump table at
//! `cs:01AEh`, `0x1745E`–`0x17465`), against [`Api`] alone. The segment
//! holds nothing else: the next one, `1746h` (location 40's), begins at
//! `0x17460`, and its code at `0x17466`. The record's hook `+3Fh` is
//! `0000h:0000h`, so the entry installs the library's empty one,
//! `09B6h:1BD2h` (`0x09E51`–`0x09E57`); its rule table `+3Bh` is null, and
//! the segment writes neither `[2D66h]` nor `[2D68h]`: no gate.
//!
//! Location 6's animation hook (`1187h:017Ah`, `0x119EA`) leads here: its
//! code `6Ch`, by the table at `cs:028Bh` (`0x11AFB`), sets location `27h`
//! with entry `7Fh` (`0x11AC5`–`0x11AD1`). No other write to `[200Ch]`
//! names `27h` itself: the image's other immediates are other rows, no
//! row's exit destination (`+19h`) and no entry of the Kaffee's row at
//! `DS:182Ch` is `27h`, the walk on (`0x00AD4`) and the location browser
//! (`0x0A14A`) pass over a row of kind 7, and the location stack
//! (`0x0F3DB`) gives back only the row pushed onto it (`[200Eh]`,
//! `0x0F2D2`). This location's code
//! `65h` leads on to location 13 with entry `7Fh` (`0x17409`–`0x1740F`),
//! the image's one write of `0Dh` to `[200Ch]`.
//!
//! The visit takes the input over (`0x1730B`) and never gives it back;
//! objects `F0h` and `F1h` run their animation 0, whose codes `64h` to
//! `67h` reach the animation hook. The frame loop plays sample `10h` each
//! time the hundredths `[1CF6h]` pass `32h`, and sets them to a roll of the
//! generator, so a higher roll brings the next sample sooner: `[2F96h]` to
//! `[2F96h] + 0Ah` while `[2F92h]` is clear, `23h` to `2Dh` from code `64h`
//! to code `67h`. The lower bound `[2F96h]` climbs by one a sample up to
//! `1Eh`, and once code `67h` has set `[2F98h]` falls by two a sample down
//! to 1 or 0: the samples come faster, then slower again.
//!
//! Code `64h` raises the effects' volume to `64h` through the digital
//! driver's entry 22 (`1975h:044Ah`) and keeps the old one in `[2F90h]`;
//! code `67h` puts it back.

use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x173B0`).
const OBJECT_STATES: u16 = 0x2018;

/// The two objects the entry sets going (`push 0F0h`, `0x172D2`; `push
/// 0F1h`, `0x172EB`).
const OBJECT_F0: u16 = 0xF0;
const OBJECT_F1: u16 = 0xF1;

/// `[1CF6h]`: the hundredths (`super::build`'s `hundredths`).
const HUNDREDTHS: u16 = 0x1CF6;
/// `cmp word [1CF6h], 32h ; jna` (`0x1734C`).
const SAMPLE_DUE: u16 = 0x32;

/// The location's own words, read and written in segment `172Bh` alone
/// and outside the `38h` bytes from `DS:2D38h` the visit clears.
/// `[2F90h]`: the effects' volume before code `64h` (`0x173EE`).
const OLD_VOLUME: u16 = 0x2F90;
/// `[2F92h]`: 1 from code `64h` to code `67h`; cleared on entry
/// (`0x17308`).
const LOUD: u16 = 0x2F92;
/// `[2F94h]`: code `66h`'s counter, cleared on entry (`0x1733C`).
const COUNT_66: u16 = 0x2F94;
/// `[2F96h]`: the wait's lower bound, signed; cleared on entry
/// (`0x17302`).
const LOW: u16 = 0x2F96;
/// `[2F98h]`: 1 after code `67h`, the bound then falling; cleared on
/// entry (`0x17305`).
const FALLING: u16 = 0x2F98;

/// Location 13 and the entry code `65h` sends it with (`0x17409`,
/// `0x1740F`).
const NEXT_LOCATION: u16 = 0x0D;
const NEXT_ENTRY: u16 = 0x7F;

/// The samples: `10h` the frame loop's, `0Fh` and `0Eh` the entry's and
/// the hook's.
const SAMPLE_10: u16 = 0x10;
const SAMPLE_0F: u16 = 0x0F;
const SAMPLE_0E: u16 = 0x0E;

/// `push 64h` before `1975h:044Ah` (`0x173E6`).
const LOUD_VOLUME: u16 = 0x64;

/// Everything before the frame loop, `0x172B9` to `0x1733C`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x172B9
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.visit_enter(loc, entry); // 0x172C6
    let r = api.obj_show_frame(OBJECT_F0, -1); // 0x172D5
    api.obj_anim(r, 0); // 0x172DF
    let r = api.obj_show_frame(OBJECT_F1, -1); // 0x172EE
    api.obj_anim(r, 0); // 0x172F8
    api.set_word(LOW, 0); // 0x17302
    api.set_word(FALLING, 0); // 0x17305
    api.set_word(LOUD, 0); // 0x17308
    api.input_lock(); // 0x1730B
    api.hover_off(); // 0x17310
    api.visit_finish_entry(); // 0x17315
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x1731D
    api.set_word(WORDS.entry, 0); // 0x17320
    api.set_word(HUNDREDTHS, 0); // 0x17326
    api.play_sample(SAMPLE_10); // 0x1732E
    api.play_sample(SAMPLE_0F); // 0x17336
    api.set_word(COUNT_66, 0); // 0x1733C
}

/// One turn of the frame loop, `0x17342` to `0x173A8`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x17342
    api.dispatch(); // 0x17347
    if api.word(HUNDREDTHS) > SAMPLE_DUE {
        // 0x1734C
        let low = num::signed(api.word(LOW));
        if api.word(FALLING) != 0 {
            // 0x17353
            if low > 1 {
                // 0x1735A, `jng`
                api.set_word(LOW, num::unsigned(low.wrapping_sub(2))); // 0x17361
            }
        } else if low < 0x1E {
            // 0x17368, `jnl`
            api.set_word(LOW, num::unsigned(low.wrapping_add(1))); // 0x1736F
        }
        let wait = if api.word(LOUD) != 0 {
            // 0x17373
            api.random(0x23, 0x2D) // 0x1737A, 0x1738B
        } else {
            let low = api.word(LOW);
            api.random(low, low.wrapping_add(0x0A)) // 0x17380–0x1738B
        };
        api.set_word(HUNDREDTHS, wait); // 0x17393
        api.play_sample(SAMPLE_10); // 0x17398
    }
    api.frame_pass(); // 0x1739E
    api.word(WORDS.entry) == 0 // 0x173A3
}

/// Everything after the frame loop, `0x173AA` to `0x173C7`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x173B5
    api.visit_leave(); // 0x173BD
    api.slot_unload(); // 0x173C2
}

/// The record names no hook (`+3Fh` is `0000h:0000h`).
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// The animation hook, `172Bh:0118h`: the code in `[bp+0Ah]`; codes `64h`
/// to `67h` have arms (`sub bx, 64h ; cmp bx, 3 ; jna`, `0x173CE`–
/// `0x173D4`, the table at `cs:01AEh`), any other does nothing.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        // 0x173E0: loud, the effects' volume raised and the old one kept,
        // the next sample due at once, sample 0Eh twice.
        0x64 => {
            api.set_word(LOUD, 1); // 0x173E0
            let old = api.effect_volume(LOUD_VOLUME); // 0x173E8, 1975h:044Ah
            api.set_word(OLD_VOLUME, old); // 0x173EE
            api.set_word(HUNDREDTHS, SAMPLE_DUE); // 0x173F1
            api.play_sample(SAMPLE_0E); // 0x173F9
            api.play_sample(SAMPLE_0E); // 0x17401
        }
        // 0x17409: on to location 13, entry 7Fh.
        0x65 => {
            api.set_word(WORDS.location, NEXT_LOCATION); // 0x17409
            api.set_word(WORDS.entry, NEXT_ENTRY); // 0x1740F
        }
        // 0x17417: sample 0Fh twice while loud; the counter stepped, and
        // sample 0Fh on three counts of four.
        0x66 => {
            if api.word(LOUD) != 0 {
                // 0x17417
                api.play_sample(SAMPLE_0F); // 0x17420
                api.play_sample(SAMPLE_0F); // 0x17428
            }
            let n = api.word(COUNT_66).wrapping_add(1);
            api.set_word(COUNT_66, n); // 0x1742E
            if n & 3 != 0 {
                // 0x17435
                api.play_sample(SAMPLE_0F); // 0x1743A, 0x17401
            }
        }
        // 0x1743E: quiet again, sample 0Eh, the old volume back, the bound
        // falling from now on.
        0x67 => {
            api.set_word(LOUD, 0); // 0x1743E
            api.play_sample(SAMPLE_0E); // 0x17446
            let old = api.word(OLD_VOLUME);
            api.effect_volume(old); // 0x17450, 1975h:044Ah, answer not read
            api.set_word(FALLING, 1); // 0x17456
        }
        _ => {}
    }
}

/// The record's rule table `+3Bh` is null: no rules of its own.
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
