//! Location 9, Jones 2, a row of kind 1 whose one exit, `6Eh`, leads back
//! to location 8 (`Jones 1`). The module is named after the location as
//! the game itself names it in its location table (`Jones 2`, the
//! record's `+37h`, `250Ah:00D7h`).
//!
//! Re-expressed from the handler at `12C1h:0162h` (image `0x12D72` to its
//! `retf` at `0x13212`), its hook `12C1h:060Bh` (`0x1321B` to `0x133E7`,
//! the record's `+3Fh`) and its eight rule handlers `12C1h:0007h` to
//! `0152h` (`0x12C17`–`0x12D71`), against [`Api`] alone. The record's
//! `+43h` is null: the library's empty animation hook. The location's
//! rules are the eight records at `2446h:03BAh` (image `0x2481A`, the
//! record's `+3Bh`), fourteen bytes each:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 1 | 4 | `80h` | — | `0134h` |
//! | 3 | 4 | `88h` | — | `0007h` |
//! | 3 | 4 | `89h` | — | `003Fh` |
//! | 3 | 4 | `8Ah` | — | `0077h` |
//! | 3 | 4 | `8Bh` | — | `00AFh` |
//! | 3 | 4 | `8Ch` | — | `00E7h` |
//! | 5 | `20h` | `13h` | `80h` | `011Fh` |
//! | 5 | `20h` | `13h` | 0 | `0152h` |
//!
//! The verbs are, by the verb text row at `DS:116Ah`: 1 `Betrachte`, 3
//! `Sprich mit`, 5 `Benutze … mit`; the items, by the item row at
//! `DS:10AAh`: `13h` `Leuchtfarbe`, `14h` `Eimer`.
//!
//! The room is dark until the `Leuchtfarbe` is used on object `80h` (rule
//! `011Fh` sets `[2BADh]`). The entry copies the first 20h colors of the
//! room's palette to `2446h:0438h` with every green doubled (at most
//! `3Fh`), the colors the room has lit. Dark, it shows objects `88h` to
//! `8Ch`, hides `F5h`, and loads the room's colors through ramp 1 of the
//! shade (`02AFh:0217h`) into the record at `DS:1846h` (start 0, count
//! 20h, the colors at `DS:1848h`); lit, it hides all six and loads the lit
//! colors. The frame loop then moves one color at a time: red and blue
//! up by one towards the lit colors, green by a step of one or two
//! towards them, or jittering around them once there.
//!
//! The hook animates six objects, `88h` to `8Ch` and `F5h`, slot `si`
//! (0 to 5) each: the bit `DS:1840h[si]` (2, 4, 8, 10h, 20h, 40h) in
//! `[2E10h]` says the slot takes part, `[2E1Eh + 2·si]` holds the frame
//! wanted, `[2E12h + 2·si]` the frame given, and `[2E2Ah + 4·si]` the
//! slot's record. Code 8's frames come from the six rows of seven at
//! `2446h:0390h`.
//!
//! Five bytes of the logic's saved state block (`DS:2B98h`, `4Eh` bytes)
//! count the first `Sprich mit` on each of `88h` to `8Ch`; `[2BAEh]`
//! counts the first visit, `[2BADh]` (read also by location 26,
//! `Vorführraum`, `0x167B6`) says the room is lit.

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x12C1;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x131F9`).
const OBJECT_STATES: u16 = 0x2018;

/// Object `80h`, the one the `Leuchtfarbe` is used on.
const OBJECT_80: u16 = 0x80;
/// Objects `88h` to `8Ch`, slots 0 to 4 of the hook.
const OBJECTS_88: [u16; 5] = [0x88, 0x89, 0x8A, 0x8B, 0x8C];
/// Object `F5h`, slot 5 of the hook.
const OBJECT_F5: u16 = 0xF5;
/// Item `13h`, `Leuchtfarbe`, and item `14h`, `Eimer`.
const LEUCHTFARBE: u16 = 0x13;
const EIMER: u16 = 0x14;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BADh]`: 1 once the room is lit (rule `011Fh`).
const LIT: u16 = 0x2BAD;
/// `[2BAEh]`: the first visit's scene played.
const FIRST_VISIT: u16 = 0x2BAE;
/// `[2BBCh]`: set on the first visit (`0x12E9E`).
const BYTE_2BBC: u16 = 0x2BBC;
/// The first `Sprich mit` on `88h`, `89h`, `8Ah`, `8Bh`, `8Ch`.
const TALKED_88: u16 = 0x2BAF;
const TALKED_89: u16 = 0x2BB0;
const TALKED_8A: u16 = 0x2BB1;
const TALKED_8B: u16 = 0x2BB3;
const TALKED_8C: u16 = 0x2BB2;

