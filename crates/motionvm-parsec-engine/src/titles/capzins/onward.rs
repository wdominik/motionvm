//! Location 0, "Extro": a row of kind 0 that is not a place, only a step
//! on to the next row that is one.
//!
//! The row names `0060h:0500h` (image `0x00B00`), and the main loop does
//! not even reach for it through the table: for a row of kind 0, and for a
//! word past the table, it calls `0x00B00` directly (`0x0074B`–`0x00768`).
//! The routine walks the table from the row in `[1A5Ch]` past every row of
//! kind 0 or 3 (`0x00B15`, `0x00B19`) and puts the first other row in
//! `[1A5Ch]` (`0x00B4C`) with entry `7Fh` in `[1A5Ah]` (`0x00B50`): the
//! generation's walk on ([`parsec1994::onward`]), here with this build's
//! table and words. From row 0 that is the Wohnung, row 1, which is how a
//! fresh game begins.

use super::build;
use super::logic::{Logic, WORDS};
use crate::titles::parsec1994::{self, Api, onward::Table};

/// The table the routine walks: `mov ax, 1EBBh` (`0x00B0B`), `imul bx, bx,
/// 2Ch` (`0x00B08`), the kind at `+7` (`0x00B10`), 14h rows (`cmp dx,
/// 14h`, `0x00B1F`).
pub(super) const TABLE: Table = Table {
    seg: 0x1EBB,
    stride: 0x2C,
    kind: 7,
    rows: 0x14,
};

pub(crate) fn enter(api: &mut dyn Api) {
    parsec1994::onward::enter(api, TABLE, &build::KINDS, WORDS);
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
