//! *Bi-Fi 2*'s global rules: the handlers the table at `2336h:0130h`
//! (image `0x23490`) names, which the rule search asks after a location's
//! own. The table is handed to the installer by `push 2336h ; push 0130h ;
//! call far 0E40h:000Ch` (`0x00926`).
//!
//! A row is fourteen bytes, read by the matcher `0E40h` as Dunkle
//! Schatten's: `+0` the link the installer writes (`mov [si], ax`,
//! `0x0E461`), `+2` the verb (`cmp cx, es:[si+2]`, `0x0E55E`), `+4` the
//! phase (`mov ax, es:[si+4]`, `0x0E541`), `+6` the object or class
//! (`cmp bx, es:[si+6]`, `0x0E549`), `+8` the second (`mov ax, es:[si+8]`,
//! `0x0E577`), `+0Ah` the handler's far pointer (`call far es:[si+0Ah]`,
//! `0x0E5B3`). The table ends at its first zero phase (`0x0E463`), the row
//! at `0x23730`: 48 rows, numbered 0 to 47 here.
//!
//! | # | verb | phase | object | second | handler |
//! |---|---|---|---|---|---|
//! | 0 | 1 | 4 | `1Ch` | — | `0112h` |
//! | 1 | 5 | `20h` | `19h` | `17h` | `014Eh` |
//! | 2 | 5 | `20h` | `19h` | 0 | `013Eh` |
//! | 3–5 | 4 | 2 | `10h`, 9, `22h` | — | `0172h` |
//! | 6 | 6 | 4 | class 1 | — | `0257h` |
//! | 7 | 6 | 4 | class 2 | — | `02B1h` |
//! | 8, 9 | 7 | 4 | class 1, class 2 | — | `063Fh` |
//! | 10 | `0Ah` | 4 | 9 | — | `01ACh` |
//! | 11 | `0Ah` | 4 | `10h` | — | `0180h` |
//! | 12 | `0Ah` | 4 | `22h` | — | `0190h` |
//! | 13, 14 | 5 | `20h` | 6 / `0Ah`, `0Ah` / 6 | | `01D7h` |
//! | 15–17 | 5 | `20h` | 6 | 9, `0Bh`, `0Eh` | `01BCh` |
//! | 18, 19 | 5 | `20h` | `0Bh` / `0Ah`, `0Ah` / `0Bh` | | `01F3h` |
//! | 20 | 5 | `20h` | `0Bh` | 9 | `01BCh` |
//! | 21, 22 | 5 | `20h` | `0Ch` / 6, 6 / `0Ch` | | `020Fh` |
//! | 23, 24 | 5 | `20h` | `0Bh` / 7, 7 / `0Bh` | | `0233h` |
//! | 25–29 | 5 | `20h` | 6, 7, `0Bh`, `0Ch`, `0Ah` | 0 | `06FDh` |
//! | 30, 32, 34 | 4 | 2 | class 1, 2, 7 | — | `0318h` |
//! | 31, 33 | 0 | 1 | class 1, 2 | — | `030Ah` |
//! | 35 | 0 | 1 | class 7 | — | `0326h` |
//! | 36 | 2 | 4 | class 5 | — | `0334h` |
//! | 37 | 6 | 4 | class 5 | — | `037Dh` |
//! | 38 | 4 | 4 | class 5 | — | `03BBh` |
//! | 39 | 3 | 4 | any | — | `04AEh` |
//! | 40 | 2 | 4 | any | — | `0518h` |
//! | 41 | 7 | 4 | any | — | `05F6h` |
//! | 42 | 4 | 4 | any | — | `068Fh` |
//! | 43 | 6 | 4 | any | — | `0733h` |
//! | 44 | 1 | 4 | any | — | `0404h` |
//! | 45 | 9 | `20h` | any | any | `0718h` |
//! | 46 | `0Ch` | 4 | any | — | `07D2h` |
//! | 47 | 0 | 1 | any | — | `083Ah` |
//!
//! A class is stored complemented (`FFFEh` is class 1; the matcher's `not
//! bx`, `0x0E4C4`); "any" is `FFFFh`. Every handler is in the segment
//! `0FE3h`, beside the door gate `0FE3h:000Eh` and the script exit's
//! `0FE3h:0848h`; none is the library's, so no row hands on to
//! [`Adventure::options_rule`] — the options' gate `09B6h:0465h` is
//! installed by the dispatcher's key arm only (`0x0AC73`). `0FE3h:02E6h`
//! (`0x10116`) lies among the handlers but no row of this table names it.
//!
//! No handler reads the phase it is called with (`push dx`, `0x0E5B2`);
//! `02B1h` hands it on to `0257h`, which does not read it either.
//!
//! Scenes are pushed as the complemented words the program pushes:
//! `push 0FFF6h` is `!9`. The rolls hand `203Dh:0024h` (`0x203F4`) the
//! bounds as pushed, the last pushed first, and the roll itself is the
//! scene; the routine compares them unsigned and answers `lo` without
//! rolling for `lo >= hi` (`cmp ax, [bp+8] ; jnc`, `0x203FC`), which
//! [`Api::random`] does too.
//!
//! Addresses are those of the unpacked `XP01.EXE`, `DGROUP` `250Ah`; the
//! comments give image addresses (`0FE3h:x` is `0xFE30 + x`).

