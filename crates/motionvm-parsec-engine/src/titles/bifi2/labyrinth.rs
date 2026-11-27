//! Locations 40, 42 to 45 and 47 to 90, the labyrinth: 47 rows of kind 4
//! that share one handler and one rules table. The location table names
//! two of them — row 69 `Ausgang`, whose fourth exit `71h` leads to
//! location 3, and row 79 `Skelett` — and leaves the rest unnamed, so the
//! module has a name of its own.
//!
//! Re-expressed from the handler at `1746h:0226h` (image `0x17686` to its
//! `retf` at `0x1773D`), its helpers `1746h:00A4h` (`0x17504`), `00F6h`
//! (`0x17556`), `0188h` (`0x175E8`) and `01FEh` (`0x1765E`), the three
//! rule handlers `1746h:0006h`, `0016h` and `006Eh` (`0x17466`–`0x17503`)
//! and row 79's hook `1746h:02DEh` (`0x1773E` to its `retf` at
//! `0x17761`), against [`Api`] alone. Every other row's `+3Fh` is null,
//! and every row's `+43h`, so those rows have the library's empty hook and
//! all of them the empty animation hook. The rules are the three records
//! at `2446h:0BA4h` (image `0x25004`, every row's `+3Bh`), fourteen bytes
//! each:
//!
//! | verb | phase | object | second | handler |
//! |---|---|---|---|---|
//! | 1 | 4 | `1Ah` | — | `0006h` |
//! | 1 | 4 | `1Bh` | — | `0016h` |
//! | 1 | 4 | `40h` | — | `006Eh` |
//!
//! Verb 1 is `Betrachte` by the verb text row at `DS:116Ah`.
//!
//! The labyrinth keeps a byte per location at `2446h:0B48h + n`, outside
//! `DGROUP`. While row 40's byte is still the image's `58h` the handler
//! lays the labyrinth out: rows 40 to 90 get `FEh`, row 41 (Schlönstadt's
//! Gefängnis) gets 1, and each row in `28h`..=`5Ah` reached through the
//! destinations `+19h` of exits 0 to 3 gets one more than the row it is
//! reached from, depth first. Then each of rows 40 to 90 gets, instead,
//! the first of its four exits that leads to a row one closer, or 4. The
//! layout reads the location table alone, so it comes out the same on
//! every run. Rule `0016h` plays the scene the way names (the bytes at
//! `DS:18ACh`) and walks the player out through that exit.
//!
//! Row 79 `Skelett`, until `[2BCFh]` is set, takes the input and sends
//! the player to node `40h` as `Betrachte` on object `40h`, so that rule
//! `006Eh` follows on arrival: scene 0, the input back, `[2BCFh]` set and
//! `[2BCEh]` raised to 3. Row 79's hook gives item `1Ah` (`Stadtplan` by
//! the item row at `DS:10AAh`) on the item event `12h`.
//!
//! The comparison with *Dunkle Schatten*'s forest maze (`04B0h:0223h`):
//! the distance step and the spread are the same code (`0x17504` against
//! `0x04B59`, one `jnl` become `jg` for the inclusive bound; `0x17556`
//! against `0x04BAB`, instruction for instruction), with four exits
//! counted from 0 instead of from 1; the routes drop the forest's three
//! special cases and stop at 4 (`jl` for `jng`); the layout has one way
//! in (row 41) instead of three. The handler has neither the forest's
//! scene every so many visits nor its sounds, nor its animation of object
//! `D7h`; it has the Skelett's walk instead.

use super::build::WALKER;
use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// The handler's segment, for the rules' far pointers.
pub(super) const SEGMENT: u16 = 0x1746;

/// `DS:2018h`: the object state rows, 32 bytes a location (`0x17726`).
const OBJECT_STATES: u16 = 0x2018;

/// `2446h`: the segment of the labyrinth's per-location bytes.
const MAZE_SEG: u16 = 0x2446;
/// `2446h:0B48h + n`: location `n`'s byte.
const BY_LOCATION: u16 = 0x0B48;
/// `2446h:0B70h`: the bytes of rows 40 onwards, row 40's first.
const ROWS: u16 = 0x0B70;
/// `2446h:0B71h`: row 41's byte.
const GOAL_BYTE: u16 = 0x0B71;
/// Row 41, the way the distances spread from (`push 29h`, `0x1767C`).
const GOAL: u16 = 0x29;
/// Row 40's byte before the first layout (`cmp byte es:[0B70h], 58h`,
/// `0x176A0`).
const UNLAID: u8 = 0x58;
/// The bytes the layout sets (`push 33h`, `0x1765E`) and the routes copy
/// (`push 34h`, `0x17645`).
const LAID: u16 = 0x33;
const COPIED: usize = 0x34;