/// The words of segment `12C1h` alone (outside the saved block).
/// `[2E0Ah]`: the paint applied, the colors move every frame and the
/// samples play (event `0Ah`, cleared at `0x131DE`).
const PAINTING: u16 = 0x2E0A;
/// `[2E0Ch]`: a task armed by event `0Bh`, started at `190h` hundredths.
const TURN_PENDING: u16 = 0x2E0C;
/// `[2E0Eh]`: the frames until the next color step, 0 none at all.
const COUNTDOWN: u16 = 0x2E0E;
/// `[2E10h]`: the slots taking part in the hook's codes 5, 6 and 8.
const ACTIVE: u16 = 0x2E10;
/// `[2E12h]`: the six frames given, a word a slot.
const GIVEN: u16 = 0x2E12;
/// `[2E1Eh]`: the six frames wanted, a word a slot.
const WANTED: u16 = 0x2E1E;
/// `[2E2Ah]`: the six slots' records, a far pointer a slot.
const RECORDS: u16 = 0x2E2A;
/// The hook's slots.
const SLOTS: u16 = 6;
/// Slot 4 (`8Ch`) and slot 5 (`F5h`).
const SLOT_8C: u16 = 4;
const SLOT_F5: u16 = 5;

/// `DS:1840h`: a slot's bit, a byte a slot.
const SLOT_BITS: u16 = 0x1840;
/// `DS:1846h`: the palette record the entry loads, start and count, its
/// colors from `DS:1848h`.
const PAL_RECORD: u16 = 0x1846;
const PAL_COLORS: u16 = 0x1848;
/// The colors the record holds, 20h of three bytes (`push 60h`).
const PAL_COUNT: u16 = 0x20;
const PAL_BYTES: u16 = 0x60;

/// The segment of the program's tables, `2446h`: the lit colors at
/// `0438h` and code 8's frames at `0390h`, seven a slot.
const TABLES: u16 = 0x2446;
const LIT_COLORS: u16 = 0x0438;
const FRAMES: u16 = 0x0390;
const FRAMES_PER_SLOT: u16 = 7;

/// `[2CFCh]`/`[2CFEh]`: the loaded room's header; its `+1Ch` points to the
/// room's palette table.
const ROOM_HEADER: u16 = 0x2CFC;
/// `[1CF6h]`: the hundredths.
const HUNDREDTHS: u16 = 0x1CF6;

/// Handler local 0: `si`, the hundredths after which the next sample
/// plays; `14h` at the entry (`0x12E01`).
const NEXT_SAMPLE: usize = 0;

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

/// Slot `si`'s record (`[bx+2E2Ah]`, `[bx+2E2Ch]`, `bx = si·4`).
fn slot_rec(api: &dyn Api, si: u16) -> Far {
    rec(api, RECORDS.wrapping_add(si << 2))
}

/// Slot `si`'s word in one of the two frame rows (`bx = si·2`).
fn slot_word(row: u16, si: u16) -> u16 {
    row.wrapping_add(si << 1)
}