use super::build::WALKER;
use crate::num;
use crate::titles::parsec1994::{Adventure, Api, Far};

/// The segment of the global rule handlers.
const RULES: u16 = 0x0FE3;

/// `[2D38h]`, the mode block's verb (the matcher's `mov cx, [2D38h]`,
/// `0x0E4A4`).
const VERB: u16 = 0x2D38;
/// `[2D3Ch]`, the object (`mov bx, [2D3Ch]`, `0x0E4B3`).
const OBJECT: u16 = 0x2D3C;
/// `[2D3Eh]`, its class (`mov bx, [2D3Eh]`, `0x0E4BC`).
const OBJECT_CLASS: u16 = 0x2D3E;
/// `[2D4Ah]`, the node the figure walks to.
const WALK_TARGET: u16 = 0x2D4A;
/// `[2D66h]`, `[2D68h]`: the gate the mode machine asks (`call far
/// [2D66h]`, `0x0A1C5`).
const GATE_OFF: u16 = 0x2D66;
const GATE_SEG: u16 = 0x2D68;
/// `[2D6Ch]`: the animation code a door's rule waits for.
const ANIM_WAIT: u16 = 0x2D6C;
/// `[1F3Ah]`: the door the location was entered through.
const ENTRY_DOOR: u16 = 0x1F3A;
/// `[1796h]`: the player record's `+28h` (`DS:176Eh + 28h`), read by the
/// walk rule (`mov ax, [1796h]`, `0x1061A`) where Captain Zins's reads its
/// `[1204h]`.
const WALKER_28: u16 = 0x1796;

/// Story bytes, all inside the logic's saved block `DS:2B98h` (`4Eh`
/// bytes): `[2B9Bh]` counts the class-1 door's first opening (`0x100C1`,
/// `0x100D0`), `[2BA2h]` lets the class-2 door open (`0x100E4`), `[2BACh]`
/// is set by the rule of item `1Ch` (`0x0FF59`–`0x0FF60`).
const BYTE_2B9B: u16 = 0x2B9B;
const BYTE_2BA2: u16 = 0x2BA2;
const BYTE_2BAC: u16 = 0x2BAC;

/// The first id past the items, `30h` (`cmp word [2D3Ch], 30h`: `0x10265`,
/// `0x102E1`, `0x1037A`, `0x104F2`, `0x10594`).
const ITEMS_END: u16 = 0x30;

/// The two per-item tables in the rules' segment, `30h` bytes each,
/// indexed by the object: `2336h:00D0h` (image `0x23430`, `mov al,
/// es:[bx+0D0h]`, `0x10502`) and `2336h:0100h` (image `0x23460`, `es:[bx+
/// 100h]`, `0x105A4`), the second ending where the rule table begins.
const TABLES: u16 = 0x2336;
const TABLE_D0: u16 = 0x00D0;
const TABLE_100: u16 = 0x0100;

