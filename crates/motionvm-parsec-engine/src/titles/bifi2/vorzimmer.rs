//! Location 5, the Vorzimmer, a row of kind 1: the anteroom between the
//! Flur Erdgeschoß (exit `6Fh`, location 18) and Schlönstadt's Büro (exit
//! `6Eh`, location 24). The module is named after the location as the game
//! itself names it in its location table (`Vorzimmer`, the record's `+37h`,
//! `250Ah:00BBh`).
//!
//! Re-expressed from the handler at `1117h:02A6h` (image `0x11416` to its
//! `retf` at `0x115FE`), its hook `1117h:050Ah` (`0x1167A`, the record's
//! `+3Fh`), its animation hook `1117h:048Fh` (`0x115FF`, the record's
//! `+43h`) and its eighteen rule handlers, against [`Api`] alone. The
//! location's rules are the nineteen records at `2446h:00B0h` (image
//! `0x24510`, the record's `+3Bh`), fourteen bytes each as in the global
//! table:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 1 | 4 | `82h` | — | `017Bh` |
//! | 1 | 4 | `83h` | — | `0212h` |
//! | 1 | 4 | `84h` | — | `01E9h` |
//! | 3 | 4 | `85h` | — | `007Bh` |
//! | 4 | 4 | `83h` | — | `00FDh` |
//! | 5 | `20h` | 1 | `83h` | `0131h` |
//! | 5 | `20h` | 1 | 0 | `028Bh` |
//! | `0Ah` | 4 | `10h` | — | `009Dh` |
//! | 8 | 4 | `83h` | — | `00CDh` |
//! | 4 | 2 | `83h` | — | `00B3h` |
//! | 5 | `20h` | `86h` | 0 | `0029h` |
//! | 5 | `20h` | `0Eh`, `86h` | `86h`, `0Eh` | `0001h` |
//! | 6 | 4 | `84h` | — | `01A5h` |
//! | 7 | 4 | `84h` | — | `01C7h` |
//! | 0 | `80h` | `84h` | — | `024Bh` |
//! | 0 | `80h` | `83h` | — | `025Bh` |
//! | 0 | `80h` | `86h` | — | `026Bh` |
//! | 0 | `80h` | 0 | — | `027Bh` |
//!
//! Object `85h` is there twice: as an object, whose record the handler
//! keeps at `[2DA0h]` (`0x1143B`), and as a record that walks, kept at
//! `[2D9Ch]` (`0x1144B`, `06E3h:04EFh`). Object `83h`'s record is kept at
//! `[2D98h]` (`0x11463`). Until `[2BA1h]` is set, object `85h` stands on
//! animation 0 and the player's graph has nodes `3E8h` to `3EAh` blocked
//! (`0x1146D`–`0x114A9`); the hook's code 4 with argument 8 takes the
//! `kandierten Apfel`, sets `[2BA1h]` and opens them again
//! (`0x116B7`–`0x11707`). When object `85h`'s animation then reports
//! `FFFFh`, the object is hidden and its walking record goes from node
//! `0BB8h` through `0BB9h`, where door `6Fh` opens, to `0BBAh`, where it
//! closes (`0x11611`–`0x11668`, `0x11528`–`0x11598`).
//!
//! Seven bytes of the logic's saved state block (`DS:2B98h`, `4Eh` bytes)
//! hold the location's progress: `[2B9Ch]` (the `Haarnadel` handed out),
//! `[2B9Dh]` (object `83h`, 0 to 3), `[2B9Eh]` (written by Schlönstadt's
//! Büro, `0x15F54`), `[2B9Fh]` (bit 0 object `84h` hidden, bit 1 the
//! `Sicherheitsausweis` given), `[2BA0h]` (the counter of the entry's
//! sequence and rule `007Bh`'s), `[2BA1h]` (read also by location 23,
//! `0x15AA8`) and `[2BA2h]` (read by the global rule `0FE3h:02B1h`,
//! `0x100E4`). The items the logic asks about or moves are, by the item
//! row at `DS:10AAh`: 1 `Infrarotclip`, 6 `Sicherheitsausweis`, `0Bh`
//! `Passfoto`, `0Dh` `Haarnadel`, `0Eh`/`0Fh` `Fotostreifen`, `10h`
//! `kandierten Apfel`.
//!
//! An object's subject slot is `080Ch:1972h(slot, code, rec)` (`0x09A32`),
//! Captain Zins's `0849h:1A4Fh` instruction for instruction
//! ([`Api::slot_set_object`]); a walking record's is `080Ch:19B4h`
//! (`0x09A74`, Captain Zins's `0849h:1A91h`, [`Api::slot_set_figure`]).
//! The record that walks by id is `06E3h:04EFh` (`0x0731F`, Captain Zins's
//! `0701h:0589h`, [`Api::mover_find`]), its graph `05DCh:0DB4h`
//! (`0x06B74`, Captain Zins's `05F2h:0E3Ah`, [`Api::walker_bind_graph`]);
//! a node blocked and opened are `05DCh:0EFDh` and `05DCh:0F57h`
//! (`0x06CBD`, `push 3FFh` at `0x06D0B`; `0x06D17`, `push 0` at `0x06D65`;
//! Dunkle Schatten's `0A9Fh:1002h` and `0A9Fh:105Ch`, [`Api::node_block`],
//! [`Api::node_open`]).

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x1117;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x115E7`).
const OBJECT_STATES: u16 = 0x2018;

/// The objects the location shows, hides or animates.
const OBJECT_83: u16 = 0x83;
const OBJECT_84: u16 = 0x84;
const OBJECT_85: u16 = 0x85;
/// Door `6Fh`, the row's exit to location 18 (`push 6Fh`, `0x11545`).
const DOOR_6F: u16 = 0x6F;

/// `[2D98h]`/`[2D9Ah]`: object `83h`'s record (`0x1145F`, `0x11463`).
const REC_83: u16 = 0x2D98;
/// `[2D9Ch]`/`[2D9Eh]`: the record that walks for id `85h` (`0x11447`,
/// `0x1144B`).
const MOVER_85: u16 = 0x2D9C;
/// `[2DA0h]`/`[2DA2h]`: object `85h`'s record (`0x11437`, `0x1143B`).
const REC_85: u16 = 0x2DA0;

/// The handler's words, read and written in segment `1117h` alone and
/// outside the `38h` bytes from `DS:2D38h` the visit clears (`push 38h ;
/// push 0 ; push ds ; push 2D38h ; call far 206Fh:0000h`, `0x09D7B`–
/// `0x09D83`; `206Fh:0000h` is Captain Zins's `1DA9h:0002h`,
/// [`Api::setmem`]).
/// `[2D94h]`: set by the hook's code 7 (arguments 5 to 7) and code 4
/// (argument 8); while set, codes 1 to 3 leave object `85h` alone.
const HELD_85: u16 = 0x2D94;
/// `[2D96h]`: bit 1 of code 0's argument (`0x117B2`).
const TRACK_85: u16 = 0x2D96;
/// `[2DA4h]`: samples still to come, 6 from the hook's code 7 with `1Fh`
/// (`0x11790`), cleared on entry (`0x11450`).
const SAMPLES_LEFT: u16 = 0x2DA4;
/// `[2DA6h]`: 1 while the walking record goes out by door `6Fh`
/// (`0x11668`), cleared on entry (`0x11453`).
const LEAVING: u16 = 0x2DA6;
/// `[2DA8h]`: set by rule `009Dh`, verb `0Ah` on the `kandierten Apfel`
/// (`0x11210`), and cleared nowhere; it picks object `85h`'s animation in
/// the hook's codes 1 to 3.
const WORD_2DA8: u16 = 0x2DA8;

