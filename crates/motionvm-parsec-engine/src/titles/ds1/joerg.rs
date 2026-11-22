//! Location 7, Jörg. The module is named after the location as the game
//! itself names it in its location table (`Jörg`).
//!
//! Re-expressed from the handler at `018B:0061` (image `0x1911`), its hook
//! at `018B:00FE` (`0x19AE`) and its one rule handler at `018B:0009`,
//! against [`Api`] alone.

use super::logic::Logic;
use super::logic::WALKER;
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// Object 129, whose animations follow the dialogue.
const PERSON: u16 = 0x81;
/// Object 200, the one event 11 walks to.
const OBJECT_200: u16 = 0xC8;
/// Entry 11 of `PSMP2L.DAT`, the song event 12 plays.
const SONG: u16 = 0x0B;

/// `[2717h]`: the state the rule switches on.
const STATE: u16 = 0x2717;
/// `[2714h]`: who is on the telephone (see the Wohnzimmer).
const CALLER: u16 = 0x2714;
/// `[18B8h]`, `[18BAh]`: the far pointer to the song event 12 loaded.
const SONG_OFF: u16 = 0x18B8;
const SONG_SEG: u16 = 0x18BA;
/// `[18C0h]` whether object 129 speaks in the dialogue that is up,
/// `[18BCh]` its next animation, `[18BEh]` the one it is playing.
const SPEAKS: u16 = 0x18C0;
const NEXT_ANIM: u16 = 0x18BC;
const PLAYING_ANIM: u16 = 0x18BE;
/// `[0212h]`: the music switch (`0590h`, tested at `0x5FE3`),
/// tested here before the music is faded (`0x1A66`).
const MUSIC_212: u16 = 0x0212;
/// `[285Dh]`, `[285Fh]`: the far pointer to the song the engine keeps
/// (written at `0x7814`), which event 13 fades back in.
const KEPT_SONG_OFF: u16 = 0x285D;
const KEPT_SONG_SEG: u16 = 0x285F;
/// `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`, `DS:1F56h`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
const HERE: u16 = 0x1F4C;
const OBJECT_STATES: u16 = 0x1F56;

/// Everything before the frame loop, `0x1911` to `0x194F`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x1911
    let entry = api.word(ENTRY);
    let loc = api.word(LOCATION);
    api.set_word(HERE, loc); // 0x191D
    api.visit_enter(loc, entry); // 0x1921
    api.set_word(SONG_SEG, 0); // 0x1929
    api.set_word(SONG_OFF, 0); // 0x192F
    if api.byte(CALLER) == 3 {
        api.set_byte(CALLER, 1); // 0x193C
    }
    api.obj_wake(PERSON); // 0x1944
    api.visit_finish_entry(); // 0x194A
    api.set_word(ENTRY, 0); // 0x194F
}

/// One turn of the frame loop, `0x1955` to `0x1969`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll();
    api.dispatch();
    api.frame_pass();
    api.word(ENTRY) == 0
}

/// Everything after the frame loop, `0x196B` to `0x19AD`: the song event
/// 12 loaded is freed after the shared leaving, before the slot.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x1976
    api.visit_leave(); // 0x197E
    // 0x1983: `mov ax,[18B8h] ; or ax,[18BAh]`.
    if api.word(SONG_OFF) | api.word(SONG_SEG) != 0 {
        let song = song(api);
        api.mem_free(song); // 0x1994
        api.set_word(SONG_SEG, 0); // 0x199C
        api.set_word(SONG_OFF, 0); // 0x19A2
    }
    api.slot_unload(); // 0x19A8
}

/// The far pointer in `[18B8h]`/`[18BAh]`.
fn song(api: &dyn Api) -> Far {
    Far {
        off: api.word(SONG_OFF),
        seg: api.word(SONG_SEG),
    }
}

/// The hook, `018B:00FE`: codes 0 to 9 through the table at `0x1ACB`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x19C6: a dialogue comes up; speaker bit 1 is object 129.
            api.set_word(SPEAKS, arg & 2);
            api.set_word(NEXT_ANIM, 0);
            api.set_word(PLAYING_ANIM, 0);
        }
        5 | 6 => {
            // 0x19D9: if it spoke, back to animation 0.
            if api.word(SPEAKS) != 0 {
                api.set_word(NEXT_ANIM, 0); // 0x19E5
                api.set_word(PLAYING_ANIM, 0);
                api.set_word(SPEAKS, 0);
                api.obj_anim_id(PERSON, 0); // 0x19F2
            }
        }
        8 => {
            // 0x19FD
            if api.word(SPEAKS) != 0 {
                api.set_word(NEXT_ANIM, arg.wrapping_add(1));
            }
        }
        9 => {
            // 0x1A10: a new animation to play; [18C0h] is not asked.
            if api.word(PLAYING_ANIM) != api.word(NEXT_ANIM) {
                let next = api.word(NEXT_ANIM);
                api.set_word(PLAYING_ANIM, next);
                api.obj_anim_id(PERSON, num::signed(next)); // 0x1A26
            }
        }
        7 => event(api, arg), // 0x1A31
        _ => {}               // 1 to 4 and above 9: 0x1AC1
    }
}

/// Code 7, `0x1A31`: script events 10 to 13 through the table at `0x1AC3`.
fn event(api: &mut dyn Api, n: u16) {
    match n {
        0x0A => {
            let walker = api.ds(WALKER);
            api.walker_walk_to(walker, PERSON, 3); // 0x1A4E
        }
        0x0B => {
            let walker = api.ds(WALKER);
            api.walker_walk_to(walker, OBJECT_200, 0); // 0x1A5E
            if api.word(MUSIC_212) != 0 {
                api.music_fade(0x07D0); // 0x1A70
            }
        }
        0x0C => {
            // 0x1A78: the song loaded and kept, and played if there is one.
            let song = api.song_load(SONG);
            api.set_word(SONG_SEG, song.seg); // 0x1A80
            api.set_word(SONG_OFF, song.off); // 0x1A84
            if song.off | song.seg != 0 {
                api.music_play(song, -1); // 0x1A94
            }
        }
        0x0D => {
            // 0x1A9E: with a song loaded, it stops and the other fades in.
            if api.word(SONG_OFF) | api.word(SONG_SEG) != 0 {
                api.music_stop(); // 0x1AA7
                let back = Far {
                    off: api.word(KEPT_SONG_OFF),
                    seg: api.word(KEPT_SONG_SEG),
                };
                api.music_play_fade(back, -1, 0x07D0); // 0x1AB9
            }
        }
        _ => {}
    }
}

/// Jörg's rule handler by its offset in segment `018Bh`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    match off {
        // 018B:0009, Sprich mit object 129, by [2717h]; answers 0Ah.
        0x0009 => {
            api.set_byte(0x26FE, 1); // 0x18BC
            if api.byte(STATE) == 3 {
                api.scene_play(0); // 0x18CD
                api.set_byte(0x2720, 0);
                api.set_byte(STATE, 4);
            } else {
                api.scene_play(1); // 0x18E1
                api.set_byte(0x2720, 0);
                api.set_byte(STATE, 6);
                api.set_byte(0x272E, 0);
                api.set_byte(0x272D, 4);
                api.set_byte(0x2700, 1);
                if api.byte(CALLER) == 3 {
                    api.set_byte(CALLER, 1); // 0x1907
                }
            }
            Some(0x0A)
        }
        _ => None,
    }
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