/// A field of a far record.
fn at(rec: Far, field: u16) -> Far {
    Far {
        off: rec.off.wrapping_add(field),
        seg: rec.seg,
    }
}

/// A global rule read against [`Api`], by its handler's far address;
/// `None` for one that is not among them. `flags` is the phase the search
/// runs with, which none of them reads.
pub(crate) fn api_rule(api: &mut dyn Api, far: (u16, u16), _flags: u16) -> Option<i16> {
    if far.0 != RULES {
        return None;
    }
    Some(match far.1 {
        // Row 0, 0xFF42: item 1Ch carried plays place 9 and sets [2BACh]
        // once; else the search goes on (0x0FF6A).
        0x0112 => {
            if api.inv_has(0x1C) == 0 {
                // 0x0FF47
                return Some(0);
            }
            api.scene_play(!9); // 0x0FF53
            if api.byte(BYTE_2BAC) == 0 {
                api.set_byte(BYTE_2BAC, 1); // 0x0FF60
            }
            0x0A
        }
        // Row 1, 0xFF7E: place 7, item 19h given up, 17h for 18h.
        0x014E => {
            api.scene_play(!7); // 0x0FF83
            api.inv_take(0x19); // 0x0FF8B
            swap(api, 0x17, 0x18); // 0x0FF95
            0x0A
        }
        // Row 2, 0xFF6E: place 8.
        0x013E => scene(api, !8), // 0x0FF73
        // Rows 3–5, 0xFFA2: the verb becomes 0Ah, answer 5.
        0x0172 => verb(api, 0x0A, 5),
        // Row 6, 0x10087: the class-1 door.
        0x0257 => door(api),
        // Row 7, 0x100E1: the class-2 door.
        0x02B1 => door_2(api),
        // Rows 8 and 9, 0x1046F: line 4, else place 74 for a live object,
        // 73 for one that is not (0x104A4).
        0x063F => {
            if !own_line(api, 4) {
                let live = api.obj_state(api.word(OBJECT)) != 0; // 0x104A4
                api.scene_play(if live { !0x4A } else { !0x49 }); // 0x104B4
            }
            0x0A
        }
        // Row 10, 0xFFDC: place 2.
        0x01AC => scene(api, !2), // 0x0FFE1
        // Row 11, 0xFFB0: place 6.
        0x0180 => scene(api, !6), // 0x0FFB5
        // Row 12, 0xFFC0: place 13, item 22h for 19h.
        0x0190 => {
            api.scene_play(!0x0D); // 0x0FFC5
            swap(api, 0x22, 0x19); // 0x0FFCF
            0x0A
        }
        // Rows 13 and 14, 0x10007: place 3, item 6 for 7.
        0x01D7 => {
            api.scene_play(!3); // 0x1000C
            swap(api, 6, 7); // 0x10016
            0x0A
        }
        // Rows 15–17 and 20, 0xFFEC: one of places 33 and 32, rolled.
        0x01BC => roll(api, 0xFFDE, 0xFFDF), // 0x0FFF3
        // Rows 18 and 19, 0x10023: place 4, item 0Bh for 0Ch.
        0x01F3 => {
            api.scene_play(!4); // 0x10028
            swap(api, 0x0B, 0x0C); // 0x10032
            0x0A
        }
        // Rows 21 and 22, 0x1003F: item 0Ch given up, 6 for 8, place 5.
        0x020F => {
            api.inv_take(0x0C); // 0x10044
            swap(api, 6, 8); // 0x1004E
            scene(api, !5) // 0x10058
        }
        // Rows 23 and 24, 0x10063: item 0Bh given up, 7 for 8, place 5.
        0x0233 => {
            api.inv_take(0x0B); // 0x10068
            swap(api, 7, 8); // 0x10072
            scene(api, !5) // 0x1007C
        }
        // Rows 25–29 (0x1052D) and 45 (0x10548): one of places 36 to 34,
        // rolled; the two handlers are the same code.
        0x06FD | 0x0718 => roll(api, 0xFFDB, 0xFFDD), // 0x10534, 0x1054F
        // Rows 30, 32, 34, 0x10148: the verb becomes 0Ch, answer 5.
        0x0318 => verb(api, 0x0C, 5),
        // The hover rules, rows 31, 33 (0x1013A) and 35 (0x10156): verb
        // 0Ch, answer 1; the two handlers are the same code.
        0x030A | 0x0326 => verb(api, 0x0C, 1),
        // Row 36, 0x10164: line 5, else one of places 53 and 52, rolled.
        0x0334 => line_or_roll(api, 5, (0xFFCA, 0xFFCB)), // 0x10175, 0x10199
        // Row 37, 0x101AD: line 1, else place 39.
        0x037D => {
            if !own_line(api, 1) {
                // 0x101BE
                api.scene_play(!0x27); // 0x101E0
            }
            0x0A
        }
        // Row 38, 0x101EB: line 2, else one of places 27 and 26, rolled.
        0x03BB => line_or_roll(api, 2, (0xFFE4, 0xFFE5)), // 0x101FC, 0x10220
        // Row 39, 0x102DE.
        0x04AE => rule_04ae(api),
        // Row 40, 0x10348.
        0x0518 => take(api),
        // Row 41, 0x10426: line 4, else one of places 31 to 29, rolled.
        0x05F6 => line_or_roll(api, 4, (0xFFE0, 0xFFE2)), // 0x10437, 0x1045B
        // Row 42, 0x104BF.
        0x068F => rule_068f(api),
        // Row 43, 0x10563.
        0x0733 => rule_0733(api),
        // Row 44, 0x10234.
        0x0404 => rule_0404(api),
        // Row 46, 0x10602: the walk verb's rule.
        0x07D2 => walk_rule(api),
        // Row 47, 0x1066A: verb 1, answer 1.
        0x083A => verb(api, 1, 1),
        _ => return None,
    })
}