/// `[1CF6h]`: the clock's hundredths the frame loop waits on (the build's
/// `hundredths`, which [`Api::word`] reads from the clock).
const HUNDREDTHS: u16 = 0x1CF6;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
const BYTE_2B9C: u16 = 0x2B9C;
const BYTE_2B9D: u16 = 0x2B9D;
const BYTE_2B9E: u16 = 0x2B9E;
const BYTE_2B9F: u16 = 0x2B9F;
const BYTE_2BA0: u16 = 0x2BA0;
const BYTE_2BA1: u16 = 0x2BA1;
const BYTE_2BA2: u16 = 0x2BA2;

/// The player's graph nodes blocked until `[2BA1h]` is set.
const BLOCKED_NODES: [u16; 3] = [0x03E8, 0x03E9, 0x03EA];

/// The walking record's nodes on its way out.
const NODE_START: u16 = 0x0BB8;
const NODE_DOOR: u16 = 0x0BB9;
const NODE_OUT: u16 = 0x0BBA;
/// The walking record's node `+28h` (`mov ax, es:[bx+28h]`, `0x11533`).
const MOVER_NODE: u16 = 0x28;

/// `[2D38h]`: the mode block's verb.
const VERB: u16 = 0x2D38;
/// `[2D40h]`, `[2D42h]`: the second object chosen and its class.
const SECOND: u16 = 0x2D40;
const SECOND_CLASS: u16 = 0x2D42;