/// `2181h`: the location table, `44h` bytes a row.
const TABLE_SEG: u16 = 0x2181;
const STRIDE: u16 = 0x44;
/// A row's exit ids from `+9`, their destinations from `+19h`.
const EXITS: u16 = 0x09;
const DESTINATIONS: u16 = 0x19;
/// The rows the distances run over (`cmp si, 28h ; jl`, `cmp si, 5Ah ;
/// jg`, `0x17522`–`0x1752A`).
const FIRST: u16 = 0x28;
const LAST: u16 = 0x5A;
/// The exits a row has for the labyrinth (`cmp dx, 4 ; jl`, `0x17636`).
const WAYS: u16 = 4;

/// `[2F9Ah]`: the last row the step answered, written only (`0x17573`).
const LAST_STEP: u16 = 0x2F9A;
/// `DS:18ACh`: the scene each of the four ways plays (`mov al,
/// [si+18ACh]`, `0x174A9`).
const WAY_SCENES: u16 = 0x18AC;
/// The doors rule `0016h` walks out by (`cmp di, 6Eh ; jl`, `cmp di, 71h
/// ; jg`, `0x174B6`–`0x174BE`).
const FIRST_DOOR: u16 = 0x6E;
const LAST_DOOR: u16 = 0x71;

/// Row 79, the Skelett (`cmp word [200Ch], 4Fh`, `0x176AB`).
const SKELETT: u16 = 0x4F;
/// Object `40h`, which the Skelett's walk goes to and rule `006Eh` names.
const OBJECT_40: u16 = 0x40;
/// The item row 79's hook gives (`push 1Ah`, `0x17758`).
const STADTPLAN: u16 = 0x1A;
/// The item event that gives it (`cmp ax, 12h`, `0x17753`).
const EVENT_12: u16 = 0x12;

/// The logic's state bytes (inside the saved block at `DS:2B98h`).
/// `[2BCFh]`: set once rule `006Eh` has played the Skelett's scene.
const BYTE_2BCF: u16 = 0x2BCF;
/// `[2BCEh]`: a state 0 to 6 that segment `109Ah` and the Requisite
/// (location 16) move too; rule `006Eh` raises it to 3.
const BYTE_2BCE: u16 = 0x2BCE;

/// The mode block: the verb `[2D38h]`, the mode `[2D3Ah]`, the object
/// `[2D3Ch]`, the two-object flag `[2D44h]` (the word the mode reset
/// clears where *Dunkle Schatten*'s clears `[2899h]`, `0x09BA8`), the
/// wait bits `[2D46h]`, the verb as chosen `[2D48h]`, the walk target
/// `[2D4Ah]`.
const VERB: u16 = 0x2D38;
const MODE: u16 = 0x2D3A;
const OBJECT: u16 = 0x2D3C;
const TWO_OBJECT: u16 = 0x2D44;
const BITS: u16 = 0x2D46;
const VERB_CHOSEN: u16 = 0x2D48;
const WALK_TARGET: u16 = 0x2D4A;
/// Verb 1, `Betrachte`.
const VERB_BETRACHTE: u16 = 1;

fn maze(off: u16) -> Far {
    Far { off, seg: MAZE_SEG }
}

fn row_byte(api: &mut dyn Api, at: u16) -> u8 {
    api.far_byte(Far {
        off: at,
        seg: TABLE_SEG,
    })
}

/// `1746h:00A4h(n, k, d)` (`0x17504`): the destination of exit `k` of
/// location `n`, if it is a labyrinth row whose byte is above `d + 1`,
/// gets `d + 1` and is answered; else 0.
fn step(api: &mut dyn Api, n: u16, k: u16, d: u16) -> u16 {
    let at = n.wrapping_mul(STRIDE).wrapping_add(k);
    let m = u16::from(row_byte(api, at.wrapping_add(DESTINATIONS))); // 0x17519
    if !(FIRST..=LAST).contains(&m) {
        return 0; // 0x17551
    }
    let there = u16::from(api.far_byte(maze(BY_LOCATION.wrapping_add(m)))); // 0x17531
    if there <= d.wrapping_add(1) {
        return 0; // 0x1753D
    }
    api.set_far_byte(
        maze(BY_LOCATION.wrapping_add(m)),
        num::low8(d).wrapping_add(1),
    ); // 0x17548
    m
}

