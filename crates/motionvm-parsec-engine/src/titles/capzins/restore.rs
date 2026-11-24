//! Location 19, the restore: not a place but the way back into a loaded
//! game. The location table gives this row (kind 3) no name, so the module
//! has an English one.
//!
//! The load sends the main loop here (`mov word [1A5Ch], 13h`, `0x10107`,
//! with `[1A5Ah] := 1`, `0x1010D`) once a game is held in the buffer at
//! `1F3Bh:0000h`. The handler at `0FBFh:0524h` (image `0x10114` to
//! `0x101C3`) is the generation's way back ([`parsec1994::restore`]) in the
//! same order — the stack and the player's record (`0x1013C`, `0x10154`),
//! the location popped (`0x1015C`–`0x10173`), then the story bytes, the
//! object rows and the carried list (`0x10189`–`0x101BA`) — here with this
//! build's save map, its buffer and its main loop's words.

use super::build;
use super::logic::{Logic, SAVE_BUFFER, WORDS};
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