fn rec(api: &dyn Api, at: u16) -> Far {
    Far {
        off: api.word(at),
        seg: api.word(at.wrapping_add(2)),
    }
}

fn keep(api: &mut dyn Api, at: u16, r: Far) {
    api.set_word(at.wrapping_add(2), r.seg);
    api.set_word(at, r.off);
}

/// `push k ; push id ; call far 06E3h:047Eh ; pop cx ; push dx ; push ax ;
/// call far 03F7h:1CF9h`: the object found and set on animation `k`.
fn anim(api: &mut dyn Api, id: u16, k: i16) {
    let r = api.obj_find(id);
    api.obj_anim(r, k);
}

/// `push [hi] ; push [lo] ; call far 03F7h:1CF9h`: the kept record `at`
/// set on animation `k`.
fn anim_kept(api: &mut dyn Api, at: u16, k: i16) {
    let r = rec(api, at);
    api.obj_anim(r, k);
}

/// Everything before the frame loop, `0x11416` to `0x1151B`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x11416
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x11422
    api.visit_enter(loc, entry); // 0x11426
    let r85 = api.obj_find(OBJECT_85); // 0x11431
    keep(api, REC_85, r85);
    let m85 = api.mover_find(OBJECT_85); // 0x11441, 06E3h:04EFh
    keep(api, MOVER_85, m85);
    api.set_word(SAMPLES_LEFT, 0); // 0x11450
    api.set_word(LEAVING, 0); // 0x11453
    let r83 = api.obj_find(OBJECT_83); // 0x11459
    keep(api, REC_83, r83);
    if api.byte(BYTE_2BA1) == 0 {
        // 0x11466
        anim_kept(api, REC_85, 0); // 0x11477
        for node in BLOCKED_NODES {
            let player = api.ds(WALKER);
            api.node_block(player, node); // 0x11486, 0x11495, 0x114A4
        }
    } else {
        api.obj_hide(OBJECT_85); // 0x114B1
    }
    // 0x114B7: object 83h by [2B9Dh].
    match api.byte(BYTE_2B9D) {
        1 => {
            api.obj_show(OBJECT_83); // 0x114D0
            anim_kept(api, REC_83, 1); // 0x114ED
        }
        2 | 3 => {
            api.obj_show(OBJECT_83); // 0x114DD
            anim_kept(api, REC_83, 3); // 0x114ED
        }
        _ => {}
    }
    api.visit_finish_entry(); // 0x114F5
    api.set_word(WORDS.entry, 0); // 0x114FA
    // 0x11500: until [2BA0h] is set, scenes 8 and 0Ah in turn as an
    // interaction with object 85h.
    if api.byte(BYTE_2BA0) == 0 {
        let counter = api.ds(BYTE_2BA0);
        // 09B6h:164Dh (0x0B1AD), Dunkle Schatten's 0EB9h:1703h.
        api.interaction_sequence(OBJECT_85, -1, counter, &[8, 0x0A]); // 0x11516
    }
}