/// `1746h:00F6h(n)` (`0x17556`): the distances spread from location `n`
/// through its exits 0 to 3, depth first; the four calls are written out
/// one after the other (`0x1756D`, `0x1758F`, `0x175B1`, `0x175D3`).
fn spread(api: &mut dyn Api, n: u16) {
    for k in 0..WAYS {
        let d = u16::from(api.far_byte(maze(BY_LOCATION.wrapping_add(n)))); // 0x17562
        let m = step(api, n, k, d);
        api.set_word(LAST_STEP, m); // 0x17573
        if m != 0 {
            spread(api, m); // 0x1757B
        }
    }
}

/// `1746h:0188h` (`0x175E8`): for each of rows 40 to 90, the first of its
/// four exits that leads to a labyrinth row one step closer, or 4; copied
/// over the rows' bytes.
fn routes(api: &mut dyn Api) {
    // `[bp-36h]`: 34h bytes copied; the routine writes 33h of them, and
    // the last is whatever its stack held (it lands on `2446h:0BA3h`, row
    // 91's byte, which nothing reads).
    let mut buf = [0u8; COPIED];
    for si in (0..LAID).rev() {
        // 0x175F8: the distance one step closer.
        let closer = i16::from(api.far_byte(maze(ROWS.wrapping_add(si)))) - 1;
        let mut dx: u16 = 0;
        while dx < WAYS {
            let at = si.wrapping_mul(STRIDE).wrapping_add(dx);
            // 0x17611: `+0AB9h` is row si + 40's destinations.
            let di = u16::from(row_byte(
                api,
                at.wrapping_add(FIRST * STRIDE + DESTINATIONS),
            ));
            if (FIRST..=LAST).contains(&di) {
                let d = i16::from(api.far_byte(maze(BY_LOCATION.wrapping_add(di)))); // 0x17629
                if d == closer {
                    break; // 0x17633
                }
            }
            dx += 1; // 0x17635
        }
        buf[usize::from(si)] = num::low8(dx); // 0x1763B
    }
    api.movmem_from_local(&buf, maze(ROWS)); // 0x17652
}

/// `1746h:01FEh` (`0x1765E`): the labyrinth laid out, the first time.
fn lay_out(api: &mut dyn Api) {
    api.setmem(maze(ROWS), 0xFE, LAID); // 0x17669
    api.set_far_byte(maze(GOAL_BYTE), 1); // 0x17676
    spread(api, GOAL); // 0x1767E
    routes(api); // 0x17682
}

/// Everything before the frame loop, `0x17686` to `0x17709`.
pub(crate) fn enter(api: &mut dyn Api) {
    api.visit_setup(); // 0x17686
    let entry = api.word(WORDS.entry);
    let loc = api.word(WORDS.location);
    api.visit_enter(loc, entry); // 0x17693
    if api.far_byte(maze(ROWS)) == UNLAID {
        lay_out(api); // 0x176A8
    }
    // 0x176AB: the Skelett, the first time, looks at object 40h.
    if api.word(WORDS.location) == SKELETT && api.byte(BYTE_2BCF) == 0 {
        api.input_lock(); // 0x176B9
        api.set_word(VERB_CHOSEN, VERB_BETRACHTE); // 0x176C1
        api.set_word(VERB, VERB_BETRACHTE); // 0x176C4
        api.set_word(OBJECT, OBJECT_40); // 0x176C7
        api.set_word(TWO_OBJECT, 3); // 0x176CD
        api.set_word(MODE, 4); // 0x176D3
        let bits = api.word(BITS) & 0x01F0;
        api.set_word(BITS, bits); // 0x176D9
        let bits = api.word(BITS) | 0x11;
        api.set_word(BITS, bits); // 0x176DF
        api.set_word(WALK_TARGET, OBJECT_40); // 0x176E9
        let player = api.ds(WALKER);
        api.walker_walk_to(player, OBJECT_40, 0); // 0x176F1, answer not read
    }
    api.visit_finish_entry(); // 0x176F9
    let loc = api.word(WORDS.location);
    api.set_word(WORDS.here, loc); // 0x17701
    api.set_word(WORDS.entry, 0); // 0x17704
}

