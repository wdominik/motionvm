//! Location 26, the Vorführraum, a row of kind 1: the room behind door
//! `79h` of the VIP-Lounge (location 25, see [`super::vip_lounge`]), with
//! door `7Ah` on to location 2 ([`super::interlude_2`]). The module is
//! named after the location as the game itself names it in its location
//! table (`Vorführraum`, the record's `+37h`, `250Ah:0193h`).
//!
//! Re-expressed from the handler at `167Bh:0085h` (image `0x16835` to its
//! `retf` at `0x1690A`), its hook `167Bh:01FEh` (`0x169AE` to `0x16A0B`,
//! the record's `+3Fh`), its record hook `167Bh:015Bh` (`0x1690B` to
//! `0x169AD`, the record's `+47h`) and its four rule handlers `167Bh:0003h`
//! to `006Ah` (`0x167B3`–`0x16834`), against [`Api`] alone. The record's
//! animation hook `+43h` is `0000h:0000h`. The location's rules are the
//! five records at `2446h:099Eh` (image `0x24DFE`, the record's `+3Bh`),
//! fourteen bytes each:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 1 | 4 | `9Fh` | — | `0003h` |
//! | 5 | `20h` | `1Fh` | `9Fh` | `0029h` |
//! | 5 | `20h` | `9Fh` | `1Fh` | `0029h` |
//! | 5 | `20h` | `9Fh` | 0 | `004Fh` |
//! | 5 | `20h` | `1Fh` | 0 | `006Ah` |
//!
//! The verbs are, by the verb text row at `DS:116Ah`: 1 `Betrachte`, 5
//! `Benutze … mit`. Item `1Fh` is the `Wagenheber` by the item row at
//! `DS:10AAh`. Two bytes of the logic's saved state block (`DS:2B98h`,
//! `4Eh` bytes) decide what the rules play: `[2BADh]`, which Jones 2
//! (location 9) sets when its room is lit (`0x12D3A`), and `[2BCBh]`, this
//! location's own, set by the hook once script event `24h` comes with code
//! 4 (`0x169EC`). The entry shows object `9Fh` on frame 1 while the first
//! is set and the second is not.
//!
//! The room runs a roll of credits through the window `(67h, 11h)` wide
//! `72h` and high `35h`. Its lines are the far pointers at `DS:0BCEh`; the
//! routines in segment `0284h` lay it out (`0284h:015Eh`), keep its cursor
//! (`0284h:0001h`), draw it (`0284h:0079h`) and free it (`0284h:029Ah`) —
//! Dunkle Schatten's `12D3h:010Ah`, `025Ah`, `02D2h` and `0249h` but for
//! their numbers ([`RollSpec`]), the `*` line that goes on by nothing here
//! (`0x02ABA`), and the clip the draw sets around itself (`0x028BD`–`0x028E9`,
//! put back at `0x02985`). What moves it is a record that lives in
//! the logic's data segment, `2446h:0960h`: live with bit 4 of `+2` set
//! (`11h`), state `+12h` `FFh`, interval `+14h` 4, layer `+2Ah` 0,
//! bundle `+8` 4. The entry puts its cel table `2446h:099Ah` in bundle 4's
//! slot `[2D34h]` of the table at `[2D24h]` — one cel, `2446h:0990h`, `72h`
//! by `35h`, the window — and links it into layer 0's chain
//! (`05DCh:0D6Ah`). From then on the animator, finding a state it has no
//! arm for (`0x041E0`), calls the record hook through `[2D1Ch]` with code
//! 0 every four ticks, and the drawer, finding bit 4 (`0x071ED`,
//! `0x0724B`), calls it with code 1 instead of drawing a cel. Code 0 moves
//! the roll on by one, waits at the pauses the cursor reports, and starts
//! it over at its end; code 1 draws it.
//!
//! The hook's words are read and written in segment `167Bh` alone and
//! set by every entry: `[2F6Ch]` the steps the roll still waits, `[2F6Eh]`
//! where it ends, `[2F70h]` where it stands.

use crate::num;
use crate::titles::parsec1994::title::roll::RollSpec;
use crate::titles::parsec1994::{Api, Far};

use super::logic::{Logic, WORDS};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x167B;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x168F3`).
const OBJECT_STATES: u16 = 0x2018;

/// Object `9Fh`, which the rules are about.
const OBJECT_9F: u16 = 0x9F;
/// Object `F0h`, shown by the hook in `9Fh`'s place.
const OBJECT_F0: u16 = 0xF0;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BADh]`: Jones 2's room lit.
const LIT: u16 = 0x2BAD;
/// `[2BCBh]`: script event `24h` seen here.
const BYTE_2BCB: u16 = 0x2BCB;

/// The hook's words.
/// `[2F6Ch]`: the steps the roll still waits.
const PAUSE: u16 = 0x2F6C;
/// `[2F6Eh]`: where the roll ends, the layout's answer (`0x1686D`).
const END: u16 = 0x2F6E;
/// `[2F70h]`: where the roll stands.
const POS: u16 = 0x2F70;

