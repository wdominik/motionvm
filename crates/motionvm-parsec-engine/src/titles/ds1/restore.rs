//! Location 60, the restore: not a place but the way back into a loaded
//! game. The location table gives this row (kind 4) no name, so the module
//! has an English one.
//!
//! The load path asks "Tatsächlich neu einladen?" (`DS:1430h`) and, when
//! the answer is yes, calls `147A:0518`, which sends the main loop here
//! (`[1F4Ah]` = 3Ch, `[1F48h]` = 1) once a game is loaded (`[1744h]`).
//! The save at `147A:044E` is the mirror image: it pushes the location
//! onto the stack at `DS:1F4Eh` and copies the same blocks out to the
//! buffer at `2163:0000`.
//!
//! The handler at `147A:052C` (image `0x14CCC`) is the generation's way
//! back ([`parsec1994::restore`]), here with this build's save map, its
//! buffer and its main loop's words.

use super::build;
use super::logic::{Logic, SAVE_BUFFER, WORDS};
use crate::titles::parsec1994::{self, Api};

/// The handler, whole: there is no frame loop.
pub(crate) fn enter(api: &mut dyn Api) {
    parsec1994::restore::enter(api, &build::SAVE, SAVE_BUFFER, WORDS);
}

/// No frame loop: the handler has returned.
pub(crate) fn frame(_api: &mut dyn Api) -> bool {
    false
}

/// No frame loop, so nothing after one.
pub(crate) fn leave(_api: &mut dyn Api) {}

/// No hook of its own (`+2Fh` is null).
pub(crate) fn hook(_api: &mut dyn Api, _code: u16, _arg: u16, _task: u16) {}

/// No rules of its own (`+2Bh` is null).
pub(crate) fn rule(_api: &mut dyn Api, _off: u16, _flags: u16) -> Option<i16> {
    None
}

/// This location's entry points, as [`super::logic`] calls them.
pub(super) const LOGIC: Logic = Logic {
    enter,
    frame,
    leave,
    hook,
    anim_hook: None,
    rule,
};