/// One turn of the frame loop, `0x1151E` to `0x115DE`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x1151E
    api.dispatch(); // 0x11523
    if api.word(LEAVING) != 0 {
        // 0x11528: by the walking record's node.
        let m = rec(api, MOVER_85);
        let node = api.far_word(Far {
            off: m.off.wrapping_add(MOVER_NODE),
            seg: m.seg,
        }); // 0x11533
        match node {
            // 0x11543: at the door, the door opens and the record walks on.
            NODE_DOOR => {
                anim(api, DOOR_6F, 1); // 0x11547, 0x1154F
                let m = rec(api, MOVER_85);
                api.walker_walk_to(m, NODE_OUT, 1); // 0x11564, answer not read
                let player = api.ds(WALKER);
                api.figure_face(player, 1); // 0x11572
                api.page_release(0); // 0x1157C
            }
            // 0x11584: out, the door closes.
            NODE_OUT => {
                anim(api, DOOR_6F, 0); // 0x11588, 0x11590
                api.set_word(LEAVING, 0); // 0x11598
            }
            _ => {}
        }
    }
    // 0x1159E: a sample from 5 to 7 once more than ten hundredths have
    // passed (`jna`, unsigned), the count restarted at 0 to 5.
    if api.word(SAMPLES_LEFT) != 0 && api.word(HUNDREDTHS) > 0x0A {
        let left = api.word(SAMPLES_LEFT).wrapping_sub(1);
        api.set_word(SAMPLES_LEFT, left); // 0x115AC
        let start = api.random(0, 5); // 0x115B4
        api.set_word(HUNDREDTHS, start); // 0x115BC
        let k = api.random(5, 7); // 0x115C3
        api.play_sample(k); // 0x115CC
    }
    api.frame_pass(); // 0x115D2
    api.word(WORDS.entry) == 0 // 0x115D7
}

/// Everything after the frame loop, `0x115E1` to `0x115FE`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x115EC
    api.visit_leave(); // 0x115F4
    api.slot_unload(); // 0x115F9
}

/// The hook, `1117h:050Ah`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read. Codes above 7 do nothing (`0x11684`); the others go
/// through the table at `cs:06F5h` (`0x1168E`): 0 `0x117B0`, 1 and 3
/// `0x117E9`, 2 `0x11818`, 4 `0x11693`, 5 and 6 `0x117BB`, 7 `0x11721`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match code {
        // 0x117B0: bit 1 of the argument kept.
        0 => {
            api.set_word(TRACK_85, arg & 2); // 0x117B5
            api.set_word(HELD_85, 0); // 0x11845
        }
        // 0x117E9: object 85h on animation 5 or 2 while tracked and not
        // held; held cleared on every path (0x11845).
        1 | 3 => {
            if api.word(TRACK_85) != 0 && api.word(HELD_85) == 0 {
                let k = if api.word(WORD_2DA8) != 0 { 5 } else { 2 }; // 0x117F7
                anim_kept(api, REC_85, k); // 0x1180C
            }
            api.set_word(HELD_85, 0); // 0x11845
        }
        // 0x11818: the same with animations 4 and 1; the animating path
        // leaves by 0x11843 without clearing held, which is 0 there.
        2 => {
            if api.word(TRACK_85) != 0 && api.word(HELD_85) == 0 {
                let k = if api.word(WORD_2DA8) != 0 { 4 } else { 1 }; // 0x11826
                anim_kept(api, REC_85, k); // 0x1183B
            } else {
                api.set_word(HELD_85, 0); // 0x11845
            }
        }
        4 => hook_items(api, arg),
        // 0x117BB.
        5 | 6 => {
            if api.word(TRACK_85) != 0 {
                api.set_word(HELD_85, 0); // 0x117C2
                if api.byte(BYTE_2BA1) == 0 {
                    anim_kept(api, REC_85, 0); // 0x117D9
                }
            }
            api.set_word(TRACK_85, 0); // 0x117E1
        }
        7 => hook_events(api, arg),
        _ => {}
    }
}