/// Every global rule of this program, by its handler's far address, for
/// the runtime; `None` for one this engine does not read. Every row is in
/// `0FE3h` and none is the library's own, so all go to [`api_rule`].
pub(crate) fn rule(adv: &mut Adventure, far: (u16, u16), flags: u16) -> Option<i16> {
    api_rule(adv, far, flags)
}

/// `mov word [2D38h], verb ; mov ax, answer`.
fn verb(api: &mut dyn Api, verb: u16, answer: i16) -> i16 {
    api.set_word(VERB, verb);
    answer
}

/// One scene (`080Ch:164Ch`), then the answer `0Ah`.
fn scene(api: &mut dyn Api, n: i16) -> i16 {
    api.scene_play(n);
    0x0A
}

/// `push hi ; push lo ; call far 203Dh:0024h ; push ax ; call far
/// 080Ch:164Ch`: the roll is the scene. Answer `0Ah`.
fn roll(api: &mut dyn Api, lo: u16, hi: u16) -> i16 {
    let n = num::signed(api.random(lo, hi));
    scene(api, n)
}

/// `0751h:0A43h(a, b)` (`0x07F53`): two near calls into the same segment,
/// `0751h:099Dh(a)` (`0x07F5A`, [`Api::inv_take`]) and then
/// `0751h:090Eh(b)` (`0x07F62`, [`Api::inv_give`]); its answer, the
/// give's, is read by no caller here.
fn swap(api: &mut dyn Api, a: u16, b: u16) {
    api.inv_take(a);
    api.inv_give(b);
}

/// The chosen hotspot's line in slot `k`, played when there is one: the
/// record `[2D5Eh]` tested for null, then the far pointer at its `+16h +
/// 4k`, then `080Ch:17D9h` (e.g. `0x10238`–`0x1025B` for `k = 3`). The
/// writers of `[2D5Eh]` are Captain Zins's of `[1E7Eh]` (`0x0A5FD`–
/// `0x0A628` against `0x0AB00`–`0x0AB2B`, ratio 1.0), and the handlers'
/// test is Captain Zins's (`0x101AD`–`0x101EB` against its `0x10FA6`–
/// `0x10FE4`, ratio 1.0).
///
/// `080Ch:17D9h` (`0x09899`) is Dunkle Schatten's `0CFDh:18C2h` but for one
/// store: it leaves task 0's speakers as they are, where Dunkle Schatten's
/// sets them to 1 (`0x0E8BC`) — the runtime reads which from the routine
/// ([`crate::titles::parsec1994::profile::Profile::own_line_sets_speakers`]).
fn own_line(api: &mut dyn Api, k: u16) -> bool {
    let text = api.hotspot_own_text(k);
    if text == Far::NULL {
        return false;
    }
    api.say_text(text);
    true
}

