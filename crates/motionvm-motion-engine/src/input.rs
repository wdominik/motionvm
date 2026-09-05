//! What the player is doing, as the engine's own words read it.
//!
//! The pointer and the keyboard reach the game by two different routes, and
//! that is the original's doing rather than a choice here: the mouse record is
//! filled by kernel words the script calls (`MOUSEX`, `MOUSELK` and their
//! relatives, all reading one four-field record at `0x253d4`), while `?KEY`
//! went to the BIOS. So the record is modelled and the key buffer is this
//! rebuild's stand-in for a type-ahead queue nobody else owns.
//!
//! The poll budget is here too, because it is the one thing in this group that
//! is not the original's: a script may spin waiting for input, and a frame
//! whose input never changes would spin forever.

use crate::Mouse;

/// The frame's input, and what has been read out of it.
#[derive(Debug, Default)]
pub(crate) struct Input {
    /// Where the pointer is and which buttons are down.
    ///
    /// Every `MOUSE…` word fills the same four-field record at 0x253d4 and
    /// reads one field back out: x at +0, y at +4, the left button at +8 and
    /// the right at +12. So the mouse does reach the game through kernel
    /// words, not only through the `_MLK` and `_MRK` variables the location
    /// handler reads — `ICTRL` calls `MOUSELK` itself and stores the result.
    pub(crate) mouse: Mouse,

    /// The key waiting to be read, or 0 for none.
    ///
    /// The mouse reaches the game through module variables, because the native
    /// loop wrote them and no bytecode does. The keyboard does not: `?KEY` and
    /// its relatives are kernel words that asked the BIOS directly, so the
    /// state has to live here instead.
    pub(crate) key: i32,

    /// The keystrokes the game has not taken yet, oldest first — the BIOS
    /// type-ahead buffer `?KEY` reads through INT 16h, [`crate::keys::SLOTS`] deep,
    /// drained one keystroke per frame the way the original's loop drains it.
    pub(crate) buffer: std::collections::VecDeque<i32>,

    /// How often this frame's bytecode has asked for the pointer or a key.
    ///
    /// The original's input words read live hardware, so a script may wait in
    /// a loop of its own — `RUN`'s start-up page does (`BEGIN … MOUSELK …
    /// MOUSEX … UNTIL`), location 5's newspaper does — and the loop turns as
    /// the player moves. Here a frame's input is fixed for the frame, so such
    /// a loop would spin forever. Past [`Input::BUDGET`] polls in one
    /// frame the word is taken to be waiting, and from then on every poll
    /// yields the frame: the loop turns once per frame, with the frame's
    /// input. A frame of `CTRL` polls a handful of times and never gets
    /// near the budget.
    pub(crate) polls: u32,

    /// Set once a frame has spent its poll budget; every later poll in the
    /// same word yields. Cleared when the word has finished.
    pub(crate) polling: bool,
}

impl Input {
    /// Polls of the pointer or a key one frame may make before the word is
    /// taken to be waiting for input.
    ///
    /// The original's input words read live hardware, so a script may wait in
    /// a loop of its own and the loop turns as the player moves. Here a
    /// frame's input is fixed for the frame, so such a loop would never end.
    /// Past this many polls the word is taken to be waiting, and every later
    /// poll yields the frame: the loop then turns once per frame, with that
    /// frame's input. A frame of `CTRL` polls a handful of times and never
    /// gets near it.
    pub(crate) const BUDGET: u32 = 256;
}