/// `2446h:0960h`: the record that carries the roll (`push 2446h ; push
/// 960h`, `0x1687C`, `0x168DA`).
const ROLL_RECORD: Far = Far {
    seg: 0x2446,
    off: 0x0960,
};
/// Its flags `+2` (`and byte es:[962h], 0FDh`, `0x1689B`).
const ROLL_RECORD_FLAGS: Far = Far {
    seg: 0x2446,
    off: 0x0962,
};
/// Bit 1 of `+2`, cleared by the entry.
const HIDDEN: u8 = 0x02;

/// `[2D34h]`/`[2D36h]`: bundle 4's slot of the cel tables at `[2D24h]`
/// (`mov ax, [bx+2D26h] ; mov dx, [bx+2D24h]` with `bx` the bundle times
/// four, `0x03FAD`), set to `2446h:099Ah` (`0x16870`, `0x16876`).
const BUNDLE_4_OFF: u16 = 0x2D34;
const BUNDLE_4_SEG: u16 = 0x2D36;
const ROLL_CELS: Far = Far {
    seg: 0x2446,
    off: 0x099A,
};

/// `DS:0BCEh`: the roll's lines, a null-terminated row of far pointers
/// (`push ds ; push 0BCEh`, `0x029A4`).
const LINES: u16 = 0x0BCE;

/// The roll as `0284h:015Eh` lays it out — the routine the game's end
/// runs its credits with, on the same lines: from `35h` (`mov si, 35h`,
/// `0x029D7`); a `c` line on by `0Eh` (`0x02A6E`), an `n` by `10h`
/// (`0x02A3D`), a `p` by `14h` (`0x02A69`); `*` by nothing, the arm of `+`
/// and `-` (`cs:01F1h`, the row at `0x02ABA`); centered in `72h` from `67h`
/// (`0x02A56`–`0x02A5D`); inks `66h`, `76h`, `74h` (`0x0294D`, `0x02937`,
/// `0x02942`); drawn through the window `(67h, 11h)` wide `72h` and high
/// `35h`, the clip `0x028D8`–`0x028E9` sets.
const ROLL: RollSpec = super::ending::ROLL;

/// `1C3Ch:2036h(67h, 11h, 72h, 35h)`: the window marked for the drawer,
/// the box every mark and test pushes (`push 35h ; push 72h ; push 11h ;
/// push 67h`, `0x168A9`, `0x16920`, `0x16939`, `0x1697F`).
fn mark(api: &mut dyn Api) {
    api.tiles_mark(0x67, 0x11, 0x72, 0x35);
}

/// Everything before the frame loop, `0x16835` to `0x168BE`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x16835
    let entry = api.word(WORDS.entry); // 0x1683A
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x16841
    api.visit_enter(loc, entry); // 0x16845
    if api.byte(LIT) == 1 && api.byte(BYTE_2BCB) == 0 {
        // 0x1684D, 0x16854
        api.obj_show_frame(OBJECT_9F, 1); // 0x16860
    }
    let row = api.ds(LINES);
    let end = api.roll_layout(row, &ROLL); // 0x16868, 0284h:015Eh
    api.set_word(END, end); // 0x1686D
    api.set_word(BUNDLE_4_SEG, ROLL_CELS.seg); // 0x16870
    api.set_word(BUNDLE_4_OFF, ROLL_CELS.off); // 0x16876
    api.record_link(ROLL_RECORD); // 0x16882, 05DCh:0D6Ah
    api.set_word(POS, 0); // 0x1688A
    api.set_word(PAUSE, 0); // 0x16890
    let flags = api.far_byte(ROLL_RECORD_FLAGS);
    api.set_far_byte(ROLL_RECORD_FLAGS, flags & !HIDDEN); // 0x1689B
    api.roll_seek(0); // 0x168A3, 0284h:0001h, answer not read
    mark(api); // 0x168B1
    api.visit_finish_entry(); // 0x168B9
    api.set_word(WORDS.entry, 0); // 0x168BE
}

/// One turn of the frame loop, `0x168C4` to `0x168D8`.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x168C4
    api.dispatch(); // 0x168C9
    api.frame_pass(); // 0x168CE
    api.word(WORDS.entry) == 0 // 0x168D3
}

/// Everything after the frame loop, `0x168DA` to `0x1690A`.
pub(crate) fn leave(api: &mut dyn Api) {
    api.walker_unlink(ROLL_RECORD); // 0x168E0, 05DCh:0D95h
    api.roll_free(); // 0x168E8, 0284h:029Ah
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x168F8
    api.visit_leave(); // 0x16900
    api.slot_unload(); // 0x16905
}