/// Line `k`, else the roll `lo..=hi` as the scene. Answer `0Ah`.
fn line_or_roll(api: &mut dyn Api, k: u16, (lo, hi): (u16, u16)) -> i16 {
    if own_line(api, k) {
        return 0x0A;
    }
    roll(api, lo, hi)
}

/// Whether the object is an item, `0 < id < 30h` with the signed tests
/// `jnl` and `jng` (`0x1026A`, `0x10271`).
fn is_item(id: u16) -> bool {
    (1..num::signed(ITEMS_END)).contains(&num::signed(id))
}

/// `0FE3h:0257h` (0x10087), verb 6 on class 1: a live object plays place
/// 76 (`0x1009A`). Anything else has the object's record (`06E3h:047Eh`,
/// `0x100AB`, not tested for null) set on animation 1 (`03F7h:1CF9h`,
/// `0x100B3`; the `push 1` before the find is its second argument) and
/// becomes the entry door (`0x100BE`); the first time (`[2B9Bh] == 0`) place
/// 0 plays and the answer is `0Ah` (`0x100CA`–`0x100D4`), after that
/// `[2D6Ch] = 2` and the answer is 12 (`0x100D6`).
fn door(api: &mut dyn Api) -> i16 {
    if api.obj_state(api.word(OBJECT)) != 0 {
        // 0x1008E
        return scene(api, !0x4C);
    }
    let rec = api.obj_find(api.word(OBJECT)); // 0x100AB
    api.obj_anim(rec, 1); // 0x100B3
    let obj = api.word(OBJECT);
    api.set_word(ENTRY_DOOR, obj); // 0x100BE
    if api.byte(BYTE_2B9B) == 0 {
        // 0x100C1
        api.scene_play(!0); // 0x100CA
        api.set_byte(BYTE_2B9B, 1); // 0x100D0: `inc` from 0
        return 0x0A;
    }
    api.set_word(ANIM_WAIT, 2); // 0x100D6
    0x0C
}

/// `0FE3h:02B1h` (0x100E1), verb 6 on class 2: with `[2BA2h] == 1` the
/// class-1 door's rule (`push cs ; call 0257h`, `0x100EF`); else sample 8
/// (`09B6h:184Dh`, `0x100F7`), place 1 (`0x100FF`), the gate cleared
/// (`0x10105`, `0x1010B`), answer `0Ah`.
fn door_2(api: &mut dyn Api) -> i16 {
    if api.byte(BYTE_2BA2) == 1 {
        return door(api);
    }
    api.play_sample(8);
    api.scene_play(!1);
    api.set_word(GATE_SEG, 0);
    api.set_word(GATE_OFF, 0);
    0x0A
}

/// `0FE3h:04AEh` (0x102DE), verb 3 on anything: a carried item rolls
/// places 60 and 59 (`0x102FD`); class 5 plays place 12 (`0x1030A`);
/// class 1 or 2 not live rolls 62 and 61 (`0x1032A`); anything else 64
/// and 63 (`0x10330`). Answer `0Ah`.
fn rule_04ae(api: &mut dyn Api) -> i16 {
    let id = api.word(OBJECT);
    if is_item(id) && api.inv_has(id) != 0 {
        // 0x102E1–0x102FB
        return roll(api, 0xFFC3, 0xFFC4);
    }
    let class = api.word(OBJECT_CLASS);
    if class == 5 {
        // 0x10303
        return scene(api, !0x0C);
    }
    if (class == 1 || class == 2) && api.obj_state(api.word(OBJECT)) == 0 {
        // 0x1030E, 0x10315, 0x10320
        return roll(api, 0xFFC1, 0xFFC2);
    }
    roll(api, 0xFFBF, 0xFFC0)
}