/// Whether slot `si`'s bit is in `bits` (`mov al, [si+1840h] ; mov ah, 0 ;
/// test …, ax`).
fn slot_in(api: &dyn Api, si: u16, bits: u16) -> bool {
    bits & u16::from(api.byte(SLOT_BITS.wrapping_add(si))) != 0
}

/// `push dir ; push ds ; push 176Eh ; call far 05DCh:0B56h`.
fn face(api: &mut dyn Api, dir: i16) {
    let player = api.ds(WALKER);
    api.figure_face(player, dir);
}

/// `push hi ; push lo ; call far 203Dh:0024h ; push ax ; call far
/// 080Ch:164Ch`: the roll is the scene.
fn roll(api: &mut dyn Api, lo: u16, hi: u16) {
    let n = api.random(lo, hi);
    api.scene_play(num::signed(n));
}

/// A byte of the program's table segment `2446h`.
fn table_byte(api: &mut dyn Api, off: u16) -> u8 {
    api.far_byte(Far { off, seg: TABLES })
}

fn set_table_byte(api: &mut dyn Api, off: u16, v: u8) {
    api.set_far_byte(Far { off, seg: TABLES }, v);
}

/// Color `i`'s byte `c` (0 red, 1 green, 2 blue) of the lit colors and of
/// the record's colors (`imul bx, bx, 3`).
fn lit(api: &mut dyn Api, i: u16, c: u16) -> u8 {
    table_byte(
        api,
        LIT_COLORS.wrapping_add(i.wrapping_mul(3)).wrapping_add(c),
    )
}

fn shown(api: &dyn Api, i: u16, c: u16) -> u8 {
    api.byte(PAL_COLORS.wrapping_add(i.wrapping_mul(3)).wrapping_add(c))
}

fn set_shown(api: &mut dyn Api, i: u16, c: u16, v: u8) {
    api.set_byte(
        PAL_COLORS.wrapping_add(i.wrapping_mul(3)).wrapping_add(c),
        v,
    );
}

/// `les bx, [2CFCh] ; les bx, es:[bx+1Ch] ; mov ax, es:[bx+2] ; mov dx,
/// es:[bx]` (`0x12D8F`–`0x12D9B`): the far pointer the room's palette
/// table starts with; its colors follow the record's first word.
fn room_palette(api: &mut dyn Api) -> Far {
    let header = rec(api, ROOM_HEADER);
    let at = Far {
        off: header.off.wrapping_add(0x1C),
        seg: header.seg,
    };
    let table = Far {
        off: api.far_word(at),
        seg: api.far_word(Far {
            off: at.off.wrapping_add(2),
            seg: at.seg,
        }),
    };
    Far {
        off: api.far_word(table),
        seg: api.far_word(Far {
            off: table.off.wrapping_add(2),
            seg: table.seg,
        }),
    }
}