/// Code 4, `0x11693`: by the argument (`cmp ax, 7/8/9`, `0x11695`–
/// `0x116A2`).
fn hook_items(api: &mut dyn Api, arg: u16) {
    match arg {
        // 0x116A7: Sicherheitsausweis given, [2B9Fh] bit 1.
        7 => {
            api.inv_give(6); // 0x116A9
            let b = api.byte(BYTE_2B9F) | 2;
            api.set_byte(BYTE_2B9F, b); // 0x116AF
        }
        // 0x116B7: kandierten Apfel taken, [2BA1h] set, object 85h on
        // animation 8, held, the player's nodes open again.
        8 => {
            api.inv_take(0x10); // 0x116B9
            api.set_byte(BYTE_2BA1, 1); // 0x116BF
            anim_kept(api, REC_85, 8); // 0x116CE
            api.set_word(HELD_85, 1); // 0x116D6
            api.set_word(TRACK_85, 0); // 0x116DC
            for node in BLOCKED_NODES {
                let player = api.ds(WALKER);
                api.node_open(player, node); // 0x116E9, 0x116F8, 0x11707
            }
        }
        // 0x1170F: [2B9Dh] 2, object 83h on animation 3.
        9 => {
            api.set_byte(BYTE_2B9D, 2); // 0x1170F
            anim_kept(api, REC_83, 3); // 0x1183B
        }
        _ => {}
    }
}

/// Code 7, `0x11721`: the argument looked up in the row at `cs:06DDh`
/// (5 to 9, `1Fh`), its arm in the row after it (`cs:06E9h`).
fn hook_events(api: &mut dyn Api, arg: u16) {
    match arg {
        // 0x1173E, 0x11752: object 85h on animation 6 or 9, held.
        5 | 6 => {
            let k = if arg == 5 { 6 } else { 9 };
            anim_kept(api, REC_85, k); // 0x11748
            api.set_word(HELD_85, 1); // 0x1176E
        }
        // 0x11756: object 85h on animation 0Ah, untracked, held.
        7 => {
            anim_kept(api, REC_85, 0x0A); // 0x11760
            api.set_word(TRACK_85, 0); // 0x11768
            api.set_word(HELD_85, 1); // 0x1176E
        }
        // 0x11777: the player walks to node 85h, facing 3.
        8 => {
            let player = api.ds(WALKER);
            api.walker_walk_to(player, OBJECT_85, 3); // 0x11780, answer not read
        }
        // 0x1178B: object 85h on animation 0Bh.
        9 => anim_kept(api, REC_85, 0x0B), // 0x1183B
        // 0x11790: six samples to come, the first with sample 5 now.
        0x1F => {
            api.set_word(SAMPLES_LEFT, 6); // 0x11790
            api.play_sample(5); // 0x11798
            let start = api.random(0, 5); // 0x117A2
            api.set_word(HUNDREDTHS, start); // 0x117AA
        }
        _ => {}
    }
}

/// The animation hook, `1117h:048Fh`: the code in `[bp+0Ah]`; the record
/// is not read. Code `FFFFh` sends object `85h` out; code `64h` plays
/// sample `0Bh`.
pub(crate) fn anim_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    match code {
        // 0x11611: the object hidden, its walking record put on the graph
        // at node 0BB8h and sent to the door.
        0xFFFF => {
            api.obj_hide(OBJECT_85); // 0x11614
            let m = rec(api, MOVER_85);
            api.slot_set_figure(1, 0, m); // 0x11626, 080Ch:19B4h
            let m = rec(api, MOVER_85);
            api.walker_bind_graph(m, 1, NODE_START, 1); // 0x1163D, 05DCh:0DB4h
            let m = rec(api, MOVER_85);
            api.walker_walk_to(m, NODE_DOOR, 1); // 0x11652, answer not read
            let player = api.ds(WALKER);
            api.figure_face(player, 0); // 0x11660
            api.set_word(LEAVING, 1); // 0x11668
        }
        0x64 => {
            api.play_sample(0x0B); // 0x11672
        }
        _ => {}
    }
}