/// `0FE3h:0518h` (0x10348), verb 2 on anything: the take.
///
/// Line 5 (`+2Ah`, `0x10359`) first. An item carried plays place 48
/// (`0x10396`). An item not carried: its record (`06E3h:047Eh`,
/// `0x1039F`); none, and the answer is `0Ah` with nothing played
/// (`0x103AE`). Else — no live-bit test, unlike Captain Zins's — the
/// figure's turn 3 (`05DCh:0FF2h`, `0x103B2`), sample `0Ah` (`09B6h:184Dh`,
/// `0x103BA`), the item given (`0751h:090Eh`, `0x103C4`), bit 0 of the
/// record's `+2` cleared (`0x103CD`), one mark for the drawer
/// (`03F7h:0001h`, `0x103D6`; Dunkle Schatten's `0883h:000Fh`, ratio 1.0,
/// 105 instructions), then, for a chosen hotspot, bit 0 of its word `+4`
/// cleared and the pointer's shape taken again (`1819h:1297h`,
/// `0x103F0`; Dunkle Schatten's `179Eh:1017h`, ratio 1.0, 20
/// instructions), and the answer is 1 (`0x103F5`). Anything not an item:
/// class 1 or 2 rolls places 44 to 42 (`0x10408`), anything else 50 and 49
/// (`0x1040E`); answer `0Ah`.
fn take(api: &mut dyn Api) -> i16 {
    if own_line(api, 5) {
        return 0x0A;
    }
    let id = api.word(OBJECT);
    if !is_item(id) {
        let class = api.word(OBJECT_CLASS);
        return if class == 1 || class == 2 {
            // 0x103FA, 0x10401
            roll(api, 0xFFD3, 0xFFD5)
        } else {
            roll(api, 0xFFCD, 0xFFCE)
        };
    }
    if api.inv_has(id) != 0 {
        // 0x1038C
        return scene(api, !0x30);
    }
    let rec = api.obj_find(api.word(OBJECT)); // 0x1039F
    if rec == Far::NULL {
        return 0x0A; // 0x103AE
    }
    api.figure_turn(3); // 0x103B2
    api.play_sample(0x0A); // 0x103BA
    api.inv_give(api.word(OBJECT)); // 0x103C4
    let flags = api.far_byte(at(rec, 2));
    api.set_far_byte(at(rec, 2), flags & 0xFE); // 0x103CD
    api.obj_invalidate(rec); // 0x103D6
    // 0x103DE–0x103F0: the chosen hotspot's active bit cleared, and the
    // pointer's shape taken again, when there is one; `mov ax, 1`
    // (0x103F5).
    if api.chosen_hotspot_off() {
        api.pointer_shape_resync();
    }
    1
}

/// `0FE3h:068Fh` (0x104BF), verb 4 on anything: line 2 (`+1Eh`,
/// `0x104D0`), else for an object below `30h` (unsigned, `jnc`, `0x104F7`;
/// id 0 included) its byte in the table `2336h:00D0h` names the place
/// (`not ax`, `0x10511`); a zero byte or any other object rolls places 31
/// to 29 (`0x10515`). Answer `0Ah`.
fn rule_068f(api: &mut dyn Api) -> i16 {
    if own_line(api, 2) {
        return 0x0A;
    }
    let id = api.word(OBJECT);
    let place = if id < ITEMS_END {
        api.far_byte(Far {
            off: TABLE_D0.wrapping_add(id),
            seg: TABLES,
        }) // 0x10502
    } else {
        0 // 0x104F0
    };
    if place != 0 {
        scene(api, !num::signed(u16::from(place))) // 0x1050F
    } else {
        roll(api, 0xFFE0, 0xFFE2)
    }
}

