//! Location 22, the Maske, a row of kind 1 with one exit: door `6Fh` to the
//! Flur 2. Stock (location 31, the row's `+9h` and `+19h`). The module is
//! named after the location as the game itself names it in its location
//! table (`Maske`, the record's `+37h`, `250Ah:0167h`).
//!
//! Re-expressed from the handler at `156Bh:02A5h` (image `0x15955` to its
//! `retf` at `0x15A1E`), its hook `156Bh:0379h` (`0x15A29` to its `retf`
//! at `0x15A55`, the record's `+3Fh`) and its thirteen rule handlers
//! `156Bh:000Ah` to `0289h` (`0x156BA`–`0x15954`), against [`Api`] alone.
//! The row's `+43h` is null, so the animation hook is the library's empty
//! one. The location's rules are the fifteen records at `2446h:0760h`
//! (image `0x24BC0`, the record's `+3Bh`), fourteen bytes each; a class is
//! stored complemented (`FFFCh` class 3, `FFFBh` class 4):
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 1 | 4 | `95h` | — | `000Ah` |
//! | 6 | 4 | `95h` | — | `0052h` |
//! | 7 | 4 | `95h` | — | `00CAh` |
//! | 5 | `20h` | `0Dh` | `95h` | `011Bh` |
//! | 5 | `20h` | class 3 | `95h` | `013Bh` |
//! | 5 | `20h` | class 4 | `95h` | `0165h` |
//! | 5 | `20h` | `19h` | `17h` | `026Fh` |
//! | 5 | `20h` | `0Dh`, class 3, class 4 | 0 | `0254h` |
//! | 4 | 4 | `95h` | — | `0289h` |
//! | 3 | 4 | `8Dh` | — | `018Ch` |
//! | 1 | 4 | `43h` | — | `01D4h` |
//! | 1 | 4 | `44h` | — | `01ACh` |
//! | 2 | 4 | `17h` | — | `0202h` |
//!
//! The verbs are, by the verb text row at `DS:116Ah`: 1 `Betrachte`, 2
//! `Nimm`, 3 `Sprich mit`, 4 `Benutze`, 5 `Benutze … mit`, 6 `Öffne`, 7
//! `Schließe`. The items the logic names are, by the item row at
//! `DS:10AAh`: 4 `Schlüssel`, 9 `Kaugummi`, `0Dh` `Haarnadel`, `17h`
//! `Theaterkostüm`, `19h` `Tätowierung`. Object `17h` is the id of item
//! `17h`.
//!
//! Object `95h` is opened and closed by the location's state, `[2BBFh]`
//! in the logic's saved state block (`DS:2B98h`, `4Eh` bytes): 0 locked, 1
//! closed with object `17h` inside, 2 open with `17h` shown, 3 open with
//! `17h` taken, 4 closed without `17h`. The `Haarnadel` on `95h` moves
//! 0 to 1 (rule `011Bh`), `Öffne` 1 to 2 and 4 to 3, `Schließe` 2 to 1 and
//! 3 to 4, `Nimm` on `17h` gives item `17h` and moves to 3. States 0 and
//! 1 show `95h` on frame 0, state 2 hides it, 3 frame 1, 4 frame 2. Until
//! `[2BC6h]` is set, which Schlönstadt's Büro (location 24) does together
//! with `[2BBFh]` 0 (`0x15FD8`, `0x15FDD`), every entry puts the state at 4.
//! The Flur's rules read `[2BBFh]` too (`0x15048`).
//!
//! Object `8Dh` stands while `[2BA8h]` is 2, which Bi-Fi (location 12)
//! writes (`0x13D2E`); the hook's event `19h` gives the `Schlüssel` and
//! moves `[2BA8h]` on to 3, which Jones 1 (location 8) reads (`0x125A2`).
//! The other saved bytes the logic touches: `[2BD5h]`, `[2BDBh]` and
//! `[2BC4h]`, the counters of three scene sequences; `[2BD4h]`, set once
//! the `Kaugummi` is given; and `[2BA6h]` and `[2BA4h]`, read alone here
//! (`[2BA6h]` written by Rambowsky, location 7, and the Kantine, location
//! 23; `[2BA4h]` Rambowsky's sequence counter, `0x11FD4`).