/// Everything before the frame loop, `0x12D72` to `0x12F46`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x12D77
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x12D83
    api.visit_enter(loc, entry); // 0x12D87
    let palette = room_palette(api); // 0x12D8F–0x12DA1
    let colors = Far {
        off: palette.off.wrapping_add(2),
        seg: palette.seg,
    };
    // 0x12DA4: `mov byte [bp-0Bh], 1`, the stack record's count, which
    // nothing else writes (see `pal_load_local`).
    // 0x12DA8–0x12DBA: the room's first 20h colors to the lit colors.
    api.movmem(
        colors,
        Far {
            off: LIT_COLORS,
            seg: TABLES,
        },
        PAL_BYTES,
    );
    // 0x12DC2–0x12DFF: colors 1Fh down to 0, green doubled, at most 3Fh.
    for i in (0..PAL_COUNT).rev() {
        let at = LIT_COLORS.wrapping_add(i.wrapping_mul(3)).wrapping_add(1);
        let g = table_byte(api, at) << 1; // 0x12DD4
        set_table_byte(api, at, g);
        if g > 0x3F {
            set_table_byte(api, at, 0x3F); // 0x12DE1, 0x12DF1
        }
    }
    api.set_local(NEXT_SAMPLE, 0x14); // 0x12E01
    if api.byte(LIT) < 1 {
        // 0x12E04: dark.
        for (si, &id) in (0u16..).zip(OBJECTS_88.iter()) {
            let r = api.obj_show(id); // 0x12E11 … 0x12E51
            keep(api, RECORDS.wrapping_add(si << 2), r); // 0x12E17 … 0x12E5B
        }
        let r = api.obj_hide(OBJECT_F5); // 0x12E61
        keep(api, RECORDS.wrapping_add(SLOT_F5 << 2), r); // 0x12E67, 0x12E6B
        // 0x12E6E–0x12E95: slots 4 down to 0 on animation 0.
        for si in (0..SLOT_F5).rev() {
            let r = slot_rec(api, si);
            api.obj_anim(r, 0); // 0x12E85
        }
        if api.byte(FIRST_VISIT) == 0 {
            // 0x12E97
            api.set_byte(BYTE_2BBC, 1); // 0x12E9E
            api.scene_play(0); // 0x12EA5
            let n = api.byte(FIRST_VISIT).wrapping_add(1);
            api.set_byte(FIRST_VISIT, n); // 0x12EAB
        } else {
            api.ambient_play(1, 5, 1); // 0x12EB7, the answer not read
        }
        api.set_word(COUNTDOWN, 0); // 0x12EBF
        let dst = api.ds(PAL_COLORS);
        api.pal_ramps(colors, dst, [1, 1, 1], PAL_COUNT); // 0x12EDB, 02AFh:0217h
    } else {
        // 0x12EE5: lit.
        for &id in &OBJECTS_88 {
            api.obj_hide(id); // 0x12EE8 … 0x12F0C
        }
        api.obj_hide(OBJECT_F5); // 0x12F15
        api.set_word(COUNTDOWN, 1); // 0x12F1B
        let dst = api.ds(PAL_COLORS);
        api.movmem(
            Far {
                off: LIT_COLORS,
                seg: TABLES,
            },
            dst,
            PAL_BYTES,
        ); // 0x12F2D
    }
    let record = api.ds(PAL_RECORD);
    api.pal_load(record); // 0x12F39, 0104h:090Eh
    api.visit_finish_entry(); // 0x12F41
    api.set_word(WORDS.entry, 0); // 0x12F46
}

/// The color step, `0x12F60` to `0x13159`: every `[2E0Eh]` frames (1 to
/// 0Ah rolled, every frame while painting) one color, rolled or the
/// highest still short of the lit colors in red or blue.
fn color_step(api: &mut dyn Api) {
    let n = api.word(COUNTDOWN).wrapping_add(1);
    api.set_word(COUNTDOWN, n); // 0x12F60
    // 0x12F67: `cmp ax, 28h ; jg` (signed); 0x12F6C.
    if num::signed(n) <= 0x28 && api.word(PAINTING) == 0 {
        return;
    }
    let n = api.random(1, 0x0A); // 0x12F7A
    api.set_word(COUNTDOWN, n); // 0x12F82
    let mut s = num::low8(api.random(0, 0x1F)); // 0x12F89, 0x12F91
    // 0x12F94–0x1302E: colors 1Fh down to 0; at the first one short in red
    // or blue (`ja` 0x12FB6, `jna` 0x12FD2), the rolled one stays if it is
    // short too (`jc` 0x12FF6, 0x1301A), else that one is taken.
    for i in (0..PAL_COUNT).rev() {
        if lit(api, i, 0) > shown(api, i, 0) || lit(api, i, 2) > shown(api, i, 2) {
            let k = u16::from(s);
            if shown(api, k, 0) >= lit(api, k, 0) && shown(api, k, 2) >= lit(api, k, 2) {
                s = num::low8(i); // 0x1301F
            }
            break;
        }
    }
    let k = u16::from(s);
    // 0x13031–0x1306D: red one up while short.
    let mut r = shown(api, k, 0);
    if lit(api, k, 0) > r {
        r = r.wrapping_add(1);
        set_shown(api, k, 0, r); // 0x1306D
    }
    // 0x13071–0x130AD: blue the same.
    let mut b = shown(api, k, 2);
    if lit(api, k, 2) > b {
        b = b.wrapping_add(1);
        set_shown(api, k, 2, b); // 0x130AD
    }
    // 0x130B1–0x13114: green by one towards the lit green when more than
    // two away (signed), else by −2, −1, 1 or 2.
    let g = shown(api, k, 1);
    let d = i16::from(lit(api, k, 1)) - i16::from(g); // 0x130DD
    let step = if d > 2 {
        1 // 0x130E8
    } else if d < -2 {
        -1 // 0x130F5
    } else {
        let roll = num::signed(api.random(0, 3)).wrapping_sub(2); // 0x13100, 0x13108
        if roll == 0 { 2 } else { roll } // 0x13114
    };
    // 0x13119–0x13134: from 0 to 3Fh.
    let v = step.wrapping_add(i16::from(g)).clamp(0, 0x3F);
    let v = num::low8(num::unsigned(v));
    set_shown(api, k, 1, v); // 0x13146
    // `push ss ; lea ax, [bp-0Ch]`: the record on the handler's stack.
    api.pal_load_local(&[s, 1, r, v, b]); // 0x13154, 0104h:090Eh
}

