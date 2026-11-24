//! Location 2, the Prof. The module is named after the location as the
//! game itself names it in its location table (`Prof`, the record's
//! `+1Fh` name).
//!
//! Re-expressed from the handler at `1156:00A8` (image `0x11608`), its
//! hook at `1156:02E3` (`0x11843`), its animation hook at `1156:0284`
//! (`0x117E4`) and its seven rule handlers at `1156:0004`, `0050`, `005E`,
//! `006C`, `007A`, `0088` and `0098`, against [`Api`] alone. Every branch carries the address it was read from.

use super::logic::Logic;
use super::logic::{ENTRY, HERE, LOCATION, OBJECT, OBJECT_STATES, WALKER_28};
use crate::titles::parsec1994::{Api, Far};

/// Object 201, and node 201.
const OBJECT_201: u16 = 0xC9;
/// Object 203, the head of the run 203 to 212, and node 203.
const OBJECT_203: u16 = 0xCB;
/// One past the last object of that run (`mov si, 0D5h`, `0x1165E`).
const RUN_END: u16 = 0xD5;
/// Node 109, the exit the frame loop walks to.
const NODE_109: u16 = 0x6D;

/// `[1CEAh]`: the opening's step; the Wohnung sets it to 1.
const OPENING: u16 = 0x1CEA;
/// `[1CECh]`: 1 asks for scene 2 on entry.
const BYTE_1CEC: u16 = 0x1CEC;
/// `[1CEDh]`: object 200's scene has run.
const SEEN_200: u16 = 0x1CED;
/// `[1CEEh]`: how many times an item was offered to object 201.
const OFFERS: u16 = 0x1CEE;
/// `[16F4h]`: the clock the ambient samples wait on.
const CLOCK: u16 = 0x16F4;
/// `[1E94h]`: the animation object 201 is asked to play.
const ANIM_WANTED: u16 = 0x1E94;
/// `[1E96h]`: the animation object 201 plays.
const ANIM_SHOWN: u16 = 0x1E96;
/// `[1E98h]`: bit 1 of the hook's code-0 argument.
const WORD_1E98: u16 = 0x1E98;
/// `[1E9Ah]`: the wait until the next ambient sample, or 0 for none.
const AMBIENT_WAIT: u16 = 0x1E9A;
/// `[1E9Ch]`: the step of the location's own sequence, 0 to 6.
const STEP: u16 = 0x1E9C;
/// `[1E9Eh]`: the item last offered.
const OFFERED: u16 = 0x1E9E;

/// `rec + n`.
fn at(rec: Far, n: u16) -> Far {
    Far {
        off: rec.off.wrapping_add(n),
        seg: rec.seg,
    }
}

/// Everything before the frame loop, `0x11608` to `0x116F5`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x1160D
    let entry = api.word(ENTRY); // 0x11612
    let loc = api.word(LOCATION); // 0x11616
    api.set_word(HERE, loc); // 0x11619
    api.visit_enter(loc, entry); // 0x1161D
    api.set_word(WORD_1E98, 0); // 0x11625
    api.set_word(ANIM_WANTED, 3); // 0x1162E
    api.set_word(ANIM_SHOWN, 3); // 0x11631
    api.obj_anim_id(OBJECT_201, 3); // 0x11638
    // 0x11640: object 203 set live, and its word +0Ch handed to the nine
    // objects behind it, 212 down to 204, as their +0Ch and +0Ah.
    let head = api.obj_find(OBJECT_203);
    let w = api.far_word(at(head, 0x0C)); // 0x11652
    let flags = api.far_byte(at(head, 2));
    api.set_far_byte(at(head, 2), flags | 3); // 0x11659
    let mut id = RUN_END; // 0x1165E
    loop {
        id -= 1; // 0x11683
        if id <= OBJECT_203 {
            break; // 0x11686
        }
        let rec = api.obj_find(id); // 0x11664
        let flags = api.far_byte(at(rec, 2));
        api.set_far_byte(at(rec, 2), flags | 3); // 0x11673
        api.set_far_word(at(rec, 0x0C), w); // 0x1167B
        api.set_far_word(at(rec, 0x0A), w); // 0x1167F
    }
    if api.byte(OPENING) == 1 {
        // 0x11692: the opening goes on from the Wohnung.
        api.input_lock();
        api.hover_off(); // 0x11697
        api.obj_anim_id(OBJECT_203, 1); // 0x116A1
        api.walk_to_node(OBJECT_201); // 0x116AC
        api.set_byte(OPENING, 2); // 0x116B2
        api.scene_play(1); // 0x116B9
    } else if api.byte(BYTE_1CEC) == 1 {
        api.obj_anim_id(OBJECT_203, 0); // 0x116CD
        api.set_byte(BYTE_1CEC, 2); // 0x116D5
        api.scene_play(2); // 0x116DA
    } else {
        api.obj_show_frame(OBJECT_203, -1); // 0x116E3
    }
    api.visit_finish_entry(); // 0x116EB
    api.set_word(ENTRY, 0); // 0x116F0
}

