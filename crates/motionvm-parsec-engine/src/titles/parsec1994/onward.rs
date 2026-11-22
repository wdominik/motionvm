//! The walk on past the rows that are not places.
//!
//! For a row of the passage kind, and for a word past the table, the main
//! loop does not dispatch through the table but calls this routine
//! directly; the passage rows also name it as their handler. It loads no
//! room and has no frame loop, no hook and no rules: it walks the table
//! from the row in the main loop's word past every passage and restore
//! row, wrapping to row 0 when it runs off the end, puts the first other
//! row in the word with the entry `7Fh`, and returns. Read from *Dunkle
//! Schatten*'s `04E3h:0582h` (image `0x053B2` to `0x05408`), against
//! [`Api`] alone; the addresses in the comments are that build's.

use super::Kinds;
use super::api::{Api, Far};
use super::restore::MainLoop;
use crate::num;

/// The location table as the walk reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Table {
    /// The segment of the records.
    pub seg: u16,
    /// Bytes per record.
    pub stride: u16,
    /// Where in a record the kind word is.
    pub kind: u16,
    /// How many rows there are: the walk stops one past the last.
    pub rows: u16,
}

/// The entry the walk hands the row it stops at (`0x05402`).
const ENTRY: u16 = 0x7F;

/// The routine, whole.
pub(crate) fn enter(api: &mut dyn Api, table: Table, kinds: &Kinds, words: MainLoop) {
    // 0x053BA–0x053CB: a row of the passage kind or the restore row's.
    let skipped = |api: &mut dyn Api, n: u16| {
        let kind = api.far_word(Far {
            off: n.wrapping_mul(table.stride).wrapping_add(table.kind),
            seg: table.seg,
        });
        kind == kinds.passage || kind == kinds.restore
    };
    let rows = num::signed(table.rows);
    let from = api.word(words.location);
    // 0x053B2–0x053D4: onward from the row itself.
    let mut n = from;
    while num::signed(n) < rows && skipped(api, n) {
        n = n.wrapping_add(1);
    }
    // 0x053D6–0x053FC: past the end, from row 0 up to the row it started
    // at; a table of nothing but such rows leaves it there.
    if num::signed(n) >= rows {
        n = 0;
        while num::signed(n) < num::signed(from) && skipped(api, n) {
            n = n.wrapping_add(1);
        }
    }
    api.set_word(words.location, n); // 0x053FE
    api.set_word(words.entry, ENTRY); // 0x05402
}