/// One turn of the frame loop, `0x12F4C` to `0x131F0`: the color step,
/// while painting a sample once the hundredths pass `si`, and the task
/// armed by event `0Bh` started past `190h` hundredths.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x12F4C
    api.dispatch(); // 0x12F51
    if api.word(COUNTDOWN) != 0 {
        // 0x12F56
        color_step(api);
    }
    // 0x1315C, 0x13163: `cmp [1CF6h], si ; jna`.
    if api.word(PAINTING) != 0 && api.word(HUNDREDTHS) > api.local(NEXT_SAMPLE) {
        api.play_sample(0x29); // 0x1316B
        let k = api.random(0, 6); // 0x13175
        // 0x13182: `cmp bx, 3 ; ja`, the table at `cs:0603h` (`0x13213`)
        // sends 0 to 3 to the same arm.
        if k <= 3 {
            api.play_sample(k.wrapping_add(0x25)); // 0x13195
        }
        let h = api.word(HUNDREDTHS);
        let extra = api.random(0, h >> 3); // 0x131A4
        let h = api.word(HUNDREDTHS); // 0x131AC
        api.set_local(NEXT_SAMPLE, h.wrapping_add(extra).wrapping_add(0x14)); // 0x131B5
    }
    // 0x131B7, 0x131BE: `cmp word [1CF6h], 190h ; jna`.
    if api.word(TURN_PENDING) != 0 && api.word(HUNDREDTHS) > 0x190 {
        face(api, 2); // 0x131CC
        api.task_start(0); // 0x131D6
        api.set_word(PAINTING, 0); // 0x131DE
        api.set_word(TURN_PENDING, 0); // 0x131E1
    }
    api.frame_pass(); // 0x131E4
    api.word(WORDS.entry) == 0 // 0x131E9
}

/// Everything after the frame loop, `0x131F3` to `0x13212`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x131FE
    api.visit_leave(); // 0x13206
    api.slot_unload(); // 0x1320B
}

