//! Locations 0 and 20: rows of kind 0 that are not places, only a step on
//! to the next row that is one. Row 0 carries the name "Extro", row 20 none.
//!
//! Both rows name the same handler, `04E3h:0582h` (image `0x053B2`), and
//! the main loop does not even reach for it through the table: for a row
//! of kind 0, and for a word past the table, it calls `0x053B2` directly
//! (`0x04F8E`–`0x04FAB`). The routine loads no room and has no frame loop,
//! no hook and no rules: it walks the
//! table from the row in `[1F4Ah]` past every row of kind 0 or 4, wrapping
//! to row 0 when it runs off the end, puts the first other row in
//! `[1F4Ah]` with entry `7Fh` in `[1F48h]`, and returns. From row 20 that
//! is the Schuppen, row 21, entered on `7Fh`.

use super::logic::Logic;
use crate::titles::parsec1994::{Api, Far};

/// `[1F48h]`, `[1F4Ah]`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
/// `204Bh:0000h`, the location table: rows of 34h bytes, the kind a word
/// at `+7` (`0x053BA`–`0x053C2`).
const TABLE: u16 = 0x204B;
const ROW: u16 = 0x34;
const KIND: u16 = 7;
/// `0x053D1`: the walk stops at row 3Dh, one past the last.
const ROWS: u16 = 0x3D;

/// Whether row `n` is one the walk steps over: kind 0 or 4 (`0x053C7`,
/// `0x053CB`).
fn skipped(api: &mut dyn Api, n: u16) -> bool {
    let kind = api.far_word(Far {
        off: n.wrapping_mul(ROW).wrapping_add(KIND),
        seg: TABLE,
    });
    kind == 0 || kind == 4
}

/// The handler, `0x053B2` to `0x05408`, whole.
pub(crate) fn enter(api: &mut dyn Api) {
    let from = api.word(LOCATION);
    // 0x053B2–0x053D4: onward from the row itself.
    let mut n = from;
    while crate::num::signed(n) < crate::num::signed(ROWS) && skipped(api, n) {
        n = n.wrapping_add(1);
    }
    // 0x053D6–0x053FC: past the end, from row 0 up to the row it started
    // at; a table of nothing but such rows leaves it there.
    if crate::num::signed(n) >= crate::num::signed(ROWS) {
        n = 0;
        while crate::num::signed(n) < crate::num::signed(from) && skipped(api, n) {
            n = n.wrapping_add(1);
        }
    }
    api.set_word(LOCATION, n); // 0x053FE
    api.set_word(ENTRY, 0x7F); // 0x05402
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