/// One turn of the frame loop, `0x116F6` to `0x117B7`. Answers whether
/// the loop goes on.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x116F6
    api.dispatch(); // 0x116FB
    match api.word(STEP) {
        // 0x1170F: the walker's word +28h is 201 — the task goes on.
        1 if api.word(WALKER_28) == OBJECT_201 => {
            api.task_start(0); // 0x11719
            api.set_word(STEP, 0);
        }
        // 0x11727: it is 203 — the ambient samples start, object 203
        // plays animation 2 and the player walks to node 109.
        2 if api.word(WALKER_28) == OBJECT_203 => {
            api.set_word(AMBIENT_WAIT, 1); // 0x1172F
            api.obj_anim_id(OBJECT_203, 2); // 0x1173A
            api.walk_to_node(NODE_109); // 0x11744
            api.task_start(0); // 0x1174C
        }
        _ => {}
    }
    // 0x11752: an ambient sample once the clock has passed the wait.
    let wait = api.word(AMBIENT_WAIT);
    if wait != 0 && wait < api.word(CLOCK) {
        api.set_word(CLOCK, 0); // 0x11762
        let next = api.random(0x14, 0x1E); // 0x1176C
        api.set_word(AMBIENT_WAIT, next);
        api.play_sample(0x19); // 0x11779
        // 0x11783: one of four through the table at 0x117DC.
        match api.random(0, 3) {
            0 => api.play_sample(4),    // 0x11799
            1 => api.play_sample(5),    // 0x1179D
            2 => api.play_sample(6),    // 0x117A1
            3 => api.play_sample(0x1F), // 0x117A5
            _ => {}
        }
    }
    api.frame_pass(); // 0x117AD
    api.word(ENTRY) == 0 // 0x117B2
}

/// Everything after the frame loop, `0x117BC` to `0x117DB`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(HERE);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x117C7
    api.visit_leave(); // 0x117CF
    api.slot_unload(); // 0x117D4
}

/// The animation hook, `1156:0284`: codes `64h` and `65h` step the
/// sequence and stop the ambient samples.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        0x64 => {
            // 0x117F6
            if api.word(STEP) == 4 {
                api.task_start(0); // 0x117FF
                api.set_word(STEP, 0);
            } else {
                api.set_word(STEP, 3); // 0x1180D
            }
            api.set_word(AMBIENT_WAIT, 0); // 0x1183B
        }
        0x65 => {
            // 0x11815
            if api.word(STEP) == 6 {
                api.task_start(0); // 0x1181E
                api.walk_to_node(OBJECT_201); // 0x11827
                api.set_word(STEP, 1);
            } else {
                api.set_word(STEP, 5); // 0x11835
            }
            api.set_word(AMBIENT_WAIT, 0); // 0x1183B
        }
        _ => {}
    }
}

