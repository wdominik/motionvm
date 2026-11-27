//! Location 1, the Kaffee, a row of kind 5: the conversation a fresh game
//! opens with and the story comes back to between its chapters.
//!
//! Re-expressed from the handler at `107Ah:0020h` (image `0x107C0` to its
//! `retf` at `0x10949`), its gate `107Ah:0008h` (`0x107A8`) and its hook
//! `107Ah:01AAh` (`0x1094A`, the record's `+3Fh`), against [`Api`] alone.
//!
//! The entry it is entered by, 0 to 8, is the scene that plays
//! (`080Ch:164Ch(entry)`, `0x107D7`, Dunkle Schatten's `0CFDh:172Fh`), with
//! the input held and the mode machine in mode 7 behind the location's own
//! gate. When the scene is over the gate lets go with entry `7Fh`, and the
//! location the story goes on to is the entry's in the row at `DS:182Ch`
//! (`0x10920`): from entry 0 the Pforte, location 4. The four faces of the
//! table, objects `F2h`, `EAh`, `EEh` and `E6h`, take the frame each speaker's
//! change names (the hook's code 9).
//!
//! The row names `FFFFh` for entry 6: the game's end (`0x108E5`–`0x10947`).
//! The faces go back to the frames the visit set them on, the band's
//! redraw is held, the music fades over two seconds while the frame pass
//! goes on alone for `C8h` hundredths; then the music stops, the slot is
//! let go — the visit is not left — and the effects' volume is set to
//! `50h`. The handler then calls the end's sequence `0D84h:0008h`
//! (`0x10942`) and returns with the location word at `FFFFh`, which ends
//! the main loop: `_main` writes the settings and exits (`0x007AD`,
//! `0x008C3`). The sequence is [`super::ending`], played as the build's
//! ending after the main loop.

use super::logic::{Logic, WORDS};
use crate::titles::parsec1994::{Api, Far};

/// The four faces: `push 0F2h`, `0EAh`, `0EEh`, `0E6h` to `06E3h:047Eh`
/// (`0x107F7`–`0x10827`), their records kept at `DS:2D72h` on, a far
/// pointer each.
const FACES: [u16; 4] = [0xF2, 0xEA, 0xEE, 0xE6];
const FACE_RECORDS: u16 = 0x2D72;
/// `[2D70h]`: the frame the faces were last set to, `FFFFh` none.
const LAST_FRAME: u16 = 0x2D70;
/// `DS:182Ch`: where each entry's scene leads.
const ONWARD: u16 = 0x182C;
/// The mode machine's mode and wait bits, and its gate (`0x1088C`–`0x1089E`).
const MODE: u16 = 0x2D3A;
const BITS: u16 = 0x2D46;
const GATE_OFF: u16 = 0x2D66;
const GATE_SEG: u16 = 0x2D68;
/// The gate's far address, this handler's segment.
pub(super) const SEGMENT: u16 = 0x107A;
const GATE: u16 = 0x0008;

/// Local 0: the entry the Kaffee was entered by (`[bp-2]`, `0x107E4`).
const ARRIVAL: usize = 0;
/// Local 1: [`END_WAIT`] while the end's wait runs (`0x10923`–`0x1092E`),
/// the loop after the frame loop.
const PHASE: usize = 1;
const END_WAIT: u16 = 1;
/// The location the row names for the game's end (`0x108D2`: `cmp word
/// [200Ch], 0FFFFh`).
const END: u16 = 0xFFFF;
/// `[1CF6h]`: the hundredths.
const HUNDREDTHS: u16 = 0x1CF6;
/// `0x10928`: `cmp word [1CF6h], 0C8h ; jc` — the end's wait.
const END_HOLD: u16 = 0xC8;
/// `0x10914`: `push 7D0h`, the music's fade in milliseconds.
const END_FADE: i16 = 0x07D0;
/// `0x1093A`: `push 50h`, the effects' volume after the end's wait.
const END_VOLUME: u16 = 0x50;
/// `0x1090C`: `push 1`, the band's owner whose redraw is held.
const END_BAND_OWNER: u16 = 1;

fn face_record(api: &dyn Api, i: u16) -> Far {
    let at = FACE_RECORDS.wrapping_add(i * 4);
    Far {
        off: api.word(at),
        seg: api.word(at.wrapping_add(2)),
    }
}

/// Everything before the frame loop, `0x107C0` to `0x108A9`.
pub(crate) fn enter(api: &mut dyn Api) {
    // 0x107C5–0x107D9: an entry past 0 to 8 ends the program.
    let entry = api.word(WORDS.entry);
    if crate::num::signed(entry) < 0 || entry >= 9 {
        api.set_word(WORDS.location, 0xFFFE); // 0x107D3
        api.set_local(ARRIVAL, 0xFFFF);
        return;
    }
    api.visit_setup(); // 0x107DC
    api.set_local(ARRIVAL, entry); // 0x107E4
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x107EB
    api.visit_enter(loc, entry); // 0x107EF
    for (i, &id) in (0u16..).zip(FACES.iter()) {
        let rec = api.obj_find(id); // 0x107FA…
        let at = FACE_RECORDS.wrapping_add(i * 4);
        api.set_word(at, rec.off);
        api.set_word(at.wrapping_add(2), rec.seg);
    }
    // 0x10837–0x1086C: each face on frame `i xor 2`, and subject slot `i`
    // a point at (A0h, 50h).
    for i in (0..4u16).rev() {
        let rec = face_record(api, i);
        api.obj_anim(rec, crate::num::signed(i ^ 2)); // 0x1084F
        api.slot_set_raw(i, 0, 0xA0, 0x50); // 0x1085F
    }
    api.input_lock(); // 0x1086E
    api.scene_play(crate::num::signed(entry)); // 0x10877
    api.hover_off(); // 0x1087D
    api.visit_finish_entry(); // 0x10882
    api.input_lock(); // 0x10887
    api.set_word(MODE, 7); // 0x1088C
    api.set_word(BITS, 0x13); // 0x10892
    api.set_word(GATE_SEG, SEGMENT); // 0x10898
    api.set_word(GATE_OFF, GATE); // 0x1089E
    api.set_word(WORDS.entry, 0); // 0x108A4
}