/// The hook, `12C1h:060Bh`: code in `[bp+6]`, argument in `[bp+8]`, task
/// in `[bp+0Ah]`. Codes above 9 and codes 1 to 4 do nothing (`0x13225`,
/// the table at `cs:07E4h`, `0x133F4`). Every slot loop runs `si` from 5
/// down to 0.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, task: u16) {
    match code {
        // 0x13234: the slots taking part are the argument's bits but 40h;
        // each wants frame 1 if its bit is in the argument, else none, and
        // has none given.
        0 => {
            let bits = arg & !0x40;
            api.set_word(ACTIVE, bits); // 0x13237
            for si in (0..SLOTS).rev() {
                let wanted = if slot_in(api, si, bits) { 1 } else { 0xFFFF };
                api.set_word(slot_word(WANTED, si), wanted); // 0x1324E, 0x1325A
                api.set_word(slot_word(GIVEN, si), 0xFFFF); // 0x13264
            }
        }
        // 0x13274: the slots taking part back on animation 0; every slot
        // with none wanted or given; then none take part.
        5 | 6 => {
            for si in (0..SLOTS).rev() {
                let bits = api.word(ACTIVE);
                // 0x1327F: `test [2E10h], ax ; jz 0x329C` skips the call
                // alone.
                if slot_in(api, si, bits) {
                    let r = slot_rec(api, si);
                    api.obj_anim(r, 0); // 0x13294
                }
                api.set_word(slot_word(GIVEN, si), 0xFFFF); // 0x132A3
                api.set_word(slot_word(WANTED, si), 0xFFFF); // 0x132AB
            }
            api.set_word(ACTIVE, 0); // 0x132B6
        }
        7 => hook_events(api, arg, task),
        // 0x132BF: each slot taking part wants its frame for the argument
        // from the row at `2446h:0390h`, or 0 for a negative argument.
        8 => {
            for si in (0..SLOTS).rev() {
                let bits = api.word(ACTIVE);
                if !slot_in(api, si, bits) {
                    continue; // 0x132CE
                }
                let wanted = if num::signed(arg) >= 0 {
                    // 0x132D4–0x132E0
                    let off = FRAMES
                        .wrapping_add(si.wrapping_mul(FRAMES_PER_SLOT))
                        .wrapping_add(arg);
                    u16::from(table_byte(api, off))
                } else {
                    0 // 0x132F5
                };
                api.set_word(slot_word(WANTED, si), wanted); // 0x132EB, 0x132F5
            }
        }
        // 0x13305: every slot whose frame wanted is not the one given gets
        // it.
        9 => {
            for si in (0..SLOTS).rev() {
                let wanted = api.word(slot_word(WANTED, si));
                if wanted == api.word(slot_word(GIVEN, si)) {
                    continue; // 0x1331A
                }
                api.set_word(slot_word(GIVEN, si), wanted); // 0x13324
                let r = slot_rec(api, si);
                api.obj_anim(r, num::signed(wanted)); // 0x13336
            }
        }
        _ => {}
    }
}

/// Code 7, `0x13348`: the argument less `0Ah` up to 5 (`cmp bx, 5 ; ja`),
/// through the table at `cs:07D8h` (`0x133E8`).
fn hook_events(api: &mut dyn Api, arg: u16, task: u16) {
    match arg {
        // 0x1335C: the Leuchtfarbe traded for the Eimer, object 80h shown,
        // the painting begun from 0 hundredths.
        0x0A => {
            api.inv_take(LEUCHTFARBE); // 0x13360, 0751h:0A43h(13h, 14h)
            api.inv_give(EIMER);
            api.set_word(PAINTING, 1); // 0x13368
            api.obj_show(OBJECT_80); // 0x13371, the record not kept
            api.set_word(COUNTDOWN, 1); // 0x13377
            api.set_word(HUNDREDTHS, 0); // 0x1337D
        }
        // 0x13385: the calling task armed, to be started at 190h.
        0x0B => {
            api.task_arm(task); // 0x13388
            api.set_word(TURN_PENDING, 1); // 0x1338E
        }
        // 0x13396: the figure faces 3; slot 5 takes part, object F5h shown
        // on frame −1, frame 3 wanted.
        0x0C => {
            face(api, 3); // 0x1339C
            let bits = api.word(ACTIVE) | 0x40;
            api.set_word(ACTIVE, bits); // 0x133A4
            api.obj_show_frame(OBJECT_F5, -1); // 0x133AE, the record not kept
            api.set_word(slot_word(WANTED, SLOT_F5), 3); // 0x133B6
        }
        // 0x133BE: slot 5 wants frame 4.
        0x0D => api.set_word(slot_word(WANTED, SLOT_F5), 4),
        // 0x133C6: slot 4 out (its bit `[1844h]`), frame 0 wanted.
        0x0E => {
            let bit = u16::from(api.byte(SLOT_BITS.wrapping_add(SLOT_8C)));
            let bits = api.word(ACTIVE) & !bit;
            api.set_word(ACTIVE, bits); // 0x133CD
            api.set_word(slot_word(WANTED, SLOT_8C), 0); // 0x133D1
        }
        // 0x133D9: location 1, entry 3.
        0x0F => {
            api.set_word(WORDS.location, 1); // 0x133D9
            api.set_word(WORDS.entry, 3); // 0x133DF
        }
        _ => {}
    }
}

