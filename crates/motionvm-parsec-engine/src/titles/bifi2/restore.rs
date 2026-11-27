//! Location 91, the restore: not a place but the way back into a loaded
//! game. The location table gives this row (kind 6) no name, so the module
//! has an English one.
//!
//! The load sends the main loop here (`mov word [200Ch], 5Bh`, `0x0F376`,
//! with `[200Ah] := 1`, `0x0F37C`) once a game is held in the buffer at
//! `2374h:0000h`. The handler at `0EE6h:0523h` (image `0x0F383` to
//! `0x0F432`) is the generation's way back ([`parsec1994::restore`]) in the
//! same order — the stack and the player's record (`0x0F39B`, `0x0F3B3`),
//! the location popped (`0x0F3CB`–`0x0F3E2`), then the story bytes, the
//! object rows and `3Ah` bytes of the last block (`0x0F3E8`–`0x0F42E`) —
//! here with this build's save map, its buffer and its main loop's words.

use super::build::{self, SAVE_BUFFER};
use super::logic::{Logic, WORDS};
use crate::titles::parsec1994::{self, Api};

pub(crate) fn enter(api: &mut dyn Api) {
    parsec1994::restore::enter(api, &build::SAVE, SAVE_BUFFER, WORDS);
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