/// One turn of the frame loop, `0x108AA` to `0x108BE`: until the gate lets
/// go. Then the next location from the row (`0x108C0`–`0x108CC`); for the
/// game's end its first steps and the end's wait, a frame pass a turn.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    if api.local(ARRIVAL) == 0xFFFF {
        return false;
    }
    if api.local(PHASE) == END_WAIT {
        // 0x10923–0x1092E: the frame pass alone until C8h hundredths have
        // passed, unsigned (`jc`).
        api.frame_pass(); // 0x10923
        return api.word(HUNDREDTHS) < END_HOLD; // 0x10928
    }
    api.input_poll(); // 0x108AA
    api.dispatch(); // 0x108AF
    api.frame_pass(); // 0x108B4
    if api.word(WORDS.entry) == 0 {
        return true; // 0x108B9
    }
    let arrival = api.local(ARRIVAL);
    let next = api.word(ONWARD.wrapping_add(arrival * 2));
    api.set_word(WORDS.location, next); // 0x108C9
    api.set_word(WORDS.here, 1); // 0x108CC
    if next != END {
        return false; // 0x108D7
    }
    end_begin(api);
    api.set_local(PHASE, END_WAIT);
    true
}

/// The game's end before its wait, `0x108E5` to `0x1091D`: each face on
/// frame `i xor 2` again, as the entry set them; the band's redraw held
/// for owner 1; the music faded over 2 000 ms; the hundredths set to 0.
fn end_begin(api: &mut dyn Api) {
    for i in (0..4u16).rev() {
        let rec = face_record(api, i);
        api.obj_anim(rec, crate::num::signed(i ^ 2)); // 0x108FD
    }
    api.band_redraw_hold(END_BAND_OWNER); // 0x1090E, 0751h:0635h
    api.music_fade(END_FADE); // 0x10917, 0104h:0798h
    api.set_word(HUNDREDTHS, 0); // 0x1091D
}

/// After the frame loop: the visit left and its slot released
/// (`0x108D9`, `0x108DE`); for the game's end, after its wait, the music
/// stopped, the slot released without the visit's leave, and the
/// effects' volume set (`0x10930`–`0x10941`).
pub(crate) fn leave(api: &mut dyn Api) {
    if api.local(ARRIVAL) == 0xFFFF {
        return;
    }
    if api.word(WORDS.location) == END {
        api.music_stop(); // 0x10930, 0104h:07E9h
        api.slot_unload(); // 0x10935
        api.effect_volume(END_VOLUME); // 0x1093C, 1975h:044Ah; answer not read
        // 0x10942: `0D84h:0008h`, the end's sequence, which the build
        // plays as its ending once the main loop is over (`super::ending`).
        return;
    }
    api.visit_leave(); // 0x108D9
    api.slot_unload(); // 0x108DE
}

/// The hook, `107Ah:01AAh`: code 0 forgets the faces' frame (`0x10960`);
/// code 9, a speaker's change to `arg`, arms the task (`080Ch:193Fh`,
/// `0x10971`, Dunkle Schatten's `0CFDh:1A2Eh`) and sets every face on frame
/// `arg` (`0x1097C`–`0x10997`), once for each new frame.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, task: u16) {
    match code {
        0 => api.set_word(LAST_FRAME, 0xFFFF),
        9 if arg != api.word(LAST_FRAME) => {
            api.task_arm(task);
            for i in (0..4u16).rev() {
                let rec = face_record(api, i);
                api.obj_anim(rec, crate::num::signed(arg));
            }
            api.set_word(LAST_FRAME, arg); // 0x10999
        }
        _ => {}
    }
}

/// The gate, `107Ah:0008h`: asked with 0 it lets go, the entry `7Fh`
/// (`0x107B5`), and answers 1; asked with anything else it answers 0.
pub(crate) fn gate(api: &mut dyn Api, off: u16, arg: i16) -> Option<i16> {
    if off != GATE {
        return None;
    }
    Some(if arg == 0 {
        api.set_word(WORDS.entry, 0x7F);
        1
    } else {
        0
    })
}

/// The Kaffee has no rules of its own.
pub(crate) fn rule(_api: &mut dyn Api, _off: u16, _flags: u16) -> Option<i16> {
    None
}

pub(super) const LOGIC: Logic = Logic {
    enter,
    frame,
    leave,
    hook,
    anim_hook: None,
    rule,
};
