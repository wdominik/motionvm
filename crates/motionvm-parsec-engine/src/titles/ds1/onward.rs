//! Locations 0 and 20: rows of kind 0 that are not places, only a step on
//! to the next row that is one. Row 0 carries the name "Extro", row 20 none.
//!
//! Both rows name the same handler, `04E3h:0582h` (image `0x053B2`), and
//! the main loop does not even reach for it through the table: for a row
//! of kind 0, and for a word past the table, it calls `0x053B2` directly
//! (`0x04F8E`–`0x04FAB`). The routine walks the table from the row in
//! `[1F4Ah]` past every row of kind 0 or 4 and puts the first other row in
//! `[1F4Ah]` with entry `7Fh` in `[1F48h]`. From row 20 that is the
//! Schuppen, row 21, entered on `7Fh`.

use super::build;
use super::logic::{Logic, WORDS};
use crate::titles::parsec1994::{self, Api, onward::Table};

/// `204Bh:0000h`, the location table: rows of 34h bytes, the kind a word
/// at `+7` (`0x053BA`–`0x053C2`), and the walk stops at row 3Dh, one past
/// the last (`0x053D1`). The same table the dispatch is read from; a test
/// holds the two together.
pub(super) const TABLE: Table = Table {
    seg: 0x204B,
    stride: 0x34,
    kind: 7,
    rows: 0x3D,
};

/// The handler, `0x053B2` to `0x05408`, whole: the generation's walk on
/// ([`parsec1994::onward`]).
pub(crate) fn enter(api: &mut dyn Api) {
    parsec1994::onward::enter(api, TABLE, &build::KINDS, WORDS);
}

/// No frame loop: the handler has returned.
pub(crate) fn frame(_api: &mut dyn Api) -> bool {
    false
}

/// No frame loop, so nothing after one.
pub(crate) fn leave(_api: &mut dyn Api) {}

/// No hook of its own.
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// No rules of its own.
pub(crate) fn rule(_api: &mut dyn Api, _off: u16, _flags: u16) -> Option<i16> {
    None
}

/// These rows' entry points, as [`super::logic`] calls them.
pub(super) const LOGIC: Logic = Logic {
    enter,
    frame,
    leave,
    hook,
    anim_hook: None,
    rule,
};