/// `0FE3h:0733h` (0x10563), verb 6 on anything: line 1 (`+1Ah`,
/// `0x10574`), else for an object below `30h` (unsigned, `0x10599`) with a
/// byte in the table `2336h:0100h`: 1 rolls places 41 and 40 (`0x105C6`),
/// 2 places 44 to 42 (`0x105CA`), 3 places 47 to 45 (`0x105D0`), any other
/// byte names the place (`0x105E6`). A zero byte or any other object hands
/// the roll `lo = FFD6h`, `hi = FFD5h` (`0x105EA`–`0x105EE`), which answers
/// `lo` unrolled: place 41. Answer `0Ah`.
fn rule_0733(api: &mut dyn Api) -> i16 {
    if own_line(api, 1) {
        return 0x0A;
    }
    let id = api.word(OBJECT);
    let v = if id < ITEMS_END {
        api.far_byte(Far {
            off: TABLE_100.wrapping_add(id),
            seg: TABLES,
        }) // 0x105A4
    } else {
        0
    };
    match v {
        0 => roll(api, 0xFFD6, 0xFFD5),
        1 => roll(api, 0xFFD6, 0xFFD7),
        2 => roll(api, 0xFFD3, 0xFFD5),
        3 => roll(api, 0xFFD0, 0xFFD2),
        v => scene(api, !num::signed(u16::from(v))),
    }
}

/// `0FE3h:0404h` (0x10234), verb 1 on anything: line 3 (`+22h`,
/// `0x10245`); else an item, `0 < id < 30h`, plays `~(id + 3FFFh)`
/// (`add ax, 3FFFh ; not ax`, `0x10276`) — inventory place `id − 1` to a
/// lookup that reads `~n − 4000h`; class 1 plays place 68 live or 69 not
/// (`0x1027D`–`0x10297`); class 2 place 70 live or 71 not
/// (`0x1029C`–`0x102B6`); class 7 place 72 (`0x102BB`); anything else
/// rolls places 67 to 65 (`0x102C6`). Answer `0Ah`.
fn rule_0404(api: &mut dyn Api) -> i16 {
    if own_line(api, 3) {
        return 0x0A;
    }
    let id = api.word(OBJECT);
    if is_item(id) {
        return scene(api, !(num::signed(id).wrapping_add(0x3FFF)));
    }
    match api.word(OBJECT_CLASS) {
        1 => {
            let live = api.obj_state(api.word(OBJECT)) != 0; // 0x10288
            scene(api, if live { !0x44 } else { !0x45 })
        }
        2 => {
            let live = api.obj_state(api.word(OBJECT)) != 0; // 0x102A7
            scene(api, if live { !0x46 } else { !0x47 })
        }
        7 => scene(api, !0x48),
        _ => roll(api, 0xFFBC, 0xFFBE),
    }
}

/// `0FE3h:07D2h` (0x10602), the walk verb's rule, Captain Zins's
/// `10C3h:03B4h` in shape (ratio 1.0, 36 instructions). It installs the
/// door gate `0FE3h:000Eh` first (`0x10605`, `0x1060B`). The door the
/// location was entered through, while the figure does not stand at it
/// (`0x10611`–`0x10621`), and an object not live (`0x10627`) are refused:
/// verb 6, answer −1 (`0x1065A`). Anything else the figure walks to (node
/// `obj + 7D0h`, `05DCh:05C8h`, `0x1063E`); if it starts, the script takes
/// the input (`09B6h:17D4h`, `0x1064A`) and the node is the walk's target
/// (`0x10655`). The answer is 9.
fn walk_rule(api: &mut dyn Api) -> i16 {
    api.set_word(GATE_SEG, RULES); // 0x10605
    api.set_word(GATE_OFF, 0x000E); // 0x1060B
    let obj = api.word(OBJECT);
    let refused = (api.word(ENTRY_DOOR) == obj && api.word(WALKER_28) != obj)
        || api.obj_state(api.word(OBJECT)) == 0; // 0x10627
    if refused {
        api.set_word(VERB, 6); // 0x1065A
        return -1; // 0x10660
    }
    let node = api.word(OBJECT).wrapping_add(0x07D0); // 0x10636
    let player = api.ds(WALKER);
    if api.walker_walk_to(player, node, -1) != 0 {
        // 0x1063E, 0x10646
        api.input_lock(); // 0x1064A
        let node = api.word(OBJECT).wrapping_add(0x07D0); // 0x10652
        api.set_word(WALK_TARGET, node); // 0x10655
    }
    9 // 0x10665
}