use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x156B;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x15A07`).
const OBJECT_STATES: u16 = 0x2018;

/// Object `8Dh`, shown while `[2BA8h]` is 2 (`0x15977`).
const OBJECT_8D: u16 = 0x8D;
/// Object `95h`, by the location's state on one of its frames.
const OBJECT_95: u16 = 0x95;
/// Object `17h`, the id of item `17h` (`Theaterkostüm`).
const OBJECT_17: u16 = 0x17;

/// The items the logic moves (the item row at `DS:10AAh`).
const SCHLUESSEL: u16 = 0x04;
const KAUGUMMI: u16 = 0x09;
const THEATERKOSTUEM: u16 = 0x17;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BBFh]`: the location's state, 0 to 4.
const STATE: u16 = 0x2BBF;
/// `[2BC6h]`: set by Schlönstadt's Büro (`0x15FD8`).
const BYTE_2BC6: u16 = 0x2BC6;
/// `[2BA8h]`: carried on from location to location; 2 here shows `8Dh`.
const BYTE_2BA8: u16 = 0x2BA8;
/// `[2BA6h]`, `[2BA4h]`: read by rule `01D4h` alone.
const BYTE_2BA6: u16 = 0x2BA6;
const BYTE_2BA4: u16 = 0x2BA4;
/// `[2BD4h]`: the `Kaugummi` given (rule `01ACh`).
const BYTE_2BD4: u16 = 0x2BD4;
/// `[2BD5h]`: rule `0052h`'s sequence counter.
const BYTE_2BD5: u16 = 0x2BD5;
/// `[2BDBh]`: rule `013Bh`'s sequence counter.
const BYTE_2BDB: u16 = 0x2BDB;
/// `[2BC4h]`: rule `018Ch`'s sequence counter.
const BYTE_2BC4: u16 = 0x2BC4;

/// `[2F20h]`/`[2F22h]`: object `8Dh`'s record as `0F48h:0784h` answers it
/// (`0x15981`, `0x1597D`); written in segment `156Bh` alone and read
/// nowhere.
const REC_8D: u16 = 0x2F20;

/// The mode block's verb and verb as chosen.
const VERB: u16 = 0x2D38;
const VERB_CHOSEN: u16 = 0x2D48;

fn keep(api: &mut dyn Api, at: u16, r: Far) {
    api.set_word(at.wrapping_add(2), r.seg);
    api.set_word(at, r.off);
}

/// `push hi ; push lo ; call far 203Dh:0024h ; push ax ; call far
/// 080Ch:164Ch`: the roll is the scene.
fn roll(api: &mut dyn Api, lo: u16, hi: u16) {
    let n = api.random(lo, hi);
    api.scene_play(num::signed(n));
}

/// Everything before the frame loop, `0x15955` to `0x159EA`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x15955
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x15961
    api.visit_enter(loc, entry); // 0x15965
    if api.byte(BYTE_2BA8) == 2 {
        // 0x1596D
        let r = api.obj_show(OBJECT_8D); // 0x15977
        keep(api, REC_8D, r); // 0x1597D, 0x15981
        api.obj_anim(r, 0); // 0x15988
    } else {
        api.obj_hide(OBJECT_8D); // 0x15995
    }
    if api.byte(BYTE_2BC6) == 0 {
        // 0x1599B
        api.set_byte(STATE, 4); // 0x159A2
        api.obj_show_frame(OBJECT_95, 2); // 0x159AC
    }
    // 0x159B4: object 95h's frame by the state through the table at
    // cs:036Fh (0x15A1F); states above 4 skip to 0x159E0.
    let frame = match api.byte(STATE) {
        0 | 1 => Some(0), // 0x159CF
        2 => Some(-1),    // 0x159C7
        3 => Some(1),     // 0x159CB
        4 => Some(2),     // 0x159D3
        _ => None,
    };
    if let Some(k) = frame {
        api.obj_show_frame(OBJECT_95, k); // 0x159D8
    }
    api.visit_finish_entry(); // 0x159E0
    api.set_word(WORDS.entry, 0); // 0x159E5
}

/// One turn of the frame loop, `0x159EB` to `0x159FF`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x159EB
    api.dispatch(); // 0x159F0
    api.frame_pass(); // 0x159F5
    api.word(WORDS.entry) == 0 // 0x159FA
}

/// Everything after the frame loop, `0x15A01` to `0x15A1E`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x15A0C
    api.visit_leave(); // 0x15A14
    api.slot_unload(); // 0x15A19
}