/// The hook, `1156:02E3`: ten codes through the table at `0x119C7`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        0 => {
            // 0x1185C
            api.set_word(WORD_1E98, arg & 2);
            api.set_word(ANIM_SHOWN, 0xFFFF);
        }
        5 | 6 => {
            // 0x1186D
            if api.word(WORD_1E98) != 0 {
                api.set_word(ANIM_WANTED, 3);
                api.set_word(ANIM_SHOWN, 3);
                api.obj_anim_id(OBJECT_201, 3); // 0x11884
                api.set_word(WORD_1E98, 0);
            }
        }
        // 0x118C6: script events through the eleven-value table at
        // 0x1199B.
        7 => event(api, arg),
        8 => api.set_word(ANIM_WANTED, arg.wrapping_add(1)), // 0x11895
        9 => {
            // 0x1189E
            if api.word(WORD_1E98) != 0 {
                let wanted = api.word(ANIM_WANTED);
                if wanted != api.word(ANIM_SHOWN) {
                    api.set_word(ANIM_SHOWN, wanted); // 0x118B4
                    api.obj_anim_id(OBJECT_201, wanted.cast_signed()); // 0x118BB
                }
            }
        }
        _ => {} // 1 to 4, and above 9: 0x11999
    }
}

/// The hook's code 7, by the script event.
fn event(api: &mut dyn Api, arg: u16) {
    match arg {
        10 => {
            api.set_word(STEP, 1); // 0x118E3
            api.task_arm(0); // 0x118EB
        }
        11 => {
            api.walk_to_node(OBJECT_203); // 0x118F6
            api.set_word(STEP, 2);
            api.task_arm(0);
        }
        12 => {
            // 0x11904
            if api.word(STEP) == 3 {
                api.set_word(STEP, 0); // 0x11913
            } else {
                api.set_word(STEP, 4); // 0x1190B
                api.task_arm(0);
            }
        }
        13 => api.script_exit(NODE_109), // 0x1191D
        14 => {
            api.obj_anim_id(OBJECT_203, 3); // 0x11929
            api.set_word(AMBIENT_WAIT, 1); // 0x11931
            api.walk_to_node(OBJECT_203); // 0x1193A
        }
        15 => {
            // 0x11941
            if api.word(STEP) == 5 {
                api.walk_to_node(OBJECT_201); // 0x11953
                api.set_word(STEP, 1);
            } else {
                api.set_word(STEP, 6); // 0x11948
                api.task_arm(0);
            }
        }
        16 => api.play_sample(0x16), // 0x11961
        17 => api.play_sample(0x15), // 0x11965
        19 => {
            api.play_sample(7); // 0x11970
            let item = api.word(OFFERED);
            api.inv_take(item); // 0x1197A
        }
        20 => {
            api.play_sample(7); // 0x11983
            api.inv_give(1); // 0x1198B
        }
        110 => api.set_word(ANIM_WANTED, 0), // 0x11993
        _ => {}
    }
}

/// `1156:0028`, the near routine four of the rules share: scene `n`, the
/// chosen object remembered as the item offered, and scene 6 queued from
/// the fourth offer on.
fn offer(api: &mut dyn Api, n: i16) {
    api.scene_play(n); // 0x1158E
    let chosen = api.word(OBJECT); // 0x11594
    api.set_word(OFFERED, chosen);
    let offers = api.byte(OFFERS).wrapping_add(1); // 0x1159D
    api.set_byte(OFFERS, offers);
    if offers >= 4 {
        api.scene_enqueue(6); // 0x115A8
    }
}

/// The Prof's rule handlers by their offset in segment `1156h`. Answers
/// what the original answers the rule search with, or `None` for an
/// offset that is not one of them.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 1156:0004, object 200: scene 3 once; after that the search
        // goes on.
        0x0004 => {
            if api.byte(SEEN_200) != 0 {
                0 // 0x1157F
            } else {
                api.set_byte(SEEN_200, 1); // 0x11570
                api.scene_play(3);
                0x0A
            }
        }
        // 1156:0050 to 007A: items 1 to 4 on object 201.
        0x0050 => {
            offer(api, 7); // 0x115B5
            0x0A
        }
        0x005E => {
            offer(api, 8); // 0x115C3
            0x0A
        }
        0x006C => {
            offer(api, 9); // 0x115D1
            0x0A
        }
        0x007A => {
            offer(api, 0x0A); // 0x115DF
            0x0A
        }
        // 1156:0088: the rule with object 0 and object 201.
        0x0088 => {
            api.scene_play(0x0B); // 0x115ED
            0x0A
        }
        // 1156:0098: verb 2 on object 201.
        0x0098 => {
            api.scene_play(0x0C); // 0x115FD
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
    anim_hook: Some(anim_hook),
    rule,
};