/// `[2D46h]`: the mode machine's wait bits.
const BITS: u16 = 0x2D46;
/// The location table, `44h` bytes a row: the sixteen exits' ids at `+9`,
/// their destinations at `+19h`, the kind at `+7` (`0x0FEB7`–`0x0FF02`).
const LOCATIONS: u16 = 0x2181;
const STRIDE: u16 = 0x44;
/// `cmp si, 5Ch` (`0x0FEF3`): the destinations inside the table.
const ROWS: u16 = 0x5C;
/// `[200Ah]`, `[200Ch]`, `[200Eh]`: the main loop's entry, location and
/// the location running.
const ENTRY: u16 = 0x200A;
const LOCATION: u16 = 0x200C;
const HERE: u16 = 0x200E;

/// `0FE3h:000Eh` (`0x0FE3E`), the door's gate, which the walk verb's rule
/// installs (row 46). Dunkle Schatten's `1586h:0000h` in shape, with this
/// program's own turns:
///
/// - asked with anything but 0 it answers 1 (`0x0FE44`);
/// - the input taken by the script (`09B6h:17D4h`, `0x0FE4D`) — no hover
///   off, where Dunkle Schatten's calls `0EB9h:18F4h` — and the verb the
///   walk verb, 0Ch (`0x0FE52`);
/// - not yet walking to a twin (`[2D4Ah] < 7D0h`, `0x0FE58`): the figure
///   walks to the object's twin (`0x0FE6D`); when that started, the twin is
///   the walk's target (`0x0FE7F`), the wait bits cleared (`0x0FE82`), and
///   for a door 64h to 6Dh the wait is animation code 3, bit `100h`
///   (`0x0FE96`, `0x0FE9C`), for anything else the figure, bit `80h`
///   (`0x0FEA4`); the answer is 0;
/// - else the exit among the running location's sixteen (`0x0FEB7`): a
///   destination inside the table (`0x0FEEF`, `0x0FEF3`) of kind 0 gives
///   the input back (`09B6h:17E2h`, `0x0FF10`) — no hover on, where Dunkle
///   Schatten's calls `0EB9h:1917h`; of any other kind, item 2Dh carried
///   becomes 2Ch (`0x0FF17`–`0x0FF2D`), and the exit and its destination go
///   into the main loop's words (`0x0FF33`, `0x0FF37`). The answer is 1.
pub(crate) fn door_gate(api: &mut dyn Api, arg: i16) -> i16 {
    if arg != 0 {
        return 1;
    }
    api.input_lock();
    api.set_word(VERB, 0x0C);
    let obj = api.word(OBJECT);
    if num::signed(api.word(WALK_TARGET)) < 0x07D0 {
        let player = api.ds(WALKER);
        let twin = obj.wrapping_add(0x07D0);
        if api.walker_walk_to(player, twin, -1) > 0 {
            api.set_word(WALK_TARGET, twin);
            let bits = api.word(BITS) & 0xFE0F;
            let door = (0x64..=0x6D).contains(&num::signed(obj));
            if door {
                api.set_word(ANIM_WAIT, 3);
            }
            api.set_word(BITS, bits | if door { 0x100 } else { 0x80 });
            return 0;
        }
    }
    let here = api.word(HERE);
    let row = here.wrapping_mul(STRIDE);
    let byte = |api: &mut dyn Api, off: u16| {
        u16::from(api.far_byte(Far {
            seg: LOCATIONS,
            off: row.wrapping_add(off),
        }))
    };
    let dest = (0..16u16)
        .find(|&k| byte(api, 9 + k) == obj)
        .map_or(0, |k| byte(api, 0x19 + k));
    if dest == 0 || dest >= ROWS {
        return 1;
    }
    let kind = api.far_word(Far {
        seg: LOCATIONS,
        off: dest.wrapping_mul(STRIDE).wrapping_add(7),
    });
    if kind == 0 {
        api.input_unlock();
        return 1;
    }
    if api.inv_has(0x2D) != 0 {
        api.inv_take(0x2D);
        api.inv_give(0x2C);
    }
    api.set_word(ENTRY, obj);
    api.set_word(LOCATION, dest);
    1
}
