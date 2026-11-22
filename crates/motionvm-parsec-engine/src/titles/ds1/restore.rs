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
//! Re-expressed from the handler at `147A:052C` (image `0x14CCC`) against
//! [`Api`] alone. The handler has no hook, no rules and no frame loop of
//! its own: it copies the loaded blocks back, pops the location it was
//! saved in and returns, and the main loop goes on there.

use super::logic::Logic;
use crate::titles::parsec1994::{Api, Far};

/// `[1744h]`: a loaded game is waiting in the buffer.
const LOADED: u16 = 0x1744;
/// `DS:1F4Eh`: the location stack — a count word, then the locations.
const STACK: u16 = 0x1F4E;
/// `[1F48h]`, `[1F4Ah]`, `[1F4Ch]`: see the Zimmer.
const ENTRY: u16 = 0x1F48;
const LOCATION: u16 = 0x1F4A;
const HERE: u16 = 0x1F4C;
/// The far buffer the save fills and this handler reads back,
/// `2163h:0000h` (`0x14CDA`: `mov [bp-2], 2163h`, `mov [bp-4], 0`). It
/// is a segment of its own below `DGROUP`, not an offset in it.
const BUFFER: u16 = 0x2163;

/// The handler, `0x14CCC` to `0x14D7B`, whole: there is no frame loop.
pub(crate) fn enter(api: &mut dyn Api) {
    // 0x14CD0: nothing loaded, nothing to do.
    if api.word(LOADED) == 0 {
        return; // 0x14CD7
    }
    let buf = |off: u16| Far { off, seg: BUFFER };
    api.movmem(buf(0x07F9), api.ds(STACK), 8); // 0x14CF4
    api.movmem(buf(0x0801), api.ds(0x1693), 0x30); // 0x14D0C: the player's walker
    api.set_word(HERE, 0xFFFF); // 0x14D14
    // 0x14D1A: the location on top of the stack, popped.
    let top = api.word(STACK);
    let loc = api.word(STACK.wrapping_add(top << 1));
    api.set_word(LOCATION, loc); // 0x14D24
    api.set_word(STACK, top.wrapping_sub(1)); // 0x14D27
    api.set_word(ENTRY, 0xFFFF); // 0x14D2B
    api.movmem(buf(0x0014), api.ds(0x26F6), 0x45); // 0x14D41: story bytes
    api.movmem(buf(0x0059), api.ds(0x1F56), 0x07A0); // 0x14D5A: object states
    // 0x14D72: 23h bytes back, though the save copied 7Fh (0x14C8B).
    api.movmem(buf(0x0831), api.ds(0x15FE), 0x23);
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