/// `push 1 ; call far 080Ch:191Fh`, then the first time scene `first` and
/// the counter up, else one of `lo` to `hi` (`0x12C17`–`0x12D2E`, five
/// handlers alike).
fn talk(api: &mut dyn Api, counter: u16, first: i16, lo: u16, hi: u16) -> i16 {
    api.task_stop(1);
    if api.byte(counter) == 0 {
        api.scene_play(first);
        let n = api.byte(counter).wrapping_add(1);
        api.set_byte(counter, n);
    } else {
        roll(api, lo, hi);
    }
    0x0A
}

/// Jones 2's rule handlers by their offset in segment `12C1h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 12C1:0007 (0x12C17), Sprich mit 88h: task 1 stopped (0x12C1C);
        // scene 7 (0x12C2B) and [2BAFh] up (0x12C31), or one of 8 and 9
        // (0x12C3B, 0x12C44).
        0x0007 => talk(api, TALKED_88, 7, 8, 9),
        // 12C1:003F (0x12C4F), Sprich mit 89h: scene 0Ah (0x12C63), or one
        // of 0Bh and 0Ch (0x12C73, 0x12C7C).
        0x003F => talk(api, TALKED_89, 0x0A, 0x0B, 0x0C),
        // 12C1:0077 (0x12C87), Sprich mit 8Ah: scene 0Dh (0x12C9B), or one
        // of 0Eh and 0Fh (0x12CAB, 0x12CB4).
        0x0077 => talk(api, TALKED_8A, 0x0D, 0x0E, 0x0F),
        // 12C1:00AF (0x12CBF), Sprich mit 8Bh: scene 10h (0x12CD3), or one
        // of 11h and 12h (0x12CE3, 0x12CEC); the counter is [2BB3h].
        0x00AF => talk(api, TALKED_8B, 0x10, 0x11, 0x12),
        // 12C1:00E7 (0x12CF7), Sprich mit 8Ch: scene 13h (0x12D0B), or one
        // of 14h and 15h (0x12D1B, 0x12D24); the counter is [2BB2h].
        0x00E7 => talk(api, TALKED_8C, 0x13, 0x14, 0x15),
        // 12C1:011F (0x12D2F), the Leuchtfarbe on object 80h: scene 6 and
        // the room lit.
        0x011F => {
            api.scene_play(6); // 0x12D34
            api.set_byte(LIT, 1); // 0x12D3A
            0x0A
        }
        // 12C1:0134 (0x12D44), Betrachte object 80h: scene 17h lit, else
        // 16h (`cmp byte [2BADh], 1 ; jnz`).
        0x0134 => {
            let n = if api.byte(LIT) == 1 { 0x17 } else { 0x16 };
            api.scene_play(n); // 0x12D57
            0x0A
        }
        // 12C1:0152 (0x12D62), the Leuchtfarbe on nothing: scene !10h.
        0x0152 => {
            api.scene_play(!0x10); // 0x12D67
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