/// `push k ; call far 080Ch:164Ch`, answer `0Ah`.
fn scene(api: &mut dyn Api, n: i16) -> i16 {
    api.scene_play(n);
    0x0A
}

/// The Vorzimmer's rule handlers by their offset in segment `1117h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 1117:0001 (0x11171), Fotostreifen on 86h: scene 1Ah, the
        // Fotostreifen 0Eh taken, 0Fh and the Passfoto given.
        0x0001 => fotostreifen(api),
        // 1117:0029 (0x11199).
        0x0029 => rule_0029(api),
        // 1117:007B (0x111EB): scenes 8, 0Ah to 0Eh in turn, counted in
        // [2BA0h].
        0x007B => {
            let counter = api.ds(BYTE_2BA0);
            api.scene_sequence(counter, &[8, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E]); // 0x11200
            0x0A
        }
        // 1117:009D (0x1120D), verb 0Ah on the kandierten Apfel: [2DA8h]
        // set, scene 0Fh.
        0x009D => {
            api.set_word(WORD_2DA8, 1); // 0x11210
            scene(api, 0x0F) // 0x11218
        }
        // 1117:00B3 (0x11223): while [2B9Dh] is 0 the verb made 8 and the
        // answer 5, else 1.
        0x00B3 => {
            if api.byte(BYTE_2B9D) == 0 {
                api.set_word(VERB, 8); // 0x1122D
                5
            } else {
                1 // 0x11238
            }
        }
        // 1117:00CD (0x1123D): scene 11h, [2B9Dh] 1, object 83h shown on
        // animation 2.
        0x00CD => {
            api.scene_play(0x11); // 0x11242
            api.set_byte(BYTE_2B9D, 1); // 0x11248
            api.obj_show(OBJECT_83); // 0x11250
            anim_kept(api, REC_83, 2); // 0x11260
            0x0A
        }
        // 1117:00FD (0x1126D).
        0x00FD => rule_00fd(api),
        // 1117:0131 (0x112A1).
        0x0131 => rule_0131(api),
        // 1117:017B (0x112EB): the first time scene 0 and the Haarnadel
        // given, [2B9Ch] 1; after that the search goes on.
        0x017B => {
            if api.byte(BYTE_2B9C) == 0 {
                api.scene_play(0); // 0x112F7
                api.inv_give(0x0D); // 0x112FF
                api.set_byte(BYTE_2B9C, 1); // 0x11305
                0x0A
            } else {
                0 // 0x1130C
            }
        }
        // 1117:01A5 (0x11315): object 84h hidden (frame -1), sample 17h,
        // [2B9Fh] bit 0; answer 1.
        0x01A5 => {
            api.obj_show_frame(OBJECT_84, -1); // 0x1131D, 0F48h:096Eh
            api.play_sample(0x17); // 0x11327
            let b = api.byte(BYTE_2B9F) | 1;
            api.set_byte(BYTE_2B9F, b); // 0x1132D
            1
        }
        // 1117:01C7 (0x11337): object 84h on frame 0, sample 18h, [2B9Fh]
        // bit 0 cleared; answer 1.
        0x01C7 => {
            api.obj_show_frame(OBJECT_84, 0); // 0x1133F
            api.play_sample(0x18); // 0x11349
            let b = api.byte(BYTE_2B9F) & 0xFE;
            api.set_byte(BYTE_2B9F, b); // 0x1134F
            1
        }
        // 1117:01E9 (0x11359): scene 5 without [2B9Fh] bit 0, else 6
        // without bit 1, else 7.
        0x01E9 => {
            let bits = api.byte(BYTE_2B9F);
            let n = if bits & 1 == 0 {
                5 // 0x11363
            } else if bits & 2 == 0 {
                6 // 0x1136E
            } else {
                7 // 0x11373
            };
            scene(api, n) // 0x11377
        }
        // 1117:0212 (0x11382): scene 1 to 4 by [2B9Dh] 0 to 3, the table at
        // cs:0243h; above 3 none.
        0x0212 => {
            let state = api.byte(BYTE_2B9D);
            if state <= 3 {
                api.scene_play(i16::from(state) + 1); // 0x1139A
            }
            0x0A
        }
        // 1117:024B (0x113BB), 025B (0x113CB), 026B (0x113DB), 027B
        // (0x113EB): scenes 1Eh, 1Dh, 1Ch, 1Dh.
        0x024B => scene(api, 0x1E), // 0x113C0
        0x025B => scene(api, 0x1D), // 0x113D0
        0x026B => scene(api, 0x1C), // 0x113E0
        0x027B => scene(api, 0x1D), // 0x113F0
        // 1117:028B (0x113FB): one of scenes !0x24 to !0x22.
        0x028B => {
            let n = api.random(!0x24, !0x22); // 0x11402
            scene(api, num::signed(n)) // 0x1140B
        }
        _ => return None,
    })
}