/// One turn of the frame loop, `0x1770A` to `0x1771E`: no check of its own.
pub(crate) fn frame(api: &mut dyn Api) -> bool {
    api.input_poll(); // 0x1770A
    api.dispatch(); // 0x1770F
    api.frame_pass(); // 0x17714
    api.word(WORDS.entry) == 0 // 0x17719
}

/// Everything after the frame loop, `0x17720` to `0x1773D`.
pub(crate) fn leave(api: &mut dyn Api) {
    let here = api.word(WORDS.here);
    let out = api.ds(OBJECT_STATES.wrapping_add(here << 5));
    api.objs_save_state(out); // 0x1772B
    api.visit_leave(); // 0x17733
    api.slot_unload(); // 0x17738
}

/// The rows without a hook of their own: `+3Fh` is null, and the entry
/// puts the library's `09B6h:1BD2h` (`0x0B732`, `retf`) in its place.
pub(crate) fn no_hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// Row 79's hook, `1746h:02DEh`: code in `[bp+6]`, argument in `[bp+8]`;
/// the task is not read. Code 4 with argument `12h` gives the
/// `Stadtplan`; code 7 lands where every other code does (`0x17749`,
/// `0x1774E`), on the `retf`.
pub(crate) fn hook(api: &mut dyn Api, code: u16, arg: u16, _task: u16) {
    if code == 4 && arg == EVENT_12 {
        // 0x17744, 0x17753
        api.inv_give(STADTPLAN); // 0x1775A, answer not read
    }
}

/// The labyrinth's rule handlers by their offset in segment `1746h`.
pub(crate) fn rule(api: &mut dyn Api, off: u16, _flags: u16) -> Option<i16> {
    Some(match off {
        // 1746:0006 (0x17466), Betrachte object 1Ah: scene 3.
        0x0006 => {
            api.scene_play(3); // 0x1746B
            0x0A // 0x17471
        }
        // 1746:0016 (0x17476), Betrachte object 1Bh: the scene of the way
        // this row's byte names, and out through that exit; answer 0 when
        // the row has none (its byte 4 or above).
        0x0016 => {
            let here = api.word(WORDS.here);
            let way = u16::from(api.far_byte(maze(BY_LOCATION.wrapping_add(here)))); // 0x17484
            let at = here.wrapping_mul(STRIDE).wrapping_add(way);
            let exit = u16::from(row_byte(api, at.wrapping_add(EXITS))); // 0x17497
            if way >= WAYS {
                return Some(0); // 0x174A0, 0x174A5
            }
            let scene = api.byte(WAY_SCENES.wrapping_add(way)); // 0x174A9
            api.scene_play(i16::from(scene)); // 0x174B0
            if (FIRST_DOOR..=LAST_DOOR).contains(&exit) {
                api.script_exit(exit); // 0x174C1
            }
            0x0A // 0x174C7
        }
        // 1746:006E (0x174CE), Betrachte object 40h: the first time scene
        // 0, the input back, [2BCFh] set, [2BCEh] at least 3 and the
        // walk bit cleared, answer 1; after that answer 0.
        0x006E => {
            if api.byte(BYTE_2BCF) != 0 {
                return Some(0); // 0x174D6, 0x17500
            }
            api.scene_play(0); // 0x174DA
            api.input_unlock(); // 0x174E0
            api.set_byte(BYTE_2BCF, 1); // 0x174E5
            if api.byte(BYTE_2BCE) < 3 {
                // 0x174EA
                api.set_byte(BYTE_2BCE, 3); // 0x174F1
            }
            let bits = api.word(BITS) & 0xFFFE;
            api.set_word(BITS, bits); // 0x174F6
            1 // 0x174FB
        }
        _ => return None,
    })
}

/// The entry points of every labyrinth row but 79, as [`super::logic`]
/// calls them.
pub(super) const LOGIC: Logic = Logic {
    enter,
    frame,
    leave,
    hook: no_hook,
    anim_hook: None,
    rule,
};

/// Row 79's: the same handler and rules, with its own hook.
pub(super) const LOGIC_SKELETT: Logic = Logic {
    enter,
    frame,
    leave,
    hook,
    anim_hook: None,
    rule,
};