/// The hook, `156Bh:0379h`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read. Only code 4 has an arm (`0x15A32`); code 7 and every
/// other code go to the `retf` (`0x15A3A`, `0x15A3C`).
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    // 0x15A3E: the script's item event 19h.
    if code == 4 && arg == 0x19 {
        api.set_byte(BYTE_2BA8, 3); // 0x15A45
        api.inv_give(SCHLUESSEL); // 0x15A4C
    }
}

/// `156Bh:0202h` (`0x158B2`), `Nimm` object `17h`: carried already, the
/// search goes on; else item `17h` given, state 3, object `17h` hidden,
/// `95h` on frame 1 and samples 0Ah, 0Bh, 0Ch.
fn rule_0202(api: &mut dyn Api) -> i16 {
    if api.inv_has(THEATERKOSTUEM) != 0 {
        // 0x158B7, 0x158BD
        return 0; // 0x158C1
    }
    api.inv_give(THEATERKOSTUEM); // 0x158C7
    api.set_byte(STATE, 3); // 0x158CD
    api.obj_hide(OBJECT_17); // 0x158D4
    api.obj_show_frame(OBJECT_95, 1); // 0x158DF
    api.play_sample(0x0A); // 0x158E9
    api.play_sample(0x0B); // 0x158F1
    api.play_sample(0x0C); // 0x158F9
    1 // 0x158FF
}

/// `156Bh:0052h` (`0x15702`), `Öffne` object `95h`: locked, one of scenes
/// 5 to 8 in turn; in state 1 object `17h` shown, `95h` hidden, state 2;
/// in state 4 state 3 and `95h` on frame 1; both with sample 3. Open
/// already, scene `!0x4C`.
fn rule_0052(api: &mut dyn Api) -> i16 {
    match api.byte(STATE) {
        0 => {
            let counter = api.ds(BYTE_2BD5);
            api.scene_sequence(counter, &[5, 6, 7, 8]); // 0x15728
            0x0A // 0x15775
        }
        1 => {
            api.obj_show(OBJECT_17); // 0x15734, the record not kept
            api.obj_show_frame(OBJECT_95, -1); // 0x1573F
            api.set_byte(STATE, 2); // 0x15747
            api.play_sample(3); // 0x1574E
            1 // 0x15754
        }
        4 => {
            api.set_byte(STATE, 3); // 0x15759
            api.obj_show_frame(OBJECT_95, 1); // 0x15763
            api.play_sample(3); // 0x1576B, 0x1574E
            1 // 0x15754
        }
        _ => {
            api.scene_play(!0x4C); // 0x1576F
            0x0A // 0x15775
        }
    }
}

/// `156Bh:00CAh` (`0x1577A`), `Schließe` object `95h`: in state 2 state 1,
/// object `17h` hidden, `95h` on frame 0; in state 3 state 4 and `95h` on
/// frame 2; both with sample 4. Else scene `!0x49`.
fn rule_00ca(api: &mut dyn Api) -> i16 {
    match api.byte(STATE) {
        2 => {
            api.set_byte(STATE, 1); // 0x1578E
            api.obj_hide(OBJECT_17); // 0x15795
            api.obj_show_frame(OBJECT_95, 0); // 0x157A0
            api.play_sample(4); // 0x157AA
            1 // 0x157B0
        }
        3 => {
            api.set_byte(STATE, 4); // 0x157B5
            api.obj_show_frame(OBJECT_95, 2); // 0x157BC, 0x157A0
            api.play_sample(4); // 0x157AA
            1 // 0x157B0
        }
        _ => {
            api.scene_play(!0x49); // 0x157C0
            0x0A // 0x157C6
        }
    }
}