/// Rule `1117h:0029h` (`0x11199`), on `86h` with the second object by
/// `[2D40h]`: `0Eh` as rule `0001h`, `0Fh` scene `1Bh`; any other with
/// the class word `[2D42h]` 6 scene `18h`, else `19h`.
fn rule_0029(api: &mut dyn Api) -> i16 {
    match api.word(SECOND) {
        0x0E => fotostreifen(api), // 0x111AB
        0x0F => scene(api, 0x1B),  // 0x111CD
        _ => {
            let n = if api.word(SECOND_CLASS) == 6 {
                0x18
            } else {
                0x19
            }; // 0x111D1
            scene(api, n) // 0x111DF
        }
    }
}

/// Rule `1117h:00FDh` (`0x1126D`): by `[2B9Dh]`; at 1 by `[2B9Eh]`, which
/// Schlönstadt's Büro sets (`0x15F54`).
fn rule_00fd(api: &mut dyn Api) -> i16 {
    let state = api.byte(BYTE_2B9D);
    let n = if state == 1 {
        if api.byte(BYTE_2B9E) == 0 { 0x12 } else { 0x13 } // 0x11277
    } else if state > 2 {
        0x17 // 0x11290
    } else {
        0x14 // 0x11294
    };
    scene(api, n) // 0x11296
}

/// Rule `1117h:0131h` (`0x112A1`), the `Infrarotclip` on `83h`: object
/// `83h` in subject slot 1; below 2 in `[2B9Dh]` scene `15h`, above 2
/// scene `17h`, at 2 scene `16h`, `[2B9Dh]` 3 and `[2BA2h]` 1.
fn rule_0131(api: &mut dyn Api) -> i16 {
    let r = rec(api, REC_83);
    api.slot_set_object(1, 0, r); // 0x112B0, 080Ch:1972h
    let state = api.byte(BYTE_2B9D);
    if state < 2 {
        api.scene_play(0x15); // 0x112C1
    } else if state > 2 {
        api.scene_play(0x17); // 0x112C1
    } else {
        api.scene_play(0x16); // 0x112D6
        api.set_byte(BYTE_2B9D, 3); // 0x112DC
        api.set_byte(BYTE_2BA2, 1); // 0x112E1
    }
    0x0A
}

/// Rule `0001h`'s body, also rule `0029h`'s for `Fotostreifen` `0Eh`
/// (`0x11174`–`0x11194`, `0x111AB`–`0x111CA`).
fn fotostreifen(api: &mut dyn Api) -> i16 {
    api.scene_play(0x1A); // 0x11176
    api.inv_take(0x0E); // 0x1117E
    api.inv_give(0x0F); // 0x11186
    api.inv_give(0x0B); // 0x1118E
    0x0A
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
