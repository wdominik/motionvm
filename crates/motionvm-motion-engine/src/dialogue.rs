//! The two cells the dialogue machine keeps between calls.
//!
//! `CALCDIALOG` is re-entered with a mode on every step of a conversation and
//! carries almost nothing across, because the conversation's own state lives
//! in the game's tables. These two are the exception: the original keeps them
//! in globals of its own, and a step reads what the step before it left.

/// What one step of a conversation leaves for the next.
#[derive(Debug, Default)]
pub(crate) struct Dialogue {
    /// `0xdbd64`: an offset queued for the next conversation step, spent when
    /// it is taken (0x7b644).
    pub(crate) offset: i32,

    /// `0xdbd60`: the answer node the player was last on, so that the node 4000
    /// can send the conversation back to it (0x7b675, 0x7b6a9).
    pub(crate) return_node: i32,
}