/// The Maske's rule handlers by their offset in segment `156Bh`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 156B:000A (0x156BA), Betrachte object 95h: by the state through
        // the table at cs:0048h (0x156F8) scenes 0, 1, 2, 3; in state 4
        // scene 3 while [2BC6h] is 0, else 4; above 4 none.
        0x000A => {
            let scene = match api.byte(STATE) {
                0 => Some(0),                             // 0x156D0
                1 => Some(1),                             // 0x156DA
                2 => Some(2),                             // 0x156DE
                3 => Some(3),                             // 0x156E2, 0x156EB
                4 if api.byte(BYTE_2BC6) == 0 => Some(3), // 0x156E4, 0x156EB
                4 => Some(4),                             // 0x156EF
                _ => None,                                // 0x156C7
            };
            if let Some(n) = scene {
                api.scene_play(n); // 0x156D2
            }
            0x0A // 0x156F3
        }
        0x0052 => rule_0052(api),
        0x00CA => rule_00ca(api),
        // 156B:011B (0x157CB), the Haarnadel on object 95h: locked, state 1
        // and scene 9; else scene 0Ah.
        0x011B => {
            if api.byte(STATE) == 0 {
                // 0x157CE
                api.set_byte(STATE, 1); // 0x157D5
                api.scene_play(9); // 0x157E0
            } else {
                api.scene_play(0x0A); // 0x157E0
            }
            0x0A // 0x157E6
        }
        // 156B:013B (0x157EB), class 3 on object 95h: locked, one of scenes
        // 0Bh to 0Eh in turn; else answer 1.
        0x013B => {
            if api.byte(STATE) == 0 {
                // 0x157EE
                let counter = api.ds(BYTE_2BDB);
                api.scene_sequence(counter, &[0x0B, 0x0C, 0x0D, 0x0E]); // 0x15803
                0x0A // 0x1580B
            } else {
                1 // 0x15810
            }
        }
        // 156B:0165 (0x15815), class 4 on object 95h: locked, one of scenes
        // 0Fh to 12h; else answer 1.
        0x0165 => {
            if api.byte(STATE) == 0 {
                // 0x15818
                roll(api, 0x0F, 0x12); // 0x15823, 0x1582C
                0x0A // 0x15832
            } else {
                1 // 0x15837
            }
        }
        // 156B:018C (0x1583C), Sprich mit object 8Dh: one of scenes 14h to
        // 18h in turn.
        0x018C => {
            let counter = api.ds(BYTE_2BC4);
            api.scene_sequence(counter, &[0x14, 0x15, 0x16, 0x17, 0x18]); // 0x1584F
            0x0A // 0x15857
        }
        // 156B:01AC (0x1585C), Betrachte object 44h: once, the Kaugummi
        // given and scene 19h; after that the search goes on.
        0x01AC => {
            if api.byte(BYTE_2BD4) == 0 {
                // 0x1585F
                api.set_byte(BYTE_2BD4, 1); // 0x15866
                api.inv_give(KAUGUMMI); // 0x1586D
                api.scene_play(0x19); // 0x15875
                0x0A // 0x1587B
            } else {
                0 // 0x15880
            }
        }
        // 156B:01D4 (0x15884), Betrachte object 43h: with [2BA6h] 3,
        // [2BA4h] set and the Theaterkostüm carried, scene 1Ah; else the
        // search goes on.
        0x01D4 => {
            // 0x15887, 0x1588E, then 0x15897 and 0x1589D.
            if api.byte(BYTE_2BA6) == 3
                && api.byte(BYTE_2BA4) != 0
                && api.inv_has(THEATERKOSTUEM) != 0
            {
                api.scene_play(0x1A); // 0x158A3
                0x0A // 0x158A9
            } else {
                0 // 0x158AE
            }
        }
        0x0202 => rule_0202(api),
        // 156B:0254 (0x15904), the Haarnadel, class 3 or class 4 on
        // nothing: one of scenes !0x24 to !0x22.
        0x0254 => {
            roll(api, !0x24, !0x22); // 0x1590B, 0x15914
            0x0A // 0x1591A
        }
        // 156B:026F (0x1591F), the Tätowierung with 17h: not carrying
        // the Theaterkostüm, rule 0202h, its answer dropped (`push 0 ;
        // push cs ; call 0202h`, 0x1592E–0x15931); the search goes on
        // either way.
        0x026F => {
            if api.inv_has(THEATERKOSTUEM) == 0 {
                // 0x15924, 0x1592A
                rule_0202(api); // 0x15931
            }
            0 // 0x15935
        }
        // 156B:0289 (0x15939), Benutze object 95h: locked, the verb becomes
        // Benutze … mit; else the search goes on.
        0x0289 => {
            if api.byte(STATE) == 0 {
                // 0x1593C
                api.set_word(VERB_CHOSEN, 5); // 0x15946
                api.set_word(VERB, 5); // 0x15949
                6 // 0x1594C
            } else {
                0 // 0x15951
            }
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
