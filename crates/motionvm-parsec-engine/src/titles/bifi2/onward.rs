//! Rows 0, 37 and 38, of kind 0: not places, only the walk on to the next
//! row that is one.
//!
//! The rows name `0060h:04BFh` (image `0x00ABF`), and the main loop calls
//! it directly for a row of kind 0 and for a word past the table
//! (`0x0074B`–`0x00768`). It is not the generation's walk on
//! ([`crate::titles::parsec1994::onward`]): this build's routine stops at
//! more kinds than one and does not always enter by `7Fh`.
//!
//! From the row in `[200Ch]` on (`0x004BF`) it reads each row's kind
//! (`+7`, `0x004CF`) and goes through the five-arm table at `cs:0540h`
//! by kind − 1 (`dec bx ; cmp bx, 4 ; ja`, `0x004D4`–`0x004D8`): `0525h`
//! for kinds 1, 2 and 3, `04EDh` (the next row) for kind 4, `04E1h` for
//! kind 5; kinds 0, 6, 7 and above are past the table and take the next
//! row too. Kind 5 is entered at once with entry 0 (`0x004E1`–`0x004EB`).
//! Kinds 1 to 3 are entered by `0x00525`: the row into `[200Ch]`, and as
//! the entry the first exit byte of the row the main loop last ran,
//! `[200Eh]` (`mov al, es:[bx+9]`, `0x00535`–`0x0053C`). Past the last row
//! (`cmp dx, 5Ch`, `0x004EE`) it starts over at row 0 and stops at the
//! first row of kind 1, 2 or 3 below the one it began at
//! (`0x004F8`–`0x00523`) — kind 5 does not stop it there — entering that
//! row, or the one it began at when there is none, by `0x00525`.
//!
//! From row 0 at the start, `[200Ch]` = 0 in the image, that is row 1,
//! the Kaffee, of kind 5, entry 0: how a fresh game begins.

use super::logic::{Logic, WORDS};
use crate::num;
use crate::titles::parsec1994::{Api, Far};

/// `mov ax, 2181h` (`0x004CA`), `imul bx, bx, 44h` (`0x004C7`).
const TABLE: u16 = 0x2181;
const STRIDE: u16 = 0x44;
/// `+7`, the kind (`0x004CF`); `+9`, the first exit (`0x00535`).
const KIND: u16 = 7;
pub(super) const FIRST_EXIT: u16 = 9;
/// `cmp dx, 5Ch` (`0x004EE`).
const ROWS: u16 = 0x5C;

fn kind(api: &mut dyn Api, row: u16) -> u16 {
    api.far_word(row_byte(row, KIND))
}

/// Where the walk from row `from` stops: the row, and `None` for entry 0
/// (a row of kind 5) or `Some(())` for the first exit of the row the main
/// loop last ran.
pub(super) fn walk(kind: &mut dyn FnMut(u16) -> u16, from: u16) -> (u16, Option<()>) {
    let mut n = from;
    // 0x004C5–0x004F1.
    while num::signed(n) < num::signed(ROWS) {
        match kind(n) {
            5 => return (n, None), // 0x004E1–0x004EB
            1..=3 => break,
            _ => n = n.wrapping_add(1),
        }
    }
    // 0x004F3–0x00523: past the end, from row 0 up to the row it began at.
    if num::signed(n) >= num::signed(ROWS) {
        n = 0;
        while num::signed(n) < num::signed(from) && !matches!(kind(n), 1..=3) {
            n = n.wrapping_add(1);
        }
    }
    (n, Some(()))
}

/// The table's row `n`, its byte at `at`.
pub(super) fn row_byte(n: u16, at: u16) -> Far {
    Far {
        seg: TABLE,
        off: n.wrapping_mul(STRIDE).wrapping_add(at),
    }
}

pub(crate) fn enter(api: &mut dyn Api) {
    let from = api.word(WORDS.location);
    let (n, exit) = walk(&mut |n| kind(api, n), from);
    api.set_word(WORDS.location, n);
    let entry = match exit {
        None => 0,
        // 0x00529–0x0053C.
        Some(()) => {
            let here = api.word(WORDS.here);
            u16::from(api.far_byte(row_byte(here, FIRST_EXIT)))
        }
    };
    api.set_word(WORDS.entry, entry);
}

pub(crate) fn frame(_api: &mut dyn Api) -> bool {
    false
}

pub(crate) fn leave(_api: &mut dyn Api) {}

pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

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