/// The hook, `167Bh:01FEh`: code in `[bp+6]`, argument in `[bp+8]`; the
/// task is not read.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    match (code, arg) {
        // 0x169B7, 0x169C5: object 9Fh hidden, F0h shown, sample 1Dh twice,
        // and the event remembered.
        (4, 0x24) => {
            api.obj_hide(OBJECT_9F); // 0x169CD
            api.obj_show(OBJECT_F0); // 0x169D6, the record not kept
            api.play_sample(0x1D); // 0x169DE
            api.play_sample(0x1D); // 0x169E6
            api.set_byte(BYTE_2BCB, 1); // 0x169EC
        }
        // 0x169BC, 0x169F5: sample 1Eh twice.
        (7, 0x1E) => {
            api.play_sample(0x1E); // 0x169FC
            api.play_sample(0x1E); // 0x16A04
        }
        _ => {}
    }
}

/// The record hook, `167Bh:015Bh`: the record in `[bp+6]`, the code in
/// `[bp+0Ah]` — 0 from the animator (`0x041E9`), 1 from the drawer
/// (`0x071FD`, `0x0725B`); any code but 0 draws (`cmp word [bp+0Ah], 0 ;
/// jz`, `0x1690E`).
pub(crate) fn record_hook(api: &mut dyn Api, _rec: Far, code: u16) {
    if code != 0 {
        // 0x16914: nothing past the end (`jng`, signed).
        if num::signed(api.word(POS)) > num::signed(api.word(END)) {
            return;
        }
        // 0x16928: only while some tile of the window is marked.
        if !api.tiles_dirty(0x67, 0x11, 0x72, 0x35) {
            return;
        }
        api.roll_draw(); // 0x16934, 0284h:0079h
        mark(api); // 0x16941
        return;
    }
    let pause = num::signed(api.word(PAUSE));
    if pause != 0 {
        // 0x16952: a pause counts down; at its end sample 9, or 0Ah past
        // the first line (`cmp word [2F70h], 1 ; jng`, `0x16958`).
        let left = pause.wrapping_sub(1);
        api.set_word(PAUSE, num::unsigned(left));
        if left > 0 {
            return; // 0x16956
        }
        let k = if num::signed(api.word(POS)) > 1 {
            0x0A
        } else {
            0x09
        };
        api.play_sample(k); // 0x16965
    }
    // 0x1696B: one on, the pause the cursor passed three steps a unit.
    let pos = api.word(POS).wrapping_add(1);
    api.set_word(POS, pos);
    let weight = api.roll_seek(pos); // 0x16973
    api.set_word(PAUSE, weight.wrapping_mul(3)); // 0x16979, 0x1697C
    mark(api); // 0x16987
    if num::signed(pos) >= num::signed(api.word(END)) {
        // 0x1698F, `jl`: at the end sample 0Eh, and from the top.
        api.play_sample(0x0E); // 0x1699A
        api.set_word(POS, 0); // 0x169A2
        api.roll_seek(0); // 0x169A6, answer not read
    }
}

/// Whether Jones 2's room is lit (`cmp byte [2BADh], 1 ; jc`: below 1 is
/// not).
fn lit(api: &dyn Api) -> bool {
    api.byte(LIT) >= 1
}

/// The Vorführraum's rule handlers by their offset in segment `167Bh`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 167B:0003 (0x167B3), Betrachte object 9Fh: scene 0 while Jones 2
        // is dark, else 1, or 2 once event 24h was seen.
        0x0003 => {
            let n = if !lit(api) {
                0 // 0x167CC
            } else if api.byte(BYTE_2BCB) == 0 {
                1 // 0x167C4
            } else {
                2 // 0x167C8
            };
            api.scene_play(n); // 0x167CE
            0x0A
        }
        // 167B:0029 (0x167D9), the Wagenheber with object 9Fh either way
        // round: scene 5 while Jones 2 is dark, else 3, or 4 once event 24h
        // was seen.
        0x0029 => {
            let n = if !lit(api) {
                5 // 0x167F2
            } else if api.byte(BYTE_2BCB) == 0 {
                3 // 0x167EA
            } else {
                4 // 0x167EE
            };
            api.scene_play(n); // 0x167F4
            0x0A
        }
        // 167B:004F (0x167FF), Benutze object 9Fh: scene 6 while Jones 2
        // is dark, else 7.
        0x004F => {
            let n = if lit(api) { 7 } else { 6 }; // 0x16809, 0x1680D
            api.scene_play(n); // 0x1680F
            0x0A
        }
        // 167B:006A (0x1681A), Benutze the Wagenheber: one of scenes !0x24
        // to !0x22 (`push 0FFDDh ; push 0FFDBh`).
        0x006A => {
            let n = api.random(!0x24, !0x22); // 0x16821
            api.scene_play(num::signed(n)); // 0x1682A
            0x0A
        }
        _ => return None,
    })
}

/// This location's entry points, as [`super::logic`] calls them. The
/// record hook is handed on by [`super::logic`]'s record hook table.
pub(super) const LOGIC: Logic = Logic {
    enter,
    frame,
    leave,
    hook,
    anim_hook: None,
    rule,
};
